use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::Response;
use serde_json::{json, Value};

fn body(response: Response) -> Value {
    response.json().expect("JSON response")
}

fn etag(response: &Response) -> String {
    response.headers()["etag"]
        .to_str()
        .expect("ETag header")
        .to_owned()
}

fn request_id(response: &Response) -> String {
    response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned()
}

fn assert_audit(
    target: &Target,
    fixture: &Fixture,
    id: &str,
    method: &str,
    controller: &str,
    action: &str,
    status: u16,
) {
    let response = target.get(
        &format!(
            "/api/v1/households/{}/admin/audit_logs",
            fixture.household_id
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let audit = body(response);
    let events = audit["data"].as_array().unwrap();
    assert!(events.len() <= 100);
    let matching: Vec<_> = events
        .iter()
        .filter(|event| event["request_id"] == id)
        .collect();
    assert_eq!(matching.len(), 1, "one visible audit event for {id}");
    assert_eq!(matching[0]["event_type"], "api.request");
    assert_eq!(matching[0]["metadata"]["http_method"], method);
    assert_eq!(matching[0]["metadata"]["controller"], controller);
    assert_eq!(matching[0]["metadata"]["action"], action);
    assert_eq!(matching[0]["metadata"]["status"], status);
}

fn assert_utc_second_timestamp(value: &Value) {
    let timestamp = value.as_str().expect("timestamp string");
    assert_eq!(timestamp.len(), 20);
    assert_eq!(&timestamp[4..5], "-");
    assert_eq!(&timestamp[7..8], "-");
    assert_eq!(&timestamp[10..11], "T");
    assert_eq!(&timestamp[13..14], ":");
    assert_eq!(&timestamp[16..17], ":");
    assert!(timestamp.ends_with('Z'));
}

fn medications_path(fixture: &Fixture) -> String {
    format!("/api/v1/households/{}/medications", fixture.household_id)
}

fn create_medication(target: &Target, fixture: &Fixture, name: &str) -> (Value, String) {
    let response = target.post_json_authorized(
        &medications_path(fixture),
        &fixture.access_token,
        &json!({"medication": {
            "name": name,
            "location_id": fixture.primary_location_id,
            "dose_amount": "2.125",
            "dose_unit": "ml",
            "current_supply": "80.00",
            "reorder_threshold": "10.25"
        }}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let created_etag = etag(&response);
    let created = body(response)["data"].clone();
    assert_eq!(created["name"], name);
    assert_eq!(created["location_id"], fixture.primary_location_id);
    assert_eq!(
        created["location_portable_id"],
        fixture.primary_location_portable_id
    );
    assert_eq!(created["dose_amount"], "2.125");
    assert_eq!(created["current_supply"], "80.0");
    assert_eq!(created["reorder_threshold"], "10.25");
    assert!(created["portable_id"]
        .as_str()
        .is_some_and(|id| !id.is_empty()));
    assert_utc_second_timestamp(&created["updated_at"]);
    (created, created_etag)
}

#[test]
fn medication_catalog_paginates_and_preserves_conditional_writes() {
    let target = Target::from_env();
    let fixture = fixture();
    let base = medications_path(&fixture);
    let (created, initial_etag) = create_medication(&target, &fixture, "Contract catalog medicine");
    let path = format!("{base}/{}", created["id"]);

    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(etag(&response), initial_etag);
    assert_eq!(body(response)["data"]["id"], created["id"]);

    let response = target.get(
        &format!("{base}?page=1&per_page=1"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let first_page = body(response);
    assert_eq!(first_page["meta"]["page"], 1);
    assert_eq!(first_page["meta"]["per_page"], 1);
    assert_eq!(first_page["data"].as_array().unwrap().len(), 1);
    assert!(first_page["meta"]["total_count"].as_u64().unwrap() >= 2);
    let response = target.get(
        &format!("{base}?page=2&per_page=1"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let second_page = body(response);
    assert_eq!(second_page["meta"]["page"], 2);
    assert_ne!(second_page["data"][0]["id"], first_page["data"][0]["id"]);
    assert_eq!(
        second_page["meta"]["total_count"],
        first_page["meta"]["total_count"]
    );

    let response = target.patch_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"medication": {"friendly_name": "Liquid dose", "current_supply": "79.75"}}),
        &initial_etag,
    );
    assert_eq!(response.status().as_u16(), 200);
    let patched_etag = etag(&response);
    assert_ne!(patched_etag, initial_etag);
    let patched = body(response);
    assert_eq!(patched["data"]["display_name"], "Liquid dose");
    assert_eq!(patched["data"]["current_supply"], "79.75");

    let response = target.put_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"medication": {"name": "Stale replacement"}}),
        &initial_etag,
    );
    assert_eq!(response.status().as_u16(), 409);
    assert_eq!(body(response)["error"]["code"], "conflict");
    let response = target.put_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"medication": {"name": "Contract replacement medicine"}}),
        &patched_etag,
    );
    assert_eq!(response.status().as_u16(), 200);
    let replaced_etag = etag(&response);
    assert_ne!(replaced_etag, patched_etag);
    let replaced = body(response);
    assert_eq!(replaced["data"]["name"], "Contract replacement medicine");
    assert_eq!(replaced["data"]["current_supply"], "79.75");
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(etag(&response), replaced_etag);
    assert_eq!(
        body(response)["data"]["name"],
        "Contract replacement medicine"
    );
}

#[test]
fn medication_validation_and_person_scope_do_not_disclose_foreign_records() {
    let target = Target::from_env();
    let fixture = fixture();
    let base = medications_path(&fixture);
    let visible_path = format!("{base}/{}", fixture.managed_medication_id);
    let hidden_path = format!("{base}/{}", fixture.hidden_medication_id);
    let foreign_path = format!("{base}/{}", fixture.foreign_medication_id);

    let response = target.get(&base, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 200);
    let visible = body(response);
    let ids: Vec<&Value> = visible["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| &item["id"])
        .collect();
    assert!(ids.contains(&&json!(fixture.managed_medication_id)));
    assert!(!ids.contains(&&json!(fixture.hidden_medication_id)));
    assert!(!ids.contains(&&json!(fixture.foreign_medication_id)));
    let response = target.get(&visible_path, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 200);
    let response = target.get(&hidden_path, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 404);
    assert_eq!(body(response)["error"]["code"], "not_found");
    let response = target.get(&foreign_path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 404);
    let error = body(response);
    assert_eq!(error["error"]["code"], "not_found");
    assert!(!error.to_string().contains(&fixture.foreign_medication_name));

    let response = target.post_json_authorized(
        &base,
        &fixture.access_token,
        &json!({"medication": {"name": "", "location_id": fixture.primary_location_id, "reorder_threshold": "1"}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert!(body(response)["error"]["errors"]["name"].is_array());
    let response = target.post_json_authorized(
        &base,
        &fixture.access_token,
        &json!({"medication": {"name": "Wrong shelf", "location_id": fixture.foreign_location_id, "reorder_threshold": "1"}}),
    );
    assert_eq!(response.status().as_u16(), 404);
    let response = target.post_json_authorized(
        &base,
        &fixture.access_token,
        &json!({"medication": {"name": "Numeric stock", "location_id": fixture.primary_location_id, "current_supply": 1.25, "reorder_threshold": "1"}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(
        body(response)["error"]["errors"]["current_supply"][0],
        "must be a string"
    );

    let response = target.post_json_authorized(
        &base,
        &fixture.view_access_token,
        &json!({"medication": {"name": "Forbidden create", "location_id": fixture.primary_location_id, "reorder_threshold": "1"}}),
    );
    assert_eq!(response.status().as_u16(), 403);
    let response = target.patch_json(
        &visible_path,
        &fixture.view_access_token,
        &json!({"medication": {"name": "Forbidden edit"}}),
    );
    assert_eq!(response.status().as_u16(), 403);
    let response = target.get(
        &format!(
            "/api/v1/households/{}/medications",
            fixture.foreign_household_id
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 403);
}

#[test]
fn stock_removal_keeps_decimal_history_and_rejects_changed_replays() {
    let target = Target::from_env();
    let fixture = fixture();
    let (medication, _) = create_medication(&target, &fixture, "Contract stock removal medicine");
    let path = format!(
        "{}/{}/stock_removals",
        medications_path(&fixture),
        medication["id"]
    );
    let payload = json!({"stock_removal": {
        "quantity": "1.25", "reason": "dropped", "note": "Broken bottle",
        "submission_id": "a1000000-0000-4000-8000-000000000001"
    }});
    let response = target.post_json_authorized(&path, &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 201);
    let first_request_id = request_id(&response);
    assert_audit(
        &target,
        &fixture,
        &first_request_id,
        "POST",
        "api/v1/stock_removals",
        "create",
        201,
    );
    let first = body(response)["data"].clone();
    assert_eq!(first["quantity"], "1.25");
    assert_eq!(first["previous_quantity"], "80");
    assert_eq!(first["remaining_quantity"], "78.75");
    assert_eq!(first["reason"], "dropped");
    assert_eq!(first["note"], "Broken bottle");
    assert_eq!(
        first["medication_id"],
        medication["id"].as_i64().unwrap().to_string()
    );
    assert!(first["actor_membership_id"]
        .as_str()
        .is_some_and(|id| !id.is_empty()));
    assert_utc_second_timestamp(&first["created_at"]);
    let response = target.post_json_authorized(&path, &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 201);
    let replay_request_id = request_id(&response);
    assert_audit(
        &target,
        &fixture,
        &replay_request_id,
        "POST",
        "api/v1/stock_removals",
        "create",
        201,
    );
    assert_eq!(body(response)["data"], first);
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(body(response)["meta"]["total_count"], 1);
    let response = target.get(
        &format!("{}/{}", medications_path(&fixture), medication["id"]),
        Some(&fixture.access_token),
    );
    assert_eq!(body(response)["data"]["current_supply"], "78.75");

    let changed = json!({"stock_removal": {
        "quantity": "2", "reason": "dropped", "note": "Broken bottle",
        "submission_id": "a1000000-0000-4000-8000-000000000001"
    }});
    let response = target.post_json_authorized(&path, &fixture.access_token, &changed);
    assert_eq!(response.status().as_u16(), 422);
    let response = target.post_json_authorized(
        &path,
        &fixture.access_token,
        &json!({"stock_removal": {"quantity": "0.001", "reason": "dropped", "submission_id": "a1000000-0000-4000-8000-000000000002"}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    let response = target.post_json_authorized(
        &path,
        &fixture.access_token,
        &json!({"stock_removal": {"quantity": "1", "reason": "unsupported", "submission_id": "a1000000-0000-4000-8000-000000000003"}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    let response = target.post_json_authorized(
        &path,
        &fixture.access_token,
        &json!({"stock_removal": {"quantity": "1", "reason": "dropped", "submission_id": "a1000000-0000-4000-8000-000000000004"}}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let second_request_id = request_id(&response);
    assert_audit(
        &target,
        &fixture,
        &second_request_id,
        "POST",
        "api/v1/stock_removals",
        "create",
        201,
    );
    let second = body(response)["data"].clone();
    assert_eq!(second["previous_quantity"], "78.75");
    assert_eq!(second["remaining_quantity"], "77.75");
    let response = target.get(
        &format!("{path}?page=1&per_page=1"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let first_page = body(response);
    assert_eq!(first_page["meta"]["total_count"], 2);
    assert_eq!(first_page["data"][0]["id"], second["id"]);
    let response = target.get(
        &format!("{path}?page=2&per_page=1"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let second_page = body(response);
    assert_eq!(second_page["data"][0]["id"], first["id"]);

    let response = target.get(
        &format!(
            "{}/{}/stock_removals",
            medications_path(&fixture),
            fixture.foreign_medication_id
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 404);
    assert!(!body(response)
        .to_string()
        .contains(&fixture.foreign_medication_name));
    let foreign_path = format!(
        "{}/{}/stock_removals",
        medications_path(&fixture),
        fixture.foreign_medication_id
    );
    let response = target.post_json_authorized(&foreign_path, &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 404);
    assert!(!body(response)
        .to_string()
        .contains(&fixture.foreign_medication_name));
    let managed_path = format!(
        "{}/{}/stock_removals",
        medications_path(&fixture),
        fixture.managed_medication_id
    );
    let response = target.post_json_authorized(&managed_path, &fixture.view_access_token, &payload);
    assert_eq!(response.status().as_u16(), 403);
    let response = target.get(&managed_path, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 403);
}

#[test]
fn inventory_adjustment_and_reorder_transitions_retain_http_state_and_audit() {
    let target = Target::from_env();
    let fixture = fixture();
    let (medication, _) = create_medication(&target, &fixture, "Contract inventory medicine");
    let path = format!("{}/{}", medications_path(&fixture), medication["id"]);
    let response = target.patch_json(
        &format!("{path}/adjust_inventory"),
        &fixture.access_token,
        &json!({"adjustment": {"new_quantity": "15.12", "reason": "counted"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let adjust_request_id = request_id(&response);
    assert_audit(
        &target,
        &fixture,
        &adjust_request_id,
        "PATCH",
        "api/v1/medications",
        "adjust_inventory",
        200,
    );
    assert_eq!(body(response)["data"]["current_supply"], "15.12");
    let response = target.patch_json(
        &format!("{path}/adjust_inventory"),
        &fixture.access_token,
        &json!({"adjustment": {"new_quantity": "-1", "reason": "bad"}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    let response = target.patch_json(
        &format!("{path}/adjust_inventory"),
        &fixture.access_token,
        &json!({"adjustment": {"new_quantity": 10}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(
        body(response)["error"]["errors"]["new_quantity"][0],
        "must be a string"
    );

    let response = target.patch_json(
        &format!("{path}/mark_as_ordered"),
        &fixture.access_token,
        &json!({"order_details": {"supplier": "Contract pharmacy", "quantity": "20.50", "expected_arrival_on": "2026-10-03"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let ordered_request_id = request_id(&response);
    assert_audit(
        &target,
        &fixture,
        &ordered_request_id,
        "PATCH",
        "api/v1/medications",
        "mark_as_ordered",
        200,
    );
    assert_eq!(body(response)["data"]["reorder_status"], "ordered");
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(body(response)["data"]["reorder_status"], "ordered");
    let response = target.patch_json(
        &format!("{path}/mark_as_received"),
        &fixture.access_token,
        &json!({}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let first_received_request_id = request_id(&response);
    assert_audit(
        &target,
        &fixture,
        &first_received_request_id,
        "PATCH",
        "api/v1/medications",
        "mark_as_received",
        200,
    );
    let received = body(response);
    assert_eq!(received["data"]["reorder_status"], "received");
    assert_eq!(received["data"]["current_supply"], "15.12");
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(body(response)["data"]["reorder_status"], "received");

    let managed_path = format!(
        "{}/{}",
        medications_path(&fixture),
        fixture.managed_medication_id
    );
    let response = target.patch_json(
        &format!("{managed_path}/adjust_inventory"),
        &fixture.view_access_token,
        &json!({"adjustment": {"new_quantity": "100"}}),
    );
    assert_eq!(response.status().as_u16(), 403);
    let response = target.patch_json(
        &format!("{managed_path}/mark_as_ordered"),
        &fixture.view_access_token,
        &json!({}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let managed_ordered_request_id = request_id(&response);
    assert_audit(
        &target,
        &fixture,
        &managed_ordered_request_id,
        "PATCH",
        "api/v1/medications",
        "mark_as_ordered",
        200,
    );
    assert_eq!(body(response)["data"]["reorder_status"], "ordered");
    let response = target.patch_json(
        &format!("{managed_path}/mark_as_received"),
        &fixture.view_access_token,
        &json!({}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let received_request_id = request_id(&response);
    assert_audit(
        &target,
        &fixture,
        &received_request_id,
        "PATCH",
        "api/v1/medications",
        "mark_as_received",
        200,
    );
    assert_eq!(body(response)["data"]["reorder_status"], "received");
    let hidden_path = format!(
        "{}/{}",
        medications_path(&fixture),
        fixture.hidden_medication_id
    );
    let response = target.patch_json(
        &format!("{hidden_path}/mark_as_ordered"),
        &fixture.view_access_token,
        &json!({}),
    );
    assert_eq!(response.status().as_u16(), 404);

    let audit_path = format!(
        "/api/v1/households/{}/admin/audit_logs",
        fixture.household_id
    );
    let response = target.get(&audit_path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let audit = body(response);
    assert!(audit["data"].as_array().unwrap().iter().any(|event| {
        event["event_type"] == "api.request"
            && event["metadata"]["controller"] == "api/v1/medications"
            && event["metadata"]["action"] == "adjust_inventory"
            && event["metadata"]["status"] == 200
    }));
}

#[test]
fn audit_list_retains_only_the_newest_hundred_request_events() {
    let target = Target::from_env();
    let fixture = fixture();
    let (medication, _) = create_medication(&target, &fixture, "Contract audit cap medicine");
    let path = format!(
        "{}/{}/adjust_inventory",
        medications_path(&fixture),
        medication["id"]
    );
    let mut request_ids = Vec::new();
    for quantity in 20..=120 {
        let response = target.patch_json(
            &path,
            &fixture.access_token,
            &json!({"adjustment": {"new_quantity": quantity.to_string(), "reason": "counted"}}),
        );
        assert_eq!(response.status().as_u16(), 200);
        request_ids.push(request_id(&response));
        assert_eq!(
            body(response)["data"]["current_supply"]
                .as_str()
                .unwrap()
                .parse::<f64>()
                .unwrap(),
            f64::from(quantity)
        );
    }
    let response = target.get(
        &format!(
            "/api/v1/households/{}/admin/audit_logs",
            fixture.household_id
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let audit = body(response);
    let events = audit["data"].as_array().unwrap();
    assert_eq!(events.len(), 100);
    let visible_ids: Vec<_> = events
        .iter()
        .map(|event| event["request_id"].as_str().unwrap())
        .collect();
    let expected_ids: Vec<_> = request_ids
        .iter()
        .rev()
        .take(100)
        .map(String::as_str)
        .collect();
    assert_eq!(visible_ids, expected_ids);
    assert!(events.iter().all(|event| {
        event["event_type"] == "api.request" && event["metadata"]["action"] == "adjust_inventory"
    }));
}

#[test]
fn medication_filters_portable_ids_and_invalid_writes_preserve_state() {
    let target = Target::from_env();
    let fixture = fixture();
    let base = medications_path(&fixture);
    let (created, initial_tag) = create_medication(&target, &fixture, "Contract edge medicine");
    let path = format!("{base}/{}", created["portable_id"].as_str().unwrap());

    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(etag(&response), initial_tag);
    assert_eq!(body(response)["data"], created);

    let response = target.get(
        &format!("{base}?updated_since=1970-01-01T00%3A00%3A00Z&per_page=100"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert!(body(response)["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["id"] == created["id"]));
    let response = target.get(
        &format!("{base}?updated_since=2099-01-01T00%3A00%3A00Z"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(body(response)["meta"]["total_count"], 0);
    let response = target.get(
        &format!("{base}?updated_since=invalid"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 422);

    let response = target.patch_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"medication": {"name": "", "current_supply": "70"}}),
        &initial_tag,
    );
    assert_eq!(response.status().as_u16(), 422);
    assert!(body(response)["error"]["errors"]["name"].is_array());
    let response = target.patch_json(
        &path,
        &fixture.access_token,
        &json!({"medication": {"location_id": fixture.foreign_location_id}}),
    );
    assert_eq!(response.status().as_u16(), 404);
    let response = target.put_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"medication": {"name": "", "current_supply": "60"}}),
        &initial_tag,
    );
    assert_eq!(response.status().as_u16(), 422);
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(etag(&response), initial_tag);
    assert_eq!(body(response)["data"], created);
}

#[test]
fn stock_history_caps_pages_at_portable_medication_identity() {
    let target = Target::from_env();
    let fixture = fixture();
    let (medication, _) = create_medication(&target, &fixture, "Contract portable stock medicine");
    let path = format!(
        "{}/{}/stock_removals",
        medications_path(&fixture),
        medication["portable_id"].as_str().unwrap()
    );
    let response = target.post_json_authorized(
        &path,
        &fixture.access_token,
        &json!({"stock_removal": {"quantity": "1.25", "reason": "dropped",
            "submission_id": "a3000000-0000-4000-8000-000000000001"}}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let removal = body(response)["data"].clone();
    let response = target.get(
        &format!("{path}?per_page=1000"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let history = body(response);
    assert_eq!(history["meta"]["per_page"], 100);
    assert_eq!(history["meta"]["total_count"], 1);
    assert_eq!(history["data"][0], removal);
}

#[test]
fn reorder_actions_reject_invalid_quantities_and_foreign_records() {
    let target = Target::from_env();
    let fixture = fixture();
    let (medication, _) = create_medication(&target, &fixture, "Contract reorder edge medicine");
    let path = format!("{}/{}", medications_path(&fixture), medication["id"]);
    let response = target.patch_json(
        &format!("{path}/mark_as_ordered"),
        &fixture.access_token,
        &json!({"order_details": {"quantity": 1.25}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(
        body(response)["error"]["errors"]["quantity"][0],
        "must be a string"
    );
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(
        body(response)["data"]["reorder_status"],
        medication["reorder_status"]
    );
    let response = target.patch_json(
        &format!("{path}/adjust_inventory"),
        &fixture.access_token,
        &json!({"adjustment": {"new_quantity": "invalid", "reason": "counted"}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(
        body(response)["data"]["current_supply"],
        medication["current_supply"]
    );

    let foreign = format!(
        "{}/{}",
        medications_path(&fixture),
        fixture.foreign_medication_id
    );
    for action in ["mark_as_ordered", "mark_as_received"] {
        let response = target.patch_json(
            &format!("{foreign}/{action}"),
            &fixture.access_token,
            &json!({}),
        );
        assert_eq!(response.status().as_u16(), 404);
        assert!(!body(response)
            .to_string()
            .contains(&fixture.foreign_medication_name));
    }
}

#[test]
fn delegated_creator_can_read_unlinked_medication_without_exposing_it_to_other_members() {
    let target = Target::from_env();
    let fixture = fixture();
    let base = medications_path(&fixture);
    let response = target.post_json_authorized(
        &base,
        &fixture.delegated_access_token,
        &json!({"medication": {"name": "Contract delegated medicine",
            "location_id": fixture.primary_location_id, "dose_amount": "1.25", "dose_unit": "ml"}}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let created = body(response)["data"].clone();
    let path = format!("{base}/{}", created["portable_id"].as_str().unwrap());
    let response = target.get(&path, Some(&fixture.delegated_access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(body(response)["data"], created);
    let response = target.get(
        &format!("{base}?per_page=100"),
        Some(&fixture.delegated_access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert!(body(response)["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["id"] == created["id"]));
    let response = target.get(&path, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 404);
    assert!(!body(response)
        .to_string()
        .contains("Contract delegated medicine"));
    let mut page = 1;
    loop {
        let response = target.get(
            &format!("{base}?page={page}&per_page=1"),
            Some(&fixture.view_access_token),
        );
        assert_eq!(response.status().as_u16(), 200);
        let collection = body(response);
        assert_eq!(collection["meta"]["page"], page);
        assert_eq!(collection["meta"]["per_page"], 1);
        assert!(collection["data"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["id"] != created["id"]));
        if page >= collection["meta"]["total_count"].as_u64().unwrap() {
            break;
        }
        page += 1;
    }
}
