use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::{Client, Response};
use reqwest::Method;
use serde_json::{json, Value};
use std::env;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration as StdDuration;
use time::{Duration, OffsetDateTime};

fn body(response: Response) -> Value {
    response.json().expect("JSON response")
}

fn assert_error(response: Response, status: u16) -> Value {
    assert_eq!(response.status().as_u16(), status);
    let error = body(response);
    assert!(error["error"]["code"].as_str().is_some());
    assert!(error["error"]["message"].as_str().is_some());
    assert!(error["error"]["request_id"].as_str().is_some());
    assert!(error.get("data").is_none());
    error
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

fn database() -> postgres::Client {
    postgres::Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract audit database URL"),
        postgres::NoTls,
    )
    .expect("contract audit database")
}

fn request_audit_count(request: &str) -> i64 {
    database()
        .query_one(
            "SELECT count(*) FROM security_audit_events WHERE request_id = $1 AND event_type = 'api.request'",
            &[&request],
        )
        .expect("request audit count")
        .get(0)
}

fn occurrence_record_id(source_type: &str, source_id: i64, date: &str, position: i32) -> i64 {
    let source_column = match source_type {
        "schedule" => "schedule_id",
        "person_medication" => "person_medication_id",
        _ => unreachable!(),
    };
    let query = format!(
        "SELECT id FROM medication_dose_occurrences WHERE {source_column} = $1 AND window_starts_on::text = $2 AND position = $3"
    );
    database()
        .query_one(&query, &[&source_id, &date, &position])
        .expect("occurrence record ID")
        .get(0)
}

fn occurrence_sync_count(id: i64) -> i64 {
    database()
        .query_one(
            "SELECT count(*) FROM api_change_events WHERE record_type = 'MedicationDoseOccurrence' AND record_id = $1",
            &[&id],
        )
        .expect("occurrence sync count")
        .get(0)
}

fn occurrence_version_count(id: i64) -> i64 {
    database()
        .query_one(
            "SELECT count(*) FROM versions WHERE item_type = 'MedicationDoseOccurrence' AND item_id = $1",
            &[&id],
        )
        .expect("occurrence version count")
        .get(0)
}

struct TemporaryCareRecordGrant {
    db: postgres::Client,
    id: i64,
    original: String,
}

impl TemporaryCareRecordGrant {
    fn new(fixture: &Fixture) -> Self {
        let mut db = database();
        let row = db
            .query_one(
                "SELECT grants.id, grants.access_level FROM person_access_grants grants JOIN household_memberships memberships ON memberships.id = grants.household_membership_id JOIN people carer ON carer.id = memberships.person_id WHERE grants.household_id = $1 AND grants.person_id = $2 AND memberships.household_id = $1 AND carer.name LIKE 'Contract carer %' AND grants.revoked_at IS NULL",
                &[&fixture.household_id, &fixture.managed_person_id],
            )
            .expect("care token grant for managed person");
        let id = row.get(0);
        let original: String = row.get(1);
        assert_eq!(original, "manage");
        db.execute(
            "UPDATE person_access_grants SET access_level = 'record' WHERE id = $1",
            &[&id],
        )
        .expect("reduce care grant to record");
        Self { db, id, original }
    }
}

impl Drop for TemporaryCareRecordGrant {
    fn drop(&mut self) {
        let _ = self.db.execute(
            "UPDATE person_access_grants SET access_level = $1 WHERE id = $2",
            &[&self.original, &self.id],
        );
    }
}

fn retire_assignment(id: i64) {
    database()
        .execute(
            "UPDATE person_medications SET active = false, retired_at = now() WHERE id = $1",
            &[&id],
        )
        .expect("retire owned assignment");
}

fn assignment_created_at(id: i64) -> String {
    database()
        .query_one(
            "SELECT created_at::text FROM person_medications WHERE id = $1",
            &[&id],
        )
        .expect("assignment creation timestamp")
        .get(0)
}

fn clock() -> (String, String) {
    let now = OffsetDateTime::now_utc();
    (
        now.date().to_string(),
        now.format(&time::format_description::well_known::Rfc3339)
            .unwrap(),
    )
}

fn client_uuid(source_type: &str, source_id: i64) -> String {
    client_uuid_variant(source_type, source_id, 0)
}

fn client_uuid_variant(source_type: &str, source_id: i64, variant: u8) -> String {
    let namespace = match source_type {
        "schedule" => "01",
        "person_medication" => "02",
        _ => unreachable!(),
    };
    format!("33333333-3333-4333-8333-{namespace}{source_id:08x}{variant:02x}")
}

fn source_path(fixture: &Fixture, source_type: &str, id: i64) -> String {
    source_path_for_household(fixture.household_id, source_type, id)
}

fn source_path_for_household(household_id: i64, source_type: &str, id: i64) -> String {
    match source_type {
        "schedule" => format!(
            "/api/v1/households/{}/schedules/{id}/dose_occurrences",
            household_id
        ),
        "person_medication" => format!(
            "/api/v1/households/{}/person_medications/{id}/dose_occurrences",
            household_id
        ),
        _ => unreachable!(),
    }
}

fn rows(target: &Target, path: &str, token: &str, date: &str) -> Vec<Value> {
    let response = target.get(
        &format!("{path}?start_date={date}&end_date={date}"),
        Some(token),
    );
    assert_eq!(response.status().as_u16(), 200);
    body(response)["data"].as_array().unwrap().clone()
}

fn rows_range(target: &Target, path: &str, token: &str, start: &str, end: &str) -> Vec<Value> {
    let response = target.get(
        &format!("{path}?start_date={start}&end_date={end}"),
        Some(token),
    );
    assert_eq!(response.status().as_u16(), 200);
    body(response)["data"].as_array().unwrap().clone()
}

fn stock(target: &Target, fixture: &Fixture, medication_id: i64) -> f64 {
    let response = target.get(
        &format!(
            "/api/v1/households/{}/medications/{medication_id}",
            fixture.household_id
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    body(response)["data"]["current_supply"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap()
}

fn rate_limited_response(response: Response) {
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
    let value: Value = response.json().expect("rate limit JSON");
    assert_eq!(value["error"]["code"], "rate_limited");
}

fn rate_request(
    client: &Client,
    base_url: &str,
    method: Method,
    path: &str,
    token: &str,
    payload: Option<&Value>,
    if_match: Option<&str>,
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
    if let Some(if_match) = if_match {
        request = request.header("If-Match", if_match);
    }
    request.send().expect("rate limit request")
}

fn create_medication(target: &Target, fixture: &Fixture) -> Value {
    let name = format!(
        "Dose occurrence {}",
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
            "current_supply": "20.0",
            "reorder_threshold": "5.0"
        }}),
    );
    assert_eq!(response.status().as_u16(), 201);
    body(response)["data"].clone()
}

fn create_source(target: &Target, fixture: &Fixture, source_type: &str) -> (i64, i64) {
    let medication = create_medication(target, fixture);
    let (path, payload) = if source_type == "schedule" {
        let today = OffsetDateTime::now_utc().date();
        let start = (today - Duration::days(1)).to_string();
        let end = (today + Duration::days(30)).to_string();
        (
            format!("/api/v1/households/{}/schedules", fixture.household_id),
            json!({
                "schedule": {
                    "person_id": fixture.managed_person_portable_id,
                    "medication_id": medication["portable_id"],
                    "dose_amount": "1.25",
                    "dose_unit": "ml",
                    "frequency": "Daily",
                    "max_daily_doses": 1,
                    "start_date": start,
                    "end_date": end
                }
            }),
        )
    } else {
        (
            format!(
                "/api/v1/households/{}/person_medications",
                fixture.household_id
            ),
            json!({
                "person_medication": {
                    "person_id": fixture.managed_person_portable_id,
                    "medication_id": medication["portable_id"],
                    "dose_amount": "1.25",
                    "dose_unit": "ml",
                    "administration_kind": "routine",
                    "dose_cycle": "daily",
                    "max_daily_doses": 1
                }
            }),
        )
    };
    let response = target.post_json_authorized(&path, &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 201);
    (
        body(response)["data"]["id"].as_i64().unwrap(),
        medication["id"].as_i64().unwrap(),
    )
}

fn assert_occurrence_schema(row: &Value) {
    let expected = [
        "key",
        "source_type",
        "source_id",
        "source_portable_id",
        "window_starts_on",
        "window_ends_on",
        "position",
        "scheduled_at",
        "outcome",
        "expected",
        "due",
        "reason",
        "note",
        "resolved_at",
        "medication_take_id",
        "etag",
    ];
    let object = row.as_object().expect("occurrence object");
    let mut keys: Vec<_> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    let mut expected = expected.to_vec();
    expected.sort_unstable();
    assert_eq!(keys, expected);
    assert!(row["key"].as_str().is_some());
    assert!(row["position"]
        .as_u64()
        .is_some_and(|position| position > 0));
    assert!(row["due"].is_boolean());
    assert!(row["expected"].is_boolean());
}

fn exercise_source_lifecycle(target: &Target, fixture: &Fixture, source_type: &str) {
    let _record_only_care_grant = TemporaryCareRecordGrant::new(fixture);
    let (id, medication_id) = create_source(target, fixture, source_type);
    let path = source_path(fixture, source_type, id);
    let (date, taken_at) = clock();
    let open = rows(target, &path, &fixture.access_token, &date);
    assert!(!open.is_empty());
    assert_occurrence_schema(&open[0]);
    assert_eq!(open[0]["source_type"], source_type);
    assert_eq!(open[0]["source_id"], id);
    assert_eq!(open[0]["outcome"], "open");
    assert!(open[0]["etag"].is_null());
    let key = open[0]["key"].as_str().unwrap().to_owned();
    let initial_stock = stock(target, fixture, medication_id);
    assert_eq!(rows(target, &path, &fixture.view_access_token, &date), open);

    let today = OffsetDateTime::now_utc().date();
    let start = today.to_string();
    let end_31 = (today + Duration::days(30)).to_string();
    let end_32 = (today + Duration::days(31)).to_string();
    let rows_31 = rows_range(target, &path, &fixture.access_token, &start, &end_31);
    assert_eq!(rows_31.len(), 31);
    assert_eq!(
        rows_range(target, &path, &fixture.access_token, &start, &end_31),
        rows_31
    );
    assert_error(
        target.get(
            &format!("{path}?start_date={start}&end_date={end_32}"),
            Some(&fixture.access_token),
        ),
        422,
    );
    assert_error(target.get(&path, Some(&fixture.access_token)), 422);
    let malformed_range = assert_error(
        target.get(
            &format!("{path}?start_date=private-clinical-text&end_date={start}"),
            Some(&fixture.access_token),
        ),
        422,
    );
    assert!(!malformed_range
        .to_string()
        .contains("private-clinical-text"));

    assert_error(
        target.post_json_authorized(
            &format!("{path}/not_taken"),
            &fixture.access_token,
            &json!({}),
        ),
        400,
    );
    assert_error(
        target.post_json_authorized(
            &format!("{path}/not_taken"),
            &fixture.access_token,
            &json!({"dose_occurrence": null}),
        ),
        400,
    );
    assert_error(
        target.post_json_authorized(
            &format!("{path}/not_taken"),
            &fixture.access_token,
            &json!({"unknown": true, "dose_occurrence": {"key": key}}),
        ),
        422,
    );
    assert_error(
        target.post_json_authorized(
            &format!("{path}/not_taken"),
            &fixture.access_token,
            &json!({"dose_occurrence": {"key": key, "reason": "invalid"}}),
        ),
        422,
    );
    let long_note = "x".repeat(2001);
    assert_error(
        target.post_json_authorized(
            &format!("{path}/not_taken"),
            &fixture.access_token,
            &json!({"dose_occurrence": {"key": key, "note": long_note}}),
        ),
        422,
    );

    let not_taken_payload =
        json!({"dose_occurrence": {"key": key, "reason": null, "note": "Resting"}});
    let reopen_path = format!("{path}/reopen");
    let reopen_payload = json!({"dose_occurrence": {"key": key}});
    assert_error(
        target.patch_json_if_match(&reopen_path, &fixture.access_token, &json!({}), "stale"),
        400,
    );
    assert_error(
        target.patch_json_if_match(
            &reopen_path,
            &fixture.access_token,
            &json!({"dose_occurrence": null}),
            "stale",
        ),
        400,
    );
    assert_error(
        target.patch_json_if_match(
            &reopen_path,
            &fixture.access_token,
            &json!({"dose_occurrence": {"key": key, "unexpected": true}}),
            "stale",
        ),
        422,
    );
    let take_path = format!("{path}/take");
    assert_error(
        target.post_json_authorized(&take_path, &fixture.access_token, &json!({})),
        400,
    );
    assert_error(
        target.post_json_authorized(
            &take_path,
            &fixture.access_token,
            &json!({"dose_occurrence": null}),
        ),
        400,
    );
    assert_error(
        target.post_json_authorized(
            &take_path,
            &fixture.access_token,
            &json!({"dose_occurrence": {"key": key, "unexpected": true}}),
        ),
        422,
    );
    assert_error(
        target.post_json_authorized(
            &take_path,
            &fixture.access_token,
            &json!({"dose_occurrence": {
                "key": key,
                "taken_at": "invalid",
                "client_uuid": client_uuid(source_type, id)
            }}),
        ),
        422,
    );
    assert_eq!(stock(target, fixture, medication_id), initial_stock);
    assert_eq!(rows(target, &path, &fixture.access_token, &date), open);
    assert_error(
        target.post_json_authorized(
            &format!("{path}/not_taken"),
            &fixture.view_access_token,
            &not_taken_payload,
        ),
        403,
    );

    let not_taken_response = target.post_json_authorized(
        &format!("{path}/not_taken"),
        &fixture.care_access_token,
        &not_taken_payload,
    );
    assert_eq!(not_taken_response.status().as_u16(), 200);
    let not_taken_etag = etag(&not_taken_response);
    let not_taken = body(not_taken_response)["data"].clone();
    assert_occurrence_schema(&not_taken);
    assert_eq!(not_taken["outcome"], "not_taken");
    assert!(not_taken["reason"].is_null());
    assert_eq!(not_taken["note"], "Resting");
    assert_eq!(not_taken["etag"], not_taken_etag);
    assert_eq!(
        rows(target, &path, &fixture.view_access_token, &date)[0],
        not_taken
    );
    assert_eq!(stock(target, fixture, medication_id), initial_stock);

    let replay = target.post_json_authorized(
        &format!("{path}/not_taken"),
        &fixture.care_access_token,
        &not_taken_payload,
    );
    assert_eq!(replay.status().as_u16(), 200);
    assert_eq!(etag(&replay), not_taken_etag);
    assert_eq!(body(replay)["data"], not_taken);
    let changed_decision = target.post_json_authorized(
        &format!("{path}/not_taken"),
        &fixture.care_access_token,
        &json!({"dose_occurrence": {"key": key, "reason": "refused", "note": "Resting"}}),
    );
    assert_error(changed_decision, 409);

    assert_eq!(
        assert_error(
            target.patch_json(&reopen_path, &fixture.access_token, &reopen_payload),
            428,
        )["error"]["code"],
        "precondition_required"
    );
    assert_eq!(
        assert_error(
            target.patch_json_if_match(
                &reopen_path,
                &fixture.access_token,
                &reopen_payload,
                "stale",
            ),
            409,
        )["error"]["code"],
        "sync_conflict"
    );
    assert_error(
        target.patch_json_if_match(
            &reopen_path,
            &fixture.care_access_token,
            &reopen_payload,
            &not_taken_etag,
        ),
        403,
    );

    let reopen_response = target.patch_json_if_match(
        &reopen_path,
        &fixture.access_token,
        &reopen_payload,
        &not_taken_etag,
    );
    assert_eq!(reopen_response.status().as_u16(), 200);
    let reopened = body(reopen_response)["data"].clone();
    assert_eq!(reopened["outcome"], "open");
    assert!(reopened["reason"].is_null());
    assert!(reopened["note"].is_null());
    assert_ne!(reopened["etag"], not_taken_etag);
    assert_eq!(
        rows(target, &path, &fixture.access_token, &date)[0],
        reopened
    );
    assert_eq!(stock(target, fixture, medication_id), initial_stock);

    let second_decision = target.post_json_authorized(
        &format!("{path}/not_taken"),
        &fixture.care_access_token,
        &not_taken_payload,
    );
    assert_eq!(second_decision.status().as_u16(), 200);
    let second_decision_etag = etag(&second_decision);
    assert_eq!(body(second_decision)["data"]["outcome"], "not_taken");
    let before_stock = stock(target, fixture, medication_id);
    let take_payload = json!({"dose_occurrence": {
        "key": key,
        "taken_at": taken_at,
        "client_uuid": client_uuid(source_type, id),
        "dose_amount": "1.25",
        "taken_from_medication_id": medication_id
    }});
    assert_eq!(
        assert_error(
            target.post_json_authorized(
                &format!("{path}/take"),
                &fixture.care_access_token,
                &take_payload,
            ),
            428,
        )["error"]["code"],
        "precondition_required"
    );
    assert_eq!(
        assert_error(
            target.post_json_if_match(
                &format!("{path}/take"),
                &fixture.care_access_token,
                &take_payload,
                "stale",
            ),
            409,
        )["error"]["code"],
        "sync_conflict"
    );
    assert_error(
        target.post_json_if_match(
            &format!("{path}/take"),
            &fixture.view_access_token,
            &take_payload,
            &second_decision_etag,
        ),
        403,
    );

    let taken_response = target.post_json_if_match(
        &format!("{path}/take"),
        &fixture.care_access_token,
        &take_payload,
        &second_decision_etag,
    );
    assert_eq!(taken_response.status().as_u16(), 200);
    let taken = body(taken_response)["data"].clone();
    assert_eq!(taken["outcome"], "taken");
    assert!(taken["medication_take_id"].as_i64().is_some());
    assert!(taken["reason"].is_null());
    assert!(taken["note"].is_null());
    assert_eq!(
        rows(target, &path, &fixture.view_access_token, &date)[0],
        taken
    );
    assert_eq!(stock(target, fixture, medication_id), before_stock - 1.25);

    let replay = target.post_json_if_match(
        &format!("{path}/take"),
        &fixture.care_access_token,
        &take_payload,
        &second_decision_etag,
    );
    assert_eq!(replay.status().as_u16(), 200);
    assert_eq!(
        body(replay)["data"]["medication_take_id"],
        taken["medication_take_id"]
    );
    assert_eq!(stock(target, fixture, medication_id), before_stock - 1.25);
    let mut changed_take = take_payload;
    changed_take["dose_occurrence"]["dose_amount"] = json!("1.5");
    let changed_replay = target.post_json_if_match(
        &format!("{path}/take"),
        &fixture.care_access_token,
        &changed_take,
        &second_decision_etag,
    );
    assert_error(changed_replay, 409);
    assert_eq!(stock(target, fixture, medication_id), before_stock - 1.25);
}

#[test]
fn schedule_occurrence_list_not_taken_reopen_and_take_follow_the_contract() {
    let target = Target::from_env();
    let fixture = fixture();
    exercise_source_lifecycle(&target, &fixture, "schedule");
}

#[test]
fn person_medication_occurrence_list_not_taken_reopen_and_take_follow_the_contract() {
    let target = Target::from_env();
    let fixture = fixture();
    exercise_source_lifecycle(&target, &fixture, "person_medication");
}

#[test]
fn occurrence_operations_enforce_household_and_source_visibility() {
    let target = Target::from_env();
    let fixture = fixture();
    let (date, taken_at) = clock();
    let cases = [
        (
            "schedule",
            fixture.managed_schedule_id,
            fixture.hidden_schedule_id,
            fixture.foreign_schedule_id,
        ),
        (
            "person_medication",
            fixture.managed_assignment_id,
            fixture.hidden_assignment_id,
            fixture.foreign_assignment_id,
        ),
    ];
    for (source_type, managed_id, hidden_id, foreign_id) in cases {
        let managed = source_path(&fixture, source_type, managed_id);
        assert_error(target.get(&managed, None), 401);
        let visible = target.get(
            &format!("{managed}?start_date={date}&end_date={date}"),
            Some(&fixture.view_access_token),
        );
        assert_eq!(visible.status().as_u16(), 200);

        for hidden_id in [hidden_id, foreign_id] {
            let hidden = source_path(&fixture, source_type, hidden_id);
            assert_error(
                target.get(
                    &format!("{hidden}?start_date={date}&end_date={date}"),
                    Some(&fixture.access_token),
                ),
                404,
            );
            assert_error(
                target.post_json_authorized(
                    &format!("{hidden}/not_taken"),
                    &fixture.access_token,
                    &json!({"dose_occurrence": {"key": "opaque-key"}}),
                ),
                404,
            );
            assert_error(
                target.patch_json_if_match(
                    &format!("{hidden}/reopen"),
                    &fixture.access_token,
                    &json!({"dose_occurrence": {"key": "opaque-key"}}),
                    "stale",
                ),
                404,
            );
            assert_error(
                target.post_json_authorized(
                    &format!("{hidden}/take"),
                    &fixture.access_token,
                    &json!({"dose_occurrence": {
                        "key": "opaque-key",
                        "taken_at": taken_at,
                        "client_uuid": client_uuid(source_type, managed_id)
                    }}),
                ),
                404,
            );
        }

        let other_household =
            source_path_for_household(fixture.foreign_household_id, source_type, managed_id);
        assert_error(
            target.get(
                &format!("{other_household}?start_date={date}&end_date={date}"),
                Some(&fixture.access_token),
            ),
            403,
        );
        assert_error(
            target.post_json_authorized(
                &format!("{other_household}/not_taken"),
                &fixture.access_token,
                &json!({"dose_occurrence": {"key": "opaque-key"}}),
            ),
            403,
        );
        assert_error(
            target.patch_json_if_match(
                &format!("{other_household}/reopen"),
                &fixture.access_token,
                &json!({"dose_occurrence": {"key": "opaque-key"}}),
                "stale",
            ),
            403,
        );
        assert_error(
            target.post_json_authorized(
                &format!("{other_household}/take"),
                &fixture.access_token,
                &json!({"dose_occurrence": {
                    "key": "opaque-key",
                    "taken_at": taken_at,
                    "client_uuid": client_uuid(source_type, managed_id)
                }}),
            ),
            403,
        );
    }
}

#[test]
fn occurrence_keys_are_bound_to_the_source_and_reject_tampering_without_mutation() {
    let target = Target::from_env();
    let fixture = fixture();
    let (schedule_id, schedule_medication_id) = create_source(&target, &fixture, "schedule");
    let (assignment_id, assignment_medication_id) =
        create_source(&target, &fixture, "person_medication");
    let (date, _) = clock();
    let schedule_path = source_path(&fixture, "schedule", schedule_id);
    let assignment_path = source_path(&fixture, "person_medication", assignment_id);
    let schedule_rows = rows(&target, &schedule_path, &fixture.access_token, &date);
    let assignment_rows = rows(&target, &assignment_path, &fixture.access_token, &date);
    let original_schedule = schedule_rows[0]["key"].as_str().unwrap();
    let original_assignment = assignment_rows[0]["key"].as_str().unwrap();
    let mut tampered = original_schedule.to_owned();
    tampered.push('x');
    let initial_schedule_stock = stock(&target, &fixture, schedule_medication_id);
    let initial_assignment_stock = stock(&target, &fixture, assignment_medication_id);

    for (path, key) in [
        (&schedule_path, original_assignment),
        (&assignment_path, original_schedule),
        (&schedule_path, tampered.as_str()),
    ] {
        assert_eq!(
            assert_error(
                target.post_json_authorized(
                    &format!("{path}/not_taken"),
                    &fixture.access_token,
                    &json!({"dose_occurrence": {"key": key, "reason": "unwell"}}),
                ),
                422,
            )["error"]["code"],
            "invalid_occurrence"
        );
    }

    assert_eq!(
        rows(&target, &schedule_path, &fixture.access_token, &date),
        schedule_rows
    );
    assert_eq!(
        rows(&target, &assignment_path, &fixture.access_token, &date),
        assignment_rows
    );
    assert_eq!(
        stock(&target, &fixture, schedule_medication_id),
        initial_schedule_stock
    );
    assert_eq!(
        stock(&target, &fixture, assignment_medication_id),
        initial_assignment_stock
    );
}

#[test]
fn simultaneous_takes_resolve_once_and_decrement_stock_once() {
    let fixture = fixture();
    let target = Target::from_env();
    let (source_id, medication_id) = create_source(&target, &fixture, "schedule");
    let path = source_path(&fixture, "schedule", source_id);
    let (date, taken_at) = clock();
    let occurrence = rows(&target, &path, &fixture.access_token, &date).remove(0);
    let key = occurrence["key"].as_str().unwrap().to_owned();
    let decision = target.post_json_authorized(
        &format!("{path}/not_taken"),
        &fixture.care_access_token,
        &json!({"dose_occurrence": {"key": key, "reason": "unwell"}}),
    );
    assert_eq!(decision.status().as_u16(), 200);
    let current_tag = etag(&decision);
    let before_stock = stock(&target, &fixture, medication_id);
    let base_url = env::var("CONTRACT_BASE_URL").expect("API base URL");
    let barrier = Arc::new(Barrier::new(2));
    let mut workers = Vec::new();
    for variant in [1, 2] {
        let barrier = Arc::clone(&barrier);
        let token = fixture.care_access_token.clone();
        let path = format!("{path}/take");
        let base_url = base_url.clone();
        let key = key.clone();
        let taken_at = taken_at.clone();
        let uuid = client_uuid_variant("schedule", source_id, variant);
        let etag = current_tag.clone();
        workers.push(thread::spawn(move || {
            let client = Client::builder().build().expect("take client");
            barrier.wait();
            client
                .post(format!("{}{}", base_url.trim_end_matches('/'), path))
                .bearer_auth(token)
                .header("If-Match", etag)
                .json(&json!({"dose_occurrence": {
                    "key": key,
                    "taken_at": taken_at,
                    "client_uuid": uuid,
                    "dose_amount": "1.25",
                    "taken_from_medication_id": medication_id
                }}))
                .send()
                .expect("concurrent take response")
        }));
    }
    let responses: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().expect("take worker"))
        .collect();
    let statuses: Vec<_> = responses
        .iter()
        .map(|response| response.status().as_u16())
        .collect();
    assert!(
        statuses.contains(&200),
        "expected one committed take, got {statuses:?}"
    );
    assert!(
        statuses.contains(&409),
        "expected one conflicting take, got {statuses:?}"
    );
    let final_rows = rows(&target, &path, &fixture.access_token, &date);
    assert_eq!(final_rows[0]["outcome"], "taken");
    assert!(final_rows[0]["medication_take_id"].as_i64().is_some());
    assert_eq!(stock(&target, &fixture, medication_id), before_stock - 1.25);
}

#[test]
fn keyed_not_taken_replay_audits_each_request_but_records_one_occurrence_change() {
    let target = Target::from_env();
    let fixture = fixture();
    let (source_id, _) = create_source(&target, &fixture, "schedule");
    let path = source_path(&fixture, "schedule", source_id);
    let (date, _) = clock();
    let occurrence = rows(&target, &path, &fixture.access_token, &date).remove(0);
    let key = occurrence["key"].as_str().unwrap().to_owned();
    let request_path = format!("{path}/not_taken");
    let payload = json!({"dose_occurrence": {"key": key, "reason": "unwell"}});
    let idempotency_key = format!("occurrence-not-taken-{}-{source_id}", fixture.household_id);

    let first = target.post_json_with_key(
        &request_path,
        &fixture.care_access_token,
        &idempotency_key,
        &payload,
    );
    assert_eq!(first.status().as_u16(), 200);
    let first_tag = etag(&first);
    let first_request = request_id(&first);
    let first_body = body(first);
    let row_id = occurrence_record_id("schedule", source_id, &date, 1);
    assert_eq!(request_audit_count(&first_request), 1);
    let sync_after_first = occurrence_sync_count(row_id);
    let version_after_first = occurrence_version_count(row_id);

    let replay = target.post_json_with_key(
        &request_path,
        &fixture.care_access_token,
        &idempotency_key,
        &payload,
    );
    assert_eq!(replay.status().as_u16(), 200);
    assert_eq!(replay.headers()["idempotency-replayed"], "true");
    assert_eq!(etag(&replay), first_tag);
    let replay_request = request_id(&replay);
    assert_ne!(replay_request, first_request);
    assert_eq!(request_audit_count(&replay_request), 1);
    assert_eq!(body(replay), first_body);
    assert_eq!(occurrence_sync_count(row_id), sync_after_first);
    assert_eq!(occurrence_version_count(row_id), version_after_first);

    let changed_payload = json!({"dose_occurrence": {"key": key, "reason": "refused"}});
    assert_eq!(
        assert_error(
            target.post_json_with_key(
                &request_path,
                &fixture.care_access_token,
                &idempotency_key,
                &changed_payload,
            ),
            409,
        )["error"]["code"],
        "idempotency_key_reused"
    );
    assert_eq!(occurrence_sync_count(row_id), sync_after_first);
    assert_eq!(occurrence_version_count(row_id), version_after_first);
}

#[test]
fn keyed_invalid_not_taken_replay_is_stable_and_rejects_changed_payload() {
    let target = Target::from_env();
    let fixture = fixture();
    let (source_id, _) = create_source(&target, &fixture, "schedule");
    let path = source_path(&fixture, "schedule", source_id);
    let (date, _) = clock();
    let key = rows(&target, &path, &fixture.access_token, &date)[0]["key"]
        .as_str()
        .unwrap()
        .to_owned();
    let request_path = format!("{path}/not_taken");
    let invalid = json!({"dose_occurrence": {"key": key, "reason": "not-a-reason"}});
    let changed = json!({"dose_occurrence": {"key": key, "reason": "unwell"}});
    let idem = format!("occurrence-invalid-{}-{source_id}", fixture.household_id);

    let first =
        target.post_json_with_key(&request_path, &fixture.care_access_token, &idem, &invalid);
    assert_eq!(first.status().as_u16(), 422);
    let first_request = request_id(&first);
    let first_body = body(first);
    assert_eq!(request_audit_count(&first_request), 1);

    let replay =
        target.post_json_with_key(&request_path, &fixture.care_access_token, &idem, &invalid);
    assert_eq!(replay.status().as_u16(), 422);
    assert_eq!(replay.headers()["idempotency-replayed"], "true");
    let replay_request = request_id(&replay);
    assert_ne!(replay_request, first_request);
    assert_eq!(request_audit_count(&replay_request), 1);
    assert_eq!(body(replay), first_body);

    assert_eq!(
        assert_error(
            target.post_json_with_key(&request_path, &fixture.care_access_token, &idem, &changed,),
            409,
        )["error"]["code"],
        "idempotency_key_reused"
    );
    let still_open = rows(&target, &path, &fixture.access_token, &date);
    assert_eq!(still_open[0]["outcome"], "open");
}

#[test]
fn unlinked_medication_take_projects_over_reopened_outcome_without_read_side_events() {
    let target = Target::from_env();
    let fixture = fixture();
    let (source_id, medication_id) = create_source(&target, &fixture, "schedule");
    let path = source_path(&fixture, "schedule", source_id);
    let (date, taken_at) = clock();
    let key = rows(&target, &path, &fixture.access_token, &date)[0]["key"]
        .as_str()
        .unwrap()
        .to_owned();
    let marked = target.post_json_authorized(
        &format!("{path}/not_taken"),
        &fixture.care_access_token,
        &json!({"dose_occurrence": {"key": key, "reason": "unwell", "note": "Observed"}}),
    );
    assert_eq!(marked.status().as_u16(), 200);
    let marked_tag = etag(&marked);
    let reopened = target.patch_json_if_match(
        &format!("{path}/reopen"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": key}}),
        &marked_tag,
    );
    assert_eq!(reopened.status().as_u16(), 200);
    assert_eq!(body(reopened)["data"]["outcome"], "open");

    let source_resource = format!(
        "/api/v1/households/{}/schedules/{source_id}",
        fixture.household_id
    );
    let source = target.get(&source_resource, Some(&fixture.access_token));
    assert_eq!(source.status().as_u16(), 200);
    let source_portable_id = body(source)["data"]["portable_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let direct_take = target.post_json_authorized(
        &format!(
            "/api/v1/households/{}/medication_takes",
            fixture.household_id
        ),
        &fixture.access_token,
        &json!({"medication_take": {
            "source_type": "schedule",
            "source_id": source_portable_id,
            "taken_at": taken_at,
            "client_uuid": client_uuid_variant("schedule", source_id, 99),
            "dose_amount": "1.25",
            "taken_from_medication_id": medication_id
        }}),
    );
    assert_eq!(direct_take.status().as_u16(), 201);
    let take_id = body(direct_take)["data"]["id"].clone();
    let occurrence_id = occurrence_record_id("schedule", source_id, &date, 1);
    let sync_before_projection = occurrence_sync_count(occurrence_id);
    let versions_before_projection = occurrence_version_count(occurrence_id);

    let projected = rows(&target, &path, &fixture.view_access_token, &date);
    assert_eq!(projected.len(), 1);
    assert_eq!(projected[0]["outcome"], "taken");
    assert_eq!(projected[0]["expected"], true);
    assert_eq!(projected[0]["medication_take_id"], take_id);
    assert_eq!(occurrence_sync_count(occurrence_id), sync_before_projection);
    assert_eq!(
        occurrence_version_count(occurrence_id),
        versions_before_projection
    );

    assert_error(
        target.post_json_authorized(
            &format!("{path}/not_taken"),
            &fixture.care_access_token,
            &json!({"dose_occurrence": {"key": key, "reason": "refused"}}),
        ),
        409,
    );
}

#[test]
fn projection_tracks_times_pause_cycle_windows_prn_and_retirement() {
    let target = Target::from_env();
    let fixture = fixture();
    let (date, _) = clock();
    let (schedule_id, _) = create_source(&target, &fixture, "schedule");
    let schedule_path = source_path(&fixture, "schedule", schedule_id);
    let schedule_resource = format!(
        "/api/v1/households/{}/schedules/{schedule_id}",
        fixture.household_id
    );
    let timed = target.patch_json(
        &schedule_resource,
        &fixture.access_token,
        &json!({"schedule": {"schedule_config": {"times": ["08:00", "20:00"]}}}),
    );
    assert_eq!(timed.status().as_u16(), 200);
    let timed_rows = rows(&target, &schedule_path, &fixture.view_access_token, &date);
    assert_eq!(timed_rows.len(), 2);
    for (row, time) in timed_rows.iter().zip(["08:00:00", "20:00:00"]) {
        assert!(row["scheduled_at"]
            .as_str()
            .unwrap()
            .starts_with(&format!("{date}T{time}")));
    }
    let tomorrow = (OffsetDateTime::now_utc().date() + Duration::days(1)).to_string();
    let tomorrow_rows = rows(
        &target,
        &schedule_path,
        &fixture.view_access_token,
        &tomorrow,
    );
    assert_eq!(tomorrow_rows.len(), 2);

    let pause = target.patch_json(
        &format!(
            "/api/v1/households/{}/schedules/{schedule_id}/pause",
            fixture.household_id
        ),
        &fixture.access_token,
        &json!({}),
    );
    assert_eq!(pause.status().as_u16(), 200);
    assert!(rows(
        &target,
        &schedule_path,
        &fixture.view_access_token,
        &tomorrow
    )
    .is_empty());
    let resume = target.patch_json(
        &format!(
            "/api/v1/households/{}/schedules/{schedule_id}/resume",
            fixture.household_id
        ),
        &fixture.access_token,
        &json!({}),
    );
    assert_eq!(resume.status().as_u16(), 200);
    assert_eq!(
        rows(&target, &schedule_path, &fixture.view_access_token, &date),
        timed_rows
    );
    assert_eq!(
        rows(
            &target,
            &schedule_path,
            &fixture.view_access_token,
            &tomorrow
        ),
        tomorrow_rows
    );

    let (assignment_id, _) = create_source(&target, &fixture, "person_medication");
    let assignment_path = source_path(&fixture, "person_medication", assignment_id);
    let assignment_resource = format!(
        "/api/v1/households/{}/person_medications/{assignment_id}",
        fixture.household_id
    );
    for cycle in ["weekly", "monthly"] {
        let update = target.patch_json(
            &assignment_resource,
            &fixture.access_token,
            &json!({"person_medication": {"dose_cycle": cycle}}),
        );
        assert_eq!(update.status().as_u16(), 200);
        let projected = rows(&target, &assignment_path, &fixture.view_access_token, &date);
        assert_eq!(projected.len(), 1);
        let today = OffsetDateTime::now_utc().date();
        let start = if cycle == "weekly" {
            today - Duration::days(today.weekday().number_days_from_monday().into())
        } else {
            time::Date::from_calendar_date(today.year(), today.month(), 1).unwrap()
        };
        let end = if cycle == "weekly" {
            start + Duration::days(6)
        } else {
            let next_month = if today.month() == time::Month::December {
                time::Date::from_calendar_date(today.year() + 1, time::Month::January, 1).unwrap()
            } else {
                time::Date::from_calendar_date(today.year(), today.month().next(), 1).unwrap()
            };
            next_month - Duration::days(1)
        };
        assert_eq!(projected[0]["window_starts_on"], start.to_string());
        assert_eq!(projected[0]["window_ends_on"], end.to_string());
    }
    let prn = target.patch_json(
        &assignment_resource,
        &fixture.access_token,
        &json!({"person_medication": {"administration_kind": "as_needed"}}),
    );
    assert_eq!(prn.status().as_u16(), 200);
    assert!(rows(&target, &assignment_path, &fixture.view_access_token, &date).is_empty());

    let (retired_id, _) = create_source(&target, &fixture, "person_medication");
    let retired_path = source_path(&fixture, "person_medication", retired_id);
    assert!(!rows(&target, &retired_path, &fixture.access_token, &date).is_empty());
    let created_at = assignment_created_at(retired_id);
    retire_assignment(retired_id);
    assert_eq!(assignment_created_at(retired_id), created_at);
    assert_error(
        target.get(
            &format!("{retired_path}?start_date={date}&end_date={date}"),
            Some(&fixture.access_token),
        ),
        404,
    );
}

#[test]
fn all_eight_occurrence_operations_use_the_documented_shared_rate_limit() {
    let fixture = fixture();
    let base_url = env::var("CONTRACT_RATE_BASE_URL").expect("non-loopback API base URL");
    let client = Client::builder()
        .no_proxy()
        .timeout(StdDuration::from_secs(5))
        .build()
        .expect("rate limit client");
    let mut limited = None;
    for _ in 0..601 {
        let response = rate_request(
            &client,
            &base_url,
            Method::GET,
            "/api/v1/capabilities",
            &fixture.access_token,
            None,
            None,
        );
        if response.status().as_u16() == 429 {
            limited = Some(response);
            break;
        }
        assert_eq!(response.status().as_u16(), 200);
    }
    rate_limited_response(limited.expect("shared rate limit response"));

    let (date, taken_at) = clock();
    for (source_type, id) in [
        ("schedule", fixture.managed_schedule_id),
        ("person_medication", fixture.managed_assignment_id),
    ] {
        let path = source_path(&fixture, source_type, id);
        let key = "opaque-rate-limit-key";
        let list_path = format!("{path}?start_date={date}&end_date={date}");
        let not_taken = json!({"dose_occurrence": {"key": key, "reason": "unwell"}});
        let reopen = json!({"dose_occurrence": {"key": key}});
        let take = json!({"dose_occurrence": {
            "key": key,
            "taken_at": taken_at,
            "client_uuid": client_uuid(source_type, id),
            "taken_from_medication_id": fixture.managed_medication_id
        }});
        rate_limited_response(rate_request(
            &client,
            &base_url,
            Method::GET,
            &list_path,
            &fixture.access_token,
            None,
            None,
        ));
        rate_limited_response(rate_request(
            &client,
            &base_url,
            Method::POST,
            &format!("{path}/not_taken"),
            &fixture.access_token,
            Some(&not_taken),
            None,
        ));
        rate_limited_response(rate_request(
            &client,
            &base_url,
            Method::PATCH,
            &format!("{path}/reopen"),
            &fixture.access_token,
            Some(&reopen),
            Some("stale"),
        ));
        rate_limited_response(rate_request(
            &client,
            &base_url,
            Method::POST,
            &format!("{path}/take"),
            &fixture.access_token,
            Some(&take),
            None,
        ));
    }
}
