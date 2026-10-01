use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::{Client, Response};
use reqwest::redirect::Policy;
use serde_json::{json, Value};
use std::env;
use std::sync::{Arc, Barrier};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use time::{Duration as CalendarDuration, OffsetDateTime};

fn schedules_path(fixture: &Fixture) -> String {
    format!("/api/v1/households/{}/schedules", fixture.household_id)
}

fn schedule_payload(fixture: &Fixture) -> Value {
    let today = OffsetDateTime::now_utc().date();
    let start_date = (today - CalendarDuration::days(1)).to_string();
    let end_date = (today + CalendarDuration::days(30)).to_string();
    json!({"schedule": {
        "person_id": fixture.managed_person_portable_id,
        "medication_id": fixture.managed_medication_portable_id,
        "dose_amount": "1.250",
        "dose_unit": "ml",
        "frequency": "Every Monday",
        "start_date": start_date,
        "end_date": end_date,
        "max_daily_doses": 2,
        "min_hours_between_doses": "8.0",
        "dose_cycle": "weekly",
        "schedule_type": "weekly",
        "schedule_config": {"weekdays": ["monday"], "times": ["08:00", "20:00"]},
        "notes": "OpenAPI schedule write"
    }})
}

fn create_schedule(target: &Target, fixture: &Fixture) -> (Value, String, String) {
    let response = target.post_json_authorized(
        &schedules_path(fixture),
        &fixture.access_token,
        &schedule_payload(fixture),
    );
    assert_eq!(response.status().as_u16(), 201);
    let tag = etag(&response);
    let request = request_id(&response);
    let created = body(response)["data"].clone();
    assert_schedule(&created);
    assert_schedule_write_event(fixture, &request, 201, &created, "create");
    (created, tag, request)
}

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
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract database URL"),
        postgres::NoTls,
    )
    .expect("contract database")
}

fn assert_schedule_write_event(
    fixture: &Fixture,
    request: &str,
    status: u16,
    schedule: &Value,
    action: &str,
) {
    let schedule_id = schedule["id"].as_i64().expect("schedule ID");
    let portable_id = schedule["portable_id"]
        .as_str()
        .expect("schedule portable ID");
    let rows = database()
        .query(
            "SELECT row_to_json(api_change_events)::text FROM api_change_events WHERE request_id = $1 AND record_type = 'Schedule' AND record_id = $2 AND action = $3",
            &[&request, &schedule_id, &action],
        )
        .expect("schedule sync event");
    assert_eq!(rows.len(), 1);
    let event: Value = serde_json::from_str(&rows[0].get::<_, String>(0)).unwrap();
    assert_eq!(event["household_id"], fixture.household_id);
    assert_eq!(event["account_id"], fixture.account_id);
    assert_eq!(
        event["household_membership_id"],
        fixture.owner_membership_id
    );
    assert_eq!(event["request_id"], request);
    assert_eq!(event["record_type"], "Schedule");
    assert_eq!(event["record_id"], schedule_id);
    assert_eq!(event["record_portable_id"], portable_id);
    assert_eq!(event["action"], action);
    assert_eq!(event["metadata"]["record_type"], "Schedule");
    assert_eq!(event["metadata"]["record_id"], schedule_id);
    assert_eq!(event["metadata"]["portable_id"], portable_id);
    assert!(!event.to_string().contains(&fixture.access_token));
    assert_request_audit(request, status);
}

fn assert_request_audit(request: &str, status: u16) {
    let audit = database()
        .query_one(
            "SELECT metadata::text FROM security_audit_events WHERE request_id = $1 AND event_type = 'api.request'",
            &[&request],
        )
        .expect("schedule request audit");
    let metadata: Value = serde_json::from_str(&audit.get::<_, String>(0)).unwrap();
    assert_eq!(metadata["status"], status);
}

fn schedule_event_count(schedule_id: i64) -> i64 {
    database()
        .query_one(
            "SELECT count(*) FROM api_change_events WHERE record_type = 'Schedule' AND record_id = $1",
            &[&schedule_id],
        )
        .expect("schedule event count")
        .get(0)
}

fn schedule_version_count(schedule_id: i64) -> i64 {
    database()
        .query_one(
            "SELECT count(*) FROM versions WHERE item_type = 'Schedule' AND item_id = $1",
            &[&schedule_id],
        )
        .expect("schedule version count")
        .get(0)
}

fn schedule_count(person_id: i64, medication_id: i64) -> i64 {
    database()
        .query_one(
            "SELECT count(*) FROM schedules WHERE person_id = $1 AND medication_id = $2",
            &[&person_id, &medication_id],
        )
        .expect("schedule count")
        .get(0)
}

fn assert_schedule_unchanged(
    target: &Target,
    path: &str,
    fixture: &Fixture,
    expected: &Value,
    expected_etag: &str,
    expected_events: i64,
) {
    let response = target.get(path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(etag(&response), expected_etag);
    let mut fetched = body(response)["data"].clone();
    let mut expected = expected.clone();
    if let Some(object) = fetched.as_object_mut() {
        object.remove("current_pause_period");
    }
    if let Some(object) = expected.as_object_mut() {
        object.remove("current_pause_period");
    }
    assert_eq!(fetched, expected);
    assert_eq!(
        schedule_event_count(expected["id"].as_i64().unwrap()),
        expected_events
    );
}

fn assert_noop_patch(
    target: &Target,
    fixture: &Fixture,
    path: &str,
    payload: &Value,
    expected: &Value,
    expected_etag: &str,
    expected_events: i64,
    expected_versions: i64,
) {
    let id = expected["id"].as_i64().unwrap();
    let response = target.patch_json(path, &fixture.access_token, payload);
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(etag(&response), expected_etag);
    let request = request_id(&response);
    assert_request_audit(&request, 200);
    let mut actual = body(response)["data"].clone();
    let mut expected = expected.clone();
    if let Some(object) = actual.as_object_mut() {
        object.remove("current_pause_period");
    }
    if let Some(object) = expected.as_object_mut() {
        object.remove("current_pause_period");
    }
    assert_eq!(actual, expected);
    assert_eq!(schedule_event_count(id), expected_events);
    assert_eq!(schedule_version_count(id), expected_versions);
}

fn assert_noop_put(
    target: &Target,
    fixture: &Fixture,
    path: &str,
    payload: &Value,
    expected: &Value,
    expected_etag: &str,
    expected_events: i64,
    expected_versions: i64,
) {
    let id = expected["id"].as_i64().unwrap();
    let response = target.put_json(path, &fixture.access_token, payload);
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(etag(&response), expected_etag);
    let request = request_id(&response);
    assert_request_audit(&request, 200);
    let mut actual = body(response)["data"].clone();
    let mut expected = expected.clone();
    if let Some(object) = actual.as_object_mut() {
        object.remove("current_pause_period");
    }
    if let Some(object) = expected.as_object_mut() {
        object.remove("current_pause_period");
    }
    assert_eq!(actual, expected);
    assert_eq!(schedule_event_count(id), expected_events);
    assert_eq!(schedule_version_count(id), expected_versions);
}

fn keyed_write(
    method: reqwest::Method,
    path: &str,
    token: &str,
    key: &str,
    payload: &Value,
) -> Response {
    let _validated_target = Target::from_env();
    let base = env::var("CONTRACT_BASE_URL").expect("API origin");
    let origin = url::Url::parse(&base).expect("API origin URL");
    assert!(match origin.host() {
        Some(url::Host::Domain("localhost")) => true,
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        _ => false,
    });
    let url = origin.join(path).expect("schedule write path");
    assert_eq!(url.origin(), origin.origin());
    let client = Client::builder()
        .no_proxy()
        .redirect(Policy::none())
        .timeout(Duration::from_secs(10))
        .build()
        .expect("idempotency request client");
    client
        .request(method, url)
        .header("Accept", "application/json")
        .bearer_auth(token)
        .header("Idempotency-Key", key)
        .json(payload)
        .send()
        .expect("keyed schedule write")
}

fn assert_schedule(data: &Value) {
    let object = data.as_object().expect("schedule resource object");
    let required = [
        "id",
        "schedule_type",
        "schedule_config",
        "portable_id",
        "person_id",
        "person_portable_id",
        "medication_id",
        "medication_portable_id",
        "dose_amount",
        "dose_unit",
        "frequency",
        "dose_cycle",
        "start_date",
        "end_date",
        "active",
        "paused",
        "can_manage",
        "notes",
        "updated_at",
        "max_daily_doses",
        "min_hours_between_doses",
    ];
    let allowed = [
        "can_record",
        "eligible_stock_medication_ids",
        "current_pause_period",
        "id",
        "schedule_type",
        "schedule_config",
        "portable_id",
        "person_id",
        "person_portable_id",
        "medication_id",
        "medication_portable_id",
        "dose_amount",
        "dose_unit",
        "frequency",
        "dose_cycle",
        "start_date",
        "end_date",
        "active",
        "paused",
        "can_manage",
        "notes",
        "updated_at",
        "max_daily_doses",
        "min_hours_between_doses",
    ];
    assert!(required.iter().all(|key| object.contains_key(*key)));
    assert!(object.keys().all(|key| allowed.contains(&key.as_str())));
    if let Some(value) = object.get("can_record") {
        assert!(value.is_boolean());
    }
    if let Some(value) = object.get("eligible_stock_medication_ids") {
        assert!(value
            .as_array()
            .is_some_and(|ids| ids.iter().all(|id| id.as_i64().is_some())));
    }
    assert!(data["current_pause_period"].is_null() || data["current_pause_period"].is_object());
    assert!(data["id"].as_i64().is_some_and(|id| id > 0));
    assert!(data["portable_id"]
        .as_str()
        .is_some_and(|id| id.len() == 36));
    assert!(data["person_id"].as_i64().is_some_and(|id| id > 0));
    assert!(data["person_portable_id"]
        .as_str()
        .is_some_and(|id| id.len() == 36));
    assert!(data["medication_id"].as_i64().is_some_and(|id| id > 0));
    assert!(data["medication_portable_id"]
        .as_str()
        .is_some_and(|id| id.len() == 36));
    let schedule_type = data["schedule_type"].as_str().unwrap();
    assert!(matches!(
        schedule_type,
        "daily"
            | "multiple_daily"
            | "weekly"
            | "specific_dates"
            | "prn"
            | "tapering"
            | "every_other_day"
    ));
    assert_schedule_config(&data["schedule_config"]);
    assert!(data["dose_amount"].is_string());
    assert!(data["dose_unit"]
        .as_str()
        .is_some_and(|unit| !unit.is_empty()));
    assert!(data["frequency"].is_null() || data["frequency"].is_string());
    assert!(
        data["dose_cycle"].is_null()
            || matches!(
                data["dose_cycle"].as_str(),
                Some("daily" | "weekly" | "monthly")
            )
    );
    assert!(data["start_date"].as_str().is_some());
    assert!(data["end_date"].as_str().is_some());
    assert!(data["active"].is_boolean());
    assert!(data["paused"].is_boolean());
    assert!(data["can_manage"].is_boolean());
    assert!(data["notes"].is_null() || data["notes"].is_string());
    assert!(data["updated_at"].as_str().is_some());
    assert!(data["max_daily_doses"].is_null() || data["max_daily_doses"].as_i64().is_some());
    assert!(
        data["min_hours_between_doses"].is_null() || data["min_hours_between_doses"].is_string()
    );
}

fn assert_schedule_config(value: &Value) {
    let config = value.as_object().expect("schedule config object");
    let allowed = ["times", "weekdays", "dates", "as_needed", "taper_steps"];
    assert!(config.keys().all(|key| allowed.contains(&key.as_str())));
    if let Some(times) = config.get("times") {
        assert!(times.as_array().is_some_and(|values| {
            values.iter().all(|time| {
                time.as_str().is_some_and(|time| {
                    let bytes = time.as_bytes();
                    bytes.len() == 5
                        && bytes[0].is_ascii_digit()
                        && bytes[1].is_ascii_digit()
                        && bytes[2] == b':'
                        && bytes[3].is_ascii_digit()
                        && bytes[4].is_ascii_digit()
                        && time[0..2].parse::<u8>().is_ok_and(|hour| hour <= 23)
                        && time[3..5].parse::<u8>().is_ok_and(|minute| minute <= 59)
                })
            })
        }));
    }
    if let Some(weekdays) = config.get("weekdays") {
        assert!(weekdays.as_array().is_some_and(|values| {
            values
                .iter()
                .all(|day| day.as_str().is_some_and(|day| !day.is_empty()))
        }));
    }
    if let Some(dates) = config.get("dates") {
        assert!(dates
            .as_array()
            .is_some_and(|values| { values.iter().all(|date| date.as_str().is_some()) }));
    }
    if let Some(as_needed) = config.get("as_needed") {
        assert!(as_needed.is_boolean());
    }
    if let Some(steps) = config.get("taper_steps") {
        let allowed = [
            "start_date",
            "end_date",
            "amount",
            "dose_amount",
            "unit",
            "dose_unit",
            "max_daily_doses",
            "min_hours_between_doses",
            "times",
        ];
        assert!(steps.as_array().is_some_and(|values| {
            values.iter().all(|step| {
                step.as_object().is_some_and(|step| {
                    step.contains_key("start_date")
                        && step.contains_key("end_date")
                        && step.keys().all(|key| allowed.contains(&key.as_str()))
                })
            })
        }));
    }
}

#[test]
fn create_schedule_returns_the_documented_resource_and_etag() {
    let fixture = fixture();
    let target = Target::from_env();
    let payload = schedule_payload(&fixture);
    let key = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos()
        .to_string();
    let key = format!("schedule-create-{key}");
    let response = target.post_json_with_key(
        &schedules_path(&fixture),
        &fixture.access_token,
        &key,
        &payload,
    );

    assert_eq!(response.status().as_u16(), 201);
    assert!(!etag(&response).is_empty());
    let created_etag = etag(&response);
    let create_request = request_id(&response);
    let envelope = body(response);
    assert_eq!(envelope.as_object().unwrap().len(), 1);
    let created = &envelope["data"];
    assert_schedule(created);
    assert_schedule_write_event(&fixture, &create_request, 201, created, "create");
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
    assert_eq!(created["frequency"], "Every Monday");
    assert_eq!(created["start_date"], payload["schedule"]["start_date"]);
    assert_eq!(created["end_date"], payload["schedule"]["end_date"]);
    assert_eq!(created["max_daily_doses"], 2);
    assert_eq!(created["min_hours_between_doses"], "8.0");
    assert_eq!(created["dose_cycle"], "weekly");
    assert_eq!(created["schedule_type"], "weekly");
    assert_eq!(
        created["schedule_config"],
        json!({"weekdays": ["monday"], "times": ["08:00", "20:00"]})
    );
    assert_eq!(created["notes"], "OpenAPI schedule write");
    assert_eq!(created["active"], true);
    assert_eq!(created["paused"], false);
    assert_eq!(created["can_manage"], true);

    let events_before_replay = schedule_event_count(created["id"].as_i64().unwrap());
    let replay = target.post_json_with_key(
        &schedules_path(&fixture),
        &fixture.access_token,
        &key,
        &payload,
    );
    assert_eq!(replay.status().as_u16(), 201);
    assert_eq!(replay.headers()["idempotency-replayed"], "true");
    assert_eq!(etag(&replay), created_etag);
    let replay_request = request_id(&replay);
    assert_ne!(replay_request, create_request);
    assert_request_audit(&replay_request, 201);
    assert_eq!(body(replay), envelope);
    assert_eq!(
        schedule_event_count(created["id"].as_i64().unwrap()),
        events_before_replay
    );
}

#[test]
fn schedule_no_op_patch_and_put_keep_etag_and_emit_only_request_audit() {
    let target = Target::from_env();
    let fixture = fixture();
    let (created, original_etag, _) = create_schedule(&target, &fixture);
    let path = format!("{}/{}", schedules_path(&fixture), created["id"]);
    let id = created["id"].as_i64().unwrap();
    let events = schedule_event_count(id);
    let versions = schedule_version_count(id);

    assert_noop_patch(
        &target,
        &fixture,
        &path,
        &json!({"schedule": {"notes": created["notes"]}}),
        &created,
        &original_etag,
        events,
        versions,
    );
    assert_noop_patch(
        &target,
        &fixture,
        &path,
        &json!({"schedule": {"person_id": fixture.managed_person_portable_id}}),
        &created,
        &original_etag,
        events,
        versions,
    );
    assert_noop_patch(
        &target,
        &fixture,
        &path,
        &json!({"schedule": {"person_id": fixture.managed_person_id.to_string()}}),
        &created,
        &original_etag,
        events,
        versions,
    );
    assert_noop_put(
        &target,
        &fixture,
        &path,
        &json!({"schedule": {"notes": created["notes"]}}),
        &created,
        &original_etag,
        events,
        versions,
    );
}

#[test]
fn future_schedule_reports_effective_inactive_state_without_being_paused() {
    let fixture = fixture();
    let target = Target::from_env();
    let today = OffsetDateTime::now_utc().date();
    let mut payload = schedule_payload(&fixture);
    payload["schedule"]["start_date"] = json!((today + CalendarDuration::days(10)).to_string());
    payload["schedule"]["end_date"] = json!((today + CalendarDuration::days(40)).to_string());
    let response =
        target.post_json_authorized(&schedules_path(&fixture), &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 201);
    assert!(!etag(&response).is_empty());
    let request = request_id(&response);
    let schedule = body(response)["data"].clone();
    assert_schedule(&schedule);
    assert_eq!(schedule["active"], false);
    assert_eq!(schedule["paused"], false);
    assert_schedule_write_event(&fixture, &request, 201, &schedule, "create");
}

#[test]
fn schedule_reads_keep_the_openapi_shape_pagination_and_visibility() {
    let fixture = fixture();
    let target = Target::from_env();
    let base = schedules_path(&fixture);
    let (created, created_etag, _) = create_schedule(&target, &fixture);
    let portable = created["portable_id"].as_str().unwrap();

    let numeric = target.get(
        &format!("{base}/{}", created["id"].as_i64().unwrap()),
        Some(&fixture.access_token),
    );
    assert_eq!(numeric.status().as_u16(), 200);
    assert_eq!(etag(&numeric), created_etag);
    let numeric_body = body(numeric);
    assert_eq!(numeric_body.as_object().unwrap().len(), 1);
    assert_schedule(&numeric_body["data"]);
    assert_eq!(numeric_body["data"]["current_pause_period"], Value::Null);

    let by_portable = target.get(&format!("{base}/{portable}"), Some(&fixture.access_token));
    assert_eq!(by_portable.status().as_u16(), 200);
    assert_eq!(etag(&by_portable), created_etag);
    assert_eq!(body(by_portable), numeric_body);

    let visible = target.get(&base, Some(&fixture.view_access_token));
    assert_eq!(visible.status().as_u16(), 200);
    let collection = body(visible);
    assert_eq!(collection.as_object().unwrap().len(), 2);
    assert!(collection["meta"]["total_count"].as_u64().is_some());
    let rows = collection["data"].as_array().unwrap();
    assert!(rows.iter().any(|row| row["id"] == created["id"]));
    assert!(rows
        .iter()
        .any(|row| row["id"] == fixture.managed_schedule_id));
    assert!(!rows
        .iter()
        .any(|row| row["id"] == fixture.hidden_schedule_id));
    assert!(!rows
        .iter()
        .any(|row| row["id"] == fixture.foreign_schedule_id));
    assert!(rows.iter().all(|row| {
        assert_schedule(row);
        row["can_manage"] == false
    }));

    let page_one = target.get(
        &format!("{base}?page=1&per_page=1"),
        Some(&fixture.view_access_token),
    );
    assert_eq!(page_one.status().as_u16(), 200);
    let page_one = body(page_one);
    assert_eq!(page_one["meta"]["page"], 1);
    assert_eq!(page_one["meta"]["per_page"], 1);
    assert_eq!(page_one["data"].as_array().unwrap().len(), 1);
    let page_two = target.get(
        &format!("{base}?page=2&per_page=1"),
        Some(&fixture.view_access_token),
    );
    assert_eq!(page_two.status().as_u16(), 200);
    let page_two = body(page_two);
    assert_eq!(
        page_two["meta"]["total_count"],
        page_one["meta"]["total_count"]
    );
    assert_ne!(page_two["data"][0]["id"], page_one["data"][0]["id"]);

    let future = target.get(
        &format!("{base}?updated_since=2099-01-01T00%3A00%3A00Z"),
        Some(&fixture.access_token),
    );
    assert_eq!(future.status().as_u16(), 200);
    assert_eq!(body(future)["meta"]["total_count"], 0);
    let invalid_filter = target.get(
        &format!("{base}?updated_since=invalid"),
        Some(&fixture.access_token),
    );
    assert_eq!(invalid_filter.status().as_u16(), 422);
    let unauthorized = target.get(&base, None);
    assert_eq!(unauthorized.status().as_u16(), 401);
    assert_eq!(
        target
            .get(&base, Some(&fixture.view_access_token))
            .status()
            .as_u16(),
        200
    );
    assert_eq!(
        target
            .get(
                &format!(
                    "/api/v1/households/{}/schedules",
                    fixture.foreign_household_id
                ),
                Some(&fixture.access_token),
            )
            .status()
            .as_u16(),
        403
    );
    assert_eq!(
        target
            .get(
                &format!("{base}/{}", fixture.hidden_schedule_id),
                Some(&fixture.view_access_token),
            )
            .status()
            .as_u16(),
        404
    );
    assert_eq!(
        target
            .get(
                &format!("{base}/{}", fixture.foreign_schedule_id),
                Some(&fixture.access_token),
            )
            .status()
            .as_u16(),
        404
    );
}

#[test]
fn schedule_create_checks_authority_references_and_strict_validation_without_writes() {
    let fixture = fixture();
    let target = Target::from_env();
    let base = schedules_path(&fixture);
    let payload = schedule_payload(&fixture);
    let before = schedule_count(fixture.managed_person_id, fixture.managed_medication_id);

    assert_error(target.post_json(&base, &payload), 401);
    assert_error(
        target.post_json_authorized(&base, "invalid-token", &payload),
        401,
    );
    assert_error(
        target.post_json_authorized(&base, &fixture.view_access_token, &payload),
        403,
    );
    assert_error(
        target.post_json_authorized(
            &format!(
                "/api/v1/households/{}/schedules",
                fixture.foreign_household_id
            ),
            &fixture.access_token,
            &payload,
        ),
        403,
    );

    let mut hidden_person = payload.clone();
    hidden_person["schedule"]["person_id"] = json!(fixture.hidden_person_id.to_string());
    assert_error(
        target.post_json_authorized(&base, &fixture.access_token, &hidden_person),
        404,
    );
    let mut foreign_person = payload.clone();
    foreign_person["schedule"]["person_id"] = json!(fixture.foreign_person_portable_id);
    assert_error(
        target.post_json_authorized(&base, &fixture.access_token, &foreign_person),
        404,
    );
    let mut foreign_medication = payload.clone();
    foreign_medication["schedule"]["medication_id"] = json!(fixture.foreign_medication_portable_id);
    assert_error(
        target.post_json_authorized(&base, &fixture.access_token, &foreign_medication),
        404,
    );

    let mut malformed_wrapper = payload.clone();
    malformed_wrapper["schedule"] = Value::Null;
    let mut unknown_root = payload.clone();
    unknown_root["unexpected"] = json!(true);
    let mut unknown_schedule_field = payload.clone();
    unknown_schedule_field["schedule"]["unexpected"] = json!(true);
    let mut numeric_person_id = payload.clone();
    numeric_person_id["schedule"]["person_id"] = json!(fixture.managed_person_id);
    let mut numeric_medication_id = payload.clone();
    numeric_medication_id["schedule"]["medication_id"] = json!(fixture.managed_medication_id);
    let mut signed_person_id = payload.clone();
    signed_person_id["schedule"]["person_id"] = json!("+1");
    let mut leading_zero_person_id = payload.clone();
    leading_zero_person_id["schedule"]["person_id"] = json!("01");
    let mut null_notes = payload.clone();
    null_notes["schedule"]["notes"] = Value::Null;
    let mut null_frequency = payload.clone();
    null_frequency["schedule"]["frequency"] = Value::Null;
    let mut null_source_option = payload.clone();
    null_source_option["schedule"]["source_dosage_option_id"] = Value::Null;
    let mut fractional_interval = payload.clone();
    fractional_interval["schedule"]["min_hours_between_doses"] = json!("8.5");
    let mut zero_interval = payload.clone();
    zero_interval["schedule"]["min_hours_between_doses"] = json!("0.0");
    let mut negative_interval = payload.clone();
    negative_interval["schedule"]["min_hours_between_doses"] = json!("-1.0");
    let mut excessive_precision = payload.clone();
    excessive_precision["schedule"]["dose_amount"] = json!("2.125");
    let mut signed_decimal = payload.clone();
    signed_decimal["schedule"]["dose_amount"] = json!("+1");
    let mut exponent_decimal = payload.clone();
    exponent_decimal["schedule"]["dose_amount"] = json!("1e0");
    let mut underscored_decimal = payload.clone();
    underscored_decimal["schedule"]["dose_amount"] = json!("1_0");
    let mut unknown_recurrence = payload.clone();
    unknown_recurrence["schedule"]["schedule_type"] = json!("unknown");
    let mut invalid_time = payload.clone();
    invalid_time["schedule"]["schedule_config"] =
        json!({"weekdays": ["monday"], "times": ["99:99"]});
    let mut invalid_weekday = payload.clone();
    invalid_weekday["schedule"]["schedule_config"] =
        json!({"weekdays": ["funday"], "times": ["08:00"]});

    assert_error(
        target.post_json_authorized(&base, &fixture.access_token, &json!({})),
        400,
    );
    assert_error(
        target.post_json_authorized(&base, &fixture.access_token, &malformed_wrapper),
        400,
    );
    for invalid in [
        &unknown_root,
        &unknown_schedule_field,
        &numeric_person_id,
        &numeric_medication_id,
        &signed_person_id,
        &leading_zero_person_id,
        &null_notes,
        &null_frequency,
        &null_source_option,
        &fractional_interval,
        &zero_interval,
        &negative_interval,
        &excessive_precision,
        &signed_decimal,
        &exponent_decimal,
        &underscored_decimal,
        &unknown_recurrence,
        &invalid_time,
        &invalid_weekday,
    ] {
        assert_error(
            target.post_json_authorized(&base, &fixture.access_token, invalid),
            422,
        );
    }
    assert_eq!(
        schedule_count(fixture.managed_person_id, fixture.managed_medication_id),
        before
    );
}

#[test]
fn schedule_idempotency_replays_authorized_validation_errors_and_rejects_changed_payloads() {
    let fixture = fixture();
    let target = Target::from_env();
    let base = schedules_path(&fixture);
    let before = schedule_count(fixture.managed_person_id, fixture.managed_medication_id);
    let mut invalid_payload = schedule_payload(&fixture);
    invalid_payload["schedule"]["min_hours_between_doses"] = json!("8.5");
    let key = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos()
        .to_string();
    let key = format!("schedule-invalid-{key}");

    let first = target.post_json_with_key(&base, &fixture.access_token, &key, &invalid_payload);
    let first = assert_error(first, 422);
    let first_request = first["error"]["request_id"].as_str().unwrap().to_owned();
    assert_request_audit(&first_request, 422);

    let replay = target.post_json_with_key(&base, &fixture.access_token, &key, &invalid_payload);
    assert_eq!(replay.status().as_u16(), 422);
    assert_eq!(replay.headers()["idempotency-replayed"], "true");
    let replay_request = request_id(&replay);
    assert_ne!(replay_request, first_request);
    assert_request_audit(&replay_request, 422);
    assert_eq!(body(replay), first);

    let mut changed_payload = schedule_payload(&fixture);
    changed_payload["schedule"]["frequency"] = json!("Changed request");
    let changed = target.post_json_with_key(&base, &fixture.access_token, &key, &changed_payload);
    let changed = assert_error(changed, 409);
    assert_eq!(changed["error"]["code"], "idempotency_key_reused");
    assert_request_audit(changed["error"]["request_id"].as_str().unwrap(), 409);
    assert_eq!(
        schedule_count(fixture.managed_person_id, fixture.managed_medication_id),
        before
    );
}

#[test]
fn schedule_patch_and_put_merge_partial_fields_with_optional_if_match_and_idempotency() {
    let fixture = fixture();
    let target = Target::from_env();
    let base = schedules_path(&fixture);
    let (created, original_etag, _) = create_schedule(&target, &fixture);
    let path = format!("{base}/{}", created["portable_id"].as_str().unwrap());
    let id = created["id"].as_i64().unwrap();

    assert_error(
        target.patch_json_without_auth(&path, &json!({"schedule": {"notes": "No auth"}})),
        401,
    );
    assert_error(
        target.patch_json(
            &path,
            &fixture.view_access_token,
            &json!({"schedule": {"notes": "Forbidden"}}),
        ),
        403,
    );
    assert_error(
        target.patch_json(
            &format!("{base}/{}", fixture.foreign_schedule_id),
            &fixture.access_token,
            &json!({"schedule": {"notes": "Foreign"}}),
        ),
        404,
    );
    assert_error(
        target.patch_json(
            &format!("{base}/{}", fixture.hidden_schedule_id),
            &fixture.access_token,
            &json!({"schedule": {"notes": "Hidden"}}),
        ),
        404,
    );
    assert_error(
        target.patch_json(&path, &fixture.access_token, &json!({})),
        400,
    );
    assert_error(
        target.patch_json(&path, &fixture.access_token, &json!({"schedule": null})),
        400,
    );
    assert_error(
        target.patch_json(
            &path,
            &fixture.access_token,
            &json!({"schedule": {"unknown": true}}),
        ),
        422,
    );

    let patch_payload =
        json!({"schedule": {"frequency": "Every eight hours", "notes": "PATCH update"}});
    let patch_key = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos()
        .to_string();
    let patch_key = format!("schedule-patch-{patch_key}");
    let patched_response = keyed_write(
        reqwest::Method::PATCH,
        &path,
        &fixture.access_token,
        &patch_key,
        &patch_payload,
    );
    assert_eq!(patched_response.status().as_u16(), 200);
    let patched_etag = etag(&patched_response);
    assert_ne!(patched_etag, original_etag);
    let patched_request = request_id(&patched_response);
    let patched = body(patched_response)["data"].clone();
    assert_schedule(&patched);
    assert_eq!(patched["frequency"], "Every eight hours");
    assert_eq!(patched["notes"], "PATCH update");
    assert_eq!(patched["dose_amount"], created["dose_amount"]);
    assert_eq!(patched["person_id"], created["person_id"]);
    assert_schedule_write_event(&fixture, &patched_request, 200, &patched, "update");
    let events_after_patch = schedule_event_count(id);
    let versions_after_patch = schedule_version_count(id);

    let patch_replay = keyed_write(
        reqwest::Method::PATCH,
        &path,
        &fixture.access_token,
        &patch_key,
        &patch_payload,
    );
    assert_eq!(patch_replay.status().as_u16(), 200);
    assert_eq!(patch_replay.headers()["idempotency-replayed"], "true");
    assert_eq!(etag(&patch_replay), patched_etag);
    let patch_replay_request = request_id(&patch_replay);
    assert_ne!(patch_replay_request, patched_request);
    assert_request_audit(&patch_replay_request, 200);
    assert_eq!(body(patch_replay)["data"], patched);
    assert_eq!(schedule_event_count(id), events_after_patch);
    assert_eq!(schedule_version_count(id), versions_after_patch);

    let stale_patch = target.patch_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"schedule": {"notes": "Stale PATCH"}}),
        &original_etag,
    );
    let stale_error = assert_error(stale_patch, 409);
    assert_eq!(stale_error["error"]["code"], "conflict");
    assert_schedule_unchanged(
        &target,
        &path,
        &fixture,
        &patched,
        &patched_etag,
        events_after_patch,
    );

    let person_etag = patched_etag.clone();
    let person_state = patched.clone();
    let person_events = events_after_patch;
    let person_versions = versions_after_patch;

    for (person_id, status) in [
        (fixture.user_person_id.to_string(), 422),
        (fixture.hidden_person_id.to_string(), 404),
        (fixture.foreign_person_id.to_string(), 404),
    ] {
        assert_error(
            target.patch_json(
                &path,
                &fixture.access_token,
                &json!({"schedule": {"person_id": person_id}}),
            ),
            status,
        );
        assert_schedule_unchanged(
            &target,
            &path,
            &fixture,
            &person_state,
            &person_etag,
            person_events,
        );
        assert_eq!(schedule_version_count(id), person_versions);
    }

    assert_error(
        target.put_json_without_auth(&path, &json!({"schedule": {"notes": "No auth"}})),
        401,
    );
    assert_error(
        target.put_json(
            &path,
            &fixture.view_access_token,
            &json!({"schedule": {"notes": "Forbidden"}}),
        ),
        403,
    );
    assert_error(
        target.put_json(
            &format!("{base}/{}", fixture.foreign_schedule_id),
            &fixture.access_token,
            &json!({"schedule": {"notes": "Foreign"}}),
        ),
        404,
    );
    assert_error(
        target.put_json(
            &format!("{base}/{}", fixture.hidden_schedule_id),
            &fixture.access_token,
            &json!({"schedule": {"notes": "Hidden"}}),
        ),
        404,
    );
    assert_error(
        target.put_json(&path, &fixture.access_token, &json!({})),
        400,
    );
    assert_error(
        target.put_json(&path, &fixture.access_token, &json!({"schedule": null})),
        400,
    );
    let unknown_put_field = json!({"schedule": {"unexpected": true}});
    assert_error(
        target.put_json(&path, &fixture.access_token, &unknown_put_field),
        422,
    );

    let put_payload =
        json!({"schedule": {"notes": "PUT merge", "frequency": "Every twelve hours"}});
    let put_key = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos()
        .to_string();
    let put_key = format!("schedule-put-{put_key}");
    let replaced_response = keyed_write(
        reqwest::Method::PUT,
        &path,
        &fixture.access_token,
        &put_key,
        &put_payload,
    );
    assert_eq!(replaced_response.status().as_u16(), 200);
    let replaced_etag = etag(&replaced_response);
    assert_ne!(replaced_etag, person_etag);
    let replaced_request = request_id(&replaced_response);
    let replaced = body(replaced_response)["data"].clone();
    assert_schedule(&replaced);
    assert_eq!(replaced["notes"], "PUT merge");
    assert_eq!(replaced["frequency"], "Every twelve hours");
    assert_eq!(replaced["dose_unit"], person_state["dose_unit"]);
    assert_eq!(replaced["dose_amount"], person_state["dose_amount"]);
    assert_schedule_write_event(&fixture, &replaced_request, 200, &replaced, "update");
    let events_after_put = schedule_event_count(id);
    let versions_after_put = schedule_version_count(id);

    let put_replay = keyed_write(
        reqwest::Method::PUT,
        &path,
        &fixture.access_token,
        &put_key,
        &put_payload,
    );
    assert_eq!(put_replay.status().as_u16(), 200);
    assert_eq!(put_replay.headers()["idempotency-replayed"], "true");
    assert_eq!(etag(&put_replay), replaced_etag);
    let put_replay_request = request_id(&put_replay);
    assert_ne!(put_replay_request, replaced_request);
    assert_request_audit(&put_replay_request, 200);
    assert_eq!(body(put_replay)["data"], replaced);
    assert_eq!(schedule_event_count(id), events_after_put);
    assert_eq!(schedule_version_count(id), versions_after_put);

    let stale_put = target.put_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"schedule": {"notes": "Stale PUT"}}),
        &person_etag,
    );
    let stale_error = assert_error(stale_put, 409);
    assert_eq!(stale_error["error"]["code"], "conflict");
    assert_schedule_unchanged(
        &target,
        &path,
        &fixture,
        &replaced,
        &replaced_etag,
        events_after_put,
    );

    let invalid_patch = target.patch_json(
        &path,
        &fixture.access_token,
        &json!({"schedule": {"dose_amount": "2.125"}}),
    );
    assert_error(invalid_patch, 422);
    assert_schedule_unchanged(
        &target,
        &path,
        &fixture,
        &replaced,
        &replaced_etag,
        events_after_put,
    );
    for field in ["notes", "frequency", "source_dosage_option_id"] {
        let mut invalid = json!({"schedule": {}});
        invalid["schedule"][field] = Value::Null;
        assert_error(
            target.patch_json(&path, &fixture.access_token, &invalid),
            422,
        );
        assert_schedule_unchanged(
            &target,
            &path,
            &fixture,
            &replaced,
            &replaced_etag,
            events_after_put,
        );
    }
    let invalid_put = target.put_json(
        &path,
        &fixture.access_token,
        &json!({"schedule": {"min_hours_between_doses": "8.5"}}),
    );
    assert_error(invalid_put, 422);
    assert_schedule_unchanged(
        &target,
        &path,
        &fixture,
        &replaced,
        &replaced_etag,
        events_after_put,
    );
    for field in ["notes", "frequency", "source_dosage_option_id"] {
        let mut invalid = json!({"schedule": {}});
        invalid["schedule"][field] = Value::Null;
        assert_error(target.put_json(&path, &fixture.access_token, &invalid), 422);
        assert_schedule_unchanged(
            &target,
            &path,
            &fixture,
            &replaced,
            &replaced_etag,
            events_after_put,
        );
    }
}

#[test]
fn schedule_updates_with_the_same_etag_allow_only_one_concurrent_writer() {
    let fixture = fixture();
    let target = Target::from_env();
    let base = schedules_path(&fixture);
    let (created, original_etag, _) = create_schedule(&target, &fixture);
    let id = created["id"].as_i64().unwrap();
    let path = format!("{base}/{}", created["portable_id"].as_str().unwrap());
    let events_before = schedule_event_count(id);
    let versions_before = schedule_version_count(id);
    let barrier = Arc::new(Barrier::new(3));
    let mut workers = Vec::new();
    for note in [
        "Concurrent schedule edit one",
        "Concurrent schedule edit two",
    ] {
        let barrier = Arc::clone(&barrier);
        let path = path.clone();
        let token = fixture.access_token.clone();
        let etag = original_etag.clone();
        let payload = json!({"schedule": {"notes": note}});
        workers.push(std::thread::spawn(move || {
            let target = Target::from_env();
            barrier.wait();
            target.patch_json_if_match(&path, &token, &payload, &etag)
        }));
    }
    barrier.wait();
    let responses: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().expect("concurrent schedule request"))
        .collect();
    assert_eq!(
        responses
            .iter()
            .filter(|response| response.status().as_u16() == 200)
            .count(),
        1
    );
    assert_eq!(
        responses
            .iter()
            .filter(|response| response.status().as_u16() == 409)
            .count(),
        1
    );

    let mut updated = None;
    for response in responses {
        let request = request_id(&response);
        match response.status().as_u16() {
            200 => {
                let tag = etag(&response);
                let data = body(response)["data"].clone();
                assert_schedule(&data);
                assert!(matches!(
                    data["notes"].as_str(),
                    Some("Concurrent schedule edit one" | "Concurrent schedule edit two")
                ));
                assert_schedule_write_event(&fixture, &request, 200, &data, "update");
                updated = Some((data, tag));
            }
            409 => {
                assert_request_audit(&request, 409);
            }
            status => panic!("unexpected concurrent update response: {status}"),
        }
    }
    let (updated, updated_etag) = updated.expect("one update should commit");
    assert_ne!(updated_etag, original_etag);
    assert_eq!(schedule_event_count(id), events_before + 1);
    assert_eq!(schedule_version_count(id), versions_before + 1);
    assert_schedule_unchanged(
        &target,
        &path,
        &fixture,
        &updated,
        &updated_etag,
        events_before + 1,
    );
}

#[test]
fn schedule_source_dosage_option_must_be_visible_and_match_the_snapshot() {
    let fixture = fixture();
    let target = Target::from_env();
    let dosage_options = format!("/api/v1/households/{}/dosage_options", fixture.household_id);
    let option_response = target.post_json_authorized(
        &dosage_options,
        &fixture.access_token,
        &json!({"dosage_option": {
            "medication_id": fixture.managed_medication_portable_id,
            "amount": "2.5",
            "unit": "ml",
            "frequency": "Twice daily",
            "default_max_daily_doses": 2,
            "default_min_hours_between_doses": "8.0",
            "default_dose_cycle": "daily"
        }}),
    );
    assert_eq!(option_response.status().as_u16(), 201);
    let option = body(option_response)["data"].clone();

    let base = schedules_path(&fixture);
    let before = schedule_count(fixture.managed_person_id, fixture.managed_medication_id);
    let mut matching = schedule_payload(&fixture);
    matching["schedule"]["person_id"] = json!(fixture.managed_person_id.to_string());
    matching["schedule"]["medication_id"] = json!(fixture.managed_medication_id.to_string());
    matching["schedule"]["source_dosage_option_id"] =
        json!(option["id"].as_i64().unwrap().to_string());
    matching["schedule"]["dose_amount"] = json!("2.5");
    matching["schedule"]["schedule_config"] = json!({"weekdays": ["mon", "1"], "times": ["08:00"]});
    let response = target.post_json_authorized(&base, &fixture.access_token, &matching);
    assert_eq!(response.status().as_u16(), 201);
    let request = request_id(&response);
    let created = body(response)["data"].clone();
    assert_schedule(&created);
    assert_eq!(created["dose_amount"], "2.5");
    assert_eq!(created["schedule_config"]["weekdays"], json!(["mon", "1"]));
    assert_schedule_write_event(&fixture, &request, 201, &created, "create");
    assert_eq!(
        schedule_count(fixture.managed_person_id, fixture.managed_medication_id),
        before + 1
    );
    let created_path = format!("{base}/{}", created["portable_id"].as_str().unwrap());
    let created_detail = target.get(&created_path, Some(&fixture.access_token));
    assert_eq!(created_detail.status().as_u16(), 200);
    let created_etag = etag(&created_detail);
    let mut different_snapshot = json!({"schedule": {"dose_amount": "3.0"}});
    different_snapshot["schedule"]["source_dosage_option_id"] = option["portable_id"].clone();
    let events_before_rejected_update = schedule_event_count(created["id"].as_i64().unwrap());
    assert_error(
        target.patch_json(&created_path, &fixture.access_token, &different_snapshot),
        422,
    );
    assert_schedule_unchanged(
        &target,
        &created_path,
        &fixture,
        &created,
        &created_etag,
        events_before_rejected_update,
    );

    let mut mismatch = matching.clone();
    mismatch["schedule"]["dose_amount"] = json!("1.25");
    assert_error(
        target.post_json_authorized(&base, &fixture.access_token, &mismatch),
        422,
    );
    let mut numeric_option_id = matching.clone();
    numeric_option_id["schedule"]["source_dosage_option_id"] =
        json!(option["id"].as_i64().unwrap());
    assert_error(
        target.post_json_authorized(&base, &fixture.access_token, &numeric_option_id),
        422,
    );

    let mut foreign = schedule_payload(&fixture);
    foreign["schedule"]["source_dosage_option_id"] = json!(fixture.foreign_dosage_portable_id);
    assert_error(
        target.post_json_authorized(&base, &fixture.access_token, &foreign),
        404,
    );

    let mut hidden = schedule_payload(&fixture);
    hidden["schedule"]["source_dosage_option_id"] = json!(fixture.hidden_dosage_portable_id);
    hidden["schedule"]["dose_amount"] = json!("1");
    assert_error(
        target.post_json_authorized(&base, &fixture.delegated_access_token, &hidden),
        404,
    );
    assert_eq!(
        schedule_count(fixture.managed_person_id, fixture.managed_medication_id),
        before + 1
    );
}

#[test]
fn z_schedule_operations_share_the_documented_rate_limit() {
    let fixture = fixture();
    let base = env::var("CONTRACT_RATE_BASE_URL").expect("rate-limited API base URL");
    let schedules = format!(
        "{base}/api/v1/households/{}/schedules",
        fixture.household_id
    );
    let detail = format!("{schedules}/{}", fixture.managed_schedule_id);
    let client = Client::builder()
        .no_proxy()
        .redirect(Policy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .expect("schedule rate limit client");
    let started = std::time::Instant::now();
    let mut rejected = None;
    for _ in 0..601 {
        let response = client
            .get(format!("{base}/api/v1/capabilities"))
            .send()
            .expect("general rate limit request");
        if response.status().as_u16() == 429 {
            rejected = Some(response);
            break;
        }
        assert_eq!(response.status().as_u16(), 200);
    }
    let response = rejected.expect("general limiter rejects within 601 requests");
    assert!(started.elapsed() < Duration::from_secs(60));
    assert_eq!(response.headers()["ratelimit-limit"], "300");
    assert_eq!(response.headers()["ratelimit-remaining"], "0");
    assert!(response.headers().get("retry-after").is_some());
    assert_eq!(body(response)["error"]["code"], "rate_limited");

    for response in [
        client
            .get(&schedules)
            .bearer_auth(&fixture.access_token)
            .send(),
        client
            .get(&detail)
            .bearer_auth(&fixture.access_token)
            .send(),
        client
            .post(&schedules)
            .bearer_auth(&fixture.access_token)
            .json(&schedule_payload(&fixture))
            .send(),
        client
            .patch(&detail)
            .bearer_auth(&fixture.access_token)
            .json(&json!({"schedule": {"notes": "Rate limited"}}))
            .send(),
        client
            .put(&detail)
            .bearer_auth(&fixture.access_token)
            .json(&json!({"schedule": {"notes": "Rate limited"}}))
            .send(),
    ] {
        assert_eq!(
            response
                .expect("rate limited schedule operation")
                .status()
                .as_u16(),
            429
        );
    }
}
