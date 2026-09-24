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

fn assignments_path(fixture: &Fixture) -> String {
    format!(
        "/api/v1/households/{}/person_medications",
        fixture.household_id
    )
}

fn periods_path(fixture: &Fixture) -> String {
    format!(
        "/api/v1/households/{}/medication_pause_periods",
        fixture.household_id
    )
}

fn assert_audit_action(
    target: &Target,
    fixture: &Fixture,
    request_id: &str,
    controller: &str,
    method: &str,
    action: &str,
    status: u16,
) {
    let path = format!(
        "/api/v1/households/{}/admin/audit_logs",
        fixture.household_id
    );
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let audit = body(response);
    let matching: Vec<_> = audit["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|event| {
            event["event_type"] == "api.request"
                && event["request_id"] == request_id
                && event["metadata"]["controller"] == controller
                && event["metadata"]["http_method"] == method
                && event["metadata"]["action"] == action
                && event["metadata"]["status"] == status
        })
        .collect();
    assert_eq!(matching.len(), 1);
}

fn create_assignment(target: &Target, fixture: &Fixture, name: &str) -> (Value, String) {
    let response = target.post_json_authorized(
        &format!("/api/v1/households/{}/medications", fixture.household_id),
        &fixture.access_token,
        &json!({"medication": {
            "name": name,
            "location_id": fixture.primary_location_id,
            "dose_amount": "1.25",
            "dose_unit": "ml",
            "reorder_threshold": "5"
        }}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let medication = body(response)["data"].clone();
    let response = target.post_json_authorized(
        &assignments_path(fixture),
        &fixture.access_token,
        &json!({"person_medication": {
            "person_id": fixture.managed_person_portable_id,
            "medication_id": medication["portable_id"],
            "dose_amount": "1.250",
            "dose_unit": "ml",
            "administration_kind": "as_needed",
            "max_daily_doses": 3,
            "min_hours_between_doses": "4.0",
            "dose_cycle": "daily"
        }}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let tag = etag(&response);
    let id = request_id(&response);
    let created = body(response)["data"].clone();
    assert_eq!(created["person_id"], fixture.managed_person_id);
    assert_eq!(
        created["person_portable_id"],
        fixture.managed_person_portable_id
    );
    assert_eq!(created["medication_id"], medication["id"]);
    assert_eq!(created["medication_portable_id"], medication["portable_id"]);
    assert_eq!(created["dose_amount"], "1.25");
    assert_eq!(created["dose_unit"], "ml");
    assert_eq!(created["administration_kind"], "as_needed");
    assert_eq!(created["min_hours_between_doses"], "4.0");
    assert_eq!(created["max_daily_doses"], 3);
    assert_eq!(created["dose_cycle"], "daily");
    assert_eq!(created["active"], true);
    assert_eq!(created["paused"], false);
    assert_eq!(created["can_manage"], true);
    assert_utc_second_timestamp(&created["updated_at"]);
    assert_audit_action(
        target,
        fixture,
        &id,
        "api/v1/person_medications",
        "POST",
        "create",
        201,
    );
    (created, tag)
}

#[test]
fn assignments_are_person_scoped_paginated_and_retrievable_by_portable_id() {
    let target = Target::from_env();
    let fixture = fixture();
    let base = assignments_path(&fixture);
    let (created, tag) = create_assignment(&target, &fixture, "Contract assignment list");

    let response = target.get(&base, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 200);
    let collection = body(response);
    let rows = collection["data"].as_array().unwrap();
    assert!(rows
        .iter()
        .any(|row| row["id"] == fixture.managed_assignment_id));
    assert!(rows.iter().any(|row| row["id"] == created["id"]));
    assert!(!rows
        .iter()
        .any(|row| row["id"] == fixture.hidden_assignment_id));
    assert!(!rows
        .iter()
        .any(|row| row["id"] == fixture.foreign_assignment_id));
    assert!(rows.iter().all(|row| row["can_manage"] == false));

    let response = target.get(
        &format!("{base}?page=1&per_page=1"),
        Some(&fixture.view_access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let first = body(response);
    assert_eq!(first["meta"]["page"], 1);
    assert_eq!(first["meta"]["per_page"], 1);
    assert!(first["meta"]["total_count"].as_u64().unwrap() >= 2);
    assert_eq!(first["data"].as_array().unwrap().len(), 1);
    let response = target.get(
        &format!("{base}?page=2&per_page=1"),
        Some(&fixture.view_access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let second = body(response);
    assert_eq!(second["meta"]["total_count"], first["meta"]["total_count"]);
    assert_ne!(second["data"][0]["id"], first["data"][0]["id"]);

    let path = format!("{base}/{}", created["portable_id"].as_str().unwrap());
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(etag(&response), tag);
    let fetched = body(response)["data"].clone();
    assert_eq!(fetched["id"], created["id"]);
    assert!(fetched["current_pause_period"].is_null());

    for (id, token) in [
        (fixture.hidden_assignment_id, &fixture.view_access_token),
        (fixture.foreign_assignment_id, &fixture.access_token),
    ] {
        let response = target.get(&format!("{base}/{id}"), Some(token));
        assert_eq!(response.status().as_u16(), 404);
        assert_eq!(body(response)["error"]["code"], "not_found");
    }
    let response = target.get(
        &format!(
            "/api/v1/households/{}/person_medications",
            fixture.foreign_household_id
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 403);
}

#[test]
fn assignment_create_patch_and_put_validate_and_retain_decimal_state() {
    let target = Target::from_env();
    let fixture = fixture();
    let base = assignments_path(&fixture);
    let (created, original_tag) = create_assignment(&target, &fixture, "Contract assignment edit");
    let path = format!("{base}/{}", created["id"]);

    let response = target.patch_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"person_medication": {"notes": "After food", "dose_amount": "2.25"}}),
        &original_tag,
    );
    assert_eq!(response.status().as_u16(), 200);
    let patched_tag = etag(&response);
    let id = request_id(&response);
    let patched = body(response)["data"].clone();
    assert_ne!(patched_tag, original_tag);
    assert_eq!(patched["notes"], "After food");
    assert_eq!(patched["dose_amount"], "2.25");
    assert_audit_action(
        &target,
        &fixture,
        &id,
        "api/v1/person_medications",
        "PATCH",
        "update",
        200,
    );

    let response = target.put_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"person_medication": {"notes": "Stale"}}),
        &original_tag,
    );
    assert_eq!(response.status().as_u16(), 409);
    assert_eq!(body(response)["error"]["code"], "conflict");
    let response = target.put_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"person_medication": {"notes": "Replaced note"}}),
        &patched_tag,
    );
    assert_eq!(response.status().as_u16(), 200);
    let replaced_tag = etag(&response);
    let id = request_id(&response);
    let replaced = body(response)["data"].clone();
    assert_ne!(replaced_tag, patched_tag);
    assert_eq!(replaced["notes"], "Replaced note");
    assert_eq!(replaced["dose_amount"], "2.25");
    assert_audit_action(
        &target,
        &fixture,
        &id,
        "api/v1/person_medications",
        "PUT",
        "update",
        200,
    );
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(etag(&response), replaced_tag);
    assert_eq!(body(response)["data"]["notes"], "Replaced note");

    let response = target.patch_json(
        &path,
        &fixture.access_token,
        &json!({"person_medication": {"dose_amount": "-5"}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert!(body(response)["error"]["errors"]["dose_amount"].is_array());
    let response = target.post_json_authorized(
        &base,
        &fixture.access_token,
        &json!({"person_medication": {
            "person_id": fixture.managed_person_portable_id,
            "medication_id": fixture.managed_medication_portable_id,
            "dose_amount": "1", "dose_unit": "ml"
        }}),
    );
    assert_eq!(response.status().as_u16(), 422);
    let response = target.post_json_authorized(
        &base,
        &fixture.access_token,
        &json!({"person_medication": {
            "person_id": fixture.foreign_person_portable_id,
            "medication_id": fixture.managed_medication_portable_id,
            "dose_amount": "1", "dose_unit": "ml"
        }}),
    );
    assert_eq!(response.status().as_u16(), 404);
    assert!(!body(response)
        .to_string()
        .contains(&fixture.foreign_person_name));
    let response = target.post_json_authorized(
        &base,
        &fixture.access_token,
        &json!({"person_medication": {
            "person_id": fixture.managed_person_portable_id,
            "medication_id": fixture.foreign_medication_portable_id,
            "dose_amount": "1", "dose_unit": "ml"
        }}),
    );
    assert_eq!(response.status().as_u16(), 404);
    assert!(!body(response)
        .to_string()
        .contains(&fixture.foreign_medication_name));
    let response = target.post_json_authorized(
        &base,
        &fixture.view_access_token,
        &json!({"person_medication": {
            "person_id": fixture.managed_person_portable_id,
            "medication_id": fixture.managed_medication_portable_id,
            "dose_amount": "1", "dose_unit": "ml"
        }}),
    );
    assert_eq!(response.status().as_u16(), 403);
    let response = target.patch_json(
        &path,
        &fixture.view_access_token,
        &json!({"person_medication": {"notes": "Forbidden"}}),
    );
    assert_eq!(response.status().as_u16(), 403);
    let response = target.patch_json(
        &format!("{base}/{}", fixture.foreign_assignment_id),
        &fixture.access_token,
        &json!({"person_medication": {"notes": "Foreign"}}),
    );
    assert_eq!(response.status().as_u16(), 404);
}

#[test]
#[ignore = "Rails currently rounds assignment dose amounts beyond two decimal places"]
fn assignment_preserves_three_decimal_dose_amount() {
    let target = Target::from_env();
    let fixture = fixture();
    let (created, tag) = create_assignment(&target, &fixture, "Contract assignment precision");
    let path = format!("{}/{}", assignments_path(&fixture), created["id"]);
    let response = target.patch_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"person_medication": {"dose_amount": "2.125"}}),
        &tag,
    );
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(body(response)["data"]["dose_amount"], "2.125");
}

#[test]
fn assignment_legacy_pause_resume_and_reorder_retain_state() {
    let target = Target::from_env();
    let fixture = fixture();
    let (first, _) = create_assignment(&target, &fixture, "Contract order first");
    let (second, _) = create_assignment(&target, &fixture, "Contract order second");
    let first_path = format!(
        "{}/{}",
        assignments_path(&fixture),
        first["portable_id"].as_str().unwrap()
    );
    let second_path = format!(
        "{}/{}",
        assignments_path(&fixture),
        second["portable_id"].as_str().unwrap()
    );

    let response = target.patch_json(
        &format!("{second_path}/pause"),
        &fixture.access_token,
        &json!({}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let id = request_id(&response);
    let paused = body(response)["data"].clone();
    assert_eq!(paused["paused"], true);
    assert_eq!(paused["active"], false);
    assert_eq!(
        paused["current_pause_period"]["reason"],
        "reason_not_recorded"
    );
    assert_eq!(paused["current_pause_period"]["legacy_context"], true);
    let period_id = paused["current_pause_period"]["id"].clone();
    assert_audit_action(
        &target,
        &fixture,
        &id,
        "api/v1/person_medications",
        "PATCH",
        "pause",
        200,
    );

    let response = target.patch_json(
        &format!("{second_path}/resume"),
        &fixture.access_token,
        &json!({}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let id = request_id(&response);
    let resumed = body(response)["data"].clone();
    assert_eq!(resumed["paused"], false);
    assert_eq!(resumed["active"], true);
    assert!(resumed["current_pause_period"].is_null());
    assert_audit_action(
        &target,
        &fixture,
        &id,
        "api/v1/person_medications",
        "PATCH",
        "resume",
        200,
    );
    let response = target.get(&second_path, Some(&fixture.access_token));
    assert_eq!(body(response)["data"]["paused"], false);
    let history_path = format!(
        "{}?source_type=person_medication&source_id={}",
        periods_path(&fixture),
        second["portable_id"].as_str().unwrap()
    );
    let response = target.get(&history_path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let history = body(response);
    assert_eq!(history["meta"]["total_count"], 1);
    assert_eq!(history["data"][0]["id"], period_id);
    assert_utc_second_timestamp(&history["data"][0]["ended_at"]);

    let response = target.patch_json(
        &format!("{second_path}/reorder"),
        &fixture.access_token,
        &json!({"direction": "up"}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let id = request_id(&response);
    let reordered = body(response)["data"].clone();
    assert_eq!(reordered["position"], first["position"]);
    assert_audit_action(
        &target,
        &fixture,
        &id,
        "api/v1/person_medications",
        "PATCH",
        "reorder",
        200,
    );
    let response = target.get(&first_path, Some(&fixture.access_token));
    assert_eq!(body(response)["data"]["position"], second["position"]);
    let response = target.get(&second_path, Some(&fixture.access_token));
    assert_eq!(body(response)["data"]["position"], first["position"]);
    let response = target.patch_json(
        &format!("{second_path}/reorder"),
        &fixture.access_token,
        &json!({"direction": "sideways"}),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(body(response)["data"]["position"], first["position"]);
    let response = target.patch_json(
        &format!("{second_path}/pause"),
        &fixture.view_access_token,
        &json!({}),
    );
    assert_eq!(response.status().as_u16(), 403);
    let response = target.patch_json(
        &format!("{second_path}/resume"),
        &fixture.view_access_token,
        &json!({}),
    );
    assert_eq!(response.status().as_u16(), 403);
}

#[test]
fn pause_periods_preserve_reason_actor_pagination_and_addressed_resume() {
    let target = Target::from_env();
    let fixture = fixture();
    let (assignment, _) = create_assignment(&target, &fixture, "Contract period source");
    let source_id = assignment["portable_id"].as_str().unwrap();
    let base = periods_path(&fixture);
    let payload = json!({"medication_pause_period": {
        "source_type": "person_medication", "source_id": source_id,
        "reason": "out_of_supply", "note": "Delivery tomorrow",
        "started_at": "2020-01-01T00:00:00Z"
    }});
    let response = target.post_json_authorized(&base, &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 201);
    let first_tag = etag(&response);
    let id = request_id(&response);
    let first = body(response)["data"].clone();
    assert_eq!(first["source_type"], "person_medication");
    assert_eq!(first["source_id"], source_id);
    assert_eq!(first["reason"], "out_of_supply");
    assert_eq!(first["note"], "Delivery tomorrow");
    assert_eq!(first["legacy_context"], false);
    assert_utc_second_timestamp(&first["started_at"]);
    assert_ne!(first["started_at"], "2020-01-01T00:00:00Z");
    assert!(first["ended_at"].is_null());
    assert!(first["recorded_by_membership_id"].as_str().is_some());
    assert!(first["recorded_by_name"].as_str().is_some());
    assert_audit_action(
        &target,
        &fixture,
        &id,
        "api/v1/medication_pause_periods",
        "POST",
        "create",
        201,
    );
    let source_path = format!("{}/{}", assignments_path(&fixture), source_id);
    let response = target.get(&source_path, Some(&fixture.access_token));
    assert_eq!(
        body(response)["data"]["current_pause_period"]["id"],
        first["id"]
    );

    let response = target.post_json_authorized(&base, &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 201);
    assert_eq!(body(response)["data"]["id"], first["id"]);
    let resume_path = format!("{base}/{}/resume", first["id"].as_str().unwrap());
    let response = target.post_json_authorized(&resume_path, &fixture.access_token, &json!({}));
    assert_eq!(response.status().as_u16(), 200);
    let resumed_tag = etag(&response);
    let id = request_id(&response);
    let ended = body(response)["data"].clone();
    assert_ne!(resumed_tag, first_tag);
    assert_eq!(ended["reason"], first["reason"]);
    assert_eq!(ended["note"], first["note"]);
    assert_eq!(
        ended["recorded_by_membership_id"],
        first["recorded_by_membership_id"]
    );
    assert_utc_second_timestamp(&ended["ended_at"]);
    assert!(ended["resumed_by_name"].as_str().is_some());
    assert_audit_action(
        &target,
        &fixture,
        &id,
        "api/v1/medication_pause_periods",
        "POST",
        "resume",
        200,
    );

    let mut second_payload = payload.clone();
    second_payload["medication_pause_period"]["reason"] = json!("other");
    second_payload["medication_pause_period"]["note"] = json!("Second pause");
    let response = target.post_json_authorized(&base, &fixture.access_token, &second_payload);
    assert_eq!(response.status().as_u16(), 201);
    let second = body(response)["data"].clone();
    let response = target.post_json_authorized(&resume_path, &fixture.access_token, &json!({}));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(body(response)["data"]["id"], first["id"]);
    let response = target.get(&source_path, Some(&fixture.access_token));
    assert_eq!(
        body(response)["data"]["current_pause_period"]["id"],
        second["id"]
    );

    let filter =
        format!("{base}?source_type=person_medication&source_id={source_id}&page=1&per_page=1");
    let response = target.get(&filter, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 200);
    let history = body(response);
    assert_eq!(history["meta"]["total_count"], 2);
    assert_eq!(history["meta"]["page"], 1);
    assert_eq!(history["meta"]["per_page"], 1);
    assert_eq!(history["data"][0]["id"], second["id"]);
    let second_page =
        format!("{base}?source_type=person_medication&source_id={source_id}&page=2&per_page=1");
    let response = target.get(&second_page, Some(&fixture.view_access_token));
    assert_eq!(body(response)["data"][0]["id"], first["id"]);
}

#[test]
fn pause_period_validation_and_visibility_do_not_disclose_foreign_context() {
    let target = Target::from_env();
    let fixture = fixture();
    let (assignment, _) = create_assignment(&target, &fixture, "Contract period permissions");
    let source_id = assignment["portable_id"].as_str().unwrap();
    let base = periods_path(&fixture);
    let payload = json!({"medication_pause_period": {
        "source_type": "person_medication", "source_id": source_id,
        "reason": "side_effects", "note": "Private context"
    }});
    let response = target.post_json_authorized(&base, &fixture.view_access_token, &payload);
    assert_eq!(response.status().as_u16(), 403);
    let response = target.post_json_authorized(&base, &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 201);
    let period = body(response)["data"].clone();
    let resume_path = format!("{base}/{}/resume", period["id"].as_str().unwrap());
    let response =
        target.post_json_authorized(&resume_path, &fixture.view_access_token, &json!({}));
    assert_eq!(response.status().as_u16(), 403);
    let response = target.post_json_authorized(
        &base,
        &fixture.access_token,
        &json!({"medication_pause_period": {"source_type": "person_medication", "source_id": source_id, "reason": "reason_not_recorded"}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    let response = target.post_json_authorized(
        &base,
        &fixture.access_token,
        &json!({"medication_pause_period": {"source_type": "person_medication", "source_id": fixture.managed_assignment_id, "reason": "other"}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    let response = target.post_json_authorized(
        &base,
        &fixture.access_token,
        &json!({"medication_pause_period": {"source_type": "person_medication", "source_id": fixture.foreign_assignment_portable_id, "reason": "other", "note": "Private context"}}),
    );
    assert_eq!(response.status().as_u16(), 404);
    assert!(!body(response).to_string().contains("Private context"));
    let response = target.get(
        &format!(
            "{base}?source_type=person_medication&source_id={}",
            fixture.foreign_assignment_portable_id
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 404);
    let response = target.get(
        &format!(
            "{base}?source_type=person_medication&source_id={}",
            fixture.hidden_assignment_portable_id
        ),
        Some(&fixture.view_access_token),
    );
    assert_eq!(response.status().as_u16(), 404);
    let response = target.get(&base, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert!(body(response)["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == period["id"]));
}
