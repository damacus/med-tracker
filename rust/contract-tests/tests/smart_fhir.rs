use medtracker_contract_tests::{fixture, Target};
use scraper::{Html, Selector};
use serde_json::Value;
use url::Url;

const VERIFIER: &str = "contract-verifier-with-more-than-forty-three-characters";
const CHALLENGE: &str = "sIEAmHTSAwOYncK3AzYmthevluqX_MuVU227zeLfBY0";

fn csrf(html: &str) -> String {
    let document = Html::parse_document(html);
    let selector =
        Selector::parse("form[action='/login'] input[name='authenticity_token']").unwrap();
    document
        .select(&selector)
        .next()
        .and_then(|input| input.value().attr("value"))
        .expect("login CSRF token")
        .to_owned()
}

fn authorize_path(client_id: &str, redirect_uri: &str) -> String {
    let mut query = url::form_urlencoded::Serializer::new(String::new());
    query
        .append_pair("response_type", "code")
        .append_pair("response_mode", "query")
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("scope", "launch/patient patient/*.rs offline_access")
        .append_pair("state", "contract-smart-state")
        .append_pair("code_challenge", CHALLENGE)
        .append_pair("code_challenge_method", "S256");
    format!("/authorize?{}", query.finish())
}

fn consent_code(target: &Target, path: &str, redirect_uri: &str) -> String {
    let consent = target.get_html(path);
    assert_eq!(consent.status().as_u16(), 200);
    let html = consent.text().expect("SMART consent HTML");
    assert!(html.contains("Contract SMART issued client"));
    let document = Html::parse_document(&html);
    let cancel = Selector::parse("a[data-consent-cancel]").unwrap();
    let denied = document
        .select(&cancel)
        .next()
        .and_then(|link| link.value().attr("href"))
        .expect("consent denial URL");
    let denied = Url::parse(denied).expect("absolute denial URL");
    let registered = Url::parse(redirect_uri).expect("registered redirect URL");
    assert_eq!(denied.scheme(), registered.scheme());
    assert_eq!(denied.username(), registered.username());
    assert_eq!(denied.password(), registered.password());
    assert_eq!(denied.host(), registered.host());
    assert_eq!(denied.port(), registered.port());
    assert_eq!(denied.path(), registered.path());
    let denied_params: std::collections::HashMap<_, _> =
        denied.query_pairs().into_owned().collect();
    assert_eq!(denied_params.get("tenant").map(String::as_str), Some("7"));
    assert_eq!(
        denied_params.get("error").map(String::as_str),
        Some("access_denied")
    );
    assert_eq!(
        denied_params.get("state").map(String::as_str),
        Some("contract-smart-state")
    );

    let form_selector = Selector::parse("form#authorize-form").unwrap();
    let form = document
        .select(&form_selector)
        .next()
        .expect("SMART consent form");
    let inputs = Selector::parse("input[name]").unwrap();
    let fields: Vec<(String, String)> = form
        .select(&inputs)
        .filter(|input| input.value().attr("type") != Some("submit"))
        .map(|input| {
            (
                input.value().attr("name").unwrap().to_owned(),
                input.value().attr("value").unwrap_or("").to_owned(),
            )
        })
        .collect();
    for scope in ["launch/patient", "patient/*.rs", "offline_access"] {
        assert!(fields
            .iter()
            .any(|(name, value)| name == "scope[]" && value == scope));
    }
    assert!(fields
        .iter()
        .any(|(name, value)| name == "code_challenge" && value == CHALLENGE));
    let approved = target.post_html_form(form.value().attr("action").unwrap(), &fields);
    assert_eq!(approved.status().as_u16(), 302);
    let callback = Url::parse(approved.headers()["location"].to_str().unwrap()).unwrap();
    assert_eq!(callback.scheme(), registered.scheme());
    assert_eq!(callback.host(), registered.host());
    assert_eq!(callback.port(), registered.port());
    assert_eq!(callback.path(), registered.path());
    let callback_params: std::collections::HashMap<_, _> =
        callback.query_pairs().into_owned().collect();
    assert_eq!(
        callback_params.get("state").map(String::as_str),
        Some("contract-smart-state")
    );
    assert_eq!(callback_params.get("tenant").map(String::as_str), Some("7"));
    callback_params
        .get("code")
        .expect("authorization code")
        .to_owned()
}

#[test]
fn smart_consent_issues_patient_scoped_token_with_refresh_and_revoke_denial() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = authorize_path(&fixture.smart_client_id, &fixture.smart_redirect_uri);
    let start = target.get_html(&path);
    assert_eq!(start.status().as_u16(), 302);
    assert_eq!(start.headers()["location"], "/login");
    let login = target.get_html("/login");
    assert_eq!(login.status().as_u16(), 200);
    let login_token = csrf(&login.text().unwrap());
    let signed_in = target.post_html_form(
        "/login",
        &[
            ("email".to_owned(), fixture.primary_email.clone()),
            ("password".to_owned(), "password".to_owned()),
            ("authenticity_token".to_owned(), login_token),
        ],
    );
    assert_eq!(signed_in.status().as_u16(), 302);
    let consent_path = signed_in.headers()["location"].to_str().unwrap();
    assert!(consent_path.starts_with("/authorize?"));

    let invalid = target.get_html(&authorize_path(
        &fixture.smart_client_id,
        "https://attacker.example/callback",
    ));
    assert_eq!(invalid.status().as_u16(), 200);
    assert!(invalid.headers().get("location").is_none());
    assert!(invalid
        .text()
        .unwrap()
        .contains("Invalid or missing 'redirect_uri'"));

    let code = consent_code(&target, consent_path, &fixture.smart_redirect_uri);
    let missing_verifier = target.post_form(
        "/token",
        &[
            ("grant_type", "authorization_code"),
            ("client_id", &fixture.smart_client_id),
            ("redirect_uri", &fixture.smart_redirect_uri),
            ("code", &code),
        ],
    );
    assert_eq!(missing_verifier.status().as_u16(), 400);
    let missing_body: Value = missing_verifier.json().expect("missing verifier error");
    assert_eq!(missing_body["error"], "invalid_request");
    let wrong_verifier = target.post_form(
        "/token",
        &[
            ("grant_type", "authorization_code"),
            ("client_id", &fixture.smart_client_id),
            ("redirect_uri", &fixture.smart_redirect_uri),
            ("code", &code),
            (
                "code_verifier",
                "wrong-verifier-with-more-than-forty-three-characters",
            ),
        ],
    );
    assert_eq!(wrong_verifier.status().as_u16(), 400);
    assert!(!wrong_verifier.text().unwrap().contains("access_token"));
    let token = target.post_form(
        "/token",
        &[
            ("grant_type", "authorization_code"),
            ("client_id", &fixture.smart_client_id),
            ("redirect_uri", &fixture.smart_redirect_uri),
            ("code", &code),
            ("code_verifier", VERIFIER),
        ],
    );
    assert_eq!(token.status().as_u16(), 200);
    let issued: Value = token.json().expect("SMART token JSON");
    let access = issued["access_token"].as_str().expect("access token");
    let refresh = issued["refresh_token"].as_str().expect("refresh token");
    let patient_path = format!("/api/fhir/R4/Patient/{}", fixture.smart_patient_portable_id);
    let patient = target.get(&patient_path, Some(access));
    assert_eq!(patient.status().as_u16(), 200);
    let patient: Value = patient.json().expect("patient JSON");
    assert_eq!(patient["resourceType"], "Patient");
    assert_eq!(patient["id"], fixture.smart_patient_portable_id);
    assert_eq!(
        target
            .get(
                &format!(
                    "/api/fhir/R4/Patient/{}",
                    fixture.managed_person_portable_id
                ),
                Some(access),
            )
            .status()
            .as_u16(),
        404
    );

    let rotated = target.post_form(
        "/token",
        &[
            ("grant_type", "refresh_token"),
            ("client_id", &fixture.smart_client_id),
            ("refresh_token", refresh),
        ],
    );
    assert_eq!(rotated.status().as_u16(), 200);
    let rotated: Value = rotated.json().expect("rotated SMART token JSON");
    let fresh_access = rotated["access_token"].as_str().expect("new access token");
    let fresh_refresh = rotated["refresh_token"]
        .as_str()
        .expect("new refresh token");
    assert_ne!(fresh_refresh, refresh);
    assert_eq!(
        target
            .get(&patient_path, Some(fresh_access))
            .status()
            .as_u16(),
        200
    );
    assert_eq!(
        target
            .post_form(
                "/token",
                &[
                    ("grant_type", "refresh_token"),
                    ("client_id", &fixture.smart_client_id),
                    ("refresh_token", refresh),
                ],
            )
            .status()
            .as_u16(),
        400
    );

    let revoked = Target::from_env().post_json(
        "/revoke",
        &serde_json::json!({
            "client_id": fixture.smart_client_id,
            "token": fresh_access,
            "token_type_hint": "access_token"
        }),
    );
    assert_eq!(revoked.status().as_u16(), 200);
    assert_eq!(
        target
            .get(&patient_path, Some(fresh_access))
            .status()
            .as_u16(),
        401
    );
    assert_eq!(
        target
            .post_form(
                "/token",
                &[
                    ("grant_type", "refresh_token"),
                    ("client_id", &fixture.smart_client_id),
                    ("refresh_token", fresh_refresh),
                ],
            )
            .status()
            .as_u16(),
        400
    );
}
