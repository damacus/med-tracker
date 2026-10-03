use medtracker_contract_tests::{fixture, Target};
use scraper::{Html, Selector};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};

static LOGIN_CLIENT: AtomicUsize = AtomicUsize::new(40);

fn csrf(html: &str) -> String {
    let document = Html::parse_document(html);
    for (selector, attribute) in [
        ("input[name='authenticity_token']", "value"),
        ("meta[name='csrf-token']", "content"),
    ] {
        let selector = Selector::parse(selector).expect("CSRF selector");
        if let Some(value) = document
            .select(&selector)
            .next()
            .and_then(|input| input.value().attr(attribute))
        {
            return value.to_owned();
        }
    }
    panic!("native household form must render a CSRF token")
}

fn sign_in_as(target: &Target, email: &str, slug: &str) -> String {
    let login = target.get_html("/login");
    assert_eq!(login.status().as_u16(), 200);
    let token = csrf(&login.text().expect("login HTML"));
    let client_ip = format!("198.18.28.{}", LOGIN_CLIENT.fetch_add(1, Ordering::Relaxed));
    let response = target.post_html_form_from_client(
        "/login",
        &client_ip,
        &[
            ("email".into(), email.into()),
            ("password".into(), "password".into()),
            ("authenticity_token".into(), token),
        ],
    );
    assert_eq!(response.status().as_u16(), 302);
    let inventory = target.get_html(&format!("/households/{slug}/medications"));
    assert_eq!(inventory.status().as_u16(), 200);
    csrf(&inventory.text().expect("inventory HTML"))
}

fn sign_in(target: &Target) -> String {
    let fixture = fixture();
    sign_in_as(target, &fixture.primary_email, &fixture.household_slug)
}

fn checked(document: &Html, name: &str) -> Option<bool> {
    let selector = Selector::parse(&format!("input[name='{name}']")).expect("switch selector");
    document
        .select(&selector)
        .next()
        .map(|input| input.value().attr("checked").is_some())
}

fn api_path(household_id: i64) -> String {
    format!("/api/v1/households/{household_id}/notification_preference")
}

fn page_path(slug: &str) -> String {
    format!("/households/{slug}/settings/notifications")
}

#[test]
fn notifications_page_offers_defaults_and_creates_absent_preferences() {
    let fixture = fixture();
    let target = Target::from_env();
    let api = target.get(
        &api_path(fixture.web_device_household_id),
        Some(&fixture.web_device_access_token),
    );
    assert_eq!(
        api.status().as_u16(),
        404,
        "fixture account must not own a preference row"
    );
    let body: Value = api.json().expect("absent preference response");
    assert_eq!(body["error"]["code"], "not_configured");
    let token = sign_in_as(
        &target,
        &fixture.web_device_email,
        &fixture.web_device_household_slug,
    );
    let response = target.get_html(&page_path(&fixture.web_device_household_slug));
    assert_eq!(response.status().as_u16(), 200);
    assert!(response.headers()["cache-control"]
        .to_str()
        .unwrap()
        .contains("no-store"));
    let html = response.text().expect("first-use page HTML");
    let document = Html::parse_document(&html);
    assert_eq!(
        document
            .select(&Selector::parse("form[method='post']").unwrap())
            .count(),
        1,
        "permitted first use must render an editable form"
    );
    for (name, expected) in [
        ("enabled", true),
        ("dose_due_enabled", true),
        ("missed_dose_enabled", true),
        ("low_stock_enabled", true),
        ("private_text_enabled", false),
    ] {
        assert_eq!(checked(&document, name), Some(expected), "{name} default");
    }
    let saved = target.post_browser_form(
        &page_path(&fixture.web_device_household_slug),
        &[
            ("authenticity_token".into(), token),
            ("enabled".into(), "true".into()),
            ("dose_due_enabled".into(), "true".into()),
            ("missed_dose_enabled".into(), "true".into()),
            ("low_stock_enabled".into(), "true".into()),
        ],
    );
    assert!([302, 303].contains(&saved.status().as_u16()));
    let created: Value = target
        .get(
            &api_path(fixture.web_device_household_id),
            Some(&fixture.web_device_access_token),
        )
        .json()
        .expect("created preference");
    assert_eq!(created["data"]["enabled"], true);
    assert_eq!(created["data"]["private_text_enabled"], false);
    let mut db = postgres::Client::connect(
        &std::env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract database URL"),
        postgres::NoTls,
    )
    .expect("contract database");
    db.execute(
        "DELETE FROM notification_preferences WHERE household_id = $1 AND id = $2",
        &[&fixture.web_device_household_id, &created["data"]["id"].as_i64().unwrap()],
    )
    .expect("reset first-use preference for browser acceptance");
}

#[test]
fn notification_switches_save_five_booleans_and_preserve_reminder_times() {
    let fixture = fixture();
    let target = Target::from_env();
    let api = api_path(fixture.household_id);
    let created = target.put_json(
        &api,
        &fixture.access_token,
        &json!({"notification_preference": {
            "enabled": true,
            "dose_due_enabled": true,
            "missed_dose_enabled": false,
            "low_stock_enabled": true,
            "private_text_enabled": false,
            "morning_time": "07:05",
            "afternoon_time": "13:30",
            "evening_time": "19:45",
            "night_time": "23:15"
        }}),
    );
    assert_eq!(created.status().as_u16(), 200);
    let token = sign_in(&target);
    let page = target.get_html(&page_path(&fixture.household_slug));
    assert_eq!(page.status().as_u16(), 200);
    let document = Html::parse_document(&page.text().expect("form HTML"));
    for (name, expected) in [
        ("enabled", true),
        ("dose_due_enabled", true),
        ("missed_dose_enabled", false),
        ("low_stock_enabled", true),
        ("private_text_enabled", false),
    ] {
        assert_eq!(checked(&document, name), Some(expected), "{name} initial");
    }
    let saved = target.post_browser_form(
        &page_path(&fixture.household_slug),
        &[
            ("authenticity_token".into(), token),
            ("missed_dose_enabled".into(), "true".into()),
            ("private_text_enabled".into(), "true".into()),
        ],
    );
    let status = saved.status().as_u16();
    assert!(
        [302, 303].contains(&status),
        "notification save must redirect, got {status}"
    );
    let after: Value = target
        .get(&api, Some(&fixture.access_token))
        .json()
        .expect("preference read-back");
    for (field, expected) in [
        ("enabled", false),
        ("dose_due_enabled", false),
        ("missed_dose_enabled", true),
        ("low_stock_enabled", false),
        ("private_text_enabled", true),
    ] {
        assert_eq!(after["data"][field], expected, "{field} persisted");
    }
    for (field, expected) in [
        ("morning_time", "07:05:00"),
        ("afternoon_time", "13:30:00"),
        ("evening_time", "19:45:00"),
        ("night_time", "23:15:00"),
    ] {
        assert_eq!(after["data"][field], expected, "{field} preserved");
    }
    let reloaded = target.get_html(&page_path(&fixture.household_slug));
    assert_eq!(reloaded.status().as_u16(), 200);
    let document = Html::parse_document(&reloaded.text().expect("reloaded HTML"));
    for (name, expected) in [
        ("enabled", false),
        ("dose_due_enabled", false),
        ("missed_dose_enabled", true),
        ("low_stock_enabled", false),
        ("private_text_enabled", true),
    ] {
        assert_eq!(checked(&document, name), Some(expected), "{name} reloaded");
    }
}

#[test]
fn notification_save_rejects_invalid_csrf_without_writing() {
    let fixture = fixture();
    let target = Target::from_env();
    let api = api_path(fixture.household_id);
    let created = target.put_json(
        &api,
        &fixture.access_token,
        &json!({"notification_preference": {"enabled": true, "private_text_enabled": true}}),
    );
    assert_eq!(created.status().as_u16(), 200);
    let token = sign_in(&target);
    let denied = target.post_browser_form(
        &page_path(&fixture.household_slug),
        &[
            ("authenticity_token".into(), "wrong-token".into()),
            ("enabled".into(), "true".into()),
        ],
    );
    assert_eq!(denied.status().as_u16(), 403);
    let malformed = target.post_browser_form(
        &page_path(&fixture.household_slug),
        &[
            ("authenticity_token".into(), token),
            ("enabled".into(), "maybe".into()),
        ],
    );
    assert_eq!(malformed.status().as_u16(), 400);
    let after: Value = target
        .get(&api, Some(&fixture.access_token))
        .json()
        .expect("preference after CSRF denial");
    assert_eq!(after["data"]["enabled"], true);
    assert_eq!(after["data"]["private_text_enabled"], true);
}

#[test]
fn notification_switches_save_is_a_noop_when_values_are_unchanged() {
    let fixture = fixture();
    let target = Target::from_env();
    let api = api_path(fixture.household_id);
    let created = target.put_json(
        &api,
        &fixture.access_token,
        &json!({"notification_preference": {
            "enabled": true,
            "dose_due_enabled": true,
            "missed_dose_enabled": false,
            "low_stock_enabled": false,
            "private_text_enabled": true
        }}),
    );
    assert_eq!(created.status().as_u16(), 200);
    let before: Value = created.json().expect("seeded preference");
    let token = sign_in(&target);
    let saved = target.post_browser_form(
        &page_path(&fixture.household_slug),
        &[
            ("authenticity_token".into(), token),
            ("enabled".into(), "true".into()),
            ("dose_due_enabled".into(), "true".into()),
            ("private_text_enabled".into(), "true".into()),
        ],
    );
    let status = saved.status().as_u16();
    assert!(
        [302, 303].contains(&status),
        "unchanged save must still redirect, got {status}"
    );
    let after: Value = target
        .get(&api, Some(&fixture.access_token))
        .json()
        .expect("preference after unchanged save");
    assert_eq!(after["data"], before["data"]);
}

struct OwnerViewGuard {
    db: postgres::Client,
    membership_id: i64,
    original_role: String,
    grant: Option<(i64, String, Option<String>)>,
    grant_id: i64,
    restored: bool,
}

impl OwnerViewGuard {
    fn new(fixture: &medtracker_contract_tests::Fixture) -> Self {
        let mut db = postgres::Client::connect(
            &std::env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract database URL"),
            postgres::NoTls,
        )
        .expect("contract database");
        let original_role: String = db
            .query_one(
                "SELECT role FROM household_memberships WHERE id = $1 AND household_id = $2",
                &[&fixture.owner_membership_id, &fixture.household_id],
            )
            .expect("owner membership role")
            .get(0);
        assert_eq!(original_role, "owner");
        let grant: Option<(i64, String, Option<String>)> = db
            .query_opt(
                "SELECT id, access_level, revoked_at::text FROM person_access_grants WHERE household_id = $1 AND household_membership_id = $2 AND person_id = $3 ORDER BY id LIMIT 1",
                &[&fixture.household_id, &fixture.owner_membership_id, &fixture.user_person_id],
            )
            .expect("owner self grant")
            .map(|row| {
                (
                    row.get::<_, i64>(0),
                    row.get::<_, String>(1),
                    row.get::<_, Option<String>>(2),
                )
            });
        db.execute(
            "UPDATE household_memberships SET role = 'member' WHERE id = $1",
            &[&fixture.owner_membership_id],
        )
        .expect("temporarily restrict owner role");
        let grant_id = if let Some((id, _, _)) = &grant {
            db.execute(
                "UPDATE person_access_grants SET access_level = 'view', revoked_at = now() WHERE id = $1",
                &[id],
            )
            .expect("temporarily revoke own view grant");
            *id
        } else {
            db.query_one(
                "INSERT INTO person_access_grants (household_id, household_membership_id, person_id, granted_by_membership_id, access_level, relationship_type, revoked_at, created_at, updated_at) VALUES ($1, $2, $3, $2, 'view', 'self', now(), now(), now()) RETURNING id",
                &[&fixture.household_id, &fixture.owner_membership_id, &fixture.user_person_id],
            )
            .expect("temporary self view grant")
            .get(0)
        };
        Self {
            db,
            membership_id: fixture.owner_membership_id,
            original_role,
            grant,
            grant_id,
            restored: false,
        }
    }

    fn expose_view(&mut self) {
        self.db
            .execute(
                "UPDATE person_access_grants SET access_level = 'view', revoked_at = NULL WHERE id = $1",
                &[&self.grant_id],
            )
            .expect("expose self view grant");
    }

    fn restore(&mut self) {
        if self.restored {
            return;
        }
        if let Some((id, access_level, revoked_at)) = &self.grant {
            self.db
                .execute(
                    "UPDATE person_access_grants SET access_level = $2, revoked_at = $3::text::timestamptz WHERE id = $1",
                    &[id, access_level, revoked_at],
                )
                .expect("restore original self grant");
        } else {
            self.db
                .execute(
                    "DELETE FROM person_access_grants WHERE id = $1",
                    &[&self.grant_id],
                )
                .expect("remove temporary self grant");
        }
        self.db
            .execute(
                "UPDATE household_memberships SET role = $2 WHERE id = $1",
                &[&self.membership_id, &self.original_role],
            )
            .expect("restore original membership role");
        self.restored = true;
    }
}

impl Drop for OwnerViewGuard {
    fn drop(&mut self) {
        self.restore();
    }
}

#[test]
fn notifications_page_masks_revoked_self_access() {
    let fixture = fixture();
    let target = Target::from_env();
    sign_in(&target);
    let mut viewer = OwnerViewGuard::new(&fixture);
    let response = target.get(&api_path(fixture.household_id), Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 404);
    let body: Value = response.json().expect("masked API response");
    assert_eq!(body["error"]["code"], "not_found");
    let dashboard = target.get_html(&format!("/households/{}/dashboard", fixture.household_slug));
    assert_eq!(dashboard.status().as_u16(), 200);
    assert!(!dashboard.text().expect("masked dashboard").contains("settings/notifications"));
    let page = target.get_html(&page_path(&fixture.household_slug));
    assert_eq!(page.status().as_u16(), 200);
    let document = Html::parse_document(&page.text().expect("masked page"));
    assert_eq!(document.select(&Selector::parse("form[method='post']").unwrap()).count(), 0);
    assert_eq!(document.select(&Selector::parse("[role='status']").unwrap()).count(), 1);
    viewer.restore();
}

#[test]
fn denied_notification_save_retains_attempted_values_without_success() {
    let fixture = fixture();
    let target = Target::from_env();
    let api = api_path(fixture.household_id);
    let created = target.put_json(
        &api,
        &fixture.access_token,
        &json!({"notification_preference": {
            "enabled": true,
            "dose_due_enabled": false,
            "missed_dose_enabled": true,
            "low_stock_enabled": false,
            "private_text_enabled": false
        }}),
    );
    assert_eq!(created.status().as_u16(), 200);
    let stored: Value = created.json().expect("seeded preference");
    let token = sign_in(&target);
    let page = target.get_html(&page_path(&fixture.household_slug));
    assert_eq!(page.status().as_u16(), 200);
    let mut viewer = OwnerViewGuard::new(&fixture);
    viewer.expose_view();
    let capabilities: Value = target
        .get(
            &format!("/api/v1/households/{}/ui_capabilities", fixture.household_id),
            Some(&fixture.access_token),
        )
        .json()
        .expect("view-only capabilities");
    assert_eq!(capabilities["data"]["notifications"]["manage"], false);
    let read_only = target.get_html(&page_path(&fixture.household_slug));
    assert_eq!(read_only.status().as_u16(), 200);
    let document = Html::parse_document(&read_only.text().expect("view-only page"));
    assert_eq!(
        document.select(&Selector::parse("button[type='submit']").unwrap()).count(),
        0,
        "view-only preferences must not offer a save button"
    );
    assert_eq!(
        document.select(&Selector::parse("input[type='checkbox'][disabled]").unwrap()).count(),
        5,
        "view-only preferences must not offer editable switches"
    );
    let denied = target.post_browser_form(
        &page_path(&fixture.household_slug),
        &[
            ("authenticity_token".into(), token),
            ("enabled".into(), "true".into()),
            ("dose_due_enabled".into(), "true".into()),
            ("private_text_enabled".into(), "true".into()),
        ],
    );
    assert_eq!(
        denied.status().as_u16(),
        403,
        "denied save must keep the API status"
    );
    let html = denied.text().expect("denied save HTML");
    assert!(!html.contains("Notification settings saved."));
    let document = Html::parse_document(&html);
    assert_eq!(checked(&document, "enabled"), Some(true));
    assert_eq!(checked(&document, "dose_due_enabled"), Some(true));
    assert_eq!(checked(&document, "missed_dose_enabled"), Some(false));
    assert_eq!(checked(&document, "low_stock_enabled"), Some(false));
    assert_eq!(checked(&document, "private_text_enabled"), Some(true));
    assert_eq!(
        document
            .select(&Selector::parse("[role='alert']").unwrap())
            .count(),
        1,
        "denied save must show an associated error"
    );
    viewer.restore();
    let after: Value = target
        .get(&api, Some(&fixture.access_token))
        .json()
        .expect("preference after denied save");
    assert_eq!(after["data"], stored["data"], "denied save must not store");
}

#[test]
fn notifications_page_hides_foreign_households_and_requires_a_session() {
    let fixture = fixture();
    let anonymous = Target::from_env();
    let response = anonymous.get_html(&page_path(&fixture.household_slug));
    assert_eq!(response.status().as_u16(), 302, "anonymous page access");
    let stale = Target::from_env();
    let expired = stale.get_html_with_header(
        &page_path(&fixture.household_slug),
        "Cookie",
        "medtracker_session=expired-session-id",
    );
    assert_eq!(expired.status().as_u16(), 302, "expired session access");
    let owner = Target::from_env();
    let token = sign_in(&owner);
    let foreign = owner.get_html(&page_path(&fixture.foreign_household_slug));
    assert_eq!(foreign.status().as_u16(), 404, "foreign household page");
    let foreign_save = owner.post_browser_form(
        &page_path(&fixture.foreign_household_slug),
        &[
            ("authenticity_token".into(), token),
            ("enabled".into(), "true".into()),
        ],
    );
    assert_eq!(
        foreign_save.status().as_u16(),
        404,
        "foreign household save"
    );
}
