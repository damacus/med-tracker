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

fn stock_for(target: &Target, household_id: i64, token: &str, medication_id: i64) -> String {
    let response = target.get(
        &format!("/api/v1/households/{household_id}/medications/{medication_id}"),
        Some(token),
    );
    assert_eq!(response.status().as_u16(), 200);
    body(response)["data"]["current_supply"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn stock(target: &Target, fixture: &Fixture, medication_id: i64) -> String {
    stock_for(
        target,
        fixture.household_id,
        &fixture.access_token,
        medication_id,
    )
}

fn take_snapshot_for(target: &Target, household_id: i64, token: &str) -> (u64, Vec<i64>) {
    let mut ids = Vec::new();
    let mut page = 1;
    let total = loop {
        let response = target.get(
            &format!("/api/v1/households/{household_id}/medication_takes?page={page}&per_page=100"),
            Some(token),
        );
        assert_eq!(response.status().as_u16(), 200);
        let collection = body(response);
        let total = collection["meta"]["total_count"].as_u64().unwrap();
        let page_rows = collection["data"].as_array().unwrap();
        assert!(!page_rows.is_empty() || total == 0);
        ids.extend(page_rows.iter().map(|row| row["id"].as_i64().unwrap()));
        if ids.len() >= total as usize {
            break total;
        }
        page += 1;
    };
    assert_eq!(ids.len(), total as usize);
    ids.sort_unstable();
    (total, ids)
}

fn take_snapshot(target: &Target, fixture: &Fixture) -> (u64, Vec<i64>) {
    take_snapshot_for(target, fixture.household_id, &fixture.access_token)
}

fn takes_for_medication(target: &Target, fixture: &Fixture, medication_id: i64) -> Vec<Value> {
    let mut matches = Vec::new();
    let mut page = 1;
    loop {
        let response = target.get(
            &format!("{}?page={page}&per_page=100", takes_path(fixture)),
            Some(&fixture.access_token),
        );
        assert_eq!(response.status().as_u16(), 200);
        let collection = body(response);
        let total = collection["meta"]["total_count"].as_u64().unwrap();
        let rows = collection["data"].as_array().unwrap();
        matches.extend(
            rows.iter()
                .filter(|row| row["medication_id"] == medication_id)
                .cloned(),
        );
        if page * 100 >= total {
            break;
        }
        page += 1;
    }
    matches
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
    let events = value["data"].as_array().unwrap();
    assert!(events.len() <= 100);
    let matching: Vec<_> = events
        .iter()
        .filter(|event| event["request_id"] == id)
        .collect();
    assert_eq!(matching.len(), 1, "one visible audit event for {id}");
    let event = matching[0];
    assert_eq!(event["event_type"], "api.request");
    assert_eq!(event["metadata"]["http_method"], method);
    assert_eq!(event["metadata"]["action"], action);
    assert_eq!(event["metadata"]["status"], status);
    assert_eq!(event["metadata"]["controller"], controller);
}

fn create_medication(target: &Target, fixture: &Fixture) -> Value {
    let name = format!(
        "Contract dose medicine {}",
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    );
    let response = target.post_json_authorized(
        &format!("/api/v1/households/{}/medications", fixture.household_id),
        &fixture.access_token,
        &json!({"medication": {
            "name": name,
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
    let (date, taken_at) = clock();
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
    let before_stock = stock(&target, &fixture, fixture.managed_medication_id);
    for id in [fixture.hidden_schedule_id, fixture.foreign_schedule_id] {
        let hidden_path = schedule_path(&fixture, id);
        let response = target.get(
            &format!("{hidden_path}?start_date={date}&end_date={date}"),
            Some(&fixture.access_token),
        );
        assert_eq!(response.status().as_u16(), 404);
        let response = target.post_json_authorized(
            &format!("{hidden_path}/not_taken"),
            &fixture.access_token,
            &json!({"dose_occurrence": {"key": row["key"], "reason": "unwell"}}),
        );
        assert_eq!(response.status().as_u16(), 404);
        let response = target.post_json_authorized(
            &format!("{hidden_path}/take"),
            &fixture.access_token,
            &json!({"dose_occurrence": {
                "key": row["key"],
                "taken_at": taken_at,
                "taken_from_medication_id": fixture.managed_medication_id
            }}),
        );
        assert_eq!(response.status().as_u16(), 404);
        let response = target.patch_json_if_match(
            &format!("{hidden_path}/reopen"),
            &fixture.access_token,
            &json!({"dose_occurrence": {"key": row["key"]}}),
            "stale",
        );
        assert_eq!(response.status().as_u16(), 404);
    }
    assert_eq!(rows(&target, &path, &fixture.access_token, &date), first);
    assert_eq!(
        stock(&target, &fixture, fixture.managed_medication_id),
        before_stock
    );
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
    let replay_request_id = request_id(&response);
    assert_eq!(etag(&response), tag);
    audit(
        &target,
        &fixture,
        &replay_request_id,
        "POST",
        "not_taken",
        200,
        "api/v1/dose_occurrences",
    );
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
    let reopen_request_id = request_id(&response);
    let reopened = body(response)["data"].clone();
    audit(
        &target,
        &fixture,
        &reopen_request_id,
        "PATCH",
        "reopen",
        200,
        "api/v1/dose_occurrences",
    );
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
            "schedule take replay: {} {}",
            response.status(),
            response.text().unwrap()
        );
    }
    let replay_request_id = request_id(&response);
    assert_eq!(
        body(response)["data"]["medication_take_id"],
        taken["medication_take_id"]
    );
    audit(
        &target,
        &fixture,
        &replay_request_id,
        "POST",
        "take",
        200,
        "api/v1/dose_occurrences",
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
    let (id, medication_id) = create_routine_assignment(&target, &fixture);
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
    let taken_at = (OffsetDateTime::now_utc() + time::Duration::seconds(1))
        .format(&Rfc3339)
        .unwrap();
    let take_payload = json!({"dose_occurrence": {
        "key": first[0]["key"],
        "taken_at": taken_at,
        "taken_from_medication_id": medication_id
    }});
    let response = target.post_json_authorized(
        &format!("{path}/take"),
        &fixture.view_access_token,
        &take_payload,
    );
    assert_eq!(response.status().as_u16(), 403);
    let response = target.patch_json_if_match(
        &format!("{path}/reopen"),
        &fixture.view_access_token,
        &json!({"dose_occurrence": {"key": first[0]["key"]}}),
        "stale",
    );
    assert_eq!(response.status().as_u16(), 403);
    assert_eq!(rows(&target, &path, &fixture.access_token, &date), first);
    assert_eq!(stock(&target, &fixture, medication_id), "20.0");
    for id in [fixture.hidden_assignment_id, fixture.foreign_assignment_id] {
        let hidden_path = assignment_path(&fixture, id);
        let response = target.get(
            &format!("{hidden_path}?start_date={date}&end_date={date}"),
            Some(&fixture.access_token),
        );
        assert_eq!(response.status().as_u16(), 404);
        let response = target.post_json_authorized(
            &format!("{hidden_path}/not_taken"),
            &fixture.access_token,
            &json!({"dose_occurrence": {"key": first[0]["key"], "reason": "unwell"}}),
        );
        assert_eq!(response.status().as_u16(), 404);
        let response = target.post_json_authorized(
            &format!("{hidden_path}/take"),
            &fixture.access_token,
            &take_payload,
        );
        assert_eq!(response.status().as_u16(), 404);
        let response = target.patch_json_if_match(
            &format!("{hidden_path}/reopen"),
            &fixture.access_token,
            &json!({"dose_occurrence": {"key": first[0]["key"]}}),
            "stale",
        );
        assert_eq!(response.status().as_u16(), 404);
    }
    assert_eq!(rows(&target, &path, &fixture.access_token, &date), first);
    assert_eq!(stock(&target, &fixture, medication_id), "20.0");
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
    let before_stock: f64 = stock(&target, &fixture, medication_id).parse().unwrap();
    let key = rows(&target, &path, &fixture.access_token, &date)[0]["key"].clone();
    let response = target.post_json_authorized(
        &format!("{path}/not_taken"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": key, "reason": "unwell", "note": "Resting after treatment"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let tag = etag(&response);
    let not_taken_request_id = request_id(&response);
    let not_taken = body(response)["data"].clone();
    assert_eq!(not_taken["outcome"], "not_taken");
    assert_eq!(not_taken["reason"], "unwell");
    assert_eq!(not_taken["note"], "Resting after treatment");
    audit(
        &target,
        &fixture,
        &not_taken_request_id,
        "POST",
        "not_taken",
        200,
        "api/v1/dose_occurrences",
    );
    assert_eq!(
        rows(&target, &path, &fixture.view_access_token, &date)[0],
        not_taken
    );
    let response = target.post_json_authorized(
        &format!("{path}/not_taken"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": key, "reason": "unwell", "note": "Resting after treatment"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(etag(&response), tag);
    assert_eq!(body(response)["data"], not_taken);
    let response = target.patch_json(
        &format!("{path}/reopen"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": key}}),
    );
    assert_eq!(response.status().as_u16(), 428);
    let response = target.patch_json_if_match(
        &format!("{path}/reopen"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": key}}),
        "stale",
    );
    assert_eq!(response.status().as_u16(), 409);
    assert_eq!(
        rows(&target, &path, &fixture.access_token, &date)[0],
        not_taken
    );
    let replacement = json!({"dose_occurrence": {
        "key": key, "taken_at": taken_at,
        "client_uuid": client_uuid(&fixture, 2),
        "taken_from_medication_id": medication_id
    }});
    let response =
        target.post_json_authorized(&format!("{path}/take"), &fixture.access_token, &replacement);
    assert_eq!(response.status().as_u16(), 428);
    let response = target.post_json_if_match(
        &format!("{path}/take"),
        &fixture.access_token,
        &replacement,
        "stale",
    );
    assert_eq!(response.status().as_u16(), 409);
    assert_eq!(
        rows(&target, &path, &fixture.access_token, &date)[0],
        not_taken
    );
    assert!(takes_for_medication(&target, &fixture, medication_id).is_empty());
    assert_eq!(
        stock(&target, &fixture, medication_id)
            .parse::<f64>()
            .unwrap(),
        before_stock
    );
    let response = target.patch_json_if_match(
        &format!("{path}/reopen"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": key}}),
        &tag,
    );
    assert_eq!(response.status().as_u16(), 200);
    let reopened_tag = etag(&response);
    let reopen_request_id = request_id(&response);
    let reopened = body(response)["data"].clone();
    audit(
        &target,
        &fixture,
        &reopen_request_id,
        "PATCH",
        "reopen",
        200,
        "api/v1/dose_occurrences",
    );
    assert_eq!(reopened["outcome"], "open");
    assert!(reopened["reason"].is_null());
    assert!(reopened["note"].is_null());
    assert_eq!(reopened["etag"], reopened_tag);
    assert_ne!(reopened_tag, tag);
    assert_eq!(
        rows(&target, &path, &fixture.access_token, &date)[0],
        reopened
    );
    assert_eq!(
        stock(&target, &fixture, medication_id)
            .parse::<f64>()
            .unwrap(),
        before_stock
    );
    assert!(takes_for_medication(&target, &fixture, medication_id).is_empty());
    let payload = json!({"dose_occurrence": {
        "key": key,
        "taken_at": taken_at,
        "client_uuid": client_uuid(&fixture, 2),
        "taken_from_medication_id": medication_id
    }});
    let invalid_time = json!({"dose_occurrence": {
        "key": key, "taken_at": "invalid",
        "client_uuid": client_uuid(&fixture, 2),
        "taken_from_medication_id": medication_id
    }});
    let response = target.post_json_authorized(
        &format!("{path}/take"),
        &fixture.access_token,
        &invalid_time,
    );
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(
        rows(&target, &path, &fixture.access_token, &date)[0],
        reopened
    );
    assert!(takes_for_medication(&target, &fixture, medication_id).is_empty());
    assert_eq!(
        stock(&target, &fixture, medication_id)
            .parse::<f64>()
            .unwrap(),
        before_stock
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
    let take_request_id = request_id(&response);
    let taken = body(response)["data"].clone();
    assert_eq!(taken["outcome"], "taken");
    audit(
        &target,
        &fixture,
        &take_request_id,
        "POST",
        "take",
        200,
        "api/v1/dose_occurrences",
    );
    assert_eq!(
        stock(&target, &fixture, medication_id)
            .parse::<f64>()
            .unwrap(),
        before_stock - 1.25
    );
    let response =
        target.post_json_authorized(&format!("{path}/take"), &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 200);
    let replay_request_id = request_id(&response);
    assert_eq!(
        body(response)["data"]["medication_take_id"],
        taken["medication_take_id"]
    );
    audit(
        &target,
        &fixture,
        &replay_request_id,
        "POST",
        "take",
        200,
        "api/v1/dose_occurrences",
    );
    assert_eq!(
        stock(&target, &fixture, medication_id)
            .parse::<f64>()
            .unwrap(),
        before_stock - 1.25
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
    assert_eq!(
        takes_for_medication(&target, &fixture, medication_id),
        vec![take]
    );

    let attempted = json!({"dose_occurrence": {"key": key, "reason": "unwell", "note": "private-clinical-text"}});
    let response = target.post_json_authorized(
        &format!("{path}/not_taken"),
        &fixture.view_access_token,
        &attempted,
    );
    assert_eq!(response.status().as_u16(), 403);
    assert!(!body(response).to_string().contains("private-clinical-text"));
    let response = target.patch_json_if_match(
        &format!("{path}/reopen"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": key}}),
        taken["etag"].as_str().unwrap(),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert!(!body(response).to_string().contains("private-clinical-text"));
    for hidden_id in [fixture.hidden_assignment_id, fixture.foreign_assignment_id] {
        let hidden_path = assignment_path(&fixture, hidden_id);
        let response = target.get(
            &format!("{hidden_path}?start_date={date}&end_date={date}"),
            Some(&fixture.view_access_token),
        );
        assert_eq!(response.status().as_u16(), 404);
        let response = target.post_json_authorized(
            &format!("{hidden_path}/not_taken"),
            &fixture.view_access_token,
            &attempted,
        );
        assert_eq!(response.status().as_u16(), 404);
        assert!(!body(response).to_string().contains("private-clinical-text"));
    }
    let response = target.post_json_authorized(
        &format!("{path}/not_taken"),
        &fixture.foreign_access_token,
        &attempted,
    );
    assert_eq!(response.status().as_u16(), 403);
    assert!(!body(response).to_string().contains("private-clinical-text"));
    assert_eq!(
        rows(&target, &path, &fixture.view_access_token, &date)[0],
        taken
    );
    assert_eq!(
        stock(&target, &fixture, medication_id)
            .parse::<f64>()
            .unwrap(),
        before_stock - 1.25
    );
    assert_eq!(
        takes_for_medication(&target, &fixture, medication_id).len(),
        1
    );
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
    let replay_request_id = request_id(&response);
    assert_eq!(etag(&response), tag);
    assert_eq!(body(response)["data"], take);
    audit(
        &target,
        &fixture,
        &replay_request_id,
        "POST",
        "create",
        200,
        "api/v1/medication_takes",
    );
    assert_eq!(
        stock(&target, &fixture, medication_id)
            .parse::<f64>()
            .unwrap(),
        before_stock - 1.25
    );
    assert_eq!(
        takes_for_medication(&target, &fixture, medication_id),
        vec![take.clone()]
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
fn medication_take_collection_excludes_takes_for_ungranted_people() {
    let target = Target::from_env();
    let fixture = fixture();
    let medication = create_medication(&target, &fixture);
    let person = target.post_json_authorized(
        &format!("/api/v1/households/{}/people", fixture.household_id),
        &fixture.care_access_token,
        &json!({"person": {"name": "Private take subject", "date_of_birth": "1980-02-03",
            "person_type": "adult", "has_capacity": true}}),
    );
    assert_eq!(person.status().as_u16(), 201);
    let person = body(person)["data"].clone();
    let assignment = target.post_json_authorized(
        &format!(
            "/api/v1/households/{}/person_medications",
            fixture.household_id
        ),
        &fixture.care_access_token,
        &json!({"person_medication": {
            "person_id": person["portable_id"],
            "medication_id": medication["portable_id"],
            "dose_amount": "1", "dose_unit": "ml", "administration_kind": "as_needed"
        }}),
    );
    assert_eq!(assignment.status().as_u16(), 201);
    let assignment = body(assignment)["data"].clone();
    let (_, taken_at) = clock();
    let take = target.post_json_authorized(
        &takes_path(&fixture),
        &fixture.care_access_token,
        &json!({"medication_take": {
            "source_type": "person_medication", "source_id": assignment["portable_id"],
            "taken_at": taken_at, "dose_amount": "1", "taken_from_medication_id": medication["id"]
        }}),
    );
    assert_eq!(take.status().as_u16(), 201);
    let take_id = body(take)["data"]["id"].clone();
    let visible = body(target.get(&takes_path(&fixture), Some(&fixture.care_access_token)));
    assert!(visible["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == take_id));
    let viewer = target.get(&takes_path(&fixture), Some(&fixture.view_access_token));
    assert_eq!(viewer.status().as_u16(), 200);
    let viewer = body(viewer);
    assert!(!viewer["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == take_id));
}

#[test]
fn medication_take_rejects_invalid_time_source_and_future_without_stock_loss() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = takes_path(&fixture);
    let before_stock = stock(&target, &fixture, fixture.managed_medication_id);
    let before_takes = take_snapshot(&target, &fixture);
    let foreign_stock = stock_for(
        &target,
        fixture.foreign_household_id,
        &fixture.foreign_access_token,
        fixture.foreign_medication_id,
    );
    let foreign_takes = take_snapshot_for(
        &target,
        fixture.foreign_household_id,
        &fixture.foreign_access_token,
    );
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
    let response = target.post_json_authorized(
        &path,
        &fixture.access_token,
        &json!({"medication_take": {
            "source_type": "person_medication", "source_id": assignment_source_id,
            "taken_at": taken_at, "dose_amount": "1",
            "taken_from_medication_id": fixture.foreign_medication_id
        }}),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert!(!body(response)
        .to_string()
        .contains(&fixture.foreign_medication_name));
    assert_eq!(
        stock_for(
            &target,
            fixture.foreign_household_id,
            &fixture.foreign_access_token,
            fixture.foreign_medication_id,
        ),
        foreign_stock
    );
    assert_eq!(
        take_snapshot_for(
            &target,
            fixture.foreign_household_id,
            &fixture.foreign_access_token,
        ),
        foreign_takes
    );
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
    assert_eq!(take_snapshot(&target, &fixture), before_takes);
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

#[test]
fn timed_schedule_and_routine_cycle_windows_project_only_expected_occurrences() {
    let target = Target::from_env();
    let fixture = fixture();
    let (date, _) = clock();
    let (schedule_id, _) = create_schedule(&target, &fixture);
    let path = schedule_path(&fixture, schedule_id);
    let response = target.patch_json(
        &format!(
            "/api/v1/households/{}/schedules/{schedule_id}",
            fixture.household_id
        ),
        &fixture.access_token,
        &json!({"schedule": {"schedule_config": {"times": ["08:00", "20:00"]}}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let timed = rows(&target, &path, &fixture.view_access_token, &date);
    assert_eq!(timed.len(), 2);
    assert_ne!(timed[0]["key"], timed[1]["key"]);
    assert_eq!(timed[0]["position"], 1);
    assert_eq!(timed[1]["position"], 2);
    for (row, clock_time) in timed.iter().zip(["08:00:00", "20:00:00"]) {
        let scheduled_at = row["scheduled_at"].as_str().unwrap();
        assert!(
            scheduled_at.starts_with(&format!("{date}T{clock_time}")),
            "unexpected local schedule time: {scheduled_at}"
        );
        assert!(scheduled_at.len() > 19);
    }
    assert_eq!(
        rows(&target, &path, &fixture.view_access_token, &date),
        timed
    );
    let response = target.patch_json(
        &format!(
            "/api/v1/households/{}/schedules/{schedule_id}",
            fixture.household_id
        ),
        &fixture.access_token,
        &json!({"schedule": {"frequency": "As needed"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert!(rows(&target, &path, &fixture.view_access_token, &date).is_empty());

    let (assignment_id, _) = create_routine_assignment(&target, &fixture);
    let assignment = assignment_path(&fixture, assignment_id);
    let source = format!(
        "/api/v1/households/{}/person_medications/{assignment_id}",
        fixture.household_id
    );
    let today = OffsetDateTime::now_utc().date();
    for cycle in ["weekly", "monthly"] {
        let response = target.patch_json(
            &source,
            &fixture.access_token,
            &json!({"person_medication": {"dose_cycle": cycle}}),
        );
        assert_eq!(response.status().as_u16(), 200);
        let projected = rows(&target, &assignment, &fixture.view_access_token, &date);
        assert_eq!(projected.len(), 1);
        let start = if cycle == "weekly" {
            today - time::Duration::days(today.weekday().number_days_from_monday().into())
        } else {
            time::Date::from_calendar_date(today.year(), today.month(), 1).unwrap()
        };
        let end = if cycle == "weekly" {
            start + time::Duration::days(6)
        } else {
            let next = if today.month() == time::Month::December {
                time::Date::from_calendar_date(today.year() + 1, time::Month::January, 1).unwrap()
            } else {
                time::Date::from_calendar_date(today.year(), today.month().next(), 1).unwrap()
            };
            next - time::Duration::days(1)
        };
        assert_eq!(projected[0]["window_starts_on"], start.to_string());
        assert_eq!(projected[0]["window_ends_on"], end.to_string());
        assert_eq!(projected[0]["expected"], true);
        assert_eq!(
            rows(&target, &assignment, &fixture.view_access_token, &date),
            projected
        );
    }
    let response = target.patch_json(
        &source,
        &fixture.access_token,
        &json!({"person_medication": {"administration_kind": "as_needed"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert!(rows(&target, &assignment, &fixture.view_access_token, &date).is_empty());
}

#[test]
fn occurrence_ranges_and_view_token_non_disclosure_are_consistent() {
    let target = Target::from_env();
    let fixture = fixture();
    let (date, _) = clock();
    for path in [
        schedule_path(&fixture, fixture.managed_schedule_id),
        assignment_path(&fixture, fixture.managed_assignment_id),
    ] {
        let response = target.get(
            &format!("{path}?start_date={date}&end_date={date}"),
            Some(&fixture.view_access_token),
        );
        assert_eq!(response.status().as_u16(), 200);
        for invalid in [
            path.clone(),
            format!("{path}?start_date=private-clinical-text&end_date={date}"),
            format!("{path}?start_date=2099-12-31&end_date=2000-01-01"),
            format!("{path}?start_date=2000-01-01&end_date=2000-02-01"),
        ] {
            let response = target.get(&invalid, Some(&fixture.view_access_token));
            assert_eq!(response.status().as_u16(), 422);
            assert!(!body(response).to_string().contains("private-clinical-text"));
        }
    }
    for id in [fixture.hidden_schedule_id, fixture.foreign_schedule_id] {
        let response = target.get(
            &format!(
                "{}?start_date={date}&end_date={date}",
                schedule_path(&fixture, id)
            ),
            Some(&fixture.view_access_token),
        );
        assert_eq!(response.status().as_u16(), 404);
    }
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
fn occurrence_writes_reject_invalid_identity_context_time_and_amount_without_mutation() {
    let target = Target::from_env();
    let fixture = fixture();
    let (date, taken_at) = clock();
    let (schedule_id, medication_id) = create_schedule(&target, &fixture);
    let path = schedule_path(&fixture, schedule_id);
    let source_id = body(target.get(
        &format!(
            "/api/v1/households/{}/schedules/{schedule_id}",
            fixture.household_id
        ),
        Some(&fixture.access_token),
    ))["data"]["portable_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let open = rows(&target, &path, &fixture.access_token, &date)[0].clone();
    let key = open["key"].clone();
    let before_stock = stock(&target, &fixture, medication_id);
    let before_takes = body(target.get(&takes_path(&fixture), Some(&fixture.access_token)))["meta"]
        ["total_count"]
        .clone();
    for (invalid_key, status) in [(json!("private-clinical-text"), 422), (json!(null), 422)] {
        let response = target.post_json_authorized(
            &format!("{path}/not_taken"),
            &fixture.access_token,
            &json!({"dose_occurrence": {"key": invalid_key, "reason": "unwell"}}),
        );
        assert_eq!(response.status().as_u16(), status);
        assert!(!body(response).to_string().contains("private-clinical-text"));
    }
    let response = target.post_json_authorized(
        &format!("{path}/not_taken"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": key, "reason": "private-clinical-text"}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert!(!body(response).to_string().contains("private-clinical-text"));
    for (time, amount) in [
        (json!("invalid"), json!("1.25")),
        (json!(taken_at), json!(1.25)),
        (json!(taken_at), json!("0")),
        (json!(taken_at), json!("-1")),
    ] {
        let response = target.post_json_authorized(
            &format!("{path}/take"),
            &fixture.access_token,
            &json!({"dose_occurrence": {"key": key, "taken_at": time, "dose_amount": amount,
                "taken_from_medication_id": medication_id}}),
        );
        assert_eq!(response.status().as_u16(), 422);
    }
    let response = target.post_json_authorized(
        &takes_path(&fixture),
        &fixture.access_token,
        &json!({"medication_take": {"source_type": "schedule", "source_id": source_id,
            "taken_at": taken_at, "dose_amount": 1.25}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(body(response)["error"]["code"], "validation_failed");
    for amount in ["0", "-1"] {
        let response = target.post_json_authorized(
            &takes_path(&fixture),
            &fixture.access_token,
            &json!({"medication_take": {"source_type": "schedule",
                "source_id": source_id,
                "taken_at": taken_at, "dose_amount": amount}}),
        );
        assert_eq!(response.status().as_u16(), 422);
    }
    assert_eq!(rows(&target, &path, &fixture.access_token, &date)[0], open);
    assert_eq!(stock(&target, &fixture, medication_id), before_stock);
    assert_eq!(
        body(target.get(&takes_path(&fixture), Some(&fixture.access_token)))["meta"]["total_count"],
        before_takes
    );
}

#[test]
fn malformed_and_future_keys_leave_schedule_and_assignment_writes_unchanged() {
    let target = Target::from_env();
    let fixture = fixture();
    let (date, taken_at) = clock();
    let tomorrow = (OffsetDateTime::now_utc().date() + time::Duration::days(1)).to_string();
    let (schedule_id, schedule_medication_id) = create_schedule(&target, &fixture);
    let (assignment_id, assignment_medication_id) = create_routine_assignment(&target, &fixture);
    for (path, medication_id) in [
        (schedule_path(&fixture, schedule_id), schedule_medication_id),
        (
            assignment_path(&fixture, assignment_id),
            assignment_medication_id,
        ),
    ] {
        let today = rows(&target, &path, &fixture.access_token, &date);
        let future = rows(&target, &path, &fixture.access_token, &tomorrow);
        assert!(!today.is_empty());
        assert!(!future.is_empty());
        assert!(future.iter().all(|row| row["due"] == false));
        let stock_before = stock(&target, &fixture, medication_id);
        let takes_before = take_snapshot(&target, &fixture);
        for key in [json!("private-clinical-text"), future[0]["key"].clone()] {
            let response = target.post_json_authorized(
                &format!("{path}/not_taken"),
                &fixture.access_token,
                &json!({"dose_occurrence": {"key": key, "reason": "unwell"}}),
            );
            assert_eq!(response.status().as_u16(), 422);
            assert_eq!(body(response)["error"]["code"], "invalid_occurrence");
            let response = target.post_json_authorized(
                &format!("{path}/take"),
                &fixture.access_token,
                &json!({"dose_occurrence": {"key": key, "taken_at": taken_at,
                    "taken_from_medication_id": medication_id}}),
            );
            assert_eq!(response.status().as_u16(), 422);
            assert_eq!(body(response)["error"]["code"], "invalid_occurrence");
        }
        assert_eq!(rows(&target, &path, &fixture.access_token, &date), today);
        assert_eq!(
            rows(&target, &path, &fixture.access_token, &tomorrow),
            future
        );
        assert_eq!(stock(&target, &fixture, medication_id), stock_before);
        assert_eq!(take_snapshot(&target, &fixture), takes_before);
    }
}

#[test]
fn future_occurrences_and_changed_cycle_keys_cannot_be_resolved() {
    let target = Target::from_env();
    let fixture = fixture();
    let (date, _) = clock();
    let tomorrow = (OffsetDateTime::now_utc().date() + time::Duration::days(1)).to_string();
    let (schedule_id, medication_id) = create_schedule(&target, &fixture);
    let path = schedule_path(&fixture, schedule_id);
    let future = rows(&target, &path, &fixture.access_token, &tomorrow);
    assert!(!future.is_empty());
    assert!(future.iter().all(|row| row["due"] == false));
    let before_stock = stock(&target, &fixture, medication_id);
    let response = target.post_json_authorized(
        &format!("{path}/not_taken"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": future[0]["key"], "reason": "unwell"}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(body(response)["error"]["code"], "invalid_occurrence");
    assert_eq!(stock(&target, &fixture, medication_id), before_stock);

    let (assignment_id, assignment_medication_id) = create_routine_assignment(&target, &fixture);
    let assignment = assignment_path(&fixture, assignment_id);
    let daily = rows(&target, &assignment, &fixture.access_token, &date)[0].clone();
    let response = target.post_json_authorized(
        &format!("{assignment}/not_taken"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": daily["key"], "reason": "unwell", "note": "Retained"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let saved = body(response)["data"].clone();
    let source = format!(
        "/api/v1/households/{}/person_medications/{assignment_id}",
        fixture.household_id
    );
    let response = target.patch_json(
        &source,
        &fixture.access_token,
        &json!({"person_medication": {"administration_kind": "as_needed"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let historical = rows(&target, &assignment, &fixture.view_access_token, &date);
    assert_eq!(historical.len(), 1);
    assert_eq!(historical[0]["key"], saved["key"]);
    assert_eq!(historical[0]["outcome"], "not_taken");
    assert_eq!(historical[0]["reason"], "unwell");
    assert_eq!(historical[0]["note"], "Retained");
    assert_eq!(historical[0]["expected"], false);
    let response = target.post_json_authorized(
        &format!("{assignment}/not_taken"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": daily["key"], "reason": "refused"}}),
    );
    assert_eq!(response.status().as_u16(), 409);
    assert_eq!(
        rows(&target, &assignment, &fixture.access_token, &date),
        historical
    );
    assert_eq!(stock(&target, &fixture, assignment_medication_id), "20.0");

    let (other_id, _) = create_routine_assignment(&target, &fixture);
    let other = assignment_path(&fixture, other_id);
    let former_key = rows(&target, &other, &fixture.access_token, &date)[0]["key"].clone();
    let response = target.patch_json(
        &format!(
            "/api/v1/households/{}/person_medications/{other_id}",
            fixture.household_id
        ),
        &fixture.access_token,
        &json!({"person_medication": {"administration_kind": "as_needed"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let response = target.post_json_authorized(
        &format!("{other}/not_taken"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": former_key, "reason": "unwell"}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert!(rows(&target, &other, &fixture.access_token, &date).is_empty());
}

#[test]
fn direct_take_failures_leave_stock_and_take_history_unchanged() {
    let target = Target::from_env();
    let fixture = fixture();
    let (_, taken_at) = clock();
    let (schedule_id, medication_id) = create_schedule(&target, &fixture);
    let source_path = format!(
        "/api/v1/households/{}/schedules/{schedule_id}",
        fixture.household_id
    );
    let source = body(target.get(&source_path, Some(&fixture.access_token)))["data"]["portable_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let path = takes_path(&fixture);
    let payload = json!({"medication_take": {"source_type": "schedule", "source_id": source,
        "taken_at": taken_at, "taken_from_medication_id": medication_id}});
    let count = body(target.get(&path, Some(&fixture.access_token)))["meta"]["total_count"].clone();
    let (date, _) = clock();
    let occurrence_path = schedule_path(&fixture, schedule_id);
    let occurrence_key =
        rows(&target, &occurrence_path, &fixture.access_token, &date)[0]["key"].clone();
    let occurrence_payload = json!({"dose_occurrence": {"key": occurrence_key,
        "taken_at": taken_at, "taken_from_medication_id": medication_id}});
    let pause_path = format!(
        "/api/v1/households/{}/schedules/{source}/pause",
        fixture.household_id
    );
    let response = target.patch_json(&pause_path, &fixture.access_token, &json!({}));
    assert_eq!(response.status().as_u16(), 200);
    let response = target.post_json_authorized(
        &format!("{occurrence_path}/take"),
        &fixture.access_token,
        &occurrence_payload,
    );
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(body(response)["error"]["code"], "paused");
    let response = target.post_json_authorized(&path, &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(
        body(response)["error"]["message"],
        "Cannot take medication: paused"
    );
    assert_eq!(stock(&target, &fixture, medication_id), "20.0");
    let resume_path = format!(
        "/api/v1/households/{}/schedules/{source}/resume",
        fixture.household_id
    );
    let response = target.patch_json(&resume_path, &fixture.access_token, &json!({}));
    assert_eq!(response.status().as_u16(), 200);
    let medication_path = format!(
        "/api/v1/households/{}/medications/{medication_id}",
        fixture.household_id
    );
    let response = target.patch_json(
        &medication_path,
        &fixture.access_token,
        &json!({"medication": {"current_supply": "0.0"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let response = target.post_json_authorized(
        &format!("{occurrence_path}/take"),
        &fixture.access_token,
        &occurrence_payload,
    );
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(body(response)["error"]["code"], "out_of_stock");
    let response = target.post_json_authorized(&path, &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(
        body(response)["error"]["message"],
        "Cannot take medication: out of stock"
    );
    assert_eq!(stock(&target, &fixture, medication_id), "0.0");
    assert_eq!(
        body(target.get(&path, Some(&fixture.access_token)))["meta"]["total_count"],
        count
    );

    let (cooldown_id, cooldown_medication_id) = create_schedule(&target, &fixture);
    let cooldown_path = format!(
        "/api/v1/households/{}/schedules/{cooldown_id}",
        fixture.household_id
    );
    let response = target.patch_json(
        &cooldown_path,
        &fixture.access_token,
        &json!({"schedule": {"max_daily_doses": 3, "min_hours_between_doses": "24.0"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let cooldown_source = body(target.get(&cooldown_path, Some(&fixture.access_token)))["data"]
        ["portable_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let cooldown_payload = json!({"medication_take": {"source_type": "schedule",
        "source_id": cooldown_source, "taken_at": taken_at,
        "taken_from_medication_id": cooldown_medication_id}});
    let response = target.post_json_authorized(&path, &fixture.access_token, &cooldown_payload);
    assert_eq!(response.status().as_u16(), 201);
    let first = body(response)["data"].clone();
    let before = stock(&target, &fixture, cooldown_medication_id);
    let takes_before = take_snapshot(&target, &fixture);
    assert!(takes_before.1.contains(&first["id"].as_i64().unwrap()));
    let response = target.post_json_authorized(&path, &fixture.access_token, &cooldown_payload);
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(
        body(response)["error"]["message"],
        "Cannot take medication: timing restrictions not met"
    );
    assert_eq!(stock(&target, &fixture, cooldown_medication_id), before);
    assert_eq!(take_snapshot(&target, &fixture), takes_before);
}
