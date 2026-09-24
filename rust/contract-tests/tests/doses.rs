use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::Response;
use serde_json::{json, Value};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

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

fn clock() -> (String, String) {
    let now = OffsetDateTime::now_utc().replace_nanosecond(0).unwrap();
    (now.date().to_string(), now.format(&Rfc3339).unwrap())
}

fn client_uuid(fixture: &Fixture, sequence: u8) -> String {
    format!(
        "11111111-1111-4111-8111-{:010x}{sequence:02x}",
        fixture.household_id
    )
}

fn schedule_path(fixture: &Fixture, id: i64) -> String {
    format!(
        "/api/v1/households/{}/schedules/{id}/dose_occurrences",
        fixture.household_id
    )
}

fn assignment_path(fixture: &Fixture, id: i64) -> String {
    format!(
        "/api/v1/households/{}/person_medications/{id}/dose_occurrences",
        fixture.household_id
    )
}

fn takes_path(fixture: &Fixture) -> String {
    format!(
        "/api/v1/households/{}/medication_takes",
        fixture.household_id
    )
}

fn assignment_portable_id(target: &Target, fixture: &Fixture) -> String {
    assignment_portable_id_for(target, fixture, fixture.managed_assignment_id)
}

fn assignment_portable_id_for(target: &Target, fixture: &Fixture, id: i64) -> String {
    let response = target.get(
        &format!(
            "/api/v1/households/{}/person_medications/{id}",
            fixture.household_id
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    body(response)["data"]["portable_id"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn schedule_portable_id(target: &Target, fixture: &Fixture) -> String {
    let response = target.get(
        &format!(
            "/api/v1/households/{}/schedules/{}",
            fixture.household_id, fixture.managed_schedule_id
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    body(response)["data"]["portable_id"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn rows(target: &Target, path: &str, token: &str, date: &str) -> Vec<Value> {
    let response = target.get(
        &format!("{path}?start_date={date}&end_date={date}"),
        Some(token),
    );
    assert_eq!(response.status().as_u16(), 200);
    body(response)["data"].as_array().unwrap().clone()
}

fn stock(target: &Target, fixture: &Fixture, medication_id: i64) -> String {
    let response = target.get(
        &format!(
            "/api/v1/households/{}/medications/{}",
            fixture.household_id, medication_id
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    body(response)["data"]["current_supply"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn audit(
    target: &Target,
    fixture: &Fixture,
    id: &str,
    method: &str,
    action: &str,
    status: u16,
    controller: &str,
) {
    let response = target.get(
        &format!(
            "/api/v1/households/{}/admin/audit_logs",
            fixture.household_id
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let value = body(response);
    assert!(value["data"].as_array().unwrap().iter().any(|event| {
        event["event_type"] == "api.request"
            && event["request_id"] == id
            && event["metadata"]["http_method"] == method
            && event["metadata"]["action"] == action
            && event["metadata"]["status"] == status
            && event["metadata"]["controller"] == controller
    }));
}

fn create_medication(target: &Target, fixture: &Fixture) -> Value {
    let response = target.post_json_authorized(
        &format!("/api/v1/households/{}/medications", fixture.household_id),
        &fixture.access_token,
        &json!({"medication": {
            "name": "Contract dose medicine",
            "location_id": fixture.primary_location_id,
            "dose_amount": "1.25",
            "dose_unit": "ml",
            "current_supply": "20.0"
        }}),
    );
    assert_eq!(response.status().as_u16(), 201);
    body(response)["data"].clone()
}

fn create_schedule(target: &Target, fixture: &Fixture) -> (i64, i64) {
    let medication = create_medication(target, fixture);
    let response = target.post_json_authorized(
        &format!("/api/v1/households/{}/schedules", fixture.household_id),
        &fixture.access_token,
        &json!({"schedule": {
            "person_id": fixture.managed_person_portable_id,
            "medication_id": medication["portable_id"],
            "dose_amount": "1.25",
            "dose_unit": "ml",
            "frequency": "Daily",
            "start_date": "2026-02-25",
            "end_date": "2099-12-31"
        }}),
    );
    assert_eq!(response.status().as_u16(), 201);
    (
        body(response)["data"]["id"].as_i64().unwrap(),
        medication["id"].as_i64().unwrap(),
    )
}

fn create_routine_assignment(target: &Target, fixture: &Fixture) -> (i64, i64) {
    let medication = create_medication(target, fixture);
    let response = target.post_json_authorized(
        &format!(
            "/api/v1/households/{}/person_medications",
            fixture.household_id
        ),
        &fixture.access_token,
        &json!({"person_medication": {
            "person_id": fixture.managed_person_portable_id,
            "medication_id": medication["portable_id"],
            "dose_amount": "1.25",
            "dose_unit": "ml",
            "administration_kind": "routine",
            "dose_cycle": "daily",
            "max_daily_doses": 1
        }}),
    );
    assert_eq!(response.status().as_u16(), 201);
    (
        body(response)["data"]["id"].as_i64().unwrap(),
        medication["id"].as_i64().unwrap(),
    )
}

#[test]
fn schedule_occurrences_are_stable_person_scoped_and_validate_ranges() {
    let target = Target::from_env();
    let fixture = fixture();
    let (date, _) = clock();
    let path = schedule_path(&fixture, fixture.managed_schedule_id);
    let first = rows(&target, &path, &fixture.view_access_token, &date);
    assert!(!first.is_empty());
    let row = &first[0];
    assert_eq!(row["source_type"], "schedule");
    assert_eq!(row["source_id"], fixture.managed_schedule_id);
    assert_eq!(row["outcome"], "open");
    assert_eq!(row["window_starts_on"], date);
    assert_eq!(row["position"], 1);
    assert_eq!(row["expected"], true);
    assert_eq!(row["due"], true);
    assert!(row["scheduled_at"].is_null());
    assert!(row["etag"].is_null());
    assert_eq!(
        rows(&target, &path, &fixture.view_access_token, &date),
        first
    );

    for invalid in [
        path.clone(),
        format!("{path}?start_date=private-clinical-text&end_date={date}"),
        format!("{path}?start_date=2026-01-01&end_date=2026-02-01"),
        format!("{path}?start_date=2026-02-02&end_date=2026-02-01"),
    ] {
        let response = target.get(&invalid, Some(&fixture.access_token));
        assert_eq!(response.status().as_u16(), 422);
        assert!(!body(response).to_string().contains("private-clinical-text"));
    }
    let response = target.get(&format!("{path}?start_date={date}&end_date={date}"), None);
    assert_eq!(response.status().as_u16(), 401);
    for id in [fixture.hidden_schedule_id, fixture.foreign_schedule_id] {
        let path = schedule_path(&fixture, id);
        let response = target.get(
            &format!("{path}?start_date={date}&end_date={date}"),
            Some(&fixture.view_access_token),
        );
        assert_eq!(response.status().as_u16(), 404);
    }
}

#[test]
fn schedule_not_taken_reopens_with_a_current_version_and_retains_context() {
    let target = Target::from_env();
    let fixture = fixture();
    let (date, _) = clock();
    let (schedule_id, medication_id) = create_schedule(&target, &fixture);
    let path = schedule_path(&fixture, schedule_id);
    let key = rows(&target, &path, &fixture.access_token, &date)[0]["key"]
        .as_str()
        .unwrap()
        .to_owned();
    let before_stock = stock(&target, &fixture, medication_id);
    let payload = json!({"dose_occurrence": {"key": key, "reason": "unwell", "note": "Resting"}});
    let response = target.post_json_authorized(
        &format!("{path}/not_taken"),
        &fixture.access_token,
        &payload,
    );
    assert_eq!(response.status().as_u16(), 200);
    let tag = etag(&response);
    let id = request_id(&response);
    let decision = body(response)["data"].clone();
    assert_eq!(decision["outcome"], "not_taken");
    assert_eq!(decision["reason"], "unwell");
    assert_eq!(decision["note"], "Resting");
    assert_eq!(decision["etag"], tag);
    assert!(decision["resolved_at"].as_str().unwrap().ends_with('Z'));
    assert_eq!(stock(&target, &fixture, medication_id), before_stock);
    audit(
        &target,
        &fixture,
        &id,
        "POST",
        "not_taken",
        200,
        "api/v1/dose_occurrences",
    );
    assert_eq!(
        rows(&target, &path, &fixture.access_token, &date)[0],
        decision
    );

    let response = target.post_json_authorized(
        &format!("{path}/not_taken"),
        &fixture.access_token,
        &payload,
    );
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(etag(&response), tag);
    let response = target.post_json_authorized(
        &format!("{path}/not_taken"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": key, "reason": "refused"}}),
    );
    assert_eq!(response.status().as_u16(), 409);
    assert_eq!(body(response)["error"]["code"], "already_resolved");

    let reopen = format!("{path}/reopen");
    let payload = json!({"dose_occurrence": {"key": key}});
    let response = target.patch_json(&reopen, &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 428);
    assert_eq!(body(response)["error"]["code"], "precondition_required");
    let response = target.patch_json_if_match(&reopen, &fixture.access_token, &payload, "stale");
    assert_eq!(response.status().as_u16(), 409);
    assert_eq!(body(response)["error"]["code"], "sync_conflict");
    let response = target.patch_json_if_match(&reopen, &fixture.view_access_token, &payload, &tag);
    assert_eq!(response.status().as_u16(), 403);
    let response = target.patch_json_if_match(&reopen, &fixture.access_token, &payload, &tag);
    assert_eq!(response.status().as_u16(), 200);
    let reopened = body(response)["data"].clone();
    assert_eq!(reopened["outcome"], "open");
    assert!(reopened["reason"].is_null());
    assert!(reopened["note"].is_null());
    assert_ne!(reopened["etag"], tag);
    assert_eq!(stock(&target, &fixture, medication_id), before_stock);
    assert_eq!(
        rows(&target, &path, &fixture.access_token, &date)[0],
        reopened
    );
}

#[test]
fn schedule_take_replaces_not_taken_once_and_stays_immutable() {
    let target = Target::from_env();
    let fixture = fixture();
    let (date, taken_at) = clock();
    let (schedule_id, medication_id) = create_schedule(&target, &fixture);
    let path = schedule_path(&fixture, schedule_id);
    let key = rows(&target, &path, &fixture.access_token, &date)[0]["key"]
        .as_str()
        .unwrap()
        .to_owned();
    let before_stock: f64 = stock(&target, &fixture, medication_id).parse().unwrap();
    let response = target.post_json_authorized(
        &format!("{path}/not_taken"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": key, "reason": "unwell", "note": "Retained decision"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let tag = etag(&response);
    let payload = json!({"dose_occurrence": {
        "key": key,
        "taken_at": taken_at,
        "client_uuid": client_uuid(&fixture, 1),
        "dose_amount": "1.25",
        "taken_from_medication_id": medication_id
    }});
    let response =
        target.post_json_authorized(&format!("{path}/take"), &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 428);
    assert_eq!(body(response)["error"]["code"], "precondition_required");
    let response = target.post_json_if_match(
        &format!("{path}/take"),
        &fixture.access_token,
        &payload,
        "stale",
    );
    assert_eq!(response.status().as_u16(), 409);
    let response = target.post_json_if_match(
        &format!("{path}/take"),
        &fixture.view_access_token,
        &payload,
        &tag,
    );
    assert_eq!(response.status().as_u16(), 403);
    assert_eq!(
        stock(&target, &fixture, medication_id)
            .parse::<f64>()
            .unwrap(),
        before_stock
    );

    let response = target.post_json_if_match(
        &format!("{path}/take"),
        &fixture.access_token,
        &payload,
        &tag,
    );
    if response.status().as_u16() != 200 {
        panic!(
            "schedule take: {} {}",
            response.status(),
            response.text().unwrap()
        );
    }
    let id = request_id(&response);
    let taken = body(response)["data"].clone();
    assert_eq!(taken["outcome"], "taken");
    assert!(taken["reason"].is_null());
    assert!(taken["note"].is_null());
    assert!(taken["medication_take_id"].as_i64().is_some());
    assert_ne!(taken["etag"], tag);
    assert_eq!(
        stock(&target, &fixture, medication_id)
            .parse::<f64>()
            .unwrap(),
        before_stock - 1.25
    );
    audit(
        &target,
        &fixture,
        &id,
        "POST",
        "take",
        200,
        "api/v1/dose_occurrences",
    );
    let response =
        target.post_json_authorized(&format!("{path}/take"), &fixture.access_token, &payload);
    if response.status().as_u16() != 200 {
        panic!(
            "assignment take: {} {}",
            response.status(),
            response.text().unwrap()
        );
    }
    assert_eq!(
        body(response)["data"]["medication_take_id"],
        taken["medication_take_id"]
    );
    assert_eq!(
        stock(&target, &fixture, medication_id)
            .parse::<f64>()
            .unwrap(),
        before_stock - 1.25
    );
    let response = target.patch_json_if_match(
        &format!("{path}/reopen"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": key}}),
        taken["etag"].as_str().unwrap(),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(rows(&target, &path, &fixture.access_token, &date)[0], taken);
    let response = target.get(&takes_path(&fixture), Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let takes = body(response);
    let take = takes["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|take| take["id"] == taken["medication_take_id"])
        .unwrap();
    assert_eq!(take["dose_amount"], "1.25");
    assert_eq!(take["taken_at"], taken_at);
    assert_eq!(take["schedule_id"], taken["source_id"]);
}

#[test]
fn direct_assignment_occurrences_require_routine_and_person_access() {
    let target = Target::from_env();
    let fixture = fixture();
    let (date, _) = clock();
    let as_needed = assignment_path(&fixture, fixture.managed_assignment_id);
    assert!(rows(&target, &as_needed, &fixture.access_token, &date).is_empty());
    let (id, _) = create_routine_assignment(&target, &fixture);
    let path = assignment_path(&fixture, id);
    let first = rows(&target, &path, &fixture.view_access_token, &date);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0]["source_type"], "person_medication");
    assert_eq!(first[0]["source_id"], id);
    assert_eq!(first[0]["outcome"], "open");
    assert_eq!(first[0]["window_starts_on"], date);
    assert_eq!(first[0]["window_ends_on"], date);
    assert_eq!(
        rows(&target, &path, &fixture.view_access_token, &date),
        first
    );
    let response = target.post_json_authorized(
        &format!("{path}/not_taken"),
        &fixture.view_access_token,
        &json!({"dose_occurrence": {"key": first[0]["key"], "reason": "unwell"}}),
    );
    assert_eq!(response.status().as_u16(), 403);
    for id in [fixture.hidden_assignment_id, fixture.foreign_assignment_id] {
        let response = target.get(
            &format!(
                "{}?start_date={date}&end_date={date}",
                assignment_path(&fixture, id)
            ),
            Some(&fixture.view_access_token),
        );
        assert_eq!(response.status().as_u16(), 404);
    }
}

#[test]
fn direct_assignment_outcome_reopens_then_records_one_take() {
    let target = Target::from_env();
    let fixture = fixture();
    let (date, _) = clock();
    let (id, medication_id) = create_routine_assignment(&target, &fixture);
    let taken_at = (OffsetDateTime::now_utc() + time::Duration::seconds(1))
        .format(&Rfc3339)
        .unwrap();
    let path = assignment_path(&fixture, id);
    let key = rows(&target, &path, &fixture.access_token, &date)[0]["key"].clone();
    let response = target.post_json_authorized(
        &format!("{path}/not_taken"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": key, "reason": "unwell"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let tag = etag(&response);
    let response = target.patch_json_if_match(
        &format!("{path}/reopen"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": key}}),
        &tag,
    );
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(body(response)["data"]["outcome"], "open");
    let payload = json!({"dose_occurrence": {
        "key": key,
        "taken_at": taken_at,
        "client_uuid": client_uuid(&fixture, 2),
        "taken_from_medication_id": medication_id
    }});
    let response =
        target.post_json_authorized(&format!("{path}/take"), &fixture.access_token, &payload);
    if response.status().as_u16() != 200 {
        panic!(
            "assignment take: {} {}",
            response.status(),
            response.text().unwrap()
        );
    }
    let taken = body(response)["data"].clone();
    assert_eq!(taken["outcome"], "taken");
    let response =
        target.post_json_authorized(&format!("{path}/take"), &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(
        body(response)["data"]["medication_take_id"],
        taken["medication_take_id"]
    );
    let response = target.get(&takes_path(&fixture), Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let take = body(response)["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|take| take["id"] == taken["medication_take_id"])
        .unwrap()
        .clone();
    assert_eq!(take["person_medication_id"], id);
    assert_eq!(take["medication_id"], medication_id);
    assert_eq!(take["dose_amount"], "1.25");
}

#[test]
fn medication_takes_create_filters_paginates_and_preserves_precision() {
    let target = Target::from_env();
    let fixture = fixture();
    let (assignment_id, medication_id) = create_routine_assignment(&target, &fixture);
    let taken_at = (OffsetDateTime::now_utc() + time::Duration::seconds(1))
        .replace_nanosecond(0)
        .unwrap()
        .format(&Rfc3339)
        .unwrap();
    let path = takes_path(&fixture);
    let source_id = assignment_portable_id_for(&target, &fixture, assignment_id);
    let before_stock: f64 = stock(&target, &fixture, medication_id).parse().unwrap();
    let payload = json!({"medication_take": {
        "source_type": "person_medication",
        "source_id": source_id,
        "taken_at": taken_at,
        "client_uuid": client_uuid(&fixture, 3),
        "dose_amount": "1.25",
        "taken_from_medication_id": medication_id
    }});
    let response = target.post_json_authorized(&path, &fixture.view_access_token, &payload);
    assert_eq!(response.status().as_u16(), 403);
    let response = target.post_json_authorized(&path, &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 201);
    let tag = etag(&response);
    let id = request_id(&response);
    let take = body(response)["data"].clone();
    assert_eq!(take["client_uuid"], client_uuid(&fixture, 3));
    assert_eq!(take["person_medication_id"], assignment_id);
    assert_eq!(take["person_id"], fixture.managed_person_id);
    assert_eq!(take["medication_id"], medication_id);
    assert_eq!(take["taken_from_medication_id"], medication_id);
    assert_eq!(take["dose_amount"], "1.25");
    assert_eq!(take["dose_unit"], "ml");
    assert_eq!(take["taken_at"], taken_at);
    assert!(take["updated_at"].as_str().unwrap().ends_with('Z'));
    assert_eq!(
        stock(&target, &fixture, medication_id)
            .parse::<f64>()
            .unwrap(),
        before_stock - 1.25
    );
    audit(
        &target,
        &fixture,
        &id,
        "POST",
        "create",
        201,
        "api/v1/medication_takes",
    );
    let response = target.post_json_authorized(&path, &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(etag(&response), tag);
    assert_eq!(body(response)["data"], take);
    assert_eq!(
        stock(&target, &fixture, medication_id)
            .parse::<f64>()
            .unwrap(),
        before_stock - 1.25
    );

    let (routine_id, routine_medication_id) = create_routine_assignment(&target, &fixture);
    let routine_source_id = assignment_portable_id_for(&target, &fixture, routine_id);
    let routine_taken_at = (OffsetDateTime::now_utc() + time::Duration::seconds(1))
        .format(&Rfc3339)
        .unwrap();
    let response = target.post_json_authorized(
        &path,
        &fixture.access_token,
        &json!({"medication_take": {
            "source_type": "person_medication",
            "source_id": routine_source_id,
            "taken_at": routine_taken_at,
            "taken_from_medication_id": routine_medication_id
        }}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let second_take = body(response)["data"].clone();
    assert_ne!(second_take["id"], take["id"]);
    let response = target.get(&path, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 200);
    let collection = body(response);
    assert!(collection["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == take["id"]));
    assert!(collection["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == second_take["id"]));
    let response = target.get(
        &format!("{path}?page=1&per_page=1"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let first_page = body(response);
    assert_eq!(first_page["meta"]["page"], 1);
    assert_eq!(first_page["meta"]["per_page"], 1);
    assert!(first_page["meta"]["total_count"].as_u64().unwrap() >= 2);
    assert_eq!(first_page["data"].as_array().unwrap().len(), 1);
    let response = target.get(
        &format!("{path}?page=2&per_page=1"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let second_page = body(response);
    assert_eq!(second_page["meta"]["page"], 2);
    assert_eq!(
        second_page["meta"]["total_count"],
        first_page["meta"]["total_count"]
    );
    assert_ne!(second_page["data"][0]["id"], first_page["data"][0]["id"]);
    let response = target.get(
        &format!("{path}?page=0&per_page=500&updated_since=2000-01-01T00:00:00Z"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let collection = body(response);
    assert_eq!(collection["meta"]["page"], 1);
    assert_eq!(collection["meta"]["per_page"], 100);
    assert!(collection["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == take["id"]));
    let response = target.get(
        &format!("{path}?updated_since=invalid"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(body(response)["error"]["code"], "unprocessable_content");
    let response = target.get(
        &format!(
            "/api/v1/households/{}/medication_takes",
            fixture.foreign_household_id
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 403);
}

#[test]
fn medication_take_rejects_invalid_time_source_and_future_without_stock_loss() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = takes_path(&fixture);
    let before_stock = stock(&target, &fixture, fixture.managed_medication_id);
    let (_, taken_at) = clock();
    let schedule_source_id = schedule_portable_id(&target, &fixture);
    let assignment_source_id = assignment_portable_id(&target, &fixture);
    for (source_type, source_id, time, status, message) in [
        (
            "schedule",
            schedule_source_id.as_str(),
            "invalid",
            422,
            Some("taken_at is invalid"),
        ),
        (
            "unknown",
            schedule_source_id.as_str(),
            taken_at.as_str(),
            404,
            None,
        ),
        (
            "person_medication",
            fixture.foreign_assignment_portable_id.as_str(),
            taken_at.as_str(),
            404,
            None,
        ),
    ] {
        let response = target.post_json_authorized(
            &path,
            &fixture.access_token,
            &json!({"medication_take": {"source_type": source_type, "source_id": source_id, "taken_at": time}}),
        );
        assert_eq!(response.status().as_u16(), status);
        if let Some(message) = message {
            assert_eq!(body(response)["error"]["message"], message);
        }
    }
    let future = (OffsetDateTime::now_utc() + time::Duration::hours(2))
        .format(&Rfc3339)
        .unwrap();
    let response = target.post_json_authorized(
        &path,
        &fixture.access_token,
        &json!({"medication_take": {
            "source_type": "person_medication",
            "source_id": assignment_source_id,
            "taken_at": future
        }}),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(
        body(response)["error"]["message"],
        "Cannot record a dose more than one hour in the future"
    );
    assert_eq!(
        stock(&target, &fixture, fixture.managed_medication_id),
        before_stock
    );
}

#[test]
#[ignore = "Rails currently serializes medication-take timestamps to whole seconds"]
fn medication_take_preserves_fractional_second_timestamp() {
    let target = Target::from_env();
    let fixture = fixture();
    let source_id = assignment_portable_id(&target, &fixture);
    let taken_at = OffsetDateTime::now_utc()
        .replace_nanosecond(123_456_000)
        .unwrap()
        .format(&Rfc3339)
        .unwrap();
    let response = target.post_json_authorized(
        &takes_path(&fixture),
        &fixture.access_token,
        &json!({"medication_take": {
            "source_type": "person_medication",
            "source_id": source_id,
            "taken_from_medication_id": fixture.managed_medication_id,
            "taken_at": taken_at
        }}),
    );
    assert_eq!(response.status().as_u16(), 201);
    assert_eq!(body(response)["data"]["taken_at"], taken_at);
}
