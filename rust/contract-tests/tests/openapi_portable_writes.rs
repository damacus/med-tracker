use medtracker_contract_tests::{Fixture, Target, fixture};
use reqwest::blocking::Response;
use serde_json::{Value, json};

const PASSPHRASE: &str = "contract portable secret";
const PASSPHRASE_HEADER: &str = "X-MedTracker-Portable-Passphrase";

fn path(household_id: i64, endpoint: &str) -> String {
    format!("/api/v1/households/{household_id}/{endpoint}")
}

fn body(response: Response) -> Value {
    response.json().expect("JSON response")
}

fn assert_keys(value: &Value, required: &[&str], allowed: &[&str]) {
    let object = value.as_object().expect("JSON object");
    for key in required {
        assert!(object.contains_key(*key), "missing key {key}");
    }
    for key in object.keys() {
        assert!(allowed.contains(&key.as_str()), "unexpected key {key}");
    }
}

fn assert_error(response: Response, status: u16) {
    assert_eq!(response.status().as_u16(), status);
    let request_id = response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned();
    let payload = body(response);
    assert_keys(&payload, &["error"], &["error"]);
    assert_keys(
        &payload["error"],
        &["code", "message", "request_id"],
        &["code", "message", "request_id", "errors"],
    );
    assert!(payload["error"]["code"].is_string());
    assert!(
        payload["error"]["message"]
            .as_str()
            .is_some_and(|s| !s.is_empty())
    );
    assert_eq!(payload["error"]["request_id"], request_id);
}

fn portable_export(target: &Target, fixture: &Fixture) -> Value {
    let response = target.get_with_header(
        &path(
            fixture.portable_source_household_id,
            "data_exports/encrypted_migration_bundle",
        ),
        &fixture.portable_source_access_token,
        PASSPHRASE_HEADER,
        PASSPHRASE,
    );
    assert_eq!(response.status().as_u16(), 200);
    let payload = body(response);
    assert_keys(&payload, &["data"], &["data"]);
    payload["data"].clone()
}

fn import(
    target: &Target,
    household_id: i64,
    token: &str,
    endpoint: &str,
    bundle: &Value,
) -> Response {
    target.post_json_with_header(
        &path(household_id, endpoint),
        token,
        PASSPHRASE_HEADER,
        PASSPHRASE,
        &json!({"bundle": bundle}),
    )
}

fn portable_snapshot(target: &Target, household_id: i64, token: &str) -> Value {
    let response = target.get(&path(household_id, "mobile_snapshot"), Some(token));
    assert_eq!(response.status().as_u16(), 200);
    body(response)["data"].clone()
}

#[test]
fn portable_imports_validate_strict_shapes_dry_run_and_apply_encrypted_bundle() {
    let target = Target::from_env();
    let fixture = fixture();
    let household_id = fixture.portable_target_household_id;

    let missing_bundle = target.post_json_with_header(
        &path(household_id, "portable_imports/dry_run"),
        &fixture.portable_target_access_token,
        PASSPHRASE_HEADER,
        PASSPHRASE,
        &json!({}),
    );
    assert_error(missing_bundle, 400);
    let missing_apply_bundle = target.post_json_with_header(
        &path(household_id, "portable_imports"),
        &fixture.portable_target_access_token,
        PASSPHRASE_HEADER,
        PASSPHRASE,
        &json!({}),
    );
    assert_error(missing_apply_bundle, 400);

    let source = portable_export(&target, &fixture);
    let before = portable_snapshot(&target, household_id, &fixture.portable_target_access_token);
    let extra_root_field = target.post_json_with_header(
        &path(household_id, "portable_imports/dry_run"),
        &fixture.portable_target_access_token,
        PASSPHRASE_HEADER,
        PASSPHRASE,
        &json!({"bundle": source, "unexpected": true}),
    );
    assert_error(extra_root_field, 422);

    let plan_response = import(
        &target,
        household_id,
        &fixture.portable_target_access_token,
        "portable_imports/dry_run",
        &source,
    );
    assert_eq!(plan_response.status().as_u16(), 200);
    let plan_response = body(plan_response);
    assert_keys(&plan_response, &["data"], &["data"]);
    let plan = &plan_response["data"];
    assert_keys(
        plan,
        &["applied", "counts", "conflicts", "errors"],
        &["applied", "counts", "conflicts", "errors"],
    );
    assert_eq!(plan["applied"], false);
    assert!(plan["counts"].is_object());
    assert!(plan["conflicts"].as_array().is_some_and(Vec::is_empty));
    assert!(plan["errors"].as_array().is_some_and(Vec::is_empty));
    assert_eq!(
        portable_snapshot(&target, household_id, &fixture.portable_target_access_token)["records"],
        before["records"]
    );

    let applied = import(
        &target,
        household_id,
        &fixture.portable_target_app_token,
        "portable_imports",
        &source,
    );
    assert_eq!(applied.status().as_u16(), 201);
    let applied = body(applied);
    assert_keys(&applied, &["data"], &["data"]);
    assert_keys(
        &applied["data"],
        &["applied", "counts", "conflicts", "errors"],
        &["applied", "counts", "conflicts", "errors"],
    );
    assert_eq!(applied["data"]["applied"], true);
    assert_eq!(applied["data"]["counts"], plan["counts"]);
    assert_error(
        target.get(
            &path(household_id, "mobile_snapshot"),
            Some(&fixture.portable_target_access_token),
        ),
        401,
    );
    let imported = portable_snapshot(&target, household_id, &fixture.portable_target_mobile_token);
    assert!(
        imported["records"]["people"]
            .as_array()
            .expect("imported people")
            .iter()
            .any(|person| person["portable_id"] == fixture.portable_source_person_portable_id)
    );
}

#[test]
fn sync_batch_validates_nonempty_strict_request_and_returns_portable_location_result() {
    let target = Target::from_env();
    let fixture = fixture();
    let route = path(fixture.household_id, "sync/batches");

    let empty_batch = target.post_json_authorized(
        &route,
        &fixture.access_token,
        &json!({"batch": {"operations": []}}),
    );
    assert_error(empty_batch, 422);
    let extra_batch_field = target.post_json_authorized(
        &route,
        &fixture.access_token,
        &json!({"batch": {"operations": [{"resource_type": "location", "action": "create", "attributes": {"name": "OpenAPI batch"}}], "unexpected": true}}),
    );
    assert_error(extra_batch_field, 422);

    let name = format!("OpenAPI batch {}", fixture.household_id);
    let response = target.post_json_authorized(
        &route,
        &fixture.access_token,
        &json!({"batch": {"operations": [{"resource_type": "location", "action": "create", "attributes": {"name": name}}]}}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let payload = body(response);
    assert_keys(&payload, &["data"], &["data"]);
    assert_keys(
        &payload["data"],
        &["applied", "results"],
        &["applied", "results"],
    );
    assert_eq!(payload["data"]["applied"], true);
    let results = payload["data"]["results"].as_array().expect("results");
    assert_eq!(results.len(), 1);
    let result = &results[0];
    assert_keys(
        result,
        &["index", "action", "record_type"],
        &[
            "index",
            "action",
            "record_type",
            "record_id",
            "record_portable_id",
            "etag",
            "replayed",
        ],
    );
    assert_eq!(result["index"], 0);
    assert_eq!(result["action"], "create");
    assert_eq!(result["record_type"], "Location");
    let portable_id = result["record_portable_id"].as_str().expect("portable ID");
    assert_eq!(portable_id.len(), 36);
    let location = target.get(
        &path(fixture.household_id, &format!("locations/{portable_id}")),
        Some(&fixture.access_token),
    );
    assert_eq!(location.status().as_u16(), 200);
    assert_eq!(body(location)["data"]["name"], name);
}
