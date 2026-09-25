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

struct AuditActor<'a> {
    household_id: i64,
    access_token: &'a str,
    account_id: i64,
    membership_id: i64,
}

fn assert_request_audit(
    target: &Target,
    actor: AuditActor<'_>,
    id: &str,
    controller: &str,
    status: u16,
) {
    let path = format!("/api/v1/households/{}/admin/audit_logs", actor.household_id);
    let response = target.get(&path, Some(actor.access_token));
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
    assert_eq!(event["actor_account_id"], actor.account_id);
    assert_eq!(event["actor_membership_id"], actor.membership_id);
}

fn assert_primary_request_audit(
    target: &Target,
    fixture: &Fixture,
    id: &str,
    controller: &str,
    status: u16,
) {
    assert_request_audit(
        target,
        AuditActor {
            household_id: fixture.household_id,
            access_token: &fixture.access_token,
            account_id: fixture.account_id,
            membership_id: fixture.owner_membership_id,
        },
        id,
        controller,
        status,
    );
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
    assert_primary_request_audit(&target, &fixture, &id, "api/v1/medication_lookup", 200);
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
    assert_eq!(results[0]["code"], fixture.lookup_code);
    assert_eq!(results[0]["system"], "https://dmd.nhs.uk");
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
    assert_primary_request_audit(&target, &fixture, &id, "api/v1/medication_lookup", 503);
}

#[test]
fn upstream_lookup_filters_deterministic_results_by_form_and_strength() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = format!("{}?q=contractupstream", lookup_path(fixture.household_id));
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let payload: Value = response.json().expect("upstream lookup JSON");
    let results = payload["results"].as_array().expect("upstream results");
    assert_eq!(results.len(), 3);

    let form_path = format!("{path}&form=liquid");
    let response = target.get(&form_path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let payload: Value = response.json().expect("form-only lookup JSON");
    let results = payload["results"].as_array().expect("form-only results");
    let mut codes = results
        .iter()
        .map(|result| result["code"].as_str().expect("result code"))
        .collect::<Vec<_>>();
    codes.sort_unstable();
    assert_eq!(codes, ["contract-upstream-125", "contract-upstream-250"]);
    assert_eq!(payload["form"], "liquid");
    assert!(payload["strength"].is_null());

    let filtered_path = format!("{path}&form=liquid&strength=250mg%2F5ml");
    let response = target.get(&filtered_path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let payload: Value = response.json().expect("filtered lookup JSON");
    let results = payload["results"].as_array().expect("filtered results");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["code"], "contract-upstream-250");
    assert_eq!(
        results[0]["display"],
        "Contractupstream 250mg/5ml oral suspension"
    );
    assert_eq!(payload["query"], "contractupstream");
    assert_eq!(payload["form"], "liquid");
    assert_eq!(payload["strength"], "250mg/5ml");
}

#[test]
fn view_grant_limits_existing_medication_enrichment_and_disables_creation() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = lookup_path(fixture.household_id);
    let visible = target.get(
        &format!("{path}?q={}", fixture.lookup_barcode),
        Some(&fixture.view_access_token),
    );
    assert_eq!(visible.status().as_u16(), 200);
    let payload: Value = visible.json().expect("view lookup JSON");
    assert_eq!(
        payload["permissions"],
        json!({"can_create": false, "can_update": false})
    );
    let results = payload["results"].as_array().expect("view lookup results");
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0]["existing_medication"]["id"],
        fixture.managed_medication_id
    );

    let hidden = target.get(
        &format!("{path}?q={}", fixture.lookup_hidden_barcode),
        Some(&fixture.view_access_token),
    );
    assert_eq!(hidden.status().as_u16(), 200);
    let payload: Value = hidden.json().expect("hidden lookup JSON");
    assert_eq!(
        payload["permissions"],
        json!({"can_create": false, "can_update": false})
    );
    let results = payload["results"]
        .as_array()
        .expect("hidden lookup results");
    assert_eq!(results.len(), 1);
    assert!(results[0].get("existing_medication").is_none());
    assert!(!payload
        .to_string()
        .contains(&fixture.hidden_medication_id.to_string()));
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
    assert_primary_request_audit(
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
    let id = request_id(&response);
    let payload: Value = response.json().expect("suggestion JSON");
    assert!(payload["data"]["medication"].is_object());
    assert!(payload["data"]["doses"].is_array());
    assert!(payload["data"]["sources"].is_array());
    assert!(payload["data"]["errors"].is_array());
    assert_eq!(payload["data"]["errors"], json!(["ruby_llm_unconfigured"]));
    assert_request_audit(
        &target,
        AuditActor {
            household_id: fixture.lookup_paid_household_id,
            access_token: &fixture.lookup_paid_access_token,
            account_id: fixture.lookup_paid_account_id,
            membership_id: fixture.lookup_paid_membership_id,
        },
        &id,
        "api/v1/ai_medication_suggestions",
        200,
    );
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
