use medtracker_contract_tests::{Fixture, Target, fixture};
use postgres::{Client, NoTls};
use scraper::{Html, Selector};
use serde_json::Value;
use std::{
    env,
    sync::atomic::{AtomicUsize, Ordering},
};

static LOGIN_CLIENT: AtomicUsize = AtomicUsize::new(1);

#[path = "household_completion_dashboard/role_matrix.rs"]
mod role_matrix;

fn sign_in(target: &Target, email: &str) {
    let login = target.get_html("/login");
    assert_eq!(login.status().as_u16(), 200);
    let document = Html::parse_document(&login.text().expect("login HTML"));
    let selector =
        Selector::parse("form[action='/login'] input[name='authenticity_token']").unwrap();
    let token = document
        .select(&selector)
        .next()
        .and_then(|input| input.value().attr("value"))
        .expect("real login CSRF");
    let client_ip = format!("198.18.27.{}", LOGIN_CLIENT.fetch_add(1, Ordering::Relaxed));
    let login = target.post_html_form_from_client(
        "/login",
        &client_ip,
        &[
            ("email".into(), email.into()),
            ("password".into(), "password".into()),
            ("authenticity_token".into(), token.into()),
        ],
    );
    assert_eq!(login.status().as_u16(), 302);
}

#[test]
fn active_limited_view_member_without_own_person_access_can_use_dashboard() {
    let fixture = fixture();
    let viewer = Target::from_env();
    let api = format!("/api/v1/households/{}", fixture.household_id);
    let me = viewer.get(&format!("{api}/me"), Some(&fixture.view_access_token));
    assert_eq!(me.status().as_u16(), 200);
    let me: Value = me.json().expect("active viewer identity");
    assert_eq!(me["data"]["membership_role"], "member");
    sign_in(
        &viewer,
        me["data"]["email_address"].as_str().expect("viewer email"),
    );
    let profile = viewer.get(&format!("{api}/profile"), None);
    assert_eq!(
        profile.status().as_u16(),
        403,
        "dashboard usability must not grant own-person profile access"
    );
    let visible = viewer.get(&format!("{api}/people"), None);
    assert_eq!(visible.status().as_u16(), 200);
    let visible: Value = visible.json().expect("authorised people");
    let people = visible["data"].as_array().unwrap();
    assert_eq!(
        people.len(),
        1,
        "real fixture grants only the managed person"
    );
    assert_eq!(people[0]["id"], fixture.managed_person_id);
    let owner = Target::from_env();
    let hidden = owner.get(
        &format!("{api}/people/{}", fixture.hidden_person_id),
        Some(&fixture.feed_access_token),
    );
    assert_eq!(hidden.status().as_u16(), 200);
    let hidden: Value = hidden.json().expect("hidden person reference");
    assert_eq!(
        viewer
            .get(&format!("{api}/people/{}", fixture.hidden_person_id), None)
            .status()
            .as_u16(),
        404
    );
    let dashboard = viewer.get_html(&format!("/households/{}/dashboard", fixture.household_slug));
    let status = dashboard.status().as_u16();
    let cache = dashboard
        .headers()
        .get("cache-control")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let html = dashboard.text().expect("limited-view dashboard response");
    assert_eq!(
        status, 200,
        "active viewer dashboard should load authorised data: {html}"
    );
    assert!(cache.contains("no-store"));
    let document = Html::parse_document(&html);
    let text = document.root_element().text().collect::<String>();
    assert!(text.contains(people[0]["name"].as_str().unwrap()));
    assert!(!text.contains(hidden["data"]["name"].as_str().unwrap()));
    assert!(!text.contains(&fixture.foreign_person_name));
    let choices: Vec<_> = document
        .select(&Selector::parse("[data-testid='dashboard-person-option']").unwrap())
        .map(|choice| choice.value().attr("href").unwrap().to_owned())
        .collect();
    assert_eq!(
        choices.len(),
        2,
        "selector must expose exactly All Family and the sole authorised person"
    );
    assert!(
        choices
            .iter()
            .any(|href| href.ends_with("dashboard_person_id=all"))
    );
    assert!(choices.iter().any(|href| href.ends_with(&format!(
        "dashboard_person_id={}",
        fixture.managed_person_id
    ))));
    for link in document.select(&Selector::parse("a[href]").unwrap()) {
        let href = link.value().attr("href").unwrap();
        assert!(!href.contains(&format!("dashboard_person_id={}", fixture.hidden_person_id)));
        assert!(
            !href.ends_with("/new") && !href.ends_with("/edit"),
            "limited viewer must not receive mutation links: {href}"
        );
    }
    for form in document.select(&Selector::parse("form[method='post']").unwrap()) {
        assert_eq!(
            form.value().attr("action"),
            Some("/logout"),
            "limited viewer must not receive clinical mutation forms"
        );
    }
    for button in document
        .select(&Selector::parse(".dashboard-task-action button, .dashboard-stock button").unwrap())
    {
        assert!(
            button.value().attr("disabled").is_some()
                || button.value().attr("aria-disabled") == Some("true")
        );
    }
    let still_forbidden = viewer.get(&format!("{api}/profile"), None);
    assert_eq!(still_forbidden.status().as_u16(), 403);
}

fn viewer(fixture: &Fixture) -> Target {
    let viewer = Target::from_env();
    let me = viewer.get(
        &format!("/api/v1/households/{}/me", fixture.household_id),
        Some(&fixture.view_access_token),
    );
    assert_eq!(me.status().as_u16(), 200);
    let me: Value = me.json().unwrap();
    sign_in(&viewer, me["data"]["email_address"].as_str().unwrap());
    viewer
}

fn people(target: &Target, fixture: &Fixture) -> Vec<i64> {
    let response = target.get(
        &format!("/api/v1/households/{}/people", fixture.household_id),
        None,
    );
    assert_eq!(response.status().as_u16(), 200);
    response.json::<Value>().unwrap()["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["id"].as_i64().unwrap())
        .collect()
}

fn database() -> Client {
    Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("disposable database URL"),
        NoTls,
    )
    .unwrap()
}

struct MembershipFixture {
    db: Client,
    id: i64,
    original_status: String,
    original_person: Option<i64>,
}

impl MembershipFixture {
    fn new(fixture: &Fixture) -> Self {
        let mut db = database();
        let row = db.query_one("SELECT status, person_id FROM household_memberships WHERE id = $1 AND household_id = $2 AND account_id = $3", &[&fixture.view_membership_id, &fixture.household_id, &fixture.view_account_id]).unwrap();
        Self {
            db,
            id: fixture.view_membership_id,
            original_status: row.get(0),
            original_person: row.get(1),
        }
    }

    fn status(&mut self, status: &str) {
        assert_eq!(
            self.db
                .execute(
                    "UPDATE household_memberships SET status = $2 WHERE id = $1",
                    &[&self.id, &status]
                )
                .unwrap(),
            1
        );
    }

    fn mismatch_profile(&mut self, fixture: &Fixture) {
        assert_eq!(
            self.db
                .execute(
                    "UPDATE household_memberships SET person_id = $2 WHERE id = $1",
                    &[&self.id, &fixture.managed_person_id]
                )
                .unwrap(),
            1
        );
    }
}

impl Drop for MembershipFixture {
    fn drop(&mut self) {
        if let Err(error) = self.db.execute(
            "UPDATE household_memberships SET status = $2, person_id = $3 WHERE id = $1",
            &[&self.id, &self.original_status, &self.original_person],
        ) {
            eprintln!("failed to restore disposable viewer membership: {error}");
        }
    }
}

struct AuditReadFailure {
    db: Client,
    name: String,
}

impl AuditReadFailure {
    fn new(fixture: &Fixture, controller: &str, action: &str, status: u16, suffix: &str) -> Self {
        let mut db = database();
        let session: String = db.query_one("SELECT audit_context->>'session_reference' FROM security_audit_events WHERE actor_account_id = $1 AND household_id = $2 AND audit_context->>'authentication_method' = 'browser_session' ORDER BY id DESC LIMIT 1", &[&fixture.view_account_id, &fixture.household_id]).expect("real viewer request must establish audit session").get(0);
        let name = format!(
            "completion_dashboard_failure_{}_{}",
            fixture.view_membership_id, suffix
        );
        let mut gate = Self { db, name };
        let session = session.replace('\'', "''");
        gate.db.batch_execute(&format!("CREATE SEQUENCE {name}_fired; GRANT USAGE, SELECT ON SEQUENCE {name}_fired TO med_tracker_app; CREATE FUNCTION {name}() RETURNS trigger LANGUAGE plpgsql AS $gate$ BEGIN IF NEW.actor_account_id = {actor} AND NEW.household_id = {household} AND NEW.metadata->>'controller' = '{controller}' AND NEW.metadata->>'action' = '{action}' AND NEW.metadata->>'http_method' = 'GET' AND NEW.metadata->>'status' = '{status}' AND NEW.audit_context->>'authentication_method' = 'browser_session' AND NEW.audit_context->>'session_reference' = '{session}' THEN PERFORM nextval('{name}_fired'); RAISE EXCEPTION 'disposable contract read failure'; END IF; RETURN NEW; END $gate$; CREATE TRIGGER {name} AFTER INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION {name}();", name = gate.name, actor = fixture.view_account_id, household = fixture.household_id)).unwrap();
        gate
    }

    fn hits(&mut self) -> i64 {
        let row = self
            .db
            .query_one(
                &format!("SELECT last_value, is_called FROM {}_fired", self.name),
                &[],
            )
            .unwrap();
        if row.get::<_, bool>(1) { row.get(0) } else { 0 }
    }
}

impl Drop for AuditReadFailure {
    fn drop(&mut self) {
        for sql in [
            format!(
                "DROP TRIGGER IF EXISTS {} ON security_audit_events",
                self.name
            ),
            format!("DROP FUNCTION IF EXISTS {}()", self.name),
            format!("DROP SEQUENCE IF EXISTS {}_fired", self.name),
        ] {
            if let Err(error) = self.db.batch_execute(&sql) {
                eprintln!("failed to remove disposable dashboard read failure: {error}");
            }
        }
    }
}

#[test]
fn suspended_and_revoked_memberships_cannot_use_an_existing_dashboard_session() {
    let fixture = fixture();
    let viewer = viewer(&fixture);
    let path = format!("/households/{}/dashboard", fixture.household_slug);
    assert_eq!(viewer.get_html(&path).status().as_u16(), 200);
    for status in ["suspended", "revoked"] {
        {
            let mut membership = MembershipFixture::new(&fixture);
            membership.status(status);
            let denied = viewer.get_html(&path);
            assert_eq!(
                denied.status().as_u16(),
                404,
                "non-active membership must disappear from authorised household selection"
            );
            assert!(
                !denied
                    .text()
                    .unwrap()
                    .contains(&fixture.managed_medication_name)
            );
        }
        assert_eq!(people(&viewer, &fixture), vec![fixture.managed_person_id]);
        assert_eq!(viewer.get_html(&path).status().as_u16(), 200);
    }
}

#[test]
fn absent_optional_profile_preserves_authenticated_people_and_usable_dashboard() {
    let fixture = fixture();
    let viewer = viewer(&fixture);
    let base = format!("/api/v1/households/{}", fixture.household_id);
    let visible = people(&viewer, &fixture);
    assert_eq!(visible, vec![fixture.managed_person_id]);
    {
        let mut membership = MembershipFixture::new(&fixture);
        membership.mismatch_profile(&fixture);
        assert_eq!(
            viewer.get(&format!("{base}/me"), None).status().as_u16(),
            200
        );
        assert_eq!(people(&viewer, &fixture), visible);
        assert_eq!(
            viewer
                .get(&format!("{base}/profile"), None)
                .status()
                .as_u16(),
            404
        );
        let dashboard =
            viewer.get_html(&format!("/households/{}/dashboard", fixture.household_slug));
        assert_eq!(dashboard.status().as_u16(), 200);
        assert_eq!(people(&viewer, &fixture), visible);
        assert_eq!(
            viewer
                .get(&format!("{base}/profile"), None)
                .status()
                .as_u16(),
            404
        );
    }
    assert_eq!(people(&viewer, &fixture), visible);
    assert_eq!(
        viewer
            .get(&format!("{base}/profile"), None)
            .status()
            .as_u16(),
        403
    );
}

#[test]
fn required_authorised_people_read_failure_keeps_dashboard_unavailable() {
    let fixture = fixture();
    let viewer = viewer(&fixture);
    assert_eq!(people(&viewer, &fixture), vec![fixture.managed_person_id]);
    let mut gate = AuditReadFailure::new(&fixture, "api/v1/people", "index", 200, "people");
    assert_eq!(gate.hits(), 0);
    let failed = viewer.get(
        &format!("/api/v1/households/{}/people", fixture.household_id),
        None,
    );
    assert_eq!(failed.status().as_u16(), 500);
    assert!(failed.json::<Value>().unwrap().get("data").is_none());
    assert_eq!(
        gate.hits(),
        1,
        "real authorised API failure must reach fixture gate"
    );
    let dashboard = viewer.get_html(&format!("/households/{}/dashboard", fixture.household_slug));
    assert_eq!(dashboard.status().as_u16(), 503);
    assert_eq!(
        gate.hits(),
        2,
        "SSR must execute the failing authorised read"
    );
    assert!(
        !dashboard
            .text()
            .unwrap()
            .contains(&fixture.managed_medication_name)
    );
    drop(gate);
    assert_eq!(people(&viewer, &fixture), vec![fixture.managed_person_id]);
}

#[test]
fn profile_server_failure_is_required_to_keep_dashboard_unavailable() {
    let fixture = fixture();
    let viewer = viewer(&fixture);
    let base = format!("/api/v1/households/{}", fixture.household_id);
    let visible = people(&viewer, &fixture);
    let mut membership = MembershipFixture::new(&fixture);
    membership.mismatch_profile(&fixture);
    assert_eq!(
        viewer.get(&format!("{base}/me"), None).status().as_u16(),
        200
    );
    assert_eq!(people(&viewer, &fixture), visible);
    assert_eq!(
        viewer
            .get(&format!("{base}/profile"), None)
            .status()
            .as_u16(),
        404
    );
    let mut gate = AuditReadFailure::new(&fixture, "api/v1/profiles", "show", 404, "profile");
    assert_eq!(gate.hits(), 0);
    assert_eq!(
        viewer
            .get(&format!("{base}/profile"), None)
            .status()
            .as_u16(),
        500
    );
    assert_eq!(gate.hits(), 1);
    let dashboard = viewer.get_html(&format!("/households/{}/dashboard", fixture.household_slug));
    assert_eq!(dashboard.status().as_u16(), 503);
    assert_eq!(
        gate.hits(),
        2,
        "SSR must execute the failing optional profile rather than swallow server errors"
    );
    assert!(
        !dashboard
            .text()
            .unwrap()
            .contains(&fixture.managed_medication_name)
    );
    drop(gate);
    assert_eq!(people(&viewer, &fixture), visible);
    assert_eq!(
        viewer
            .get(&format!("{base}/profile"), None)
            .status()
            .as_u16(),
        404
    );
    drop(membership);
    assert_eq!(people(&viewer, &fixture), visible);
    assert_eq!(
        viewer
            .get(&format!("{base}/profile"), None)
            .status()
            .as_u16(),
        403
    );
}
