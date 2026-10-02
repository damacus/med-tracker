use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::Response;
use scraper::{Html, Selector};
use serde_json::Value;
use url::Url;

fn login(target: &Target, email: &str) {
    login_with_client_ip(target, email, None);
}

fn login_with_client_ip(target: &Target, email: &str, client_ip: Option<&str>) {
    let response = target.get_html("/login");
    assert_eq!(response.status().as_u16(), 200);
    let document = Html::parse_document(&response.text().expect("login HTML"));
    let selector =
        Selector::parse("form[action='/login'] input[name='authenticity_token']").unwrap();
    let token = document
        .select(&selector)
        .next()
        .and_then(|input| input.value().attr("value"))
        .expect("login CSRF token");
    let fields = [
        ("email".to_string(), email.to_string()),
        ("password".to_string(), "password".to_string()),
        ("authenticity_token".to_string(), token.to_string()),
    ];
    let response = match client_ip {
        Some(client_ip) => target.post_html_form_from_local_client("/login", &fields, client_ip),
        None => target.post_html_form("/login", &fields),
    };
    assert_eq!(response.status().as_u16(), 302);
}

fn csrf(target: &Target, path: &str) -> String {
    let response = target.get_html(path);
    assert_eq!(response.status().as_u16(), 200);
    let document = Html::parse_document(&response.text().expect("HTML"));
    let selector =
        Selector::parse("input[name='authenticity_token'], meta[name='csrf-token']").unwrap();
    document
        .select(&selector)
        .next()
        .and_then(|meta| {
            meta.value()
                .attr("value")
                .or_else(|| meta.value().attr("content"))
        })
        .expect("CSRF token")
        .to_string()
}

fn fields(token: &str, values: &[(&str, &str)]) -> Vec<(String, String)> {
    let mut form = vec![("authenticity_token".to_string(), token.to_string())];
    form.extend(
        values
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string())),
    );
    form
}

fn redirect(response: Response, location: &str) {
    redirect_with_status(response, 302, location);
}

fn redirect_with_status(response: Response, status: u16, location: &str) {
    assert_eq!(response.status().as_u16(), status);
    let actual = response.headers()["location"]
        .to_str()
        .expect("redirect location");
    let base = Url::parse(&std::env::var("CONTRACT_BASE_URL").expect("target origin"))
        .expect("target URL");
    let url = base.join(actual).expect("redirect URL");
    assert_eq!(url.origin(), base.origin());
    assert_eq!(url.path(), location);
}

fn user_row(target: &Target, email: &str) -> String {
    let response = target.get_html("/platform/users");
    assert_eq!(response.status().as_u16(), 200);
    let document = Html::parse_document(&response.text().expect("users HTML"));
    let selector = Selector::parse("tr").unwrap();
    document
        .select(&selector)
        .find(|row| row.text().any(|text| text.contains(email)))
        .expect("target user row")
        .text()
        .collect::<String>()
}

fn invite_only_checked(target: &Target) -> bool {
    let response = target.get_html("/platform/settings");
    assert_eq!(response.status().as_u16(), 200);
    let document = Html::parse_document(&response.text().expect("settings HTML"));
    let checkbox =
        Selector::parse("input[type='checkbox'][name='app_settings[invite_only]']").unwrap();
    document
        .select(&checkbox)
        .next()
        .expect("invite setting")
        .value()
        .attr("checked")
        .is_some()
}

fn support_audit_events(fixture: &Fixture) -> Vec<Value> {
    let target = Target::from_env();
    let path = format!(
        "/api/v1/households/{}/admin/audit_logs",
        fixture.platform_support_household_id
    );
    let response = target.get(&path, Some(&fixture.platform_support_audit_token));
    assert_eq!(response.status().as_u16(), 200);
    response.json::<Value>().expect("audit JSON")["data"]
        .as_array()
        .expect("audit events")
        .clone()
}

#[test]
fn platform_routes_require_a_web_session_and_platform_role() {
    let fixture = fixture();
    let anonymous = Target::from_env();
    let user_path = format!("/platform/users/{}", fixture.platform_denied_user_id);
    let promote_path = format!(
        "/platform/households/{}/memberships/{}/promote_owner",
        fixture.platform_household_id, fixture.platform_denied_membership_id
    );
    for path in ["/platform/settings", "/platform/users"] {
        redirect(anonymous.get_html(path), "/login");
    }
    let anonymous_token = csrf(&anonymous, "/login");
    let anonymous_fields = fields(&anonymous_token, &[]);
    for response in [
        anonymous.patch_html_form("/platform/settings", &anonymous_fields),
        anonymous.put_html_form("/platform/settings", &anonymous_fields),
        anonymous.patch_html_form(&user_path, &anonymous_fields),
        anonymous.put_html_form(&user_path, &anonymous_fields),
        anonymous.post_html_form("/platform/support_access_sessions", &anonymous_fields),
        anonymous.delete_html_form("/platform/support_access_sessions/0", &anonymous_fields),
        anonymous.patch_html_form(&promote_path, &anonymous_fields),
    ] {
        redirect(response, "/login");
    }

    let ordinary = Target::from_env();
    login(&ordinary, &fixture.primary_email);
    let ordinary_token = csrf(
        &ordinary,
        &format!("/households/{}/profile", fixture.household_slug),
    );
    let ordinary_fields = fields(&ordinary_token, &[]);
    for path in ["/platform/settings", "/platform/users"] {
        let response = ordinary.get_html(path);
        assert_eq!(response.status().as_u16(), 302);
        assert!(response.headers()["location"]
            .to_str()
            .unwrap()
            .contains("household_slug="));
    }
    for response in [
        ordinary.patch_html_form("/platform/settings", &ordinary_fields),
        ordinary.patch_html_form(&user_path, &ordinary_fields),
        ordinary.post_html_form("/platform/support_access_sessions", &ordinary_fields),
        ordinary.delete_html_form("/platform/support_access_sessions/0", &ordinary_fields),
        ordinary.patch_html_form(&promote_path, &ordinary_fields),
    ] {
        redirect(response, "/");
    }
    let admin = Target::from_env();
    login(&admin, &fixture.platform_admin_email);
    let row = user_row(&admin, &fixture.platform_denied_email);
    assert!(row.contains("Household User"));
    assert!(row.contains("Member"));
}

#[test]
fn platform_settings_accept_updates_and_reject_invalid_lookup_url() {
    let fixture = fixture();
    let target = Target::from_env();
    login(&target, &fixture.platform_admin_email);
    let token = csrf(&target, "/platform/settings");
    let response = target.patch_html_form(
        "/platform/settings",
        &fields(&token, &[("app_settings[invite_only]", "0")]),
    );
    redirect(response, "/platform/settings");
    assert!(!invite_only_checked(&target));
    let response = target.put_html_form(
        "/platform/settings",
        &fields(&token, &[("app_settings[invite_only]", "1")]),
    );
    redirect(response, "/platform/settings");
    assert!(invite_only_checked(&target));

    let invalid = target.patch_html_form(
        "/platform/settings",
        &fields(
            &token,
            &[(
                "app_settings[medicine_lookup_base_url]",
                "http://insecure.example.test",
            )],
        ),
    );
    assert_eq!(invalid.status().as_u16(), 422);
    assert!(invalid
        .text()
        .expect("validation HTML")
        .contains("Platform Settings"));
    assert!(invite_only_checked(&target));

    let missing = target.patch_html_form(
        "/platform/settings",
        &[("app_settings[invite_only]".to_string(), "0".to_string())],
    );
    redirect_with_status(missing, 303, "/login");
    let observer = Target::from_env();
    login_with_client_ip(
        &observer,
        &fixture.platform_admin_email,
        Some("198.51.100.40"),
    );
    assert!(invite_only_checked(&observer));
}

#[test]
fn platform_user_access_and_owner_promotion_have_public_read_back() {
    let fixture: Fixture = fixture();
    let target = Target::from_env();
    login(&target, &fixture.platform_admin_email);
    let token = csrf(&target, "/platform/users");
    let user_path = format!("/platform/users/{}", fixture.platform_target_user_id);
    assert!(user_row(&target, &fixture.platform_target_email).contains("Household User"));
    redirect(
        target.patch_html_form(
            &user_path,
            &fields(&token, &[("platform_user[system_administrator]", "1")]),
        ),
        "/platform/users",
    );
    assert!(user_row(&target, &fixture.platform_target_email).contains("System Administrator"));
    redirect(
        target.put_html_form(
            &user_path,
            &fields(&token, &[("platform_user[system_administrator]", "0")]),
        ),
        "/platform/users",
    );
    assert!(user_row(&target, &fixture.platform_target_email).contains("Household User"));

    let promote_path = format!(
        "/platform/households/{}/memberships/{}/promote_owner",
        fixture.platform_household_id, fixture.platform_promote_membership_id
    );
    assert!(user_row(&target, &fixture.platform_promote_email).contains("Member"));
    redirect(
        target.patch_html_form(&promote_path, &fields(&token, &[])),
        "/platform/users",
    );
    assert!(user_row(&target, &fixture.platform_promote_email).contains("Owner"));
    let mismatch = format!(
        "/platform/households/{}/memberships/{}/promote_owner",
        fixture.platform_support_household_id, fixture.platform_denied_membership_id
    );
    assert!(user_row(&target, &fixture.platform_denied_email).contains("Member"));
    assert_eq!(
        target
            .patch_html_form(&mismatch, &fields(&token, &[]))
            .status()
            .as_u16(),
        404
    );
    assert!(user_row(&target, &fixture.platform_denied_email).contains("Member"));
}

#[test]
fn support_access_requires_reason_grants_one_household_and_can_end() {
    let fixture = fixture();
    let target = Target::from_env();
    login(&target, &fixture.platform_admin_email);
    let token = csrf(&target, "/platform/settings");
    let support_path = format!(
        "/households/{}/admin",
        fixture.platform_support_household_slug
    );
    assert_eq!(target.get_html(&support_path).status().as_u16(), 302);
    let invalid = fields(
        &token,
        &[
            (
                "support_access_session[household_id]",
                fixture.platform_support_household_id.to_string().as_str(),
            ),
            ("support_access_session[reason]", ""),
        ],
    );
    redirect(
        target.post_html_form("/platform/support_access_sessions", &invalid),
        "/platform/settings",
    );
    assert_eq!(target.get_html(&support_path).status().as_u16(), 302);
    assert!(!support_audit_events(&fixture)
        .iter()
        .any(|event| event["event_type"] == "support_access_session.started"));
    let valid = fields(
        &token,
        &[
            (
                "support_access_session[household_id]",
                fixture.platform_support_household_id.to_string().as_str(),
            ),
            (
                "support_access_session[reason]",
                "Contract support investigation",
            ),
        ],
    );
    let created = target.post_html_form("/platform/support_access_sessions", &valid);
    let request_id = created.headers()["x-request-id"]
        .to_str()
        .expect("support request ID")
        .to_string();
    redirect(created, "/platform/settings");
    let events = support_audit_events(&fixture);
    let started = events
        .iter()
        .find(|event| {
            event["event_type"] == "support_access_session.started"
                && event["request_id"].as_str() == Some(request_id.as_str())
        })
        .expect("started support audit event");
    let session_id = started["metadata"]["support_access_session_id"]
        .as_i64()
        .expect("created support session ID");
    assert_eq!(target.get_html(&support_path).status().as_u16(), 200);
    let unrelated_path = format!(
        "/households/{}/admin",
        fixture.platform_unrelated_household_slug
    );
    assert_eq!(target.get_html(&unrelated_path).status().as_u16(), 302);
    let end_path = format!("/platform/support_access_sessions/{session_id}");
    redirect(
        target.delete_html_form(&end_path, &fields(&token, &[])),
        "/platform/settings",
    );
    assert_eq!(target.get_html(&support_path).status().as_u16(), 302);
    let ended = support_audit_events(&fixture)
        .iter()
        .filter(|event| {
            event["event_type"] == "support_access_session.ended"
                && event["metadata"]["support_access_session_id"] == session_id
        })
        .count();
    assert_eq!(ended, 1);
    redirect(
        target.delete_html_form(&end_path, &fields(&token, &[])),
        "/platform/settings",
    );
    let ended_again = support_audit_events(&fixture)
        .iter()
        .filter(|event| {
            event["event_type"] == "support_access_session.ended"
                && event["metadata"]["support_access_session_id"] == session_id
        })
        .count();
    assert_eq!(ended_again, 1);
    late_support_close_records_natural_expiry_without_an_end_event(&target, &fixture, &token);
    held_household_rejects_support_access(&target, &fixture, &token);
}

fn held_household_rejects_support_access(target: &Target, fixture: &Fixture, token: &str) {
    let held_household_id = fixture.auth_operational_states["held"]["household_id"]
        .as_i64()
        .expect("held household ID");
    let attempt = fields(
        token,
        &[
            (
                "support_access_session[household_id]",
                held_household_id.to_string().as_str(),
            ),
            (
                "support_access_session[reason]",
                "Contract held household denial",
            ),
        ],
    );
    redirect(
        target.post_html_form("/platform/support_access_sessions", &attempt),
        "/",
    );
    let root = target.get_html("/");
    assert_eq!(root.status().as_u16(), 303);
    let landing = root.headers()["location"]
        .to_str()
        .expect("landing redirect");
    let landing = Url::parse(landing).expect("absolute landing URL");
    let base = Url::parse(&std::env::var("CONTRACT_BASE_URL").expect("target origin")).unwrap();
    assert_eq!(landing.origin(), base.origin());
    let page = target.get_html(landing.path());
    assert_eq!(page.status().as_u16(), 200);
    assert!(page
        .text()
        .expect("landing HTML")
        .contains("You are not authorized to perform this action."));
}

fn late_support_close_records_natural_expiry_without_an_end_event(
    target: &Target,
    fixture: &Fixture,
    token: &str,
) {
    let support_path = format!(
        "/households/{}/admin",
        fixture.platform_support_household_slug
    );
    assert_eq!(target.get_html(&support_path).status().as_u16(), 302);
    let end_path = format!(
        "/platform/support_access_sessions/{}",
        fixture.platform_expired_support_session_id
    );
    let matching = |event: &&Value, kind: &str| {
        event["event_type"] == kind
            && event["metadata"]["support_access_session_id"]
                == fixture.platform_expired_support_session_id
    };
    let before = support_audit_events(fixture);
    assert_eq!(
        before
            .iter()
            .filter(|event| matching(event, "support_access_session.expired"))
            .count(),
        0
    );
    assert_eq!(
        before
            .iter()
            .filter(|event| matching(event, "support_access_session.ended"))
            .count(),
        0
    );
    redirect(
        target.delete_html_form(&end_path, &fields(token, &[])),
        "/platform/settings",
    );
    assert_eq!(target.get_html(&support_path).status().as_u16(), 302);
    let events = support_audit_events(fixture);
    let expired: Vec<&Value> = events
        .iter()
        .filter(|event| matching(event, "support_access_session.expired"))
        .collect();
    assert_eq!(expired.len(), 1);
    assert_eq!(
        expired[0]["actor_account_id"],
        fixture.platform_admin_account_id
    );
    assert_eq!(expired[0]["metadata"]["outcome"], "expired");
    assert!(expired[0]["metadata"]["expired_at"].as_str().is_some());
    assert_eq!(
        events
            .iter()
            .filter(|event| matching(event, "support_access_session.ended"))
            .count(),
        0
    );
    redirect(
        target.delete_html_form(&end_path, &fields(token, &[])),
        "/platform/settings",
    );
    let events = support_audit_events(fixture);
    assert_eq!(
        events
            .iter()
            .filter(|event| matching(event, "support_access_session.expired"))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matching(event, "support_access_session.ended"))
            .count(),
        0
    );
}
