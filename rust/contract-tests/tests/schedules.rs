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

fn schedules_path(fixture: &Fixture) -> String {
    format!("/api/v1/households/{}/schedules", fixture.household_id)
}

fn schedule_payload(fixture: &Fixture) -> Value {
    json!({"schedule": {
        "person_id": fixture.managed_person_portable_id,
        "medication_id": fixture.managed_medication_portable_id,
        "dose_amount": "1.250",
        "dose_unit": "ml",
        "frequency": "Every Monday",
        "start_date": "2026-02-25",
        "end_date": "2099-12-31",
        "max_daily_doses": 2,
        "min_hours_between_doses": "8.0",
        "dose_cycle": "weekly",
        "schedule_type": "weekly",
        "schedule_config": {"weekdays": ["monday"], "times": ["08:00", "20:00"]},
        "notes": "Contract schedule"
    }})
}

fn create_schedule(target: &Target, fixture: &Fixture) -> (Value, String) {
    let response = target.post_json_authorized(
        &schedules_path(fixture),
        &fixture.access_token,
        &schedule_payload(fixture),
    );
    assert_eq!(response.status().as_u16(), 201);
    let tag = etag(&response);
    let request_id = request_id(&response);
    let created = body(response)["data"].clone();
    assert_eq!(created["person_id"], fixture.managed_person_id);
    assert_eq!(
        created["person_portable_id"],
        fixture.managed_person_portable_id
    );
    assert_eq!(created["medication_id"], fixture.managed_medication_id);
    assert_eq!(
        created["medication_portable_id"],
        fixture.managed_medication_portable_id
    );
    assert_eq!(created["dose_amount"], "1.25");
    assert_eq!(created["dose_unit"], "ml");
    assert_eq!(created["dose_cycle"], "weekly");
    assert_eq!(created["schedule_type"], "weekly");
    assert_eq!(
        created["schedule_config"],
        json!({"weekdays": ["monday"], "times": ["08:00", "20:00"]})
    );
    assert_eq!(created["start_date"], "2026-02-25");
    assert_eq!(created["end_date"], "2099-12-31");
    assert_eq!(created["max_daily_doses"], 2);
    assert_eq!(created["min_hours_between_doses"], "8.0");
    assert_eq!(created["active"], true);
    assert_eq!(created["paused"], false);
    assert_eq!(created["can_manage"], true);
    assert_utc_second_timestamp(&created["updated_at"]);
    assert_audit_action(target, fixture, &request_id, "POST", "create", 201);
    (created, tag)
}

#[test]
#[ignore = "Rails currently truncates fractional minimum hours"]
fn schedule_preserves_fractional_min_hours_between_doses() {
    let target = Target::from_env();
    let fixture = fixture();
    let mut payload = schedule_payload(&fixture);
    payload["schedule"]["min_hours_between_doses"] = json!("8.5");
    let response =
        target.post_json_authorized(&schedules_path(&fixture), &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 201);
    assert_eq!(body(response)["data"]["min_hours_between_doses"], "8.5");
}

fn assert_utc_second_timestamp(value: &Value) {
    let timestamp = value.as_str().expect("timestamp string");
    assert_eq!(timestamp.len(), 20);
    assert_eq!(&timestamp[10..11], "T");
    assert!(timestamp.ends_with('Z'));
}

fn request_id(response: &Response) -> String {
    let id = response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned();
    assert!(!id.is_empty());
    id
}

fn assert_audit_action(
    target: &Target,
    fixture: &Fixture,
    request_id: &str,
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
                && event["metadata"]["http_method"] == method
                && event["metadata"]["controller"] == "api/v1/schedules"
                && event["metadata"]["action"] == action
                && event["metadata"]["status"] == status
        })
        .collect();
    assert_eq!(matching.len(), 1);
}

#[test]
fn schedule_collection_is_person_scoped_and_paginated() {
    let target = Target::from_env();
    let fixture = fixture();
    let base = schedules_path(&fixture);
    let (created, _) = create_schedule(&target, &fixture);

    let response = target.get(&base, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 200);
    let collection = body(response);
    let rows = collection["data"].as_array().unwrap();
    assert!(rows
        .iter()
        .any(|row| row["id"] == fixture.managed_schedule_id));
    assert!(rows.iter().any(|row| row["id"] == created["id"]));
    assert!(!rows
        .iter()
        .any(|row| row["id"] == fixture.hidden_schedule_id));
    assert!(!rows
        .iter()
        .any(|row| row["id"] == fixture.foreign_schedule_id));
    assert!(rows.iter().all(|row| row["can_manage"] == false));
    let response = target.get(&base, Some(&fixture.care_access_token));
    assert_eq!(response.status().as_u16(), 200);
    let care_collection = body(response);
    assert!(care_collection["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == created["id"] && row["can_manage"] == true));

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
    assert_eq!(second["meta"]["page"], 2);
    assert_eq!(second["meta"]["total_count"], first["meta"]["total_count"]);
    assert_ne!(second["data"][0]["id"], first["data"][0]["id"]);

    let response = target.get(
        &format!(
            "/api/v1/households/{}/schedules",
            fixture.foreign_household_id
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 403);
}

#[test]
fn schedule_collection_filters_updates_and_normalizes_pagination() {
    let target = Target::from_env();
    let fixture = fixture();
    let base = schedules_path(&fixture);
    let (created, _) = create_schedule(&target, &fixture);

    let response = target.get(
        &format!("{base}?updated_since=1970-01-01T00%3A00%3A00Z&per_page=100"),
        Some(&fixture.view_access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert!(body(response)["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == created["id"]));
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
    assert_eq!(body(response)["error"]["code"], "unprocessable_content");

    let response = target.get(
        &format!("{base}?page=bogus&per_page=0"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let normalized = body(response);
    assert_eq!(normalized["meta"]["page"], 1);
    assert_eq!(normalized["meta"]["per_page"], 1);
    assert_eq!(normalized["data"].as_array().unwrap().len(), 1);
    let response = target.get(
        &format!("{base}?page=-3&per_page=1000"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let normalized = body(response);
    assert_eq!(normalized["meta"]["page"], 1);
    assert_eq!(normalized["meta"]["per_page"], 100);
}

#[test]
fn schedule_detail_uses_the_same_version_and_body_for_numeric_and_portable_ids() {
    let target = Target::from_env();
    let fixture = fixture();
    let numeric = format!(
        "{}/{}",
        schedules_path(&fixture),
        fixture.managed_schedule_id
    );
    let response = target.get(&numeric, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 200);
    let numeric_tag = etag(&response);
    let numeric_body = body(response);
    let portable_id = numeric_body["data"]["portable_id"].as_str().unwrap();
    let response = target.get(
        &format!("{}/{portable_id}", schedules_path(&fixture)),
        Some(&fixture.view_access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(etag(&response), numeric_tag);
    assert_eq!(body(response), numeric_body);
}

#[test]
fn schedule_source_dosage_links_only_to_a_matching_visible_option() {
    let target = Target::from_env();
    let fixture = fixture();
    let dosage_path = format!("/api/v1/households/{}/dosage_options", fixture.household_id);
    let response = target.post_json_authorized(
        &dosage_path,
        &fixture.access_token,
        &json!({"dosage_option": {
            "medication_id": fixture.managed_medication_portable_id,
            "amount": "2.5", "unit": "ml", "frequency": "Twice daily",
            "default_max_daily_doses": 2, "default_min_hours_between_doses": "8.0",
            "default_dose_cycle": "daily"
        }}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let dosage = body(response)["data"].clone();
    let mut payload = schedule_payload(&fixture);
    payload["schedule"]["source_dosage_option_id"] = dosage["portable_id"].clone();
    payload["schedule"]["dose_amount"] = json!("2.5");
    let response =
        target.post_json_authorized(&schedules_path(&fixture), &fixture.access_token, &payload);
    let status = response.status().as_u16();
    let result = body(response);
    assert_eq!(status, 201, "{result}");
    let created = result["data"].clone();
    assert_eq!(created["dose_amount"], "2.5");
    assert_eq!(
        created["medication_portable_id"],
        fixture.managed_medication_portable_id
    );
    let response = target.post_json_authorized(
        &format!("/api/v1/households/{}/medications", fixture.household_id),
        &fixture.access_token,
        &json!({"medication": {"name": "Different source option medicine",
            "location_id": fixture.primary_location_id,
            "dose_amount": "1", "dose_unit": "ml"}}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let alternate = body(response)["data"].clone();
    let path = format!(
        "{}/{}",
        schedules_path(&fixture),
        created["portable_id"].as_str().unwrap()
    );
    let response = target.patch_json(
        &path,
        &fixture.access_token,
        &json!({"schedule": {"medication_id": alternate["portable_id"]}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert!(body(response)["error"]["errors"]["source_dosage_option"].is_array());
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(
        body(response)["data"]["medication_id"],
        created["medication_id"]
    );

    payload["schedule"]["dose_amount"] = json!("1.25");
    let response =
        target.post_json_authorized(&schedules_path(&fixture), &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 422);
    assert!(body(response)["error"]["errors"]["source_dosage_option"].is_array());
    payload["schedule"]["source_dosage_option_id"] = json!(fixture.foreign_dosage_id.to_string());
    let response =
        target.post_json_authorized(&schedules_path(&fixture), &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 404);
    let response = target.post_json_authorized(
        &schedules_path(&fixture),
        &fixture.delegated_access_token,
        &schedule_payload(&fixture),
    );
    assert_eq!(response.status().as_u16(), 201);
}

#[test]
#[ignore = "Rails currently resolves an ungranted household dosage option before rejecting the medication mismatch"]
fn schedule_create_does_not_disclose_an_ungranted_source_dosage_option() {
    let target = Target::from_env();
    let fixture = fixture();
    let mut payload = schedule_payload(&fixture);
    payload["schedule"]["source_dosage_option_id"] = json!(fixture.hidden_dosage_id.to_string());
    payload["schedule"]["dose_amount"] = json!("1");
    let response = target.post_json_authorized(
        &schedules_path(&fixture),
        &fixture.delegated_access_token,
        &payload,
    );
    assert_eq!(response.status().as_u16(), 404);
}

#[test]
fn schedule_recurrence_retains_valid_rules() {
    let target = Target::from_env();
    let fixture = fixture();
    let mut payload = schedule_payload(&fixture);
    payload["schedule"]["schedule_type"] = json!("specific_dates");
    payload["schedule"]["schedule_config"] =
        json!({"dates": ["2026-04-21", "2026-04-24"], "times": ["08:00"]});
    let response =
        target.post_json_authorized(&schedules_path(&fixture), &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 201);
    let created = body(response)["data"].clone();
    let path = format!(
        "{}/{}",
        schedules_path(&fixture),
        created["portable_id"].as_str().unwrap()
    );
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(
        body(response)["data"]["schedule_config"],
        payload["schedule"]["schedule_config"]
    );
}

#[test]
#[ignore = "Rails currently returns 500 for an unknown schedule type"]
fn schedule_recurrence_rejects_unknown_type_without_server_error() {
    let target = Target::from_env();
    let fixture = fixture();
    let mut payload = schedule_payload(&fixture);
    payload["schedule"]["schedule_type"] = json!("unknown");
    let response =
        target.post_json_authorized(&schedules_path(&fixture), &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 422);
}

#[test]
#[ignore = "Rails currently accepts a recurrence time outside the OpenAPI time format"]
fn schedule_recurrence_rejects_invalid_config_time() {
    let target = Target::from_env();
    let fixture = fixture();
    let mut payload = schedule_payload(&fixture);
    payload["schedule"]["schedule_config"] = json!({"weekdays": ["monday"], "times": ["99:99"]});
    let response =
        target.post_json_authorized(&schedules_path(&fixture), &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 422);
}

#[test]
fn schedule_create_get_and_invalid_inputs_preserve_the_public_contract() {
    let target = Target::from_env();
    let fixture = fixture();
    let base = schedules_path(&fixture);
    let (created, created_etag) = create_schedule(&target, &fixture);
    let path = format!("{base}/{}", created["portable_id"].as_str().unwrap());
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(etag(&response), created_etag);
    assert!(created.get("current_pause_period").is_none());
    let mut fetched = body(response)["data"].clone();
    assert!(fetched["current_pause_period"].is_null());
    fetched
        .as_object_mut()
        .unwrap()
        .remove("current_pause_period");
    assert_eq!(fetched, created);

    let response = target.get(
        &format!("{base}/{}", fixture.managed_schedule_id),
        Some(&fixture.view_access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(body(response)["data"]["can_manage"], false);
    let response = target.get(
        &format!("{base}/{}", fixture.hidden_schedule_id),
        Some(&fixture.view_access_token),
    );
    assert_eq!(response.status().as_u16(), 404);
    assert_eq!(body(response)["error"]["code"], "not_found");
    let response = target.get(
        &format!("{base}/{}", fixture.foreign_schedule_id),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 404);
    assert_eq!(body(response)["error"]["code"], "not_found");

    let response = target.post_json_authorized(
        &base,
        &fixture.access_token,
        &json!({"schedule": {"person_id": fixture.managed_person_portable_id,
            "medication_id": fixture.managed_medication_portable_id,
            "start_date": "2026-02-25"}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    let errors = body(response)["error"]["errors"].clone();
    assert!(errors["dose_amount"].is_array());
    assert!(errors["dose_unit"].is_array());
    assert!(errors["end_date"].is_array());
    let mut numeric = schedule_payload(&fixture);
    numeric["schedule"]["dose_amount"] = json!(1.25);
    let response = target.post_json_authorized(&base, &fixture.access_token, &numeric);
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(
        body(response)["error"]["errors"]["dose_amount"][0],
        "must be a string"
    );

    let mut foreign_person = schedule_payload(&fixture);
    foreign_person["schedule"]["person_id"] = json!(fixture.foreign_person_portable_id);
    let response = target.post_json_authorized(&base, &fixture.access_token, &foreign_person);
    assert_eq!(response.status().as_u16(), 404);
    assert!(!body(response)
        .to_string()
        .contains(&fixture.foreign_person_name));
    let mut foreign_medication = schedule_payload(&fixture);
    foreign_medication["schedule"]["medication_id"] =
        json!(fixture.foreign_medication_id.to_string());
    let response = target.post_json_authorized(&base, &fixture.access_token, &foreign_medication);
    assert_eq!(response.status().as_u16(), 404);
    assert!(!body(response)
        .to_string()
        .contains(&fixture.foreign_medication_name));
    let response = target.post_json_authorized(
        &base,
        &fixture.view_access_token,
        &schedule_payload(&fixture),
    );
    assert_eq!(response.status().as_u16(), 403);
}

#[test]
fn schedule_patch_and_put_keep_etags_validation_and_audit() {
    let target = Target::from_env();
    let fixture = fixture();
    let (created, original_etag) = create_schedule(&target, &fixture);
    let base = schedules_path(&fixture);
    let path = format!("{base}/{}", created["id"]);

    let response = target.patch_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"schedule": {"frequency": "Every eight hours", "notes": "Revised"}}),
        &original_etag,
    );
    assert_eq!(response.status().as_u16(), 200);
    let patched_request_id = request_id(&response);
    let patched_etag = etag(&response);
    let patched = body(response)["data"].clone();
    assert_eq!(patched["frequency"], "Every eight hours");
    assert_eq!(patched["notes"], "Revised");
    assert_ne!(patched_etag, original_etag);
    assert_audit_action(
        &target,
        &fixture,
        &patched_request_id,
        "PATCH",
        "update",
        200,
    );

    let response = target.put_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"schedule": {"frequency": "Stale replacement"}}),
        &original_etag,
    );
    assert_eq!(response.status().as_u16(), 409);
    assert_eq!(body(response)["error"]["code"], "conflict");
    let response = target.put_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"schedule": {"frequency": "Twice daily"}}),
        &patched_etag,
    );
    assert_eq!(response.status().as_u16(), 200);
    let replaced_request_id = request_id(&response);
    let replaced_etag = etag(&response);
    let replaced = body(response)["data"].clone();
    assert_eq!(replaced["frequency"], "Twice daily");
    assert_eq!(replaced["notes"], "Revised");
    assert_eq!(replaced["dose_amount"], "1.25");
    assert_ne!(replaced_etag, patched_etag);
    assert_audit_action(
        &target,
        &fixture,
        &replaced_request_id,
        "PUT",
        "update",
        200,
    );
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(etag(&response), replaced_etag);
    assert!(replaced.get("current_pause_period").is_none());
    let mut fetched = body(response)["data"].clone();
    assert!(fetched["current_pause_period"].is_null());
    fetched
        .as_object_mut()
        .unwrap()
        .remove("current_pause_period");
    assert_eq!(fetched, replaced);

    let response = target.patch_json(
        &path,
        &fixture.access_token,
        &json!({"schedule": {"dose_amount": null}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert!(body(response)["error"]["errors"]["dose_amount"].is_array());
    let response = target.patch_json(
        &path,
        &fixture.view_access_token,
        &json!({"schedule": {"frequency": "Forbidden"}}),
    );
    assert_eq!(response.status().as_u16(), 403);
    let response = target.patch_json(
        &format!("{base}/{}", fixture.foreign_schedule_id),
        &fixture.access_token,
        &json!({"schedule": {"frequency": "Foreign"}}),
    );
    assert_eq!(response.status().as_u16(), 404);
}

#[test]
fn schedule_patch_relinks_medication_but_keeps_the_person_fixed() {
    let target = Target::from_env();
    let fixture = fixture();
    let (created, _) = create_schedule(&target, &fixture);
    let path = format!(
        "{}/{}",
        schedules_path(&fixture),
        created["portable_id"].as_str().unwrap()
    );
    let medications_path = format!("/api/v1/households/{}/medications", fixture.household_id);
    let response = target.post_json_authorized(
        &medications_path,
        &fixture.access_token,
        &json!({"medication": {"name": "Schedule relink medicine",
            "location_id": fixture.primary_location_id,
            "dose_amount": "1", "dose_unit": "ml"}}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let alternate = body(response)["data"].clone();
    let response = target.patch_json(
        &path,
        &fixture.access_token,
        &json!({"schedule": {"medication_id": alternate["portable_id"]}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let relinked = body(response)["data"].clone();
    assert_eq!(relinked["medication_id"], alternate["id"]);
    assert_eq!(relinked["medication_portable_id"], alternate["portable_id"]);
    assert_eq!(relinked["person_id"], created["person_id"]);

    let response = target.patch_json(
        &path,
        &fixture.access_token,
        &json!({"schedule": {"medication_id": fixture.foreign_medication_portable_id}}),
    );
    assert_eq!(response.status().as_u16(), 404);
    let response = target.patch_json(
        &path,
        &fixture.access_token,
        &json!({"schedule": {"person_id": fixture.hidden_person_id.to_string()}}),
    );
    assert_eq!(response.status().as_u16(), 404);
    assert_ne!(fixture.user_person_id, fixture.managed_person_id);
    let response = target.get(
        &format!(
            "/api/v1/households/{}/people/{}",
            fixture.household_id, fixture.user_person_id
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let response = target.patch_json(
        &path,
        &fixture.access_token,
        &json!({"schedule": {"person_id": fixture.user_person_id.to_string()}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(body(response)["data"]["person_id"], created["person_id"]);
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let persisted = body(response)["data"].clone();
    assert_eq!(persisted["medication_id"], alternate["id"]);
    assert_eq!(persisted["person_id"], created["person_id"]);
}

#[test]
fn schedule_full_put_replaces_mutable_fields_and_rejects_invalid_values() {
    let target = Target::from_env();
    let fixture = fixture();
    let (created, original_etag) = create_schedule(&target, &fixture);
    let path = format!(
        "{}/{}",
        schedules_path(&fixture),
        created["portable_id"].as_str().unwrap()
    );
    let response = target.put_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"schedule": {
            "person_id": fixture.user_person_id.to_string(),
            "medication_id": fixture.managed_medication_portable_id,
            "dose_amount": "2.5", "dose_unit": "tablet", "frequency": "Alternate days",
            "start_date": "2026-03-01", "end_date": "2098-12-31",
            "notes": "Complete replacement", "max_daily_doses": 3,
            "min_hours_between_doses": "6.0", "dose_cycle": "daily",
            "schedule_type": "every_other_day", "schedule_config": {"times": ["09:30"]}
        }}),
        &original_etag,
    );
    assert_eq!(response.status().as_u16(), 200);
    let replaced_etag = etag(&response);
    let replaced = body(response)["data"].clone();
    assert_ne!(replaced_etag, original_etag);
    assert_eq!(replaced["person_id"], created["person_id"]);
    assert_eq!(replaced["dose_amount"], "2.5");
    assert_eq!(replaced["dose_unit"], "tablet");
    assert_eq!(replaced["frequency"], "Alternate days");
    assert_eq!(replaced["start_date"], "2026-03-01");
    assert_eq!(replaced["end_date"], "2098-12-31");
    assert_eq!(replaced["notes"], "Complete replacement");
    assert_eq!(replaced["max_daily_doses"], 3);
    assert_eq!(replaced["min_hours_between_doses"], "6.0");
    assert_eq!(replaced["dose_cycle"], "daily");
    assert_eq!(replaced["schedule_type"], "every_other_day");
    assert_eq!(replaced["schedule_config"], json!({"times": ["09:30"]}));

    let response = target.put_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"schedule": {"dose_amount": 2.5}}),
        &replaced_etag,
    );
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(
        body(response)["error"]["errors"]["dose_amount"][0],
        "must be a string"
    );
    let response = target.put_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"schedule": {"end_date": "2026-02-28"}}),
        &replaced_etag,
    );
    assert_eq!(response.status().as_u16(), 422);
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(etag(&response), replaced_etag);
    let persisted = body(response)["data"].clone();
    assert_eq!(persisted["dose_amount"], replaced["dose_amount"]);
    assert_eq!(persisted["dose_unit"], replaced["dose_unit"]);
    assert_eq!(persisted["end_date"], replaced["end_date"]);
    assert_eq!(persisted["schedule_config"], replaced["schedule_config"]);
}

#[test]
fn schedule_legacy_pause_and_resume_authorize_and_preserve_repeated_transitions() {
    let target = Target::from_env();
    let fixture = fixture();
    let (created, _) = create_schedule(&target, &fixture);
    let path = format!(
        "{}/{}",
        schedules_path(&fixture),
        created["portable_id"].as_str().unwrap()
    );
    let pause = format!("{path}/pause");
    let resume = format!("{path}/resume");
    let response = target.patch_json(&resume, &fixture.access_token, &json!({}));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(body(response)["data"]["active"], true);
    for route in [&pause, &resume] {
        let response = target.patch_json(route, &fixture.view_access_token, &json!({}));
        assert_eq!(response.status().as_u16(), 403);
        let foreign = route.replace(
            created["portable_id"].as_str().unwrap(),
            &fixture.foreign_schedule_id.to_string(),
        );
        let response = target.patch_json(&foreign, &fixture.access_token, &json!({}));
        assert_eq!(response.status().as_u16(), 404);
    }
    let response = target.patch_json(&pause, &fixture.access_token, &json!({}));
    assert_eq!(response.status().as_u16(), 200);
    let first_period = body(response)["data"]["current_pause_period"]["id"].clone();
    let response = target.patch_json(&pause, &fixture.access_token, &json!({}));
    assert_eq!(response.status().as_u16(), 200);
    let repeated = body(response)["data"].clone();
    assert_eq!(repeated["current_pause_period"]["id"], first_period);
    assert_eq!(repeated["active"], false);
    let response = target.patch_json(&resume, &fixture.access_token, &json!({}));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(body(response)["data"]["active"], true);
    let response = target.patch_json(&resume, &fixture.access_token, &json!({}));
    assert_eq!(response.status().as_u16(), 200);
    let repeated = body(response)["data"].clone();
    assert_eq!(repeated["active"], true);
    assert!(repeated["current_pause_period"].is_null());
    let history_path = format!(
        "/api/v1/households/{}/medication_pause_periods?source_type=schedule&source_id={}",
        fixture.household_id,
        created["portable_id"].as_str().unwrap()
    );
    let response = target.get(&history_path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let periods = body(response)["data"].as_array().unwrap().to_vec();
    assert_eq!(periods.len(), 1);
    assert_eq!(periods[0]["id"], first_period);
    assert_utc_second_timestamp(&periods[0]["ended_at"]);
}

#[test]
fn schedule_pause_and_resume_retain_history_and_audit() {
    let target = Target::from_env();
    let fixture = fixture();
    let (created, _) = create_schedule(&target, &fixture);
    let path = format!(
        "{}/{}",
        schedules_path(&fixture),
        created["portable_id"].as_str().unwrap()
    );

    let response = target.patch_json(&format!("{path}/pause"), &fixture.access_token, &json!({}));
    assert_eq!(response.status().as_u16(), 200);
    let pause_request_id = request_id(&response);
    let paused = body(response)["data"].clone();
    assert_eq!(paused["active"], false);
    assert_eq!(paused["paused"], true);
    assert_eq!(
        paused["current_pause_period"]["reason"],
        "reason_not_recorded"
    );
    assert_eq!(paused["current_pause_period"]["legacy_context"], true);
    assert_eq!(
        paused["current_pause_period"]["source_id"],
        created["portable_id"]
    );
    assert_utc_second_timestamp(&paused["current_pause_period"]["started_at"]);
    assert!(paused["current_pause_period"]["ended_at"].is_null());
    let period_id = paused["current_pause_period"]["id"].clone();
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(
        body(response)["data"]["current_pause_period"]["id"],
        period_id
    );
    assert_audit_action(&target, &fixture, &pause_request_id, "PATCH", "pause", 200);

    let response = target.patch_json(&format!("{path}/resume"), &fixture.access_token, &json!({}));
    assert_eq!(response.status().as_u16(), 200);
    let resume_request_id = request_id(&response);
    let resumed = body(response)["data"].clone();
    assert_eq!(resumed["active"], true);
    assert_eq!(resumed["paused"], false);
    assert!(resumed["current_pause_period"].is_null());
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert!(body(response)["data"]["current_pause_period"].is_null());
    let history_path = format!(
        "/api/v1/households/{}/medication_pause_periods?source_type=schedule&source_id={}",
        fixture.household_id,
        created["portable_id"].as_str().unwrap()
    );
    let response = target.get(&history_path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let history = body(response);
    let period = history["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == period_id)
        .expect("resumed period retained in history");
    assert_eq!(period["reason"], "reason_not_recorded");
    assert_utc_second_timestamp(&period["ended_at"]);
    assert_audit_action(
        &target,
        &fixture,
        &resume_request_id,
        "PATCH",
        "resume",
        200,
    );

    let response = target.patch_json(
        &format!(
            "{}/{}{}",
            schedules_path(&fixture),
            fixture.managed_schedule_id,
            "/pause"
        ),
        &fixture.view_access_token,
        &json!({}),
    );
    assert_eq!(response.status().as_u16(), 403);
    let response = target.patch_json(
        &format!(
            "{}/{}{}",
            schedules_path(&fixture),
            fixture.foreign_schedule_id,
            "/pause"
        ),
        &fixture.access_token,
        &json!({}),
    );
    assert_eq!(response.status().as_u16(), 404);
}
