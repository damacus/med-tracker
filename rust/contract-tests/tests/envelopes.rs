use medtracker_contract_tests::{fixture, Target};
use serde_json::{json, Value};

fn assert_error(
    response: reqwest::blocking::Response,
    status: u16,
    code: &str,
    message: &str,
) -> Value {
    assert_eq!(response.status().as_u16(), status);
    let request_id = response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned();
    assert!(!request_id.is_empty());
    let body: Value = response.json().expect("JSON error envelope");
    assert_eq!(body["error"]["code"], code);
    assert_eq!(body["error"]["message"], message);
    assert_eq!(body["error"]["request_id"], request_id);
    body
}

#[test]
fn missing_bearer_has_correlated_unauthorized_envelope() {
    let fixture = fixture();
    let path = format!("/api/v1/households/{}/people", fixture.household_id);
    let response = Target::from_env().get(&path, None);
    assert_error(response, 401, "unauthorized", "Authentication required");
}

#[test]
fn cross_household_denial_does_not_disclose_foreign_account() {
    let fixture = fixture();
    let path = format!("/api/v1/households/{}/me", fixture.foreign_household_id);
    let response = Target::from_env().get(&path, Some(&fixture.access_token));
    let body = assert_error(
        response,
        403,
        "forbidden",
        "You are not authorized to perform this action.",
    );
    assert!(!body.to_string().contains(&fixture.foreign_email));
}

#[test]
fn invalid_filter_has_correlated_unprocessable_envelope() {
    let fixture = fixture();
    let path = format!(
        "/api/v1/households/{}/people?updated_since=not-a-date",
        fixture.household_id
    );
    let response = Target::from_env().get(&path, Some(&fixture.access_token));
    assert_error(
        response,
        422,
        "unprocessable_content",
        "updated_since must be ISO8601",
    );
}

#[test]
fn invalid_profile_preference_is_rejected_without_changing_account_state() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = format!("/api/v1/households/{}/profile", fixture.household_id);
    let before = target.get(&path, Some(&fixture.access_token));
    assert_eq!(before.status().as_u16(), 200);
    assert!(before.headers()["cache-control"]
        .to_str()
        .unwrap()
        .contains("no-store"));
    let original: Value = before.json().expect("JSON profile");
    assert_eq!(
        original["data"]["account_id"],
        fixture.account_id.to_string()
    );

    let response = target.patch_json(
        &path,
        &fixture.access_token,
        &json!({"profile": {"time_zone": "Invalid/Place"}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    let body: Value = response.json().expect("JSON validation error");
    assert_eq!(body["error"]["code"], "validation_failed");
    assert_eq!(body["error"]["message"], "Validation failed");
    assert!(body["error"]["request_id"].is_string());
    assert!(body["error"]["errors"]["time_zone"].is_array());

    let after = target.get(&path, Some(&fixture.access_token));
    assert_eq!(after.status().as_u16(), 200);
    let current: Value = after.json().expect("JSON profile");
    assert_eq!(current["data"]["time_zone"], original["data"]["time_zone"]);
}

#[test]
fn rate_limited_api_response_has_retry_metadata() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = format!(
        "/api/v1/households/{}/data_exports/health_data",
        fixture.household_id
    );
    let client_ip = format!(
        "198.18.{}.{}",
        (fixture.household_id / 256) % 256,
        fixture.household_id % 256
    );
    for _ in 0..10 {
        let response = target.get_from_local_client(&path, &client_ip);
        assert_eq!(response.status().as_u16(), 401);
    }
    let response = target.get_from_local_client(&path, &client_ip);
    assert_eq!(response.status().as_u16(), 429);
    assert_eq!(response.headers()["ratelimit-limit"], "10");
    assert_eq!(response.headers()["ratelimit-remaining"], "0");
    assert!(
        response.headers()["retry-after"]
            .to_str()
            .unwrap()
            .parse::<u64>()
            .unwrap()
            > 0
    );
    assert_eq!(response.headers()["content-type"], "application/json");
    let body: Value = response.json().expect("JSON rate limit");
    assert_eq!(body["error"]["code"], "rate_limited");
}
