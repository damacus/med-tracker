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
        "min_hours_between_doses": "8.5",
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
    (created, tag)
}

fn assert_utc_second_timestamp(value: &Value) {
    let timestamp = value.as_str().expect("timestamp string");
    assert_eq!(timestamp.len(), 20);
    assert_eq!(&timestamp[10..11], "T");
    assert!(timestamp.ends_with('Z'));
}

fn assert_audit_action(target: &Target, fixture: &Fixture, action: &str, status: u16) {
    let path = format!(
        "/api/v1/households/{}/admin/audit_logs",
        fixture.household_id
    );
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let audit = body(response);
    assert!(audit["data"].as_array().unwrap().iter().any(|event| {
        event["event_type"] == "api.request"
            && event["metadata"]["controller"] == "api/v1/schedules"
            && event["metadata"]["action"] == action
            && event["metadata"]["status"] == status
    }));
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
fn schedule_create_get_and_invalid_inputs_preserve_the_public_contract() {
    let target = Target::from_env();
    let fixture = fixture();
    let base = schedules_path(&fixture);
    let (created, created_etag) = create_schedule(&target, &fixture);
    assert_audit_action(&target, &fixture, "create", 201);
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
    let patched_etag = etag(&response);
    let patched = body(response)["data"].clone();
    assert_eq!(patched["frequency"], "Every eight hours");
    assert_eq!(patched["notes"], "Revised");
    assert_ne!(patched_etag, original_etag);

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
    let replaced_etag = etag(&response);
    let replaced = body(response)["data"].clone();
    assert_eq!(replaced["frequency"], "Twice daily");
    assert_eq!(replaced["notes"], "Revised");
    assert_eq!(replaced["dose_amount"], "1.25");
    assert_ne!(replaced_etag, patched_etag);
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
    assert_audit_action(&target, &fixture, "update", 200);
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
    assert_audit_action(&target, &fixture, "pause", 200);

    let response = target.patch_json(&format!("{path}/resume"), &fixture.access_token, &json!({}));
    assert_eq!(response.status().as_u16(), 200);
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
    assert_audit_action(&target, &fixture, "resume", 200);

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
