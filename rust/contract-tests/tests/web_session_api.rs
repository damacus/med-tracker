use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::{Client, Response};
use reqwest::header;
use reqwest::redirect::Policy;
use scraper::{Html, Selector};
use serde_json::{json, Value};
use std::env;
use std::time::Duration;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use url::{Host, Url};

struct BrowserClient {
    client: Client,
    origin: Url,
}

struct Login {
    csrf: String,
    old_cookie: String,
}

struct SecondaryActorSetup {
    db: postgres::Client,
    membership_id: i64,
    original_membership_person_id: Option<i64>,
    primary_user_id: i64,
    original_primary_active: bool,
    secondary_user_id: i64,
    secondary_person_id: i64,
}

impl SecondaryActorSetup {
    fn new(fixture: &Fixture) -> Self {
        let mut db = database();
        let membership = db
            .query_one(
                "SELECT id, person_id FROM household_memberships WHERE account_id = $1 AND household_id = $2",
                &[&fixture.account_id, &fixture.medication_read_household_id],
            )
            .unwrap();
        let membership_id = membership.get(0);
        let original_membership_person_id = membership.get(1);
        let original_primary_active = db
            .query_one(
                "SELECT active FROM users WHERE id = $1",
                &[&fixture.user_id],
            )
            .unwrap()
            .get(0);
        let secondary_person_id = db
            .query_one(
                "INSERT INTO people (account_id, household_id, name, person_type, has_capacity, created_at, updated_at) VALUES ($1, $2, $3, 0, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) RETURNING id",
                &[&fixture.account_id, &fixture.medication_read_household_id, &format!("Contract secondary actor {}", fixture.account_id)],
            )
            .unwrap()
            .get(0);
        let secondary_user_id = db
            .query_one(
                "INSERT INTO users (person_id, email_address, active, created_at, updated_at) VALUES ($1, $2, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) RETURNING id",
                &[&secondary_person_id, &format!("contract-secondary-actor-{}@example.test", fixture.account_id)],
            )
            .unwrap()
            .get(0);
        db.execute(
            "UPDATE household_memberships SET person_id = $1 WHERE id = $2",
            &[&secondary_person_id, &membership_id],
        )
        .unwrap();
        Self {
            db,
            membership_id,
            original_membership_person_id,
            primary_user_id: fixture.user_id,
            original_primary_active,
            secondary_user_id,
            secondary_person_id,
        }
    }

    fn deactivate_primary(&mut self) {
        self.db
            .execute(
                "UPDATE users SET active = false WHERE id = $1",
                &[&self.primary_user_id],
            )
            .unwrap();
    }
}

impl Drop for SecondaryActorSetup {
    fn drop(&mut self) {
        let _ = self.db.execute(
            "UPDATE users SET active = $1 WHERE id = $2",
            &[&self.original_primary_active, &self.primary_user_id],
        );
        let _ = self.db.execute(
            "UPDATE household_memberships SET person_id = $1 WHERE id = $2",
            &[&self.original_membership_person_id, &self.membership_id],
        );
        let _ = self.db.execute(
            "DELETE FROM users WHERE id = $1",
            &[&self.secondary_user_id],
        );
        let _ = self.db.execute(
            "DELETE FROM people WHERE id = $1",
            &[&self.secondary_person_id],
        );
    }
}

impl BrowserClient {
    fn new() -> Self {
        let _validated_target = Target::from_env();
        let origin = Url::parse(&env::var("CONTRACT_BASE_URL").unwrap()).unwrap();
        let local = match origin.host() {
            Some(Host::Domain("localhost")) => true,
            Some(Host::Ipv4(ip)) => ip.is_loopback(),
            Some(Host::Ipv6(ip)) => ip.is_loopback(),
            _ => false,
        };
        assert!(local, "browser contract writes require a loopback target");
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .redirect(Policy::none())
            .cookie_store(true)
            .no_proxy()
            .build()
            .unwrap();
        Self { client, origin }
    }

    fn url(&self, path: &str) -> Url {
        assert!(path.starts_with('/') && !path.starts_with("//"));
        let url = self.origin.join(path).unwrap();
        assert_eq!(url.origin(), self.origin.origin());
        url
    }

    fn get(&self, path: &str) -> Response {
        self.client.get(self.url(path)).send().unwrap()
    }

    fn login(&self, email: &str) -> Login {
        let page = self.get("/login");
        assert_eq!(page.status().as_u16(), 200);
        let login_html = page.text().unwrap();
        let intent_csrf = csrf_token(
            &login_html,
            "form[action='/login'] input[name='authenticity_token']",
        );
        let origin = self.origin.origin().ascii_serialization();
        let response = self
            .client
            .post(self.url("/login"))
            .header(header::ORIGIN, &origin)
            .form(&[
                ("email", email),
                ("password", "password"),
                ("authenticity_token", intent_csrf.as_str()),
            ])
            .send()
            .unwrap();
        assert!(matches!(response.status().as_u16(), 302 | 303));
        let old_cookie = session_cookie(&response);
        let location = response.headers()[header::LOCATION].to_str().unwrap();
        assert!(location.starts_with('/') && !location.starts_with("//"));
        let destination = self.get(location);
        assert_eq!(destination.status().as_u16(), 200);
        let csrf = csrf_token(&destination.text().unwrap(), "meta[name='csrf-token']");
        assert_ne!(csrf, intent_csrf, "login must rotate the CSRF token");
        Login { csrf, old_cookie }
    }

    fn post_dose(
        &self,
        path: &str,
        body: &Value,
        csrf: Option<&str>,
        origin: Option<&str>,
        referer: Option<&str>,
    ) -> Response {
        let mut request = self.client.post(self.url(path)).json(body);
        if let Some(csrf) = csrf {
            request = request.header("X-CSRF-Token", csrf);
        }
        if let Some(origin) = origin {
            request = request.header(header::ORIGIN, origin);
        }
        if let Some(referer) = referer {
            request = request.header(header::REFERER, referer);
        }
        request.send().unwrap()
    }
}

fn csrf_token(html: &str, selector: &str) -> String {
    let document = Html::parse_document(html);
    document
        .select(&Selector::parse(selector).unwrap())
        .next()
        .and_then(|element| {
            element
                .value()
                .attr("content")
                .or_else(|| element.value().attr("value"))
        })
        .filter(|value| !value.is_empty())
        .expect("CSRF token in HTML")
        .to_string()
}

fn session_cookie(response: &Response) -> String {
    session_set_cookie(response)
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

fn session_set_cookie(response: &Response) -> &str {
    response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|value| value.starts_with("mt_oauth_session="))
        .expect("signed browser session cookie")
}

fn session_cookie_max_age(response: &Response) -> i64 {
    session_set_cookie(response)
        .split(';')
        .map(str::trim)
        .find_map(|attribute| attribute.strip_prefix("Max-Age="))
        .expect("session cookie Max-Age")
        .parse()
        .unwrap()
}

fn database() -> postgres::Client {
    postgres::Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").unwrap(),
        postgres::NoTls,
    )
    .unwrap()
}

fn stock_source(fixture: &Fixture) -> (i64, String) {
    let row = database()
        .query_one(
            "SELECT medications.id, person_medications.portable_id FROM medications JOIN person_medications ON person_medications.medication_id = medications.id WHERE medications.household_id = $1 AND medications.portable_id = $2",
            &[&fixture.household_id, &fixture.visible_low_stock_portable_id],
        )
        .expect("fixture-only low-stock assignment");
    (row.get(0), row.get(1))
}

fn supply(medication_id: i64) -> String {
    database()
        .query_one(
            "SELECT current_supply::text FROM medications WHERE id = $1",
            &[&medication_id],
        )
        .unwrap()
        .get(0)
}

fn take_count(uuid: &str) -> i64 {
    database()
        .query_one(
            "SELECT count(*) FROM medication_takes WHERE client_uuid = $1",
            &[&uuid],
        )
        .unwrap()
        .get(0)
}

fn assert_cookie_denial_audit(
    response: &Response,
    fixture: &Fixture,
    cookie: &str,
    session_csrf: &str,
    body: &Value,
) {
    let request_id = response.headers()["x-request-id"].to_str().unwrap();
    let row = database()
        .query_one(
            "SELECT row_to_json(security_audit_events)::text FROM security_audit_events WHERE request_id = $1",
            &[&request_id],
        )
        .expect("cookie denial request audit");
    let event: Value = serde_json::from_str(&row.get::<_, String>(0)).unwrap();
    assert_eq!(event["event_type"], "api.request");
    assert_eq!(event["household_id"], fixture.household_id);
    assert_eq!(event["actor_account_id"], fixture.account_id);
    assert_eq!(event["actor_membership_id"], fixture.owner_membership_id);
    assert_eq!(event["metadata"]["http_method"], "POST");
    assert_eq!(event["metadata"]["action"], "create");
    assert_eq!(event["metadata"]["status"], 403);
    assert_eq!(event["metadata"]["outcome"], "failure");
    let audit = event.to_string();
    for secret in [
        cookie,
        session_csrf,
        fixture.access_token.as_str(),
        body["medication_take"]["client_uuid"].as_str().unwrap(),
        body["medication_take"]["source_id"].as_str().unwrap(),
    ] {
        assert!(!audit.contains(secret));
    }
}

fn medication_path(household_id: i64) -> String {
    format!("/api/v1/households/{household_id}/medications")
}

fn dose_path(household_id: i64) -> String {
    format!("/api/v1/households/{household_id}/medication_takes")
}

#[test]
fn cookie_reads_use_current_household_membership_with_primary_account_actor() {
    let fixture = fixture();
    let browser = BrowserClient::new();
    browser.login(&fixture.primary_email);
    let secondary_membership: i64 = database()
        .query_one(
            "SELECT id FROM household_memberships WHERE account_id = $1 AND household_id = $2 AND status = 'active'",
            &[&fixture.account_id, &fixture.medication_read_household_id],
        )
        .unwrap()
        .get(0);
    let response = browser.get(&medication_path(fixture.medication_read_household_id));
    assert_eq!(response.status().as_u16(), 200);
    let request_id = response.headers()["x-request-id"].to_str().unwrap();
    let audit = database()
        .query_one(
            "SELECT row_to_json(security_audit_events)::text FROM security_audit_events WHERE request_id = $1",
            &[&request_id],
        )
        .expect("secondary-household cookie audit");
    let event: Value = serde_json::from_str(&audit.get::<_, String>(0)).unwrap();
    assert_eq!(event["event_type"], "api.request");
    assert_eq!(event["household_id"], fixture.medication_read_household_id);
    assert_eq!(event["actor_account_id"], fixture.account_id);
    assert_eq!(event["actor_membership_id"], secondary_membership);
    assert_eq!(event["audit_context"]["actor_user_id"], fixture.user_id);
}

#[test]
fn inactive_primary_user_denies_cookie_web_and_api_even_with_active_secondary_user() {
    let fixture = fixture();
    let mut secondary = SecondaryActorSetup::new(&fixture);
    let browser = BrowserClient::new();
    let login = browser.login(&fixture.primary_email);
    let primary_path = medication_path(fixture.household_id);
    let secondary_path = medication_path(fixture.medication_read_household_id);
    assert_eq!(browser.get(&primary_path).status().as_u16(), 200);
    assert_eq!(browser.get(&secondary_path).status().as_u16(), 200);

    secondary.deactivate_primary();
    for path in [&primary_path, &secondary_path] {
        let denied = browser
            .client
            .get(browser.url(path))
            .header(header::COOKIE, &login.old_cookie)
            .send()
            .unwrap();
        assert_eq!(denied.status().as_u16(), 401);
        assert!(denied.json::<Value>().unwrap().get("data").is_none());
    }
    let web = browser
        .client
        .get(browser.url(&format!("/households/{}/dashboard", fixture.household_slug)))
        .header(header::COOKIE, &login.old_cookie)
        .send()
        .unwrap();
    assert!(matches!(web.status().as_u16(), 302 | 303));
    assert!(web.headers()[header::LOCATION]
        .to_str()
        .unwrap()
        .starts_with("/login"));
}

#[test]
fn standalone_login_creates_cookie_for_shared_reads_and_invalid_bearer_takes_precedence() {
    let fixture = fixture();
    let browser = BrowserClient::new();
    let login = browser.login(&fixture.primary_email);
    let path = medication_path(fixture.household_id);
    let cookie_read = browser.get(&path);
    assert_eq!(cookie_read.status().as_u16(), 200);
    let cookie_data: Value = cookie_read.json().unwrap();
    assert!(cookie_data["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == fixture.managed_medication_id));
    let bearer_read = Target::from_env().get(&path, Some(&fixture.access_token));
    assert_eq!(bearer_read.status().as_u16(), 200);
    let bearer_data: Value = bearer_read.json().unwrap();
    assert_eq!(cookie_data["data"], bearer_data["data"]);
    let bad_bearer = browser
        .client
        .get(browser.url(&path))
        .bearer_auth("explicitly-invalid")
        .header(header::COOKIE, &login.old_cookie)
        .send()
        .unwrap();
    assert_eq!(bad_bearer.status().as_u16(), 401);
    assert!(!bad_bearer
        .text()
        .unwrap()
        .contains(&fixture.managed_medication_name));
    assert!(!login.old_cookie.contains(&fixture.access_token));
}

#[test]
fn valid_browser_activity_renews_cookie_without_rotating_session_or_csrf() {
    let fixture = fixture();
    let browser = BrowserClient::new();
    let login = browser.login(&fixture.primary_email);
    let dashboard = browser.get(&format!("/households/{}/dashboard", fixture.household_slug));
    assert_eq!(dashboard.status().as_u16(), 200);
    assert_eq!(session_cookie(&dashboard), login.old_cookie);
    let dashboard_age = session_cookie_max_age(&dashboard);
    assert!(dashboard_age > 0);
    assert_eq!(
        csrf_token(&dashboard.text().unwrap(), "meta[name='csrf-token']"),
        login.csrf
    );

    let path = medication_path(fixture.household_id);
    let api = browser.get(&path);
    assert_eq!(api.status().as_u16(), 200);
    assert_eq!(session_cookie(&api), login.old_cookie);
    let api_age = session_cookie_max_age(&api);
    assert!(api_age > 0 && api_age <= dashboard_age);

    let invalid_bearer = browser
        .client
        .get(browser.url(&path))
        .bearer_auth("explicitly-invalid")
        .header(header::COOKIE, &login.old_cookie)
        .send()
        .unwrap();
    assert_eq!(invalid_bearer.status().as_u16(), 401);
    assert!(invalid_bearer
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .all(|value| !value.starts_with("mt_oauth_session=")));

    let logout = browser
        .client
        .post(browser.url("/logout"))
        .header("X-CSRF-Token", &login.csrf)
        .header(
            header::ORIGIN,
            browser.origin.origin().ascii_serialization(),
        )
        .form(&[("authenticity_token", login.csrf.as_str())])
        .send()
        .unwrap();
    assert!(matches!(logout.status().as_u16(), 302 | 303));
    assert_eq!(session_cookie(&logout), "mt_oauth_session=");
    assert_eq!(session_cookie_max_age(&logout), 0);
    let copied = browser
        .client
        .get(browser.url(&path))
        .header(header::COOKIE, &login.old_cookie)
        .send()
        .unwrap();
    assert_eq!(copied.status().as_u16(), 401);
}

#[test]
fn cookie_access_respects_foreign_households_and_ungranted_people() {
    let fixture = fixture();
    let browser = BrowserClient::new();
    browser.login(&fixture.web_view_email);
    let visible = browser.get(&medication_path(fixture.household_id));
    assert_eq!(visible.status().as_u16(), 200);
    let visible: Value = visible.json().unwrap();
    assert!(visible["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == fixture.managed_medication_id));
    assert!(!visible["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == fixture.hidden_medication_id));

    let hidden = browser.get(&format!(
        "{}/{}",
        medication_path(fixture.household_id),
        fixture.hidden_medication_id
    ));
    assert_eq!(hidden.status().as_u16(), 404);
    assert!(!hidden
        .text()
        .unwrap()
        .contains(&fixture.hidden_person_portable_id));
    let foreign = browser.get(&medication_path(fixture.foreign_household_id));
    assert_eq!(foreign.status().as_u16(), 403);
    assert!(!foreign
        .text()
        .unwrap()
        .contains(&fixture.foreign_medication_name));
}

#[test]
fn cookie_dose_write_requires_session_csrf_and_trusted_origin() {
    let fixture = fixture();
    let browser = BrowserClient::new();
    let login = browser.login(&fixture.primary_email);
    let (medication_id, source_id) = stock_source(&fixture);
    let uuid = format!("88888888-8888-4888-8888-{medication_id:012x}");
    let body = json!({"medication_take": {
        "client_uuid": uuid,
        "source_type": "person_medication",
        "source_id": source_id,
        "taken_at": OffsetDateTime::now_utc().replace_nanosecond(0).unwrap().format(&Rfc3339).unwrap(),
        "dose_amount": "1",
        "dose_unit": "tablet",
        "taken_from_medication_id": medication_id
    }});
    let path = dose_path(fixture.household_id);
    let trusted_origin = browser.origin.origin().ascii_serialization();
    let trusted_referer = format!(
        "{trusted_origin}/households/{}/medications",
        fixture.household_slug
    );
    assert_eq!(supply(medication_id), "1.00");
    for (csrf, origin, referer) in [
        (None, Some(trusted_origin.as_str()), None),
        (Some("wrong-token"), Some(trusted_origin.as_str()), None),
        (Some(login.csrf.as_str()), None, None),
        (
            Some(login.csrf.as_str()),
            Some("https://attacker.example"),
            None,
        ),
        (
            Some(login.csrf.as_str()),
            Some("https://attacker.example"),
            Some(trusted_referer.as_str()),
        ),
    ] {
        let denied = browser.post_dose(&path, &body, csrf, origin, referer);
        assert_eq!(denied.status().as_u16(), 403);
        assert_cookie_denial_audit(&denied, &fixture, &login.old_cookie, &login.csrf, &body);
        assert_eq!(take_count(&uuid), 0);
        assert_eq!(supply(medication_id), "1.00");
    }
    let invalid_bearer = browser
        .client
        .post(browser.url(&path))
        .bearer_auth("explicitly-invalid")
        .header(header::COOKIE, &login.old_cookie)
        .header("X-CSRF-Token", &login.csrf)
        .header(header::ORIGIN, &trusted_origin)
        .json(&body)
        .send()
        .unwrap();
    assert_eq!(invalid_bearer.status().as_u16(), 401);
    assert_eq!(take_count(&uuid), 0);
    let created = browser.post_dose(
        &path,
        &body,
        Some(&login.csrf),
        None,
        Some(&trusted_referer),
    );
    assert_eq!(created.status().as_u16(), 201);
    let created: Value = created.json().unwrap();
    let id = created["data"]["id"].as_i64().unwrap();
    assert_eq!(supply(medication_id), "0.00");
    assert_eq!(take_count(&uuid), 1);
    let replay = browser.post_dose(&path, &body, Some(&login.csrf), Some(&trusted_origin), None);
    assert_eq!(replay.status().as_u16(), 200);
    let replay: Value = replay.json().unwrap();
    assert_eq!(replay["data"]["id"], id);
    assert_eq!(supply(medication_id), "0.00");
    let bearer_history = Target::from_env().get(&path, Some(&fixture.access_token));
    assert_eq!(bearer_history.status().as_u16(), 200);
    let bearer_history: Value = bearer_history.json().unwrap();
    assert!(bearer_history["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == id));
}

#[test]
fn logout_revokes_copied_cookie_and_session_key_revocation_or_expiry_rechecks_each_request() {
    let fixture = fixture();
    let browser = BrowserClient::new();
    let login = browser.login(&fixture.primary_email);
    let path = medication_path(fixture.household_id);
    assert_eq!(browser.get(&path).status().as_u16(), 200);
    let missing_origin = browser
        .client
        .post(browser.url("/logout"))
        .header("X-CSRF-Token", &login.csrf)
        .form(&[("authenticity_token", login.csrf.as_str())])
        .send()
        .unwrap();
    assert_eq!(missing_origin.status().as_u16(), 403);
    assert_eq!(browser.get(&path).status().as_u16(), 200);
    let trusted_origin = browser.origin.origin().ascii_serialization();
    let logout = browser
        .client
        .post(browser.url("/logout"))
        .header("X-CSRF-Token", &login.csrf)
        .header(header::REFERER, format!("{trusted_origin}/households"))
        .form(&[("authenticity_token", login.csrf.as_str())])
        .send()
        .unwrap();
    assert!(matches!(logout.status().as_u16(), 302 | 303));
    let copied = browser
        .client
        .get(browser.url(&path))
        .header(header::COOKIE, &login.old_cookie)
        .send()
        .unwrap();
    assert_eq!(copied.status().as_u16(), 401);

    let before: Vec<String> = database()
        .query(
            "SELECT session_id FROM account_active_session_keys WHERE account_id = $1",
            &[&fixture.account_id],
        )
        .unwrap()
        .into_iter()
        .map(|row| row.get(0))
        .collect();
    let renewed = BrowserClient::new();
    renewed.login(&fixture.primary_email);
    assert_eq!(renewed.get(&path).status().as_u16(), 200);
    let new_keys: Vec<String> = database()
        .query(
            "SELECT session_id FROM account_active_session_keys WHERE account_id = $1",
            &[&fixture.account_id],
        )
        .unwrap()
        .into_iter()
        .map(|row| row.get(0))
        .filter(|key| !before.contains(key))
        .collect();
    assert_eq!(new_keys.len(), 1);
    database()
        .execute(
            "DELETE FROM account_active_session_keys WHERE account_id = $1 AND session_id = $2",
            &[&fixture.account_id, &new_keys[0]],
        )
        .unwrap();
    assert_eq!(renewed.get(&path).status().as_u16(), 401);

    let before_expiry: Vec<String> = database()
        .query(
            "SELECT session_id FROM account_active_session_keys WHERE account_id = $1",
            &[&fixture.account_id],
        )
        .unwrap()
        .into_iter()
        .map(|row| row.get(0))
        .collect();
    let expiring = BrowserClient::new();
    expiring.login(&fixture.primary_email);
    assert_eq!(expiring.get(&path).status().as_u16(), 200);
    let expiry_keys: Vec<String> = database()
        .query(
            "SELECT session_id FROM account_active_session_keys WHERE account_id = $1",
            &[&fixture.account_id],
        )
        .unwrap()
        .into_iter()
        .map(|row| row.get(0))
        .filter(|key| !before_expiry.contains(key))
        .collect();
    assert_eq!(expiry_keys.len(), 1);
    database()
        .execute(
            "UPDATE account_active_session_keys SET last_use = CURRENT_TIMESTAMP - INTERVAL '31 days' WHERE account_id = $1 AND session_id = $2",
            &[&fixture.account_id, &expiry_keys[0]],
        )
        .unwrap();
    assert_eq!(expiring.get(&path).status().as_u16(), 401);
}

#[test]
fn suspended_membership_denies_cookie_access_without_changing_other_sessions() {
    let fixture = fixture();
    let browser = BrowserClient::new();
    browser.login(&fixture.web_view_email);
    let path = medication_path(fixture.household_id);
    assert_eq!(browser.get(&path).status().as_u16(), 200);
    let mut database = database();
    database
        .execute(
            "UPDATE household_memberships SET status = 'suspended' WHERE id = $1",
            &[&fixture.view_membership_id],
        )
        .unwrap();
    let denied = browser.get(&path);
    let status = denied.status().as_u16();
    database
        .execute(
            "UPDATE household_memberships SET status = 'active' WHERE id = $1",
            &[&fixture.view_membership_id],
        )
        .unwrap();
    assert_eq!(status, 403);
    assert!(!denied
        .text()
        .unwrap()
        .contains(&fixture.managed_medication_name));
}
