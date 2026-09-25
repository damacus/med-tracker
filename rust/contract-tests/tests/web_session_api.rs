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
    ) -> Response {
        let mut request = self.client.post(self.url(path)).json(body);
        if let Some(csrf) = csrf {
            request = request.header("X-CSRF-Token", csrf);
        }
        if let Some(origin) = origin {
            request = request.header(header::ORIGIN, origin);
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
    response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find_map(|value| {
            value
                .split(';')
                .next()
                .filter(|cookie| cookie.starts_with("mt_oauth_session="))
        })
        .expect("signed browser session cookie")
        .to_string()
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

fn medication_path(household_id: i64) -> String {
    format!("/api/v1/households/{household_id}/medications")
}

fn dose_path(household_id: i64) -> String {
    format!("/api/v1/households/{household_id}/medication_takes")
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
    assert_eq!(supply(medication_id), "1.00");
    for (csrf, origin) in [
        (None, Some(trusted_origin.as_str())),
        (Some("wrong-token"), Some(trusted_origin.as_str())),
        (Some(login.csrf.as_str()), Some("https://attacker.example")),
    ] {
        let denied = browser.post_dose(&path, &body, csrf, origin);
        assert_eq!(denied.status().as_u16(), 403);
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
    let created = browser.post_dose(&path, &body, Some(&login.csrf), Some(&trusted_origin));
    assert_eq!(created.status().as_u16(), 201);
    let created: Value = created.json().unwrap();
    let id = created["data"]["id"].as_i64().unwrap();
    assert_eq!(supply(medication_id), "0.00");
    assert_eq!(take_count(&uuid), 1);
    let replay = browser.post_dose(&path, &body, Some(&login.csrf), Some(&trusted_origin));
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
fn logout_revokes_copied_cookie_and_session_key_revocation_rechecks_each_request() {
    let fixture = fixture();
    let browser = BrowserClient::new();
    let login = browser.login(&fixture.primary_email);
    let path = medication_path(fixture.household_id);
    assert_eq!(browser.get(&path).status().as_u16(), 200);
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
