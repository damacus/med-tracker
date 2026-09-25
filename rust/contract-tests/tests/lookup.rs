use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::Response;
use serde_json::{json, Value};

fn lookup_path(household_id: i64) -> String {
    format!("/api/v1/households/{household_id}/medication_lookup")
}

fn suggestion_path(household_id: i64) -> String {
    format!("/api/v1/households/{household_id}/ai_medication_suggestions")
}

fn request_id(response: &Response) -> String {
    response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned()
}

fn assert_error(response: Response, status: u16, code: &str) -> String {
    assert_eq!(response.status().as_u16(), status);
    let id = request_id(&response);
    let payload: Value = response.json().expect("JSON error");
    assert_eq!(payload["error"]["code"], code);
    assert_eq!(payload["error"]["request_id"], id);
    assert!(payload["error"]["message"].as_str().is_some());
    assert!(payload.get("data").is_none());
    id
}

fn assert_request_audit(
    target: &Target,
    fixture: &Fixture,
    id: &str,
    controller: &str,
    status: u16,
) {
    let path = format!(
        "/api/v1/households/{}/admin/audit_logs",
        fixture.household_id
    );
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let payload: Value = response.json().expect("audit JSON");
    let events = payload["data"].as_array().expect("audit events");
    let event = events
        .iter()
        .find(|event| event["request_id"] == id)
        .expect("request-linked audit event");
    assert_eq!(event["event_type"], "api.request");
    assert_eq!(event["metadata"]["controller"], controller);
    assert_eq!(event["metadata"]["status"], status);
}

#[test]
fn lookup_returns_empty_results_and_permission_flags_without_external_search() {
    let target = Target::from_env();
    let fixture = fixture();
    let response = target.get(
        &lookup_path(fixture.household_id),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let id = request_id(&response);
    let payload: Value = response.json().expect("lookup JSON");
    assert_eq!(payload["results"], json!([]));
    assert_eq!(
        payload["permissions"],
        json!({"can_create": true, "can_update": false})
    );
    assert_request_audit(&target, &fixture, &id, "api/v1/medication_lookup", 200);
}

#[test]
fn lookup_resolves_a_disposable_catalogue_barcode_without_external_search() {
    let target = Target::from_env();
    let fixture = fixture();
    let response = target.get(
        &format!(
            "{}?q={}",
            lookup_path(fixture.household_id),
            fixture.lookup_barcode
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let payload: Value = response.json().expect("lookup JSON");
    assert_eq!(payload["query"], fixture.lookup_display);
    assert_eq!(payload["barcode"], fixture.lookup_barcode);
    assert_eq!(payload["barcode_resolution"]["status"], "resolved");
    assert_eq!(payload["barcode_resolution"]["source"], "contract_catalog");
    let results = payload["results"].as_array().expect("lookup results");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["display"], fixture.lookup_display);
    assert_eq!(results[0]["concept_class"], "AMPP");
    assert_eq!(results[0]["match_reason"], "barcode_match");
    assert_eq!(
        payload["permissions"],
        json!({"can_create": true, "can_update": false})
    );
}

#[test]
fn lookup_reports_unavailable_when_external_catalogue_is_unconfigured() {
    let target = Target::from_env();
    let fixture = fixture();
    let response = target.get(
        &format!(
            "{}?q=contractunknownmedicine&form=liquid&strength=250mg%2F5ml",
            lookup_path(fixture.household_id)
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 503);
    let id = request_id(&response);
    let payload: Value = response.json().expect("lookup JSON");
    assert_eq!(payload["results"], json!([]));
    assert_eq!(
        payload["error"],
        "Medication search is temporarily unavailable."
    );
    assert_request_audit(&target, &fixture, &id, "api/v1/medication_lookup", 503);
}

#[test]
fn lookup_requires_authentication_and_household_access() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = lookup_path(fixture.household_id);
    assert_error(target.get(&path, None), 401, "unauthorized");
    assert_error(
        target.get(&path, Some(&fixture.foreign_access_token)),
        403,
        "forbidden",
    );
    assert_error(
        target.get(&lookup_path(i64::MAX), Some(&fixture.access_token)),
        404,
        "not_found",
    );
    assert_error(
        target.get(&path, Some(&fixture.admin_target_access_token)),
        403,
        "forbidden",
    );
}

#[test]
fn suggestion_feature_gate_precedes_external_adapter() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = suggestion_path(fixture.household_id);
    let response = target.post_json_authorized(
        &path,
        &fixture.access_token,
        &json!({"medication": {"name": "Contract medicine"}}),
    );
    let id = assert_error(response, 404, "not_found");
    assert_request_audit(
        &target,
        &fixture,
        &id,
        "api/v1/ai_medication_suggestions",
        404,
    );
}

#[test]
fn paid_suggestion_returns_a_draft_shape_without_a_live_model() {
    let target = Target::from_env();
    let fixture = fixture();
    let response = target.post_json_authorized(
        &suggestion_path(fixture.lookup_paid_household_id),
        &fixture.lookup_paid_access_token,
        &json!({"medication": {"name": "Contract medicine"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let payload: Value = response.json().expect("suggestion JSON");
    assert!(payload["data"]["medication"].is_object());
    assert!(payload["data"]["doses"].is_array());
    assert!(payload["data"]["sources"].is_array());
    assert!(payload["data"]["errors"].is_array());
    assert_eq!(payload["data"]["errors"], json!(["ruby_llm_unconfigured"]));
}

#[test]
fn suggestions_require_authentication_and_household_access() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = suggestion_path(fixture.lookup_paid_household_id);
    let body = json!({"medication": {"name": "Contract medicine"}});
    assert_error(target.post_json(&path, &body), 401, "unauthorized");
    assert_error(
        target.post_json_authorized(&path, &fixture.foreign_access_token, &body),
        403,
        "forbidden",
    );
    assert_error(
        target.post_json_authorized(&suggestion_path(i64::MAX), &fixture.access_token, &body),
        404,
        "not_found",
    );
}

#[test]
fn paid_suggestion_accepts_absent_identity_and_unlisted_fields() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = suggestion_path(fixture.lookup_paid_household_id);
    let response =
        target.post_json_authorized(&path, &fixture.lookup_paid_access_token, &json!({}));
    assert_eq!(response.status().as_u16(), 200);
    let payload: Value = response.json().expect("suggestion JSON");
    assert!(payload["data"]["errors"].is_array());
    let response = target.post_json_authorized(
        &path,
        &fixture.lookup_paid_access_token,
        &json!({"medication": {"name": "Contract medicine", "unlisted": "ignored"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
}

#[test]
fn paid_suggestion_rejects_an_invalid_medication_shape() {
    let target = Target::from_env();
    let fixture = fixture();
    let response = target.post_json_authorized(
        &suggestion_path(fixture.lookup_paid_household_id),
        &fixture.lookup_paid_access_token,
        &json!({"medication": "invalid"}),
    );
    assert_error(response, 400, "bad_request");
}

#[test]
fn lookup_rate_limit_returns_retry_metadata() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = lookup_path(fixture.household_id);
    for _ in 0..60 {
        let response = target.get_from_local_client(&path, "198.51.100.61");
        assert_eq!(response.status().as_u16(), 401);
    }
    let response = target.get_from_local_client(&path, "198.51.100.61");
    assert_eq!(response.status().as_u16(), 429);
    assert!(
        response.headers()["retry-after"]
            .to_str()
            .unwrap()
            .parse::<u64>()
            .unwrap()
            > 0
    );
    let payload: Value = response.json().expect("rate limit JSON");
    assert_eq!(payload["error"]["code"], "rate_limited");
}

#[test]
fn suggestion_rate_limit_returns_retry_metadata() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = suggestion_path(fixture.household_id);
    let body = json!({"medication": {"name": "Contract medicine"}});
    for _ in 0..10 {
        let response = target.post_json_with_header(
            &path,
            &fixture.access_token,
            "X-Forwarded-For",
            "198.51.100.62",
            &body,
        );
        assert_eq!(response.status().as_u16(), 404);
    }
    let response = target.post_json_with_header(
        &path,
        &fixture.access_token,
        "X-Forwarded-For",
        "198.51.100.62",
        &body,
    );
    assert_eq!(response.status().as_u16(), 429);
    assert!(
        response.headers()["retry-after"]
            .to_str()
            .unwrap()
            .parse::<u64>()
            .unwrap()
            > 0
    );
    let payload: Value = response.json().expect("rate limit JSON");
    assert_eq!(payload["error"]["code"], "rate_limited");
}
