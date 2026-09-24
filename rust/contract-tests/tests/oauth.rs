use medtracker_contract_tests::{fixture, Target};
use serde_json::Value;

#[test]
fn discovery_advertises_authorization_code_pkce_and_local_endpoints() {
    let response = Target::from_env().get("/.well-known/oauth-authorization-server", None);
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().expect("JSON OAuth discovery");
    for endpoint in [
        "authorization_endpoint",
        "token_endpoint",
        "revocation_endpoint",
    ] {
        let value = body[endpoint].as_str().expect("endpoint URL");
        assert!(value.starts_with("http://") || value.starts_with("https://"));
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
fn invalid_code_redemption_rejects_verifier_without_leaking_it() {
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
