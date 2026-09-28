use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::Response;
use serde_json::{json, Value};
use std::env;
use std::sync::{Arc, Barrier};
use std::thread;
use time::{Duration, OffsetDateTime};

fn body(response: Response) -> Value {
    response.json().expect("JSON response")
}

fn household_path(fixture: &Fixture, resource: &str) -> String {
    format!("/api/v1/households/{}/{resource}", fixture.household_id)
}

fn request_id(response: &Response) -> String {
    response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned()
}

fn etag(response: &Response) -> String {
    response.headers()["etag"]
        .to_str()
        .expect("ETag")
        .to_owned()
}

fn assert_error(response: Response, status: u16) -> Value {
    assert_eq!(response.status().as_u16(), status);
    let error = body(response);
    assert!(error["error"]["code"].is_string());
    assert!(error["error"]["message"].is_string());
    assert!(error["error"]["request_id"].is_string());
    assert!(error.get("data").is_none());
    error
}

fn assert_pause_period(data: &Value) {
    let expected = [
        "id",
        "portable_id",
        "source_type",
        "source_id",
        "reason",
        "note",
        "legacy_context",
        "started_at",
        "ended_at",
        "recorded_by_membership_id",
        "resumed_by_membership_id",
        "recorded_by_name",
        "resumed_by_name",
        "created_at",
        "updated_at",
    ];
    let keys: std::collections::BTreeSet<_> = data
        .as_object()
        .expect("period object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(keys, expected.into_iter().collect());
    for key in ["id", "portable_id", "source_id", "reason"] {
        assert!(data[key].as_str().is_some(), "{key} is a string");
    }
    for key in ["id", "portable_id", "source_id"] {
        assert_uuid(data[key].as_str().unwrap());
    }
    assert!(data["legacy_context"].is_boolean());
    for key in [
        "note",
        "started_at",
        "ended_at",
        "recorded_by_membership_id",
        "resumed_by_membership_id",
        "recorded_by_name",
        "resumed_by_name",
    ] {
        assert!(
            data[key].is_null() || data[key].is_string(),
            "{key} is a nullable string"
        );
    }
    for key in ["created_at", "updated_at"] {
        assert_rfc3339(&data[key]);
    }
    for key in ["started_at", "ended_at"] {
        if data[key].is_string() {
            assert_rfc3339(&data[key]);
        }
    }
}

fn assert_uuid(value: &str) {
    let segments: Vec<_> = value.split('-').collect();
    assert_eq!(
        segments
            .iter()
            .map(|segment| segment.len())
            .collect::<Vec<_>>(),
        [8, 4, 4, 4, 12]
    );
    assert!(segments
        .iter()
        .flat_map(|segment| segment.chars())
        .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase()));
}

fn assert_rfc3339(value: &Value) {
    let timestamp = value.as_str().expect("timestamp string");
    time::OffsetDateTime::parse(timestamp, &time::format_description::well_known::Rfc3339)
        .expect("RFC 3339 timestamp");
}

fn history_for(target: &Target, fixture: &Fixture, source_id: &str) -> Value {
    let path = format!(
        "{}?source_type=person_medication&source_id={source_id}",
        household_path(fixture, "medication_pause_periods")
    );
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    body(response)
}

fn database() -> postgres::Client {
    postgres::Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract database URL"),
        postgres::NoTls,
    )
    .expect("contract database")
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

fn source_update_count(record_type: &str, portable_id: &str) -> i64 {
    database()
        .query_one(
            "SELECT count(*) FROM api_change_events WHERE record_type = $1 AND record_portable_id = $2 AND action = 'update'",
            &[&record_type, &portable_id],
        )
        .expect("source sync event count")
        .get(0)
}

fn pause_version_count(portable_id: &str) -> i64 {
    database()
        .query_one(
            "SELECT count(*) FROM versions WHERE item_type = 'MedicationPausePeriod' AND item_id = (SELECT id FROM medication_pause_periods WHERE portable_id = $1)",
            &[&portable_id],
        )
        .expect("pause period version count")
        .get(0)
}

struct PersonGrantGuard {
    db: postgres::Client,
    saved: Vec<(i64, String)>,
    inserted: Vec<i64>,
}

impl PersonGrantGuard {
    fn new(fixture: &Fixture, membership_id: i64, level: &str) -> Self {
        let mut db = database();
        let rows = db
            .query(
                "SELECT id, access_level FROM person_access_grants WHERE household_id = $1 AND household_membership_id = $2 AND person_id = $3 AND revoked_at IS NULL AND (expires_at IS NULL OR expires_at > now())",
                &[&fixture.household_id, &membership_id, &fixture.managed_person_id],
            )
            .expect("active person grants");
        let saved: Vec<_> = rows
            .iter()
            .map(|row| (row.get::<_, i64>(0), row.get::<_, String>(1)))
            .collect();
        let inserted = if saved.is_empty() {
            vec![db
                .query_one(
                    "INSERT INTO person_access_grants (household_id, household_membership_id, person_id, access_level, relationship_type, created_at, updated_at) VALUES ($1, $2, $3, $4, 'family_member', now(), now()) RETURNING id",
                    &[&fixture.household_id, &membership_id, &fixture.managed_person_id, &level],
                )
                .expect("temporary person grant")
                .get(0)]
        } else {
            for (grant_id, _) in &saved {
                db.execute(
                    "UPDATE person_access_grants SET access_level = $2 WHERE id = $1",
                    &[grant_id, &level],
                )
                .expect("update person grant");
            }
            Vec::new()
        };
        Self {
            db,
            saved,
            inserted,
        }
    }
}

impl Drop for PersonGrantGuard {
    fn drop(&mut self) {
        for (grant_id, level) in &self.saved {
            self.db
                .execute(
                    "UPDATE person_access_grants SET access_level = $2 WHERE id = $1",
                    &[grant_id, level],
                )
                .expect("restore person grant");
        }
        for grant_id in &self.inserted {
            self.db
                .execute(
                    "DELETE FROM person_access_grants WHERE id = $1",
                    &[grant_id],
                )
                .expect("delete temporary person grant");
        }
    }
}

fn create_assignment(target: &Target, fixture: &Fixture, suffix: &str) -> Value {
    let medication = target.post_json_authorized(
        &household_path(fixture, "medications"),
        &fixture.access_token,
        &json!({"medication": {
            "name": format!("Pause lifecycle {suffix}"),
            "location_id": fixture.primary_location_id,
            "dose_amount": "1.25",
            "dose_unit": "ml",
            "reorder_threshold": "5"
        }}),
    );
    assert_eq!(medication.status().as_u16(), 201);
    let medication = body(medication)["data"].clone();
    let assignment = target.post_json_authorized(
        &household_path(fixture, "person_medications"),
        &fixture.access_token,
        &json!({"person_medication": {
            "person_id": fixture.managed_person_portable_id,
            "medication_id": medication["portable_id"],
            "dose_amount": "1.25",
            "dose_unit": "ml",
            "administration_kind": "as_needed"
        }}),
    );
    assert_eq!(assignment.status().as_u16(), 201);
    body(assignment)["data"].clone()
}

fn create_schedule(target: &Target, fixture: &Fixture, suffix: &str) -> Value {
    let today = OffsetDateTime::now_utc().date();
    let schedule = target.post_json_authorized(
        &household_path(fixture, "schedules"),
        &fixture.access_token,
        &json!({"schedule": {
            "person_id": fixture.managed_person_portable_id,
            "medication_id": fixture.managed_medication_portable_id,
            "dose_amount": "1.25",
            "dose_unit": "ml",
            "frequency": format!("Pause lifecycle {suffix}"),
            "start_date": (today - Duration::days(1)).to_string(),
            "end_date": (today + Duration::days(30)).to_string(),
            "max_daily_doses": 2,
            "min_hours_between_doses": "8.0",
            "dose_cycle": "weekly",
            "schedule_type": "weekly",
            "schedule_config": {"weekdays": ["monday"], "times": ["08:00"]},
            "notes": format!("Pause lifecycle {suffix}")
        }}),
    );
    assert_eq!(schedule.status().as_u16(), 201);
    body(schedule)["data"].clone()
}

fn create_period(target: &Target, fixture: &Fixture, source_id: &str) -> (Value, String, String) {
    let response = target.post_json_authorized(
        &household_path(fixture, "medication_pause_periods"),
        &fixture.access_token,
        &json!({"medication_pause_period": {
            "source_type": "person_medication",
            "source_id": source_id,
            "reason": "temporarily_not_needed",
            "note": "Lifecycle contract"
        }}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let tag = etag(&response);
    let request = request_id(&response);
    let data = body(response)["data"].clone();
    (data, tag, request)
}

fn bodyless_request(
    client: &reqwest::blocking::Client,
    method: &str,
    path: &str,
    token: &str,
) -> Response {
    let base = env::var("CONTRACT_BASE_URL").expect("API origin");
    let method = reqwest::Method::from_bytes(method.as_bytes()).expect("HTTP method");
    client
        .request(method, format!("{base}{path}"))
        .header("Accept", "application/json")
        .bearer_auth(token)
        .send()
        .expect("bodyless request")
}

#[test]
fn pause_period_history_lists_visible_records() {
    let target = Target::from_env();
    let fixture = fixture();
    let assignment = create_assignment(&target, &fixture, "history");
    let (older, older_tag, _) = create_period(
        &target,
        &fixture,
        assignment["portable_id"].as_str().unwrap(),
    );
    let resumed = target.post_json_if_match(
        &household_path(
            &fixture,
            &format!(
                "medication_pause_periods/{}/resume",
                older["id"].as_str().unwrap()
            ),
        ),
        &fixture.access_token,
        &json!({}),
        &older_tag,
    );
    assert_eq!(resumed.status().as_u16(), 200);
    thread::sleep(std::time::Duration::from_millis(1_100));
    let (newer, _, _) = create_period(
        &target,
        &fixture,
        assignment["portable_id"].as_str().unwrap(),
    );
    let base = household_path(&fixture, "medication_pause_periods");
    let filtered = format!(
        "{base}?source_type=person_medication&source_id={}&page=1&per_page=1",
        assignment["portable_id"].as_str().unwrap()
    );
    let response = target.get(&filtered, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 200);
    let page_one = body(response);
    assert_eq!(page_one.as_object().unwrap().len(), 2);
    assert_eq!(page_one["meta"]["total_count"], 2);
    assert_eq!(page_one["meta"]["page"], 1);
    assert_eq!(page_one["meta"]["per_page"], 1);
    assert_eq!(page_one["data"].as_array().unwrap().len(), 1);
    assert_pause_period(&page_one["data"][0]);
    assert_eq!(page_one["data"][0]["id"], newer["id"]);
    let second_page = format!(
        "{base}?source_type=person_medication&source_id={}&page=2&per_page=1",
        assignment["portable_id"].as_str().unwrap()
    );
    let response = target.get(&second_page, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 200);
    let page_two = body(response);
    assert_eq!(page_two["data"].as_array().unwrap().len(), 1);
    assert_eq!(page_two["data"][0]["id"], older["id"]);
    let invalid_filter = target.get(
        &format!("{base}?source_type=person_medication"),
        Some(&fixture.access_token),
    );
    assert_error(invalid_filter, 422);
    let invalid_page = target.get(&format!("{base}?page=0"), Some(&fixture.access_token));
    assert_error(invalid_page, 422);
    let unauthorized = target.get(&base, None);
    assert_error(unauthorized, 401);
    let foreign = target.get(
        &format!(
            "{base}?source_type=person_medication&source_id={}",
            fixture.foreign_assignment_portable_id
        ),
        Some(&fixture.access_token),
    );
    assert_error(foreign, 404);

    let retired = target.get(
        &format!(
            "{base}?source_type=person_medication&source_id={}",
            fixture.retired_assignment_portable_id
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(retired.status().as_u16(), 200);
    let retired = body(retired);
    assert!(retired["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|period| { period["id"] == fixture.retired_assignment_period_id }));

    let response = target.get(&base, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let response = body(response);
    assert!(response["data"].is_array());
    assert!(response["meta"].is_object());
}

#[test]
fn addressed_pause_creation_and_resume_preserve_period_identity() {
    let target = Target::from_env();
    let fixture = fixture();
    let assignment = create_assignment(&target, &fixture, "addressed");
    let (period, _period_tag, create_request) = create_period(
        &target,
        &fixture,
        assignment["portable_id"].as_str().unwrap(),
    );
    assert_pause_period(&period);
    assert_eq!(period["reason"], "temporarily_not_needed");
    assert_eq!(period["note"], "Lifecycle contract");
    assert_eq!(period["legacy_context"], false);
    assert_rfc3339(&period["started_at"]);
    assert!(period["ended_at"].is_null());
    assert!(period["recorded_by_membership_id"].is_string());
    assert!(period["recorded_by_name"].is_string());
    assert_eq!(request_audit_count(&create_request), 1);
    assert_eq!(
        source_update_count(
            "PersonMedication",
            assignment["portable_id"].as_str().unwrap()
        ),
        1
    );
    assert_eq!(
        pause_version_count(period["portable_id"].as_str().unwrap()),
        1
    );

    let stale = target.post_json_if_match(
        &household_path(
            &fixture,
            &format!(
                "medication_pause_periods/{}/resume",
                period["id"].as_str().unwrap()
            ),
        ),
        &fixture.access_token,
        &json!({}),
        "\"stale\"",
    );
    assert_error(stale, 409);
    assert_error(
        target.post_json_authorized(
            &household_path(
                &fixture,
                &format!(
                    "medication_pause_periods/{}/resume",
                    period["id"].as_str().unwrap()
                ),
            ),
            &fixture.view_access_token,
            &json!({}),
        ),
        403,
    );
    assert_error(
        target.post_json(
            &household_path(
                &fixture,
                &format!(
                    "medication_pause_periods/{}/resume",
                    period["id"].as_str().unwrap()
                ),
            ),
            &json!({}),
        ),
        401,
    );
    assert_error(
        target.post_json_authorized(
            &household_path(
                &fixture,
                &format!(
                    "medication_pause_periods/{}/resume",
                    fixture.foreign_pause_period_id
                ),
            ),
            &fixture.access_token,
            &json!({}),
        ),
        404,
    );
    assert_error(
        target.post_json_authorized(
            &household_path(
                &fixture,
                &format!(
                    "medication_pause_periods/{}/resume",
                    fixture.retired_assignment_period_id
                ),
            ),
            &fixture.access_token,
            &json!({}),
        ),
        404,
    );
    let still_open = history_for(
        &target,
        &fixture,
        assignment["portable_id"].as_str().unwrap(),
    );
    assert!(still_open["data"][0]["ended_at"].is_null());

    let resume_path = household_path(
        &fixture,
        &format!(
            "medication_pause_periods/{}/resume",
            period["id"].as_str().unwrap()
        ),
    );
    let resume_key = format!("pause-resume-{}-{}", fixture.household_id, period["id"]);
    let response = target.post_json_with_header(
        &resume_path,
        &fixture.access_token,
        "Idempotency-Key",
        &resume_key,
        &json!({}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let response_tag = etag(&response);
    let resume_request = request_id(&response);
    let resumed = body(response)["data"].clone();
    assert_eq!(request_audit_count(&resume_request), 1);
    assert_eq!(resumed["id"], period["id"]);
    assert_rfc3339(&resumed["ended_at"]);
    assert_eq!(resumed["reason"], period["reason"]);
    assert_eq!(resumed["note"], period["note"]);
    assert_eq!(
        resumed["recorded_by_membership_id"],
        period["recorded_by_membership_id"]
    );
    assert!(resumed["resumed_by_membership_id"].is_string());
    assert_eq!(
        source_update_count(
            "PersonMedication",
            assignment["portable_id"].as_str().unwrap()
        ),
        2
    );
    assert_eq!(
        pause_version_count(period["portable_id"].as_str().unwrap()),
        2
    );

    let (newer, _, _) = create_period(
        &target,
        &fixture,
        assignment["portable_id"].as_str().unwrap(),
    );
    let retry = target.post_json_if_match(
        &household_path(
            &fixture,
            &format!(
                "medication_pause_periods/{}/resume",
                period["id"].as_str().unwrap()
            ),
        ),
        &fixture.access_token,
        &json!({}),
        &response_tag,
    );
    assert_eq!(retry.status().as_u16(), 200);
    assert_eq!(body(retry)["data"]["id"], period["id"]);
    let current = target.get(
        &household_path(
            &fixture,
            &format!(
                "person_medications/{}",
                assignment["portable_id"].as_str().unwrap()
            ),
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(current.status().as_u16(), 200);
    assert_eq!(
        body(current)["data"]["current_pause_period"]["id"],
        newer["id"]
    );
    let _owner_view_grant = PersonGrantGuard::new(&fixture, fixture.owner_membership_id, "view");
    assert_error(
        target.post_json_with_header(
            &resume_path,
            &fixture.access_token,
            "Idempotency-Key",
            &resume_key,
            &json!({}),
        ),
        403,
    );
    assert_eq!(
        source_update_count(
            "PersonMedication",
            assignment["portable_id"].as_str().unwrap()
        ),
        3
    );
    assert_eq!(
        pause_version_count(period["portable_id"].as_str().unwrap()),
        2
    );
}

#[test]
fn pause_create_enforces_strict_payload_authority_and_idempotency() {
    let target = Target::from_env();
    let fixture = fixture();
    let assignment = create_assignment(&target, &fixture, "strict-create");
    let source_id = assignment["portable_id"].as_str().unwrap();
    let base = household_path(&fixture, "medication_pause_periods");
    let payload = json!({"medication_pause_period": {
        "source_type": "person_medication",
        "source_id": source_id,
        "reason": "out_of_supply",
        "note": "Strict create"
    }});

    let missing_wrapper = target.post_json_authorized(&base, &fixture.access_token, &json!({}));
    assert_error(missing_wrapper, 400);
    let mut extra_root = payload.clone();
    extra_root["extra"] = json!(true);
    assert_error(
        target.post_json_authorized(&base, &fixture.access_token, &extra_root),
        422,
    );
    let mut extra_field = payload.clone();
    extra_field["medication_pause_period"]["unknown"] = json!(true);
    assert_error(
        target.post_json_authorized(&base, &fixture.access_token, &extra_field),
        422,
    );
    let mut client_time = payload.clone();
    client_time["medication_pause_period"]["started_at"] = json!("2001-01-01T00:00:00Z");
    assert_error(
        target.post_json_authorized(&base, &fixture.access_token, &client_time),
        422,
    );
    let mut invalid_reason = payload.clone();
    invalid_reason["medication_pause_period"]["reason"] = json!("reason_not_recorded");
    assert_error(
        target.post_json_authorized(&base, &fixture.access_token, &invalid_reason),
        422,
    );
    assert_error(
        target.post_json_authorized(&base, &fixture.view_access_token, &payload),
        403,
    );
    let mut foreign_source = payload.clone();
    foreign_source["medication_pause_period"]["source_id"] =
        json!(fixture.foreign_assignment_portable_id);
    assert_error(
        target.post_json_authorized(&base, &fixture.access_token, &foreign_source),
        404,
    );
    assert!(history_for(&target, &fixture, source_id)["data"]
        .as_array()
        .unwrap()
        .is_empty());

    let key = format!("pause-create-{}-{source_id}", fixture.household_id);
    let first = target.post_json_with_key(&base, &fixture.access_token, &key, &payload);
    assert_eq!(first.status().as_u16(), 201);
    let first_tag = etag(&first);
    let first_request = request_id(&first);
    let first_body = body(first);
    assert_eq!(first_body.as_object().unwrap().len(), 1);
    assert_pause_period(&first_body["data"]);
    assert_eq!(request_audit_count(&first_request), 1);
    let replay = target.post_json_with_key(&base, &fixture.access_token, &key, &payload);
    assert_eq!(replay.status().as_u16(), 201);
    assert_eq!(replay.headers()["idempotency-replayed"], "true");
    assert_eq!(etag(&replay), first_tag);
    let replay_request = request_id(&replay);
    assert_eq!(body(replay), first_body);
    assert_eq!(request_audit_count(&replay_request), 1);
    assert_eq!(source_update_count("PersonMedication", source_id), 1);
    assert_eq!(
        pause_version_count(first_body["data"]["portable_id"].as_str().unwrap()),
        1
    );
    assert_error(
        target.post_json_with_key(&base, &fixture.view_access_token, &key, &payload),
        403,
    );
    let mut changed_payload = payload.clone();
    changed_payload["medication_pause_period"]["note"] = json!("Different request");
    let changed = assert_error(
        target.post_json_with_key(&base, &fixture.access_token, &key, &changed_payload),
        409,
    );
    assert_eq!(changed["error"]["code"], "idempotency_key_reused");
    let repeated = target.post_json_authorized(&base, &fixture.access_token, &payload);
    assert_eq!(repeated.status().as_u16(), 201);
    assert_eq!(body(repeated)["data"]["id"], first_body["data"]["id"]);
    assert_eq!(
        history_for(&target, &fixture, source_id)["data"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(source_update_count("PersonMedication", source_id), 1);
    assert_eq!(
        pause_version_count(first_body["data"]["portable_id"].as_str().unwrap()),
        1
    );
}

#[test]
fn concurrent_pause_creates_replay_one_period_and_record_one_change() {
    let fixture = fixture();
    let setup_target = Target::from_env();
    let assignment = create_assignment(&setup_target, &fixture, "concurrent-create");
    let source_id = assignment["portable_id"].as_str().unwrap().to_owned();
    let base = household_path(&fixture, "medication_pause_periods");
    let payload = json!({"medication_pause_period": {
        "source_type": "person_medication",
        "source_id": source_id,
        "reason": "other",
        "note": "Concurrent pause"
    }});
    let key = format!("pause-concurrent-{}-{source_id}", fixture.household_id);
    let barrier = Arc::new(Barrier::new(2));
    let mut handles = Vec::new();
    for _ in 0..2 {
        let barrier = Arc::clone(&barrier);
        let base = base.clone();
        let payload = payload.clone();
        let key = key.clone();
        let token = fixture.access_token.clone();
        handles.push(thread::spawn(move || {
            let target = Target::from_env();
            barrier.wait();
            let response = target.post_json_with_key(&base, &token, &key, &payload);
            let status = response.status().as_u16();
            assert_eq!(status, 201);
            let replayed = response
                .headers()
                .get("idempotency-replayed")
                .and_then(|value| value.to_str().ok())
                == Some("true");
            let request = request_id(&response);
            let response_body = body(response);
            (response_body, request, replayed)
        }));
    }
    let first = handles.remove(0).join().expect("first concurrent create");
    let second = handles.remove(0).join().expect("second concurrent create");
    assert_eq!(first.0, second.0);
    assert!(first.2 ^ second.2);
    assert_eq!(request_audit_count(&first.1), 1);
    assert_eq!(request_audit_count(&second.1), 1);
    assert_eq!(
        history_for(&setup_target, &fixture, &source_id)["data"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(source_update_count("PersonMedication", &source_id), 1);
    assert_eq!(
        pause_version_count(first.0["data"]["portable_id"].as_str().unwrap()),
        1
    );
}

#[test]
fn owner_and_manager_pause_writes_follow_current_person_grants() {
    let fixture = fixture();
    let target = Target::from_env();
    let first = create_assignment(&target, &fixture, "grant-first");
    let second = create_assignment(&target, &fixture, "grant-second");
    let schedule = create_schedule(&target, &fixture, "grant-schedule");
    let _manager_manage = PersonGrantGuard::new(&fixture, fixture.manager_membership_id, "manage");
    let period_payload = json!({"medication_pause_period": {
        "source_type": "person_medication",
        "source_id": first["portable_id"],
        "reason": "clinician_advice"
    }});
    let period = target.post_json_authorized(
        &household_path(&fixture, "medication_pause_periods"),
        &fixture.manager_access_token,
        &period_payload,
    );
    assert_eq!(period.status().as_u16(), 201);
    let period = body(period)["data"].clone();

    let _owner_view = PersonGrantGuard::new(&fixture, fixture.owner_membership_id, "view");
    let assignment_path = household_path(
        &fixture,
        &format!(
            "person_medications/{}",
            first["portable_id"].as_str().unwrap()
        ),
    );
    let before_assignment = target.get(&assignment_path, Some(&fixture.access_token));
    assert_eq!(before_assignment.status().as_u16(), 200);
    let before_assignment_tag = etag(&before_assignment);
    let before_assignment: Value = before_assignment
        .json()
        .expect("assignment before denied writes");
    let history_count = history_for(&target, &fixture, first["portable_id"].as_str().unwrap())
        ["data"]
        .as_array()
        .unwrap()
        .len();
    assert_eq!(history_count, 1);
    assert_error(
        target.post_json_authorized(
            &household_path(&fixture, "medication_pause_periods"),
            &fixture.access_token,
            &json!({"medication_pause_period": {
                "source_type": "person_medication",
                "source_id": second["portable_id"],
                "reason": "other"
            }}),
        ),
        403,
    );
    for action in ["pause", "resume"] {
        assert_error(
            target.patch_json(
                &household_path(
                    &fixture,
                    &format!(
                        "person_medications/{}/{action}",
                        first["portable_id"].as_str().unwrap()
                    ),
                ),
                &fixture.access_token,
                &json!({}),
            ),
            403,
        );
        assert_error(
            target.patch_json(
                &household_path(
                    &fixture,
                    &format!(
                        "schedules/{}/{action}",
                        schedule["portable_id"].as_str().unwrap()
                    ),
                ),
                &fixture.access_token,
                &json!({}),
            ),
            403,
        );
    }
    assert_error(
        target.post_json_authorized(
            &household_path(
                &fixture,
                &format!(
                    "medication_pause_periods/{}/resume",
                    period["id"].as_str().unwrap()
                ),
            ),
            &fixture.access_token,
            &json!({}),
        ),
        403,
    );
    assert_error(
        target.patch_json(
            &household_path(
                &fixture,
                &format!(
                    "person_medications/{}/reorder",
                    second["portable_id"].as_str().unwrap()
                ),
            ),
            &fixture.access_token,
            &json!({"direction": "up"}),
        ),
        403,
    );
    let after_assignment = target.get(&assignment_path, Some(&fixture.access_token));
    assert_eq!(after_assignment.status().as_u16(), 200);
    assert_eq!(etag(&after_assignment), before_assignment_tag);
    assert_eq!(
        after_assignment
            .json::<Value>()
            .expect("assignment after denied writes"),
        before_assignment
    );
    assert_eq!(
        history_for(&target, &fixture, first["portable_id"].as_str().unwrap())["data"]
            .as_array()
            .unwrap()
            .len(),
        history_count
    );
}

#[test]
fn legacy_person_medication_pause_and_resume_update_owned_assignment() {
    let target = Target::from_env();
    let fixture = fixture();
    let assignment = create_assignment(&target, &fixture, "legacy-person");
    let path = household_path(
        &fixture,
        &format!(
            "person_medications/{}/pause",
            assignment["portable_id"].as_str().unwrap()
        ),
    );
    let paused = target.patch_json(&path, &fixture.access_token, &json!({}));
    assert_eq!(paused.status().as_u16(), 200);
    let paused = body(paused)["data"].clone();
    assert_eq!(paused["paused"], true);
    assert_eq!(
        paused["current_pause_period"]["reason"],
        "reason_not_recorded"
    );
    assert_eq!(paused["current_pause_period"]["legacy_context"], true);
    let period_id = paused["current_pause_period"]["id"].clone();
    let repeated = target.patch_json(&path, &fixture.access_token, &json!({}));
    assert_eq!(repeated.status().as_u16(), 200);
    assert_eq!(
        body(repeated)["data"]["current_pause_period"]["id"],
        period_id
    );
    assert_error(
        target.patch_json(&path, &fixture.view_access_token, &json!({})),
        403,
    );
    let foreign_path = household_path(
        &fixture,
        &format!(
            "person_medications/{}/pause",
            fixture.foreign_assignment_portable_id
        ),
    );
    assert_error(
        target.patch_json(&foreign_path, &fixture.access_token, &json!({})),
        404,
    );
    let resume_path = path.replace("/pause", "/resume");
    let resumed = target.patch_json(&resume_path, &fixture.access_token, &json!({}));
    assert_eq!(resumed.status().as_u16(), 200);
    assert_eq!(body(resumed)["data"]["paused"], false);
    let repeated = target.patch_json(&resume_path, &fixture.access_token, &json!({}));
    assert_eq!(repeated.status().as_u16(), 200);
    assert!(body(repeated)["data"]["current_pause_period"].is_null());
}

#[test]
fn legacy_schedule_pause_and_resume_update_owned_schedule() {
    let target = Target::from_env();
    let fixture = fixture();
    let schedule = create_schedule(&target, &fixture, "legacy-schedule");
    let path = household_path(
        &fixture,
        &format!(
            "schedules/{}/pause",
            schedule["portable_id"].as_str().unwrap()
        ),
    );
    let paused = target.patch_json(&path, &fixture.access_token, &json!({}));
    assert_eq!(paused.status().as_u16(), 200);
    let paused = body(paused)["data"].clone();
    assert_eq!(paused["paused"], true);
    assert_eq!(
        paused["current_pause_period"]["reason"],
        "reason_not_recorded"
    );
    assert_eq!(paused["current_pause_period"]["legacy_context"], true);
    let period_id = paused["current_pause_period"]["id"].clone();
    let repeated = target.patch_json(&path, &fixture.access_token, &json!({}));
    assert_eq!(repeated.status().as_u16(), 200);
    assert_eq!(
        body(repeated)["data"]["current_pause_period"]["id"],
        period_id
    );
    assert_error(
        target.patch_json(&path, &fixture.view_access_token, &json!({})),
        403,
    );
    let foreign_path = household_path(
        &fixture,
        &format!("schedules/{}/pause", fixture.foreign_schedule_portable_id),
    );
    assert_error(
        target.patch_json(&foreign_path, &fixture.access_token, &json!({})),
        404,
    );
    let resumed = target.patch_json(
        &path.replace("/pause", "/resume"),
        &fixture.access_token,
        &json!({}),
    );
    assert_eq!(resumed.status().as_u16(), 200);
    assert_eq!(body(resumed)["data"]["paused"], false);
    let repeated = target.patch_json(
        &path.replace("/pause", "/resume"),
        &fixture.access_token,
        &json!({}),
    );
    assert_eq!(repeated.status().as_u16(), 200);
    assert!(body(repeated)["data"]["current_pause_period"].is_null());
}

#[test]
fn person_medication_reorder_moves_only_within_its_person() {
    let target = Target::from_env();
    let fixture = fixture();
    let first = create_assignment(&target, &fixture, "reorder-first");
    let second = create_assignment(&target, &fixture, "reorder-second");
    let response = target.patch_json(
        &household_path(
            &fixture,
            &format!(
                "person_medications/{}/reorder",
                second["portable_id"].as_str().unwrap()
            ),
        ),
        &fixture.access_token,
        &json!({"direction": "up"}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let moved = body(response)["data"].clone();
    assert_eq!(moved["id"], second["id"]);
    assert_eq!(moved["position"], first["position"]);
    let reorder_path = household_path(
        &fixture,
        &format!(
            "person_medications/{}/reorder",
            second["portable_id"].as_str().unwrap()
        ),
    );
    assert_error(
        target.patch_json(
            &reorder_path,
            &fixture.access_token,
            &json!({"direction": "sideways"}),
        ),
        422,
    );
    assert_error(
        target.patch_json(
            &reorder_path,
            &fixture.view_access_token,
            &json!({"direction": "down"}),
        ),
        403,
    );
    let first_detail = target.get(
        &household_path(
            &fixture,
            &format!(
                "person_medications/{}",
                first["portable_id"].as_str().unwrap()
            ),
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(first_detail.status().as_u16(), 200);
    assert_eq!(body(first_detail)["data"]["position"], second["position"]);
    let second_detail = target.get(
        &household_path(
            &fixture,
            &format!(
                "person_medications/{}",
                second["portable_id"].as_str().unwrap()
            ),
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(second_detail.status().as_u16(), 200);
    assert_eq!(body(second_detail)["data"]["position"], first["position"]);
}

#[test]
fn bodyless_pause_and_resume_operations_follow_the_openapi_routes() {
    let fixture = fixture();
    let target = Target::from_env();
    let explicit_assignment = create_assignment(&target, &fixture, "bodyless-period");
    let (period, _, _) = create_period(
        &target,
        &fixture,
        explicit_assignment["portable_id"].as_str().unwrap(),
    );
    let legacy_assignment = create_assignment(&target, &fixture, "bodyless-person");
    let schedule = create_schedule(&target, &fixture, "bodyless-schedule");
    let explicit_resume = household_path(
        &fixture,
        &format!(
            "medication_pause_periods/{}/resume",
            period["id"].as_str().unwrap()
        ),
    );
    let person_pause = household_path(
        &fixture,
        &format!(
            "person_medications/{}/pause",
            legacy_assignment["portable_id"].as_str().unwrap()
        ),
    );
    let person_resume = person_pause.replace("/pause", "/resume");
    let schedule_pause = household_path(
        &fixture,
        &format!(
            "schedules/{}/pause",
            schedule["portable_id"].as_str().unwrap()
        ),
    );
    let schedule_resume = schedule_pause.replace("/pause", "/resume");
    let operations = [
        ("POST", explicit_resume, "ended_at"),
        ("PATCH", person_pause, "paused"),
        ("PATCH", person_resume, "paused"),
        ("PATCH", schedule_pause, "paused"),
        ("PATCH", schedule_resume, "paused"),
    ];
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("bodyless request client");
    for (method, path, field) in operations {
        let response = bodyless_request(&client, method, &path, &fixture.access_token);
        assert_eq!(response.status().as_u16(), 200, "{method} {path}");
        let data = body(response)["data"].clone();
        match field {
            "ended_at" => assert!(data[field].is_string(), "{method} {path}"),
            "paused" if path.ends_with("/pause") => assert_eq!(data[field], true),
            "paused" => assert_eq!(data[field], false),
            _ => unreachable!(),
        }
    }
}

#[test]
fn z_rate_limiter_wires_all_pause_lifecycle_operations() {
    let fixture = fixture();
    let target = Target::from_env();
    let first = create_assignment(&target, &fixture, "rate-first");
    let second = create_assignment(&target, &fixture, "rate-second");
    let schedule = create_schedule(&target, &fixture, "rate-schedule");
    let (period, _, _) = create_period(&target, &fixture, first["portable_id"].as_str().unwrap());
    let rate_base = env::var("CONTRACT_RATE_BASE_URL").expect("non-loopback API base URL");
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("rate limit client");
    let started = std::time::Instant::now();
    let mut limited = false;
    for _ in 0..601 {
        let response = client
            .get(format!("{rate_base}/api/v1/capabilities"))
            .send()
            .expect("rate limit request");
        if response.status().as_u16() == 429 {
            for header in [
                "retry-after",
                "ratelimit-limit",
                "ratelimit-remaining",
                "ratelimit-reset",
            ] {
                assert!(response.headers().get(header).is_some(), "missing {header}");
            }
            assert_eq!(response.headers()["ratelimit-remaining"], "0");
            let body: Value = response.json().expect("rate limited JSON");
            assert_eq!(body["error"]["code"], "rate_limited");
            limited = true;
            break;
        }
        assert_eq!(response.status().as_u16(), 200);
    }
    assert!(limited && started.elapsed() < std::time::Duration::from_secs(60));

    let history = format!(
        "{rate_base}/api/v1/households/{}/medication_pause_periods",
        fixture.household_id
    );
    let create_body = json!({"medication_pause_period": {
        "source_type": "person_medication",
        "source_id": second["portable_id"],
        "reason": "other"
    }});
    let requests = [
        client
            .get(&history)
            .bearer_auth(&fixture.access_token)
            .send(),
        client
            .post(&history)
            .bearer_auth(&fixture.access_token)
            .json(&create_body)
            .send(),
        client
            .post(format!("{history}/{}/resume", period["id"]))
            .bearer_auth(&fixture.access_token)
            .json(&json!({}))
            .send(),
        client
            .patch(format!(
                "{rate_base}/api/v1/households/{}/schedules/{}/pause",
                fixture.household_id, schedule["portable_id"]
            ))
            .bearer_auth(&fixture.access_token)
            .json(&json!({}))
            .send(),
        client
            .patch(format!(
                "{rate_base}/api/v1/households/{}/schedules/{}/resume",
                fixture.household_id, schedule["portable_id"]
            ))
            .bearer_auth(&fixture.access_token)
            .json(&json!({}))
            .send(),
        client
            .patch(format!(
                "{rate_base}/api/v1/households/{}/person_medications/{}/pause",
                fixture.household_id, first["portable_id"]
            ))
            .bearer_auth(&fixture.access_token)
            .json(&json!({}))
            .send(),
        client
            .patch(format!(
                "{rate_base}/api/v1/households/{}/person_medications/{}/resume",
                fixture.household_id, first["portable_id"]
            ))
            .bearer_auth(&fixture.access_token)
            .json(&json!({}))
            .send(),
        client
            .patch(format!(
                "{rate_base}/api/v1/households/{}/person_medications/{}/reorder",
                fixture.household_id, second["portable_id"]
            ))
            .bearer_auth(&fixture.access_token)
            .json(&json!({"direction": "up"}))
            .send(),
    ];
    for response in requests {
        let response = response.expect("rate limited pause operation");
        assert_eq!(response.status().as_u16(), 429);
        assert_eq!(response.headers()["ratelimit-remaining"], "0");
        let body: Value = response.json().expect("rate limit error JSON");
        assert_eq!(body["error"]["code"], "rate_limited");
    }
}
