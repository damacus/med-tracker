use medtracker_contract_tests::{Fixture, Target, fixture};
use reqwest::Method;
use reqwest::blocking::{Client, Response};
use serde_json::{Value, json};
use std::env;
use std::time::Duration as StdDuration;

fn body(response: Response) -> Value {
    response.json().expect("JSON response")
}

fn medications_path(fixture: &Fixture) -> String {
    format!("/api/v1/households/{}/medications", fixture.household_id)
}

fn create_medication(target: &Target, fixture: &Fixture, name: &str) -> Value {
    let response = target.post_json_authorized(
        &medications_path(fixture),
        &fixture.access_token,
        &json!({"medication": {
            "name": name,
            "location_id": fixture.primary_location_id,
            "dose_amount": "1.25",
            "dose_unit": "ml",
            "current_supply": "80.00",
            "reorder_threshold": "10.25"
        }}),
    );
    assert_eq!(response.status().as_u16(), 201);
    body(response)["data"].clone()
}

fn medication_path(fixture: &Fixture, medication: &Value) -> String {
    let portable_id = medication["portable_id"]
        .as_str()
        .expect("medication portable ID");
    format!("{}/{}", medications_path(fixture), portable_id)
}

fn stored_order_details(medication_id: i64) -> (Option<String>, Option<String>, Option<String>) {
    let mut db = postgres::Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract database URL"),
        postgres::NoTls,
    )
    .expect("contract database");
    let row = db
        .query_one(
            "SELECT order_supplier, order_quantity::text, expected_arrival_on::text FROM medications WHERE id = $1",
            &[&medication_id],
        )
        .expect("medication order details");
    (row.get(0), row.get(1), row.get(2))
}

fn stock_path(fixture: &Fixture, medication: &Value) -> String {
    format!("{}/stock_removals", medication_path(fixture, medication))
}

fn assert_error_status(response: Response, status: u16) -> Value {
    assert_eq!(response.status().as_u16(), status);
    let payload = body(response);
    assert!(payload["error"]["code"].is_string());
    assert!(payload["error"]["request_id"].is_string());
    assert!(payload["error"]["message"].is_string());
    assert!(payload.get("data").is_none());
    payload
}

fn patch_without_body(path: &str, token: &str) -> Response {
    let base = env::var("CONTRACT_BASE_URL").expect("contract API origin");
    Client::builder()
        .no_proxy()
        .build()
        .expect("HTTP client")
        .patch(format!("{base}{path}"))
        .bearer_auth(token)
        .header("Accept", "application/json")
        .send()
        .expect("bodyless PATCH response")
}

fn rate_limited(response: Response) {
    assert_eq!(response.status().as_u16(), 429);
    for header in [
        "retry-after",
        "ratelimit-limit",
        "ratelimit-remaining",
        "ratelimit-reset",
    ] {
        assert!(response.headers().get(header).is_some(), "missing {header}");
    }
    assert_eq!(response.headers()["ratelimit-limit"], "300");
    assert_eq!(response.headers()["ratelimit-remaining"], "0");
    assert_eq!(body(response)["error"]["code"], "rate_limited");
}

fn rate_request(
    client: &Client,
    base_url: &str,
    method: Method,
    path: &str,
    token: &str,
    payload: Option<&Value>,
) -> Response {
    let mut request = client
        .request(
            method,
            format!("{}{}", base_url.trim_end_matches('/'), path),
        )
        .bearer_auth(token);
    if let Some(payload) = payload {
        request = request.json(payload);
    }
    request.send().expect("rate-limit request")
}

#[test]
fn stock_removal_request_validation_and_collection_shape() {
    let target = Target::from_env();
    let fixture = fixture();
    let medication = create_medication(&target, &fixture, "Strict stock workflow probe");
    let assignment = target.post_json_authorized(
        &format!(
            "/api/v1/households/{}/person_medications",
            fixture.household_id
        ),
        &fixture.access_token,
        &json!({"person_medication": {
            "person_id": fixture.managed_person_portable_id,
            "medication_id": medication["portable_id"],
            "dose_amount": "2.50",
            "dose_unit": "ml",
            "administration_kind": "as_needed"
        }}),
    );
    assert_eq!(assignment.status().as_u16(), 201);
    let stock = stock_path(&fixture, &medication);
    let initial = body(target.get(
        &medication_path(&fixture, &medication),
        Some(&fixture.access_token),
    ));

    assert_error_status(
        target.post_json_authorized(&stock, &fixture.access_token, &json!({})),
        400,
    );
    for payload in [
        json!({"stock_removal": {"quantity": "1", "reason": "dropped", "submission_id": "b1000000-0000-4000-8000-000000000001", "extra": true}}),
        json!({"stock_removal": {"quantity": 1, "reason": "dropped", "submission_id": "b1000000-0000-4000-8000-000000000002"}}),
        json!({"stock_removal": {"quantity": "1", "reason": "unknown", "submission_id": "b1000000-0000-4000-8000-000000000003"}}),
        json!({"stock_removal": {"quantity": "1", "reason": "dropped", "submission_id": "not-a-uuid"}}),
        json!({"stock_removal": {"quantity": "1", "reason": "dropped", "note": null, "submission_id": "b1000000-0000-4000-8000-000000000004"}}),
    ] {
        assert_error_status(
            target.post_json_authorized(&stock, &fixture.access_token, &payload),
            422,
        );
    }
    let unchanged = body(target.get(
        &medication_path(&fixture, &medication),
        Some(&fixture.access_token),
    ));
    assert_eq!(
        unchanged["data"]["current_supply"],
        initial["data"]["current_supply"]
    );
    let empty = body(target.get(&stock, Some(&fixture.access_token)));
    assert_eq!(empty["data"], json!([]));
    assert_eq!(empty["meta"]["total_count"], 0);

    let created = target.post_json_authorized(
        &stock,
        &fixture.access_token,
        &json!({"stock_removal": {
            "quantity": "1.25",
            "reason": "dropped",
            "note": "Contract stock adjustment",
            "submission_id": "b1000000-0000-4000-8000-000000000005"
        }}),
    );
    assert_eq!(created.status().as_u16(), 201);
    let row = body(created)["data"].clone();
    let object = row.as_object().expect("stock-removal resource");
    assert_eq!(object.len(), 12);
    for field in [
        "id",
        "medication_id",
        "dosage_id",
        "quantity",
        "reason",
        "note",
        "submission_id",
        "previous_quantity",
        "remaining_quantity",
        "unit",
        "created_at",
        "actor_membership_id",
    ] {
        assert!(
            object.contains_key(field),
            "missing stock removal field {field}"
        );
    }
    assert!(row["id"].as_str().is_some_and(|v| !v.is_empty()));
    assert!(row["medication_id"].as_str().is_some_and(|v| !v.is_empty()));
    assert!(row["dosage_id"].is_null() || row["dosage_id"].is_string());
    assert_eq!(row["quantity"], "1.25");
    assert_eq!(row["reason"], "dropped");
    assert_eq!(row["note"], "Contract stock adjustment");
    assert!(row["actor_membership_id"].is_null() || row["actor_membership_id"].is_string());

    let collection = body(target.get(&stock, Some(&fixture.access_token)));
    assert_eq!(collection["data"], json!([row]));
    assert_eq!(collection["meta"]["total_count"], 1);
    assert_eq!(collection["meta"]["page"], 1);
    assert_eq!(collection["meta"]["per_page"], 20);
    assert_error_status(target.get(&stock, Some(&fixture.view_access_token)), 403);
    let after_removal = body(target.get(
        &medication_path(&fixture, &medication),
        Some(&fixture.access_token),
    ));

    for parameter in ["page", "per_page"] {
        for value in ["0", "-1", "not-a-number", "999999999999999999999999999999"] {
            assert_error_status(
                target.get(
                    &format!("{stock}?{parameter}={value}"),
                    Some(&fixture.access_token),
                ),
                422,
            );
        }
    }
    let after_invalid_pages = body(target.get(
        &medication_path(&fixture, &medication),
        Some(&fixture.access_token),
    ));
    assert_eq!(
        after_invalid_pages["data"]["current_supply"],
        after_removal["data"]["current_supply"]
    );
    assert_eq!(
        body(target.get(&stock, Some(&fixture.access_token)))["data"],
        json!([row])
    );
}

#[test]
fn inventory_adjustment_and_order_transitions_validate_and_persist_state() {
    let target = Target::from_env();
    let fixture = fixture();
    let medication = create_medication(&target, &fixture, "Inventory operation contract probe");
    let path = medication_path(&fixture, &medication);

    let adjusted = target.patch_json(
        &format!("{path}/adjust_inventory"),
        &fixture.access_token,
        &json!({"adjustment": {"new_quantity": "15.12", "reason": "counted"}}),
    );
    assert_eq!(adjusted.status().as_u16(), 200);
    assert_eq!(body(adjusted)["data"]["current_supply"], "15.12");
    assert_error_status(
        target.patch_json(
            &format!("{path}/adjust_inventory"),
            &fixture.access_token,
            &json!({}),
        ),
        400,
    );
    for payload in [
        json!({"adjustment": {"new_quantity": "16"}, "extra": true}),
        json!({"adjustment": {"new_quantity": "16", "extra": true}}),
        json!({"adjustment": {"new_quantity": 16}}),
        json!({"adjustment": {"new_quantity": "-1"}}),
    ] {
        assert_error_status(
            target.patch_json(
                &format!("{path}/adjust_inventory"),
                &fixture.access_token,
                &payload,
            ),
            422,
        );
    }
    let unchanged = body(target.get(&path, Some(&fixture.access_token)));
    assert_eq!(unchanged["data"]["current_supply"], "15.12");

    let before_order = body(target.get(&path, Some(&fixture.access_token)));
    let medication_id = medication["id"].as_i64().expect("medication ID");
    let before_order_details = stored_order_details(medication_id);
    assert_error_status(
        target.patch_json(
            &format!("{path}/mark_as_ordered"),
            &fixture.access_token,
            &json!({"order_details": {"supplier": "Ignored extra field"}, "extra": true}),
        ),
        422,
    );
    assert_error_status(
        target.patch_json(
            &format!("{path}/mark_as_ordered"),
            &fixture.access_token,
            &json!({"order_details": {"supplier": "Ignored inner field", "extra": true}}),
        ),
        422,
    );
    let after_invalid_order = body(target.get(&path, Some(&fixture.access_token)));
    assert_eq!(
        after_invalid_order["data"]["reorder_status"],
        before_order["data"]["reorder_status"]
    );
    assert_eq!(stored_order_details(medication_id), before_order_details);

    let ordered = target.patch_json(
        &format!("{path}/mark_as_ordered"),
        &fixture.access_token,
        &json!({"order_details": {
            "supplier": "Contract pharmacy",
            "quantity": "20.50",
            "expected_arrival_on": "2026-10-03"
        }}),
    );
    assert_eq!(ordered.status().as_u16(), 200);
    let ordered_body = body(ordered);
    assert_eq!(ordered_body["data"]["reorder_status"], "ordered");
    assert_eq!(
        stored_order_details(medication_id),
        (
            Some("Contract pharmacy".to_owned()),
            Some("20.50".to_owned()),
            Some("2026-10-03".to_owned())
        )
    );

    let bodyless_received =
        patch_without_body(&format!("{path}/mark_as_received"), &fixture.access_token);
    assert_eq!(bodyless_received.status().as_u16(), 200);

    let received = target.patch_json(
        &format!("{path}/mark_as_received"),
        &fixture.access_token,
        &json!({}),
    );
    assert_eq!(received.status().as_u16(), 200);
    assert_eq!(body(received)["data"]["reorder_status"], "received");
}

#[test]
fn all_five_stock_workflow_operations_use_the_shared_rate_limiter() {
    let target = Target::from_env();
    let fixture = fixture();
    let medication = create_medication(&target, &fixture, "Rate-limited stock workflow probe");
    let path = medication_path(&fixture, &medication);
    let stock = stock_path(&fixture, &medication);
    let ordered = target.patch_json(
        &format!("{path}/mark_as_ordered"),
        &fixture.access_token,
        &json!({"order_details": {"supplier": "Rate check", "quantity": "2"}}),
    );
    assert_eq!(ordered.status().as_u16(), 200);
    let received = target.patch_json(
        &format!("{path}/mark_as_received"),
        &fixture.access_token,
        &json!({}),
    );
    assert_eq!(received.status().as_u16(), 200);

    let base_url = env::var("CONTRACT_RATE_BASE_URL").expect("non-loopback API base URL");
    let client = Client::builder()
        .no_proxy()
        .timeout(StdDuration::from_secs(5))
        .build()
        .expect("rate-limit client");
    let mut limited = None;
    for _ in 0..601 {
        let response = rate_request(
            &client,
            &base_url,
            Method::GET,
            "/api/v1/capabilities",
            &fixture.access_token,
            None,
        );
        if response.status().as_u16() == 429 {
            limited = Some(response);
            break;
        }
        assert_eq!(response.status().as_u16(), 200);
    }
    rate_limited(limited.expect("shared rate limit response"));

    let malformed = json!({});
    rate_limited(rate_request(
        &client,
        &base_url,
        Method::GET,
        &stock,
        &fixture.access_token,
        None,
    ));
    rate_limited(rate_request(
        &client,
        &base_url,
        Method::POST,
        &stock,
        &fixture.access_token,
        Some(&malformed),
    ));
    rate_limited(rate_request(
        &client,
        &base_url,
        Method::PATCH,
        &format!("{path}/adjust_inventory"),
        &fixture.access_token,
        Some(&malformed),
    ));
    rate_limited(rate_request(
        &client,
        &base_url,
        Method::PATCH,
        &format!("{path}/mark_as_ordered"),
        &fixture.access_token,
        Some(&malformed),
    ));
    rate_limited(rate_request(
        &client,
        &base_url,
        Method::PATCH,
        &format!("{path}/mark_as_received"),
        &fixture.access_token,
        Some(&malformed),
    ));
}
