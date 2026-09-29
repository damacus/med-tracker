use medtracker_contract_tests::{fixture, Target};
use scraper::{Html, Selector};
use serde_json::Value;
use std::env;
use url::Url;

fn audit_database() -> postgres::Client {
    let url = env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract audit database URL");
    postgres::Client::connect(&url, postgres::NoTls).expect("contract audit database")
}

fn grant_id_for_code(code: &str) -> i64 {
    audit_database()
        .query_one("SELECT id FROM oauth_grants WHERE code = $1", &[&code])
        .expect("issued authorization grant")
        .get(0)
}

fn grant_activity(id: i64) -> (f64, f64) {
    let row = audit_database()
        .query_one(
            "SELECT EXTRACT(EPOCH FROM last_used_at)::double precision, EXTRACT(EPOCH FROM authenticated_at)::double precision FROM oauth_grants WHERE id = $1",
            &[&id],
        )
        .expect("authorization grant activity");
    (row.get(0), row.get(1))
}

fn authorization_path(client_id: &str, redirect_uri: &str) -> String {
    let mut query = url::form_urlencoded::Serializer::new(String::new());
    query
        .append_pair("response_type", "code")
        .append_pair("response_mode", "query")
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("scope", "medtracker offline_access")
        .append_pair("state", "contract-security-state")
        .append_pair(
            "code_challenge",
            "sIEAmHTSAwOYncK3AzYmthevluqX_MuVU227zeLfBY0",
        )
        .append_pair("code_challenge_method", "S256");
    format!("/authorize?{}", query.finish())
}

fn login_csrf(target: &Target) -> String {
    let login = target.get_html("/login");
    assert_eq!(login.status().as_u16(), 200);
    let document = Html::parse_document(&login.text().expect("login HTML"));
    let selector =
        Selector::parse("form[action='/login'] input[name='authenticity_token']").unwrap();
    document
        .select(&selector)
        .next()
        .and_then(|input| input.value().attr("value"))
        .expect("login CSRF token")
        .to_string()
}

fn consent_for_primary_account() -> (Target, String, Vec<(String, String)>) {
    let fixture = fixture();
    let target = Target::from_env();
    let path = authorization_path(&fixture.oauth_client_id, &fixture.oauth_redirect_uri);
    let start = target.get_html(&path);
    assert_eq!(start.status().as_u16(), 302);
    assert_eq!(start.headers()["location"], "/login");
    let csrf = login_csrf(&target);
    let login = target.post_html_form(
        "/login",
        &[
            ("email".to_string(), fixture.primary_email),
            ("password".to_string(), "password".to_string()),
            ("authenticity_token".to_string(), csrf),
        ],
    );
    assert_eq!(login.status().as_u16(), 302);
    let path = login.headers()["location"]
        .to_str()
        .expect("consent redirect");
    assert!(path.starts_with("/authorize?"));
    let consent = target.get_html(path);
    assert_eq!(consent.status().as_u16(), 200);
    let document = Html::parse_document(&consent.text().expect("consent HTML"));
    let selector = Selector::parse("form#authorize-form").unwrap();
    let form = document.select(&selector).next().expect("consent form");
    let action = form.value().attr("action").expect("consent action");
    let inputs = Selector::parse("input[name]").unwrap();
    let fields = form
        .select(&inputs)
        .filter(|input| input.value().attr("type") != Some("submit"))
        .map(|input| {
            (
                input.value().attr("name").unwrap().to_string(),
                input.value().attr("value").unwrap_or("").to_string(),
            )
        })
        .collect();
    (target, action.to_string(), fields)
}

fn assert_no_authorization_code(response: reqwest::blocking::Response, redirect_uri: &str) {
    let status = response.status().as_u16();
    assert!(
        (300..500).contains(&status),
        "authorization denial status: {status}"
    );
    if let Some(location) = response.headers().get("location") {
        let location = location.to_str().expect("denial redirect");
        if location.starts_with(redirect_uri) {
            let callback = Url::parse(location).expect("native denial callback");
            assert!(!callback.query_pairs().any(|(key, _)| key == "code"));
        } else {
            assert!(!location.contains("code="));
        }
    }
}

#[test]
fn discovery_advertises_authorization_code_pkce_and_local_endpoints() {
    let response = Target::from_env().get("/.well-known/oauth-authorization-server", None);
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().expect("JSON OAuth discovery");
    for (endpoint, expected_path) in [
        ("authorization_endpoint", "/authorize"),
        ("token_endpoint", "/token"),
        ("revocation_endpoint", "/revoke"),
    ] {
        let value = body[endpoint].as_str().expect("endpoint URL");
        let advertised = Url::parse(value).expect("absolute endpoint URL");
        assert!(matches!(advertised.scheme(), "http" | "https"));
        assert_eq!(advertised.path(), expected_path);
        assert!(advertised.query().is_none());
        assert!(advertised.fragment().is_none());
    }
    assert!(body["code_challenge_methods_supported"]
        .as_array()
        .expect("PKCE methods")
        .contains(&Value::from("S256")));
}

#[test]
fn capabilities_publish_registered_public_client_without_a_secret() {
    let fixture = fixture();
    let response = Target::from_env().get("/api/v1/capabilities", None);
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().expect("JSON capabilities");
    let oauth = &body["data"]["authentication"]["mobile_oauth"];
    assert!(oauth["discovery_url"]
        .as_str()
        .expect("discovery URL")
        .ends_with("/.well-known/oauth-authorization-server"));
    let clients = oauth["clients"].as_array().expect("public clients");
    let client = clients
        .iter()
        .find(|item| item["client_id"] == fixture.oauth_client_id)
        .expect("contract client");
    assert_eq!(client["redirect_uris"][0], fixture.oauth_redirect_uri);
    assert!(client["scopes"]
        .as_array()
        .unwrap()
        .contains(&Value::from("medtracker")));
    assert!(client.get("client_secret").is_none());
}

#[test]
fn unauthenticated_pkce_authorization_resumes_at_local_login() {
    let fixture = fixture();
    let path = format!(
        "/authorize?response_type=code&response_mode=query&client_id={}&redirect_uri={}&scope=medtracker&state=contract-state&code_challenge=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA&code_challenge_method=S256",
        fixture.oauth_client_id,
        url::form_urlencoded::byte_serialize(fixture.oauth_redirect_uri.as_bytes()).collect::<String>()
    );
    let response = Target::from_env().get(&path, None);
    assert_eq!(response.status().as_u16(), 302);
    assert_eq!(response.headers()["location"], "/login");
}

#[test]
fn invalid_code_redemption_does_not_leak_credentials() {
    let fixture = fixture();
    let code = "contract-nonexistent-authorization-code";
    let verifier = "contract-verifier-with-more-than-forty-three-characters";
    let response = Target::from_env().post_form(
        "/token",
        &[
            ("grant_type", "authorization_code"),
            ("client_id", &fixture.oauth_client_id),
            ("redirect_uri", &fixture.oauth_redirect_uri),
            ("code", code),
            ("code_verifier", verifier),
        ],
    );
    assert_eq!(response.status().as_u16(), 400);
    let body = response.text().expect("OAuth error body");
    assert!(!body.contains("access_token"));
    assert!(!body.contains(code));
    assert!(!body.contains(verifier));
}

#[test]
fn issued_code_enforces_pkce_and_supports_refresh_rotation_and_revocation() {
    let fixture = fixture();
    let target = Target::from_env();
    let verifier = "contract-verifier-with-more-than-forty-three-characters";
    let challenge = "sIEAmHTSAwOYncK3AzYmthevluqX_MuVU227zeLfBY0";
    let state = "contract-issued-code-state";
    let mut query = url::form_urlencoded::Serializer::new(String::new());
    query
        .append_pair("response_type", "code")
        .append_pair("response_mode", "query")
        .append_pair("client_id", &fixture.oauth_client_id)
        .append_pair("redirect_uri", &fixture.oauth_redirect_uri)
        .append_pair("scope", "medtracker offline_access")
        .append_pair("state", state)
        .append_pair("code_challenge", challenge)
        .append_pair("code_challenge_method", "S256");
    let authorize_path = format!("/authorize?{}", query.finish());

    let response = target.get_html(&authorize_path);
    assert_eq!(response.status().as_u16(), 302);
    assert_eq!(response.headers()["location"], "/login");

    let login = target.get_html("/login");
    assert_eq!(login.status().as_u16(), 200);
    let login_html = login.text().expect("login HTML");
    let login_document = Html::parse_document(&login_html);
    let login_token_selector =
        Selector::parse("form[action='/login'] input[name='authenticity_token']").unwrap();
    let login_token = login_document
        .select(&login_token_selector)
        .next()
        .and_then(|input| input.value().attr("value"))
        .expect("login CSRF token");
    let login_fields = vec![
        ("email".to_string(), fixture.primary_email.clone()),
        ("password".to_string(), "password".to_string()),
        ("authenticity_token".to_string(), login_token.to_string()),
    ];
    let login_response = target.post_html_form("/login", &login_fields);
    assert_eq!(login_response.status().as_u16(), 302);
    let consent_path = login_response.headers()["location"]
        .to_str()
        .expect("consent redirect")
        .to_string();
    assert!(consent_path.starts_with("/authorize?"));

    let consent = target.get_html(&consent_path);
    assert_eq!(consent.status().as_u16(), 200);
    let consent_html = consent.text().expect("consent HTML");
    let consent_document = Html::parse_document(&consent_html);
    let consent_selector = Selector::parse("form#authorize-form").unwrap();
    let form = consent_document
        .select(&consent_selector)
        .next()
        .expect("authorization consent form");
    let action = form.value().attr("action").expect("consent action");
    let input_selector = Selector::parse("input[name]").unwrap();
    let fields: Vec<(String, String)> = form
        .select(&input_selector)
        .filter(|input| input.value().attr("type") != Some("submit"))
        .map(|input| {
            (
                input.value().attr("name").unwrap().to_string(),
                input.value().attr("value").unwrap_or("").to_string(),
            )
        })
        .collect();
    assert!(fields
        .iter()
        .any(|(name, value)| name == "code_challenge" && value == challenge));
    assert!(fields
        .iter()
        .any(|(name, value)| name == "scope[]" && value == "medtracker"));
    assert!(fields
        .iter()
        .any(|(name, value)| name == "scope[]" && value == "offline_access"));
    let approved = target.post_html_form(action, &fields);
    assert_eq!(approved.status().as_u16(), 302);
    let callback = Url::parse(
        approved.headers()["location"]
            .to_str()
            .expect("native callback"),
    )
    .expect("callback URL");
    assert!(callback
        .as_str()
        .starts_with(&format!("{}?", fixture.oauth_redirect_uri)));
    let callback_query: std::collections::HashMap<_, _> =
        callback.query_pairs().into_owned().collect();
    assert_eq!(callback_query.get("state").map(String::as_str), Some(state));
    let code = callback_query
        .get("code")
        .expect("issued authorization code");
    assert!(!code.is_empty());
    let grant_id = grant_id_for_code(code);

    let wrong_verifier = "another-contract-verifier-with-more-than-forty-three-characters";
    let rejected = target.post_form(
        "/token",
        &[
            ("grant_type", "authorization_code"),
            ("client_id", &fixture.oauth_client_id),
            ("redirect_uri", &fixture.oauth_redirect_uri),
            ("code", code),
            ("code_verifier", wrong_verifier),
        ],
    );
    assert_eq!(rejected.status().as_u16(), 400);
    let rejected_body = rejected.text().expect("PKCE rejection body");
    assert!(!rejected_body.contains("access_token"));
    assert!(!rejected_body.contains(code));
    assert!(!rejected_body.contains(wrong_verifier));

    for (field, value) in [
        ("client_id", "contract-unknown-mobile-client"),
        ("redirect_uri", "io.damacus.medtracker.contract:/other"),
    ] {
        let attempted = target.post_form(
            "/token",
            &[
                ("grant_type", "authorization_code"),
                (
                    "client_id",
                    if field == "client_id" {
                        value
                    } else {
                        &fixture.oauth_client_id
                    },
                ),
                (
                    "redirect_uri",
                    if field == "redirect_uri" {
                        value
                    } else {
                        &fixture.oauth_redirect_uri
                    },
                ),
                ("code", code),
                ("code_verifier", verifier),
            ],
        );
        assert_eq!(attempted.status().as_u16(), 400, "{field}");
        let body = attempted.text().expect("binding denial");
        assert!(!body.contains("access_token"), "{field}");
    }

    let redeemed = target.post_form(
        "/token",
        &[
            ("grant_type", "authorization_code"),
            ("client_id", &fixture.oauth_client_id),
            ("redirect_uri", &fixture.oauth_redirect_uri),
            ("code", code),
            ("code_verifier", verifier),
        ],
    );
    assert_eq!(redeemed.status().as_u16(), 200);
    let tokens: Value = redeemed.json().expect("issued token JSON");
    let access = tokens["access_token"].as_str().expect("access token");
    let refresh = tokens["refresh_token"].as_str().expect("refresh token");
    let spent = target.post_form(
        "/token",
        &[
            ("grant_type", "authorization_code"),
            ("client_id", &fixture.oauth_client_id),
            ("redirect_uri", &fixture.oauth_redirect_uri),
            ("code", code),
            ("code_verifier", verifier),
        ],
    );
    assert_eq!(spent.status().as_u16(), 400);
    let spent_body = spent.text().expect("spent code denial");
    assert!(!spent_body.contains("access_token"));
    assert!(!spent_body.contains("refresh_token"));
    let households = target.get("/api/v1/auth/households", Some(access));
    assert_eq!(households.status().as_u16(), 200);
    let listing: Value = households.json().expect("issued token households");
    assert_eq!(listing["account_id"], fixture.account_id);
    let selected = listing["data"].as_array().expect("operational households");
    assert!(selected
        .iter()
        .any(|household| household["id"] == fixture.household_id));
    assert!(!selected
        .iter()
        .any(|household| household["id"] == fixture.foreign_household_id));
    let medication_path = format!("/api/v1/households/{}/medications", fixture.household_id);
    let medication_list = target.get(&medication_path, Some(access));
    assert_eq!(medication_list.status().as_u16(), 200);
    let medications: Value = medication_list
        .json()
        .expect("issued token medication list");
    assert!(medications["data"]
        .as_array()
        .expect("medication list")
        .iter()
        .any(|medication| medication["id"] == fixture.managed_medication_id));
    let foreign_path = format!(
        "/api/v1/households/{}/medications",
        fixture.foreign_household_id
    );
    assert_eq!(
        target.get(&foreign_path, Some(access)).status().as_u16(),
        403
    );

    let activity_before_refresh = grant_activity(grant_id);
    let rotated = target.post_form(
        "/token",
        &[
            ("grant_type", "refresh_token"),
            ("client_id", &fixture.oauth_client_id),
            ("refresh_token", refresh),
        ],
    );
    assert_eq!(rotated.status().as_u16(), 200);
    assert_eq!(grant_activity(grant_id), activity_before_refresh);
    let fresh_tokens: Value = rotated.json().expect("rotated token JSON");
    let fresh_access = fresh_tokens["access_token"]
        .as_str()
        .expect("rotated access token");
    let fresh_refresh = fresh_tokens["refresh_token"]
        .as_str()
        .expect("rotated refresh token");
    assert_ne!(fresh_refresh, refresh);
    assert_eq!(
        target
            .get("/api/v1/auth/households", Some(fresh_access))
            .status()
            .as_u16(),
        200
    );
    let replay = target.post_form(
        "/token",
        &[
            ("grant_type", "refresh_token"),
            ("client_id", &fixture.oauth_client_id),
            ("refresh_token", refresh),
        ],
    );
    assert_eq!(replay.status().as_u16(), 400);

    let revoked = Target::from_env().post_json(
        "/revoke",
        &serde_json::json!({
            "client_id": fixture.oauth_client_id,
            "token": fresh_access,
            "token_type_hint": "access_token"
        }),
    );
    assert_eq!(revoked.status().as_u16(), 200);
    assert_eq!(
        target
            .get("/api/v1/auth/households", Some(fresh_access))
            .status()
            .as_u16(),
        401
    );
    let revoked_refresh = target.post_form(
        "/token",
        &[
            ("grant_type", "refresh_token"),
            ("client_id", &fixture.oauth_client_id),
            ("refresh_token", fresh_refresh),
        ],
    );
    assert_eq!(revoked_refresh.status().as_u16(), 400);
}

#[test]
fn unknown_refresh_token_does_not_issue_a_new_access_token() {
    let fixture = fixture();
    let refresh = "contract-nonexistent-refresh-token";
    let response = Target::from_env().post_form(
        "/token",
        &[
            ("grant_type", "refresh_token"),
            ("client_id", &fixture.oauth_client_id),
            ("refresh_token", refresh),
        ],
    );
    assert_eq!(response.status().as_u16(), 400);
    let body = response.text().expect("OAuth error body");
    assert!(!body.contains("access_token"));
    assert!(!body.contains(refresh));
}

#[test]
fn password_post_without_login_csrf_cannot_resume_mobile_authorization() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = authorization_path(&fixture.oauth_client_id, &fixture.oauth_redirect_uri);
    let start = target.get_html(&path);
    assert_eq!(start.status().as_u16(), 302);
    login_csrf(&target);

    let response = target.post_html_form(
        "/login",
        &[
            ("email".to_string(), fixture.primary_email),
            ("password".to_string(), "password".to_string()),
        ],
    );
    if let Some(location) = response.headers().get("location") {
        let location = location.to_str().expect("login redirect");
        assert!(!location.starts_with("/authorize?"));
        assert!(!location.starts_with(&fixture.oauth_redirect_uri));
    }
    let retry = target.get_html(&path);
    assert_eq!(retry.status().as_u16(), 302);
    assert_eq!(retry.headers()["location"], "/login");
}

#[test]
fn enrolled_second_factor_blocks_password_only_mobile_authorization() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = authorization_path(&fixture.oauth_client_id, &fixture.oauth_redirect_uri);
    let start = target.get_html(&path);
    assert_eq!(start.status().as_u16(), 302);
    assert_eq!(start.headers()["location"], "/login");
    let csrf = login_csrf(&target);
    let response = target.post_html_form(
        "/login",
        &[
            ("email".to_string(), fixture.oauth_mfa_email),
            ("password".to_string(), "password".to_string()),
            ("authenticity_token".to_string(), csrf),
        ],
    );
    if let Some(location) = response.headers().get("location") {
        let location = location.to_str().expect("second factor redirect");
        assert!(!location.starts_with(&fixture.oauth_redirect_uri));
    }
    let authorize = target.get_html(&path);
    assert_ne!(authorize.status().as_u16(), 200);
    if let Some(location) = authorize.headers().get("location") {
        let location = location.to_str().expect("authorization redirect");
        assert!(!location.starts_with(&fixture.oauth_redirect_uri));
    }
}

#[test]
fn consent_post_without_csrf_cannot_issue_a_code() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = authorization_path(&fixture.oauth_client_id, &fixture.oauth_redirect_uri);
    let start = target.get_html(&path);
    assert_eq!(start.status().as_u16(), 302);
    assert_eq!(start.headers()["location"], "/login");
    let csrf = login_csrf(&target);
    let login = target.post_html_form(
        "/login",
        &[
            ("email".to_string(), fixture.primary_email),
            ("password".to_string(), "password".to_string()),
            ("authenticity_token".to_string(), csrf),
        ],
    );
    assert_eq!(login.status().as_u16(), 302);
    let consent_path = login.headers()["location"]
        .to_str()
        .expect("consent redirect")
        .to_string();
    assert!(consent_path.starts_with("/authorize?"));
    let consent = target.get_html(&consent_path);
    assert_eq!(consent.status().as_u16(), 200);
    let document = Html::parse_document(&consent.text().expect("consent HTML"));
    let selector = Selector::parse("form#authorize-form").unwrap();
    let form = document.select(&selector).next().expect("consent form");
    let action = form.value().attr("action").expect("consent action");
    let inputs = Selector::parse("input[name]").unwrap();
    let fields: Vec<(String, String)> = form
        .select(&inputs)
        .filter(|input| {
            input.value().attr("type") != Some("submit")
                && input.value().attr("name") != Some("authenticity_token")
        })
        .map(|input| {
            (
                input.value().attr("name").unwrap().to_string(),
                input.value().attr("value").unwrap_or("").to_string(),
            )
        })
        .collect();
    let denied = target.post_html_form(action, &fields);
    if let Some(location) = denied.headers().get("location") {
        let location = location
            .to_str()
            .expect("consent redirect after CSRF denial");
        assert!(!location.starts_with(&fixture.oauth_redirect_uri));
    }
    assert_ne!(denied.status().as_u16(), 200);
}

#[test]
fn consent_rejects_unregistered_client_redirect_and_scope_without_issuing_codes() {
    let fixture = fixture();
    for (field, value) in [
        ("client_id", "contract-unknown-mobile-client"),
        ("redirect_uri", "io.damacus.medtracker.contract:/other"),
        ("scope[]", "contract-unsupported-scope"),
    ] {
        let (target, action, mut fields) = consent_for_primary_account();
        fields.retain(|(name, _)| name != field);
        fields.push((field.to_string(), value.to_string()));
        let denied = target.post_html_form(&action, &fields);
        if field == "redirect_uri" {
            if let Some(location) = denied.headers().get("location") {
                assert!(!location
                    .to_str()
                    .expect("denial redirect")
                    .starts_with(value));
            }
        }
        assert_no_authorization_code(denied, &fixture.oauth_redirect_uri);
    }
}

#[test]
fn consent_requires_an_s256_challenge_before_issuing_a_code() {
    let fixture = fixture();
    for (field, value) in [
        ("code_challenge", None),
        ("code_challenge_method", Some("plain")),
    ] {
        let (target, action, mut fields) = consent_for_primary_account();
        fields.retain(|(name, _)| name != field);
        if let Some(value) = value {
            fields.push((field.to_string(), value.to_string()));
        }
        let denied = target.post_html_form(&action, &fields);
        assert_no_authorization_code(denied, &fixture.oauth_redirect_uri);
    }
}
