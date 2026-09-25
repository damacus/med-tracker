use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::Response;
use scraper::{Html, Selector};
use url::Url;

fn login(target: &Target, email: &str) {
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
    let response = target.post_html_form(
        "/login",
        &[
            ("email".to_string(), email.to_string()),
            ("password".to_string(), "password".to_string()),
            ("authenticity_token".to_string(), token.to_string()),
        ],
    );
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
    assert_eq!(response.status().as_u16(), 302);
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
    for response in [
        anonymous.patch_html_form("/platform/settings", &[]),
        anonymous.put_html_form("/platform/settings", &[]),
        anonymous.patch_html_form(&user_path, &[]),
        anonymous.put_html_form(&user_path, &[]),
        anonymous.post_html_form("/platform/support_access_sessions", &[]),
        anonymous.delete_html_form(
            &format!(
                "/platform/support_access_sessions/{}",
                fixture.platform_support_session_id
            ),
            &[],
        ),
        anonymous.patch_html_form(&promote_path, &[]),
    ] {
        redirect(response, "/login");
    }

    let ordinary = Target::from_env();
    login(&ordinary, &fixture.primary_email);
    for path in ["/platform/settings", "/platform/users"] {
        let response = ordinary.get_html(path);
        assert_eq!(response.status().as_u16(), 302);
        assert!(response.headers()["location"]
            .to_str()
            .unwrap()
            .contains("household_slug="));
    }
    for response in [
        ordinary.patch_html_form("/platform/settings", &[]),
        ordinary.patch_html_form(&user_path, &[]),
        ordinary.post_html_form("/platform/support_access_sessions", &[]),
        ordinary.delete_html_form(
            &format!(
                "/platform/support_access_sessions/{}",
                fixture.platform_support_session_id
            ),
            &[],
        ),
        ordinary.patch_html_form(&promote_path, &[]),
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
    let response = target.put_html_form(
        "/platform/settings",
        &fields(&token, &[("app_settings[invite_only]", "1")]),
    );
    redirect(response, "/platform/settings");
    let response = target.get_html("/platform/settings");
    let html = response.text().expect("settings HTML");
    let document = Html::parse_document(&html);
    let checkbox =
        Selector::parse("input[type='checkbox'][name='app_settings[invite_only]']").unwrap();
    assert!(document
        .select(&checkbox)
        .next()
        .expect("invite setting")
        .value()
        .attr("checked")
        .is_some());

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
    let unchanged = target
        .get_html("/platform/settings")
        .text()
        .expect("settings HTML");
    let document = Html::parse_document(&unchanged);
    assert!(document
        .select(&checkbox)
        .next()
        .expect("invite setting")
        .value()
        .attr("checked")
        .is_some());
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
        fixture.platform_support_household_id, fixture.platform_promote_membership_id
    );
    assert_eq!(
        target
            .patch_html_form(&mismatch, &fields(&token, &[]))
            .status()
            .as_u16(),
        404
    );
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
    redirect(
        target.post_html_form("/platform/support_access_sessions", &valid),
        "/platform/settings",
    );
    assert_eq!(target.get_html(&support_path).status().as_u16(), 200);
    let end_path = format!(
        "/platform/support_access_sessions/{}",
        fixture.platform_support_session_id
    );
    let existing_path = format!(
        "/households/{}/admin",
        fixture.platform_existing_support_household_slug
    );
    assert_eq!(target.get_html(&existing_path).status().as_u16(), 200);
    redirect(
        target.delete_html_form(&end_path, &fields(&token, &[])),
        "/platform/settings",
    );
    assert_eq!(target.get_html(&existing_path).status().as_u16(), 302);
    redirect(
        target.delete_html_form(&end_path, &fields(&token, &[])),
        "/platform/settings",
    );
}
