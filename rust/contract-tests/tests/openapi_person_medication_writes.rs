use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::{Client, RequestBuilder, Response};
use serde_json::{json, Value};
use std::env;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

fn assignments_path(fixture: &Fixture) -> String {
    format!(
        "/api/v1/households/{}/person_medications",
        fixture.household_id
    )
}

fn create_payload(person_id: &str, medication_id: &str) -> Value {
    json!({"person_medication": {
        "person_id": person_id,
        "medication_id": medication_id,
        "dose_amount": "1.25",
        "dose_unit": "ml",
        "administration_kind": "as_needed"
    }})
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

fn assert_error(response: Response, status: u16) -> Value {
    assert_eq!(response.status().as_u16(), status);
    let body: Value = response.json().expect("error JSON");
    assert!(body["error"]["code"].is_string());
    assert!(body["error"]["request_id"].is_string());
    assert!(body["error"]["message"].is_string());
    assert!(body.get("data").is_none());
    body
}

fn create_request(
    path: &str,
    token: &str,
    payload: &Value,
    idempotency_key: Option<&str>,
) -> Response {
    let base = env::var("CONTRACT_BASE_URL").expect("API origin");
    let client = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("assignment request client");
    let request: RequestBuilder = client.post(format!("{base}{path}"));
    let request = request.bearer_auth(token);
    let request = if let Some(key) = idempotency_key {
        request.header("Idempotency-Key", key)
    } else {
        request
    };
    request
        .json(payload)
        .send()
        .expect("assignment create request")
}

fn create_request_without_auth(path: &str, payload: &Value) -> Response {
    let base = env::var("CONTRACT_BASE_URL").expect("API origin");
    Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("unauthenticated create client")
        .post(format!("{base}{path}"))
        .json(payload)
        .send()
        .expect("unauthenticated create request")
}

fn assignment_write_request(
    method: &str,
    path: &str,
    token: Option<&str>,
    payload: &Value,
    idempotency_key: Option<&str>,
    if_match: Option<&str>,
) -> Response {
    let base = env::var("CONTRACT_BASE_URL").expect("API origin");
    let client = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("assignment write client");
    let mut request = match method {
        "PATCH" => client.patch(format!("{base}{path}")),
        "PUT" => client.put(format!("{base}{path}")),
        "POST" => client.post(format!("{base}{path}")),
        _ => panic!("unsupported write method {method}"),
    };
    if let Some(token) = token {
        request = request.bearer_auth(token);
    }
    if let Some(key) = idempotency_key {
        request = request.header("Idempotency-Key", key);
    }
    if let Some(tag) = if_match {
        request = request.header("If-Match", tag);
    }
    request
        .json(payload)
        .send()
        .expect("assignment write request")
}

fn malformed_assignment_request(method: &str, path: &str, token: &str) -> Response {
    let base = env::var("CONTRACT_BASE_URL").expect("API origin");
    let client = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("malformed assignment client");
    let request = match method {
        "PATCH" => client.patch(format!("{base}{path}")),
        "PUT" => client.put(format!("{base}{path}")),
        _ => panic!("unsupported write method {method}"),
    };
    request
        .bearer_auth(token)
        .header("Content-Type", "application/json")
        .body("{")
        .send()
        .expect("malformed assignment request")
}

fn database() -> postgres::Client {
    postgres::Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("audit database URL"),
        postgres::NoTls,
    )
    .expect("contract database")
}

fn assignment_count(person_id: i64, medication_id: i64) -> i64 {
    database()
        .query_one(
            "SELECT count(*) FROM person_medications WHERE person_id = $1 AND medication_id = $2 AND retired_at IS NULL",
            &[&person_id, &medication_id],
        )
        .expect("assignment count")
        .get(0)
}

fn medication_portable_id(medication_id: i64) -> String {
    database()
        .query_one(
            "SELECT portable_id FROM medications WHERE id = $1",
            &[&medication_id],
        )
        .expect("medication portable ID")
        .get(0)
}

fn person_portable_id(person_id: i64) -> String {
    database()
        .query_one(
            "SELECT portable_id FROM people WHERE id = $1",
            &[&person_id],
        )
        .expect("person portable ID")
        .get(0)
}

fn assert_create_audit(request: &str, assignment_id: i64) {
    let mut db = database();
    let change_count: i64 = db
        .query_one(
            "SELECT count(*) FROM api_change_events WHERE request_id = $1 AND record_type = 'PersonMedication' AND record_id = $2 AND action = 'create'",
            &[&request, &assignment_id],
        )
        .expect("assignment sync event count")
        .get(0);
    assert_eq!(change_count, 1);
    let version_count: i64 = db
        .query_one(
            "SELECT count(*) FROM versions WHERE request_id = $1 AND item_type = 'PersonMedication' AND item_id = $2 AND event = 'create'",
            &[&request, &assignment_id],
        )
        .expect("assignment create version count")
        .get(0);
    assert_eq!(version_count, 1);
}

fn assert_update_audit(request: &str, assignment_id: i64) {
    let count: i64 = database()
        .query_one(
            "SELECT count(*) FROM api_change_events WHERE request_id = $1 AND record_type = 'PersonMedication' AND record_id = $2 AND action = 'update'",
            &[&request, &assignment_id],
        )
        .expect("assignment update event count")
        .get(0);
    assert_eq!(count, 1);
}

fn assignment_update_count(assignment_id: i64) -> i64 {
    database()
        .query_one(
            "SELECT count(*) FROM api_change_events WHERE record_type = 'PersonMedication' AND record_id = $1 AND action = 'update'",
            &[&assignment_id],
        )
        .expect("assignment update event count")
        .get(0)
}

fn assignment_update_version_count(assignment_id: i64) -> i64 {
    database()
        .query_one(
            "SELECT count(*) FROM versions WHERE item_type = 'PersonMedication' AND item_id = $1 AND event = 'update'",
            &[&assignment_id],
        )
        .expect("assignment update version count")
        .get(0)
}

fn request_audit_count(household_id: i64, request: &str) -> i64 {
    database()
        .query_one(
            "SELECT count(*) FROM security_audit_events WHERE household_id = $1 AND event_type = 'api.request' AND request_id = $2",
            &[&household_id, &request],
        )
        .expect("API request audit count")
        .get(0)
}

struct AssignmentFieldsGuard {
    db: postgres::Client,
    assignment_id: i64,
    notes: Option<String>,
    max_daily_doses: Option<i32>,
    min_hours_between_doses: Option<i32>,
    dose_cycle: Option<i32>,
    updated_at: String,
}

impl AssignmentFieldsGuard {
    fn new(assignment_id: i64) -> Self {
        let mut db = database();
        let row = db
            .query_one(
                "SELECT notes, max_daily_doses, min_hours_between_doses, dose_cycle, updated_at::text FROM person_medications WHERE id = $1",
                &[&assignment_id],
            )
            .expect("assignment fields before changes");
        Self {
            notes: row.get(0),
            max_daily_doses: row.get(1),
            min_hours_between_doses: row.get(2),
            dose_cycle: row.get(3),
            updated_at: row.get(4),
            assignment_id,
            db,
        }
    }
}

impl Drop for AssignmentFieldsGuard {
    fn drop(&mut self) {
        self.db
            .execute(
                "UPDATE person_medications SET notes = $2, max_daily_doses = $3, min_hours_between_doses = $4, dose_cycle = $5, updated_at = $6::text::timestamp WHERE id = $1",
                &[
                    &self.assignment_id,
                    &self.notes,
                    &self.max_daily_doses,
                    &self.min_hours_between_doses,
                    &self.dose_cycle,
                    &self.updated_at,
                ],
            )
            .expect("restore assignment fields");
    }
}

fn latest_position(person_id: i64) -> i64 {
    database()
        .query_one(
            "SELECT CAST(COALESCE(MAX(position), 0) AS BIGINT) FROM person_medications WHERE person_id = $1",
            &[&person_id],
        )
        .expect("person assignment position")
        .get(0)
}

struct AccessGrantGuard {
    db: postgres::Client,
    saved: Vec<(i64, String)>,
    inserted: Vec<i64>,
}

impl AccessGrantGuard {
    fn new(household_id: i64, membership_id: i64, person_id: i64, level: &str) -> Self {
        let mut db = database();
        let rows = db
            .query(
                "SELECT id, access_level FROM person_access_grants WHERE household_id = $1 AND household_membership_id = $2 AND person_id = $3 AND revoked_at IS NULL AND (expires_at IS NULL OR expires_at > now())",
                &[&household_id, &membership_id, &person_id],
            )
            .expect("active person access grants");
        let saved: Vec<_> = rows
            .iter()
            .map(|row| (row.get::<_, i64>(0), row.get::<_, String>(1)))
            .collect();
        let inserted = if saved.is_empty() {
            vec![db
                .query_one(
                    "INSERT INTO person_access_grants (household_id, household_membership_id, person_id, access_level, relationship_type, created_at, updated_at) VALUES ($1, $2, $3, $4, 'family_member', now(), now()) RETURNING id",
                    &[&household_id, &membership_id, &person_id, &level],
                )
                .expect("temporary view grant")
                .get(0)]
        } else {
            for (grant_id, _) in &saved {
                db.execute(
                    "UPDATE person_access_grants SET access_level = $2 WHERE id = $1",
                    &[grant_id, &level],
                )
                .expect("reduce grant to view");
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

impl Drop for AccessGrantGuard {
    fn drop(&mut self) {
        for (grant_id, access_level) in &self.saved {
            self.db
                .execute(
                    "UPDATE person_access_grants SET access_level = $2 WHERE id = $1",
                    &[grant_id, access_level],
                )
                .expect("restore person access grant");
        }
        for grant_id in &self.inserted {
            self.db
                .execute(
                    "DELETE FROM person_access_grants WHERE id = $1",
                    &[grant_id],
                )
                .expect("remove temporary view grant");
        }
    }
}

fn assert_assignment(data: &Value) {
    let object = data.as_object().expect("assignment data object");
    for field in [
        "id",
        "portable_id",
        "person_id",
        "person_portable_id",
        "medication_id",
        "medication_portable_id",
        "dose_amount",
        "dose_unit",
        "active",
        "paused",
        "can_manage",
        "dose_cycle",
        "administration_kind",
        "notes",
        "position",
        "updated_at",
        "max_daily_doses",
        "min_hours_between_doses",
    ] {
        assert!(
            object.contains_key(field),
            "missing assignment field {field}"
        );
    }
    let allowed = [
        "can_record",
        "eligible_stock_medication_ids",
        "current_pause_period",
        "id",
        "portable_id",
        "person_id",
        "person_portable_id",
        "medication_id",
        "medication_portable_id",
        "dose_amount",
        "dose_unit",
        "active",
        "paused",
        "can_manage",
        "dose_cycle",
        "administration_kind",
        "notes",
        "position",
        "updated_at",
        "max_daily_doses",
        "min_hours_between_doses",
    ];
    assert!(object.keys().all(|key| allowed.contains(&key.as_str())));
    if let Some(value) = object.get("can_record") {
        assert!(value.is_boolean());
    }
    if let Some(value) = object.get("eligible_stock_medication_ids") {
        assert!(value
            .as_array()
            .is_some_and(|ids| ids.iter().all(|id| id.as_i64().is_some())));
    }
    assert!(data["id"].as_i64().is_some_and(|id| id > 0));
    assert_uuid(data["portable_id"].as_str().expect("portable ID"));
    assert!(data["person_id"].as_i64().is_some_and(|id| id > 0));
    assert_uuid(
        data["person_portable_id"]
            .as_str()
            .expect("person portable ID"),
    );
    assert!(data["medication_id"].as_i64().is_some_and(|id| id > 0));
    assert_uuid(
        data["medication_portable_id"]
            .as_str()
            .expect("medication portable ID"),
    );
    assert!(data["dose_amount"].is_null() || data["dose_amount"].is_string());
    assert!(data["dose_unit"].is_null() || data["dose_unit"].is_string());
    assert!(data["active"].is_boolean());
    assert!(data["paused"].is_boolean());
    assert!(data["can_manage"].is_boolean());
    assert!(matches!(
        data["administration_kind"].as_str(),
        Some("routine" | "as_needed")
    ));
    assert!(data["notes"].is_null() || data["notes"].is_string());
    assert!(data["position"].is_null() || data["position"].as_i64().is_some());
    assert!(data["max_daily_doses"].is_null() || data["max_daily_doses"].as_i64().is_some());
    assert!(
        data["min_hours_between_doses"].is_null() || data["min_hours_between_doses"].is_string()
    );
    assert!(data["current_pause_period"].is_null() || data["current_pause_period"].is_object());
    OffsetDateTime::parse(data["updated_at"].as_str().expect("updated_at"), &Rfc3339)
        .expect("RFC3339 updated_at");
}

fn assert_uuid(value: &str) {
    let parts: Vec<_> = value.split('-').collect();
    assert_eq!(
        parts.iter().map(|part| part.len()).collect::<Vec<_>>(),
        [8, 4, 4, 4, 12]
    );
    assert!(parts
        .iter()
        .all(|part| part.bytes().all(|byte| byte.is_ascii_hexdigit())));
}

fn assert_decimal_equals(value: &Value, expected: &str) {
    let actual = value.as_str().expect("decimal string");
    let canonical = |number: &str| {
        let (whole, fraction) = number.split_once('.').unwrap_or((number, ""));
        let fraction = fraction.trim_end_matches('0');
        if fraction.is_empty() {
            whole.to_owned()
        } else {
            format!("{whole}.{fraction}")
        }
    };
    assert_eq!(canonical(actual), canonical(expected));
}

#[test]
fn owner_can_create_a_person_medication_assignment() {
    let fixture = fixture();
    let target = Target::from_env();
    let before_position = latest_position(fixture.managed_person_id);
    let response = target.post_json_authorized(
        &assignments_path(&fixture),
        &fixture.access_token,
        &create_payload(
            &fixture.managed_person_portable_id,
            &fixture.dose_write_schedule_medication_id.to_string(),
        ),
    );
    assert_eq!(response.status().as_u16(), 201);
    let created_tag = etag(&response);
    assert!(!created_tag.is_empty());
    let request = request_id(&response);
    let body: Value = response.json().expect("assignment JSON");
    assert_eq!(body.as_object().expect("response object").len(), 1);
    assert_assignment(&body["data"]);
    assert_eq!(body["data"]["person_id"], fixture.managed_person_id);
    assert_eq!(
        body["data"]["medication_id"],
        fixture.dose_write_schedule_medication_id
    );
    assert_eq!(
        body["data"]["person_portable_id"],
        fixture.managed_person_portable_id
    );
    assert_eq!(
        body["data"]["medication_portable_id"],
        medication_portable_id(fixture.dose_write_schedule_medication_id)
    );
    assert_eq!(body["data"]["position"], before_position + 1);
    assert_eq!(body["data"]["dose_amount"], "1.25");
    assert_eq!(body["data"]["dose_unit"], "ml");
    assert_eq!(body["data"]["administration_kind"], "as_needed");
    assert_eq!(body["data"]["active"], true);
    assert_eq!(body["data"]["paused"], false);
    assert_eq!(body["data"]["can_manage"], true);
    let assignment_id = body["data"]["id"].as_i64().expect("assignment ID");
    assert_create_audit(&request, assignment_id);
    assert_eq!(
        assignment_count(
            fixture.managed_person_id,
            fixture.dose_write_schedule_medication_id
        ),
        1
    );
    let detail = target.get(
        &format!(
            "{}/{}",
            assignments_path(&fixture),
            body["data"]["portable_id"].as_str().unwrap()
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(detail.status().as_u16(), 200);
    assert_eq!(etag(&detail), created_tag);
    let detail_body: Value = detail.json().expect("created assignment detail");
    assert_eq!(detail_body.as_object().expect("detail envelope").len(), 1);
    assert_assignment(&detail_body["data"]);
    assert_eq!(detail_body["data"], body["data"]);
}

#[test]
fn create_uses_the_default_dose_and_matching_dosage_option() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = assignments_path(&fixture);
    let payload = json!({"person_medication": {
        "person_id": fixture.user_person_id.to_string(),
        "medication_id": fixture.managed_medication_portable_id
    }});
    let mismatched_snapshot = json!({"person_medication": {
        "person_id": fixture.user_person_id.to_string(),
        "medication_id": fixture.managed_medication_portable_id,
        "source_dosage_option_id": fixture.managed_dosage_portable_id,
        "dose_amount": "1.25",
        "dose_unit": "ml"
    }});
    assert_error(
        create_request(&path, &fixture.access_token, &mismatched_snapshot, None),
        422,
    );
    assert_eq!(
        assignment_count(fixture.user_person_id, fixture.managed_medication_id),
        0
    );
    for (option, status) in [
        (fixture.hidden_dosage_portable_id.clone(), 422),
        (fixture.foreign_dosage_portable_id.clone(), 404),
        (fixture.managed_dosage_portable_id.clone(), 422),
    ] {
        assert_error(
            create_request(
                &path,
                &fixture.access_token,
                &json!({"person_medication": {
                    "person_id": fixture.user_person_id.to_string(),
                    "medication_id": fixture.dose_write_other_medication_id.to_string(),
                    "source_dosage_option_id": option
                }}),
                None,
            ),
            status,
        );
        assert_eq!(
            assignment_count(
                fixture.user_person_id,
                fixture.dose_write_other_medication_id
            ),
            0
        );
    }
    let response = create_request(&path, &fixture.access_token, &payload, None);
    assert_eq!(response.status().as_u16(), 201);
    let created: Value = response.json().expect("default assignment JSON");
    assert_assignment(&created["data"]);
    let assignment_id = created["data"]["id"].as_i64().expect("assignment ID");
    let _fields_guard = AssignmentFieldsGuard::new(assignment_id);
    assert_eq!(created["data"]["person_id"], fixture.user_person_id);
    assert_eq!(
        created["data"]["medication_id"],
        fixture.managed_medication_id
    );
    assert_eq!(
        created["data"]["medication_portable_id"],
        fixture.managed_medication_portable_id
    );
    assert_decimal_equals(&created["data"]["dose_amount"], "1");
    assert_eq!(created["data"]["dose_unit"], "ml");
    let source_option: Option<i64> = database()
        .query_one(
            "SELECT source_dosage_option_id FROM person_medications WHERE id = $1",
            &[&assignment_id],
        )
        .expect("assignment dosage source")
        .get(0);
    let expected_option: i64 = database()
        .query_one(
            "SELECT id FROM dosages WHERE portable_id = $1",
            &[&fixture.managed_dosage_portable_id],
        )
        .expect("default dosage option")
        .get(0);
    assert_eq!(source_option, Some(expected_option));
    let created_path = format!(
        "{path}/{}",
        created["data"]["portable_id"]
            .as_str()
            .expect("assignment portable ID")
    );
    let before_invalid_dose = target.get(&created_path, Some(&fixture.access_token));
    assert_eq!(before_invalid_dose.status().as_u16(), 200);
    let before_invalid_dose_tag = etag(&before_invalid_dose);
    let before_invalid_dose: Value = before_invalid_dose.json().expect("bound dosage assignment");
    assert_error(
        target.patch_json(
            &created_path,
            &fixture.access_token,
            &json!({"person_medication": {"dose_amount": "2.5"}}),
        ),
        422,
    );
    let after_invalid_dose = target.get(&created_path, Some(&fixture.access_token));
    assert_eq!(etag(&after_invalid_dose), before_invalid_dose_tag);
    assert_eq!(
        after_invalid_dose
            .json::<Value>()
            .expect("bound assignment unchanged"),
        before_invalid_dose
    );
    let notes_update = target.patch_json(
        &created_path,
        &fixture.access_token,
        &json!({"person_medication": {"notes": "Retain source dosage"}}),
    );
    assert_eq!(notes_update.status().as_u16(), 200);
    let source_after_notes: Option<i64> = database()
        .query_one(
            "SELECT source_dosage_option_id FROM person_medications WHERE id = $1",
            &[&assignment_id],
        )
        .expect("source dosage after notes update")
        .get(0);
    assert_eq!(source_after_notes, Some(expected_option));
    assert_eq!(
        assignment_count(fixture.user_person_id, fixture.managed_medication_id),
        1
    );

    let hidden_option_path = format!("{path}/{}", fixture.dose_write_assignment_portable_id);
    let before_hidden_option = target.get(&hidden_option_path, Some(&fixture.access_token));
    assert_eq!(before_hidden_option.status().as_u16(), 200);
    let before_hidden_option_tag = etag(&before_hidden_option);
    let before_hidden_option: Value = before_hidden_option
        .json()
        .expect("assignment before hidden option reference");
    assert_error(
        target.patch_json(
            &hidden_option_path,
            &fixture.delegated_access_token,
            &json!({"person_medication": {
                "source_dosage_option_id": fixture.hidden_dosage_portable_id
            }}),
        ),
        404,
    );
    let after_hidden_option = target.get(&hidden_option_path, Some(&fixture.access_token));
    assert_eq!(etag(&after_hidden_option), before_hidden_option_tag);
    assert_eq!(
        after_hidden_option
            .json::<Value>()
            .expect("assignment after hidden option reference"),
        before_hidden_option
    );

    assert_error(
        create_request(&path, &fixture.access_token, &payload, None),
        422,
    );
    assert_eq!(
        assignment_count(fixture.user_person_id, fixture.managed_medication_id),
        1
    );
}

#[test]
fn person_medication_reads_keep_schema_pagination_filter_and_visibility() {
    let fixture = fixture();
    let target = Target::from_env();
    let base = assignments_path(&fixture);
    let response = target.get(
        &format!("{base}?page=1&per_page=100"),
        Some(&fixture.view_access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let collection: Value = response.json().expect("assignment collection JSON");
    assert_eq!(
        collection.as_object().expect("collection envelope").len(),
        2
    );
    assert_eq!(collection["meta"]["page"], 1);
    assert_eq!(collection["meta"]["per_page"], 100);
    assert!(collection["meta"]["total_count"].as_u64().is_some());
    let rows = collection["data"].as_array().expect("assignment rows");
    assert!(rows
        .iter()
        .any(|row| row["id"] == fixture.managed_assignment_id));
    assert!(!rows
        .iter()
        .any(|row| row["id"] == fixture.hidden_assignment_id));
    assert!(!rows
        .iter()
        .any(|row| row["id"] == fixture.foreign_assignment_id));
    assert!(rows.iter().all(|row| {
        assert_assignment(row);
        row["can_manage"] == false
    }));

    let paged = target.get(
        &format!("{base}?page=1&per_page=1"),
        Some(&fixture.access_token),
    );
    assert_eq!(paged.status().as_u16(), 200);
    let page_one: Value = paged.json().expect("first assignment page");
    assert_eq!(page_one["meta"]["per_page"], 1);
    assert_eq!(page_one["data"].as_array().unwrap().len(), 1);
    let page_two = target.get(
        &format!("{base}?page=2&per_page=1"),
        Some(&fixture.access_token),
    );
    assert_eq!(page_two.status().as_u16(), 200);
    let page_two: Value = page_two.json().expect("second assignment page");
    assert_eq!(
        page_two["meta"]["total_count"],
        page_one["meta"]["total_count"]
    );
    assert_eq!(page_two["data"].as_array().unwrap().len(), 1);
    assert_ne!(page_two["data"][0]["id"], page_one["data"][0]["id"]);

    let future = target.get(
        &format!("{base}?updated_since=2099-01-01T00:00:00Z"),
        Some(&fixture.access_token),
    );
    assert_eq!(future.status().as_u16(), 200);
    let future: Value = future.json().expect("updated_since collection");
    assert_eq!(future["data"].as_array().unwrap().len(), 0);
    assert_eq!(future["meta"]["total_count"], 0);

    for query in [
        "page=0",
        "per_page=101",
        "page=not-a-number",
        "updated_since=invalid",
    ] {
        assert_error(
            target.get(&format!("{base}?{query}"), Some(&fixture.access_token)),
            422,
        );
    }

    let detail_path = format!("{base}/{}", fixture.managed_assignment_portable_id);
    let detail = target.get(&detail_path, Some(&fixture.access_token));
    assert_eq!(detail.status().as_u16(), 200);
    assert!(!etag(&detail).is_empty());
    let detail: Value = detail.json().expect("assignment detail JSON");
    assert_eq!(detail.as_object().expect("detail envelope").len(), 1);
    assert_assignment(&detail["data"]);
    assert_eq!(detail["data"]["id"], fixture.managed_assignment_id);

    assert_error(target.get(&base, None), 401);
    let view_detail = target.get(&detail_path, Some(&fixture.view_access_token));
    assert_eq!(view_detail.status().as_u16(), 200);
    assert_eq!(
        view_detail.json::<Value>().expect("view assignment detail")["data"]["can_manage"],
        false
    );
    assert_error(target.get(&detail_path, None), 401);
    assert_error(
        target.get(
            &format!("{base}/{}", fixture.hidden_assignment_portable_id),
            Some(&fixture.view_access_token),
        ),
        404,
    );
    assert_error(
        target.get(
            &format!("{base}/{}", fixture.foreign_assignment_portable_id),
            Some(&fixture.access_token),
        ),
        404,
    );
    assert_error(
        target.get(
            &format!(
                "/api/v1/households/{}/person_medications",
                fixture.foreign_household_id
            ),
            Some(&fixture.access_token),
        ),
        403,
    );
    assert_error(
        target.get(
            &format!(
                "/api/v1/households/{}/person_medications/{}",
                fixture.foreign_household_id, fixture.foreign_assignment_portable_id
            ),
            Some(&fixture.access_token),
        ),
        403,
    );
    assert_error(
        target.get(
            "/api/v1/households/999999999/person_medications",
            Some(&fixture.access_token),
        ),
        404,
    );
}

#[test]
fn patch_and_put_merge_fields_without_required_etag_and_invalid_updates_keep_state() {
    let fixture = fixture();
    let _fields_guard = AssignmentFieldsGuard::new(fixture.dose_write_assignment_id);
    let target = Target::from_env();
    let path = format!(
        "{}/{}",
        assignments_path(&fixture),
        fixture.dose_write_assignment_id
    );
    let original_response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(original_response.status().as_u16(), 200);
    let original_tag = etag(&original_response);
    let original: Value = original_response.json().expect("original assignment");
    let original_amount = original["data"]["dose_amount"].clone();
    let original_unit = original["data"]["dose_unit"].clone();
    let original_kind = original["data"]["administration_kind"].clone();

    let same_numeric_person = target.patch_json(
        &path,
        &fixture.access_token,
        &json!({"person_medication": {"person_id": fixture.managed_person_id.to_string()}}),
    );
    assert_eq!(same_numeric_person.status().as_u16(), 200);
    let same_portable_person = target.put_json(
        &path,
        &fixture.access_token,
        &json!({"person_medication": {"person_id": fixture.managed_person_portable_id}}),
    );
    assert_eq!(same_portable_person.status().as_u16(), 200);
    let before_person_change = target.get(&path, Some(&fixture.access_token));
    assert_eq!(before_person_change.status().as_u16(), 200);
    let before_person_change_tag = etag(&before_person_change);
    let before_person_change: Value = before_person_change
        .json()
        .expect("assignment before person change");
    for (method, person_identifier, status) in [
        ("PATCH", fixture.user_person_id.to_string(), 422),
        ("PUT", person_portable_id(fixture.user_person_id), 422),
        ("PATCH", fixture.hidden_person_portable_id.clone(), 404),
        ("PUT", fixture.foreign_person_portable_id.clone(), 404),
    ] {
        let payload = json!({"person_medication": {"person_id": person_identifier}});
        let response = if method == "PATCH" {
            target.patch_json(&path, &fixture.access_token, &payload)
        } else {
            target.put_json(&path, &fixture.access_token, &payload)
        };
        assert_error(response, status);
        let after = target.get(&path, Some(&fixture.access_token));
        assert_eq!(after.status().as_u16(), 200);
        assert_eq!(etag(&after), before_person_change_tag);
        let after: Value = after.json().expect("assignment after person change");
        assert_eq!(after["data"], before_person_change["data"]);
    }

    let patched = target.patch_json(
        &path,
        &fixture.access_token,
        &json!({"person_medication": {"notes": "PATCH merge"}}),
    );
    assert_eq!(patched.status().as_u16(), 200);
    let patched_tag = etag(&patched);
    assert_ne!(patched_tag, original_tag);
    let patched_request = request_id(&patched);
    let patched: Value = patched.json().expect("PATCH assignment JSON");
    assert_assignment(&patched["data"]);
    assert_eq!(patched["data"]["notes"], "PATCH merge");
    assert_eq!(patched["data"]["dose_amount"], original_amount);
    assert_eq!(patched["data"]["dose_unit"], original_unit);
    assert_eq!(patched["data"]["administration_kind"], original_kind);
    assert_update_audit(&patched_request, fixture.dose_write_assignment_id);

    assert_error(
        target.patch_json_if_match(
            &path,
            &fixture.access_token,
            &json!({"person_medication": {"notes": "Stale PATCH"}}),
            &original_tag,
        ),
        409,
    );

    let replaced = target.put_json(
        &path,
        &fixture.access_token,
        &json!({"person_medication": {"notes": "PUT merge"}}),
    );
    assert_eq!(replaced.status().as_u16(), 200);
    let replaced_tag = etag(&replaced);
    assert_ne!(replaced_tag, patched_tag);
    let replaced_request = request_id(&replaced);
    let replaced: Value = replaced.json().expect("PUT assignment JSON");
    assert_assignment(&replaced["data"]);
    assert_eq!(replaced["data"]["notes"], "PUT merge");
    assert_eq!(replaced["data"]["dose_amount"], original_amount);
    assert_eq!(replaced["data"]["dose_unit"], original_unit);
    assert_eq!(replaced["data"]["administration_kind"], original_kind);
    assert_update_audit(&replaced_request, fixture.dose_write_assignment_id);

    assert_error(
        target.patch_json(
            &path,
            &fixture.access_token,
            &json!({"person_medication": {"dose_amount": "3.125"}}),
        ),
        422,
    );
    assert_error(
        target.patch_json(
            &path,
            &fixture.access_token,
            &json!({"person_medication": {"min_hours_between_doses": "1.5"}}),
        ),
        422,
    );
    let unchanged = target.get(&path, Some(&fixture.access_token));
    assert_eq!(unchanged.status().as_u16(), 200);
    assert_eq!(etag(&unchanged), replaced_tag);
    let unchanged: Value = unchanged.json().expect("assignment after invalid writes");
    assert_eq!(unchanged["data"], replaced["data"]);

    let field_patch = target.patch_json(
        &path,
        &fixture.access_token,
        &json!({"person_medication": {
            "notes": "Contract field update",
            "max_daily_doses": 3,
            "min_hours_between_doses": "2.0",
            "dose_cycle": "weekly"
        }}),
    );
    assert_eq!(field_patch.status().as_u16(), 200);
    let field_patch: Value = field_patch
        .json()
        .expect("documented assignment field update");
    assert_eq!(field_patch["data"]["notes"], "Contract field update");
    assert_eq!(field_patch["data"]["max_daily_doses"], 3);
    assert_decimal_equals(&field_patch["data"]["min_hours_between_doses"], "2");
    assert_eq!(field_patch["data"]["dose_cycle"], "weekly");

    let field_put = target.put_json(
        &path,
        &fixture.access_token,
        &json!({"person_medication": {"min_hours_between_doses": null}}),
    );
    assert_eq!(field_put.status().as_u16(), 200);
    let field_put: Value = field_put.json().expect("nullable interval clear");
    assert_eq!(field_put["data"]["notes"], "Contract field update");
    assert_eq!(field_put["data"]["max_daily_doses"], 3);
    assert!(field_put["data"]["min_hours_between_doses"].is_null());
    assert_eq!(field_put["data"]["dose_cycle"], "weekly");
}

#[test]
fn no_op_patch_and_put_preserve_response_and_emit_only_request_audits() {
    let fixture = fixture();
    let target = Target::from_env();
    let assignment_id = fixture.dose_write_assignment_id;
    let path = format!(
        "{}/{}",
        assignments_path(&fixture),
        fixture.dose_write_assignment_portable_id
    );
    let initial = target.get(&path, Some(&fixture.access_token));
    assert_eq!(initial.status().as_u16(), 200);
    let initial_tag = etag(&initial);
    let initial: Value = initial.json().expect("initial no-op assignment");
    let updates_before = assignment_update_count(assignment_id);
    let versions_before = assignment_update_version_count(assignment_id);
    let key = format!("person-medication-noop-{}", fixture.household_id);
    let same_person = json!({"person_medication": {
        "person_id": fixture.managed_person_id.to_string()
    }});
    let first = assignment_write_request(
        "PATCH",
        &path,
        Some(&fixture.access_token),
        &same_person,
        Some(&key),
        None,
    );
    assert_eq!(first.status().as_u16(), 200);
    assert_eq!(etag(&first), initial_tag);
    let first_request = request_id(&first);
    let first: Value = first.json().expect("no-op PATCH response");
    assert_eq!(first, initial);
    assert_eq!(request_audit_count(fixture.household_id, &first_request), 1);

    let replay = assignment_write_request(
        "PATCH",
        &path,
        Some(&fixture.access_token),
        &same_person,
        Some(&key),
        None,
    );
    assert_eq!(replay.status().as_u16(), 200);
    assert_eq!(replay.headers()["idempotency-replayed"], "true");
    assert_eq!(etag(&replay), initial_tag);
    let replay_request = request_id(&replay);
    assert_eq!(replay.json::<Value>().expect("no-op replay"), initial);
    assert_eq!(
        request_audit_count(fixture.household_id, &replay_request),
        1
    );

    let same_amount = json!({"person_medication": {
        "dose_amount": initial["data"]["dose_amount"]
    }});
    let put = assignment_write_request(
        "PUT",
        &path,
        Some(&fixture.access_token),
        &same_amount,
        None,
        None,
    );
    assert_eq!(put.status().as_u16(), 200);
    assert_eq!(etag(&put), initial_tag);
    let put_request = request_id(&put);
    assert_eq!(put.json::<Value>().expect("no-op PUT response"), initial);
    assert_eq!(request_audit_count(fixture.household_id, &put_request), 1);
    assert_eq!(assignment_update_count(assignment_id), updates_before);
    assert_eq!(
        assignment_update_version_count(assignment_id),
        versions_before
    );
}

#[test]
fn patch_and_put_enforce_write_boundaries_and_replay_keys() {
    let fixture = fixture();
    let _fields_guard = AssignmentFieldsGuard::new(fixture.dose_write_assignment_id);
    let target = Target::from_env();
    let path = format!(
        "{}/{}",
        assignments_path(&fixture),
        fixture.dose_write_assignment_portable_id
    );
    let initial = target.get(&path, Some(&fixture.access_token));
    assert_eq!(initial.status().as_u16(), 200);
    let initial_tag = etag(&initial);
    let initial: Value = initial.json().expect("initial assignment for boundaries");
    let valid = json!({"person_medication": {"notes": "Rejected boundary write"}});
    let foreign_path = format!(
        "{}/{}",
        assignments_path(&fixture),
        fixture.foreign_assignment_portable_id
    );
    let missing_household_path = format!(
        "/api/v1/households/999999999/person_medications/{}",
        fixture.dose_write_assignment_portable_id
    );
    let methods = ["PATCH", "PUT"];
    for method in methods {
        assert_error(
            assignment_write_request(method, &path, None, &valid, None, None),
            401,
        );
        assert_error(
            assignment_write_request(method, &path, Some("invalid-token"), &valid, None, None),
            401,
        );
        assert_error(
            assignment_write_request(
                method,
                &path,
                Some(&fixture.view_access_token),
                &valid,
                None,
                None,
            ),
            403,
        );
        assert_error(
            assignment_write_request(
                method,
                &foreign_path,
                Some(&fixture.access_token),
                &valid,
                None,
                None,
            ),
            404,
        );
        assert_error(
            assignment_write_request(
                method,
                &missing_household_path,
                Some(&fixture.access_token),
                &valid,
                None,
                None,
            ),
            404,
        );
        assert_error(
            malformed_assignment_request(method, &path, &fixture.access_token),
            400,
        );
        assert_error(
            assignment_write_request(
                method,
                &path,
                Some(&fixture.access_token),
                &json!({"person_medication": {"unknown_contract_field": true}}),
                None,
                None,
            ),
            422,
        );
        assert_error(
            assignment_write_request(
                method,
                &path,
                Some(&fixture.access_token),
                &valid,
                None,
                Some("\"stale\""),
            ),
            409,
        );
        let after = target.get(&path, Some(&fixture.access_token));
        assert_eq!(after.status().as_u16(), 200);
        assert_eq!(etag(&after), initial_tag);
        let after: Value = after.json().expect("assignment after rejected write");
        assert_eq!(after, initial);
    }

    for (method, first_note, changed_note) in [
        ("PATCH", "PATCH first", "PATCH changed"),
        ("PUT", "PUT first", "PUT changed"),
    ] {
        let key = format!(
            "person-medication-{method}-{}",
            fixture.dose_write_assignment_id
        );
        let payload = json!({"person_medication": {"notes": first_note}});
        let first = assignment_write_request(
            method,
            &path,
            Some(&fixture.access_token),
            &payload,
            Some(&key),
            None,
        );
        assert_eq!(first.status().as_u16(), 200);
        let first_body: Value = first.json().expect("first keyed assignment write");
        let replay = assignment_write_request(
            method,
            &path,
            Some(&fixture.access_token),
            &payload,
            Some(&key),
            None,
        );
        assert_eq!(replay.status().as_u16(), 200);
        assert_eq!(replay.headers()["idempotency-replayed"], "true");
        assert_eq!(
            replay.json::<Value>().expect("replayed write body"),
            first_body
        );
        assert_error(
            assignment_write_request(
                method,
                &path,
                Some(&fixture.access_token),
                &json!({"person_medication": {"notes": changed_note}}),
                Some(&key),
                None,
            ),
            409,
        );
    }
}

#[test]
fn create_rejects_unrepresentable_dose_precision_and_replays_idempotently() {
    let fixture = fixture();
    let path = assignments_path(&fixture);
    let payload = create_payload(
        &fixture.user_person_id.to_string(),
        &fixture.dose_write_schedule_medication_id.to_string(),
    );
    let mut payload = payload;
    payload["person_medication"]["dose_amount"] = json!("1.250");
    payload["person_medication"]["min_hours_between_doses"] = json!("2.0");
    assert_eq!(
        assignment_count(
            fixture.user_person_id,
            fixture.dose_write_schedule_medication_id
        ),
        0
    );
    let invalid_payloads = [
        (
            json!({"person_medication": {
                "person_id": fixture.user_person_id.to_string(),
                "medication_id": fixture.dose_write_schedule_medication_id.to_string(),
                "unknown_contract_field": true
            }}),
            422,
        ),
        (
            json!({"unexpected": true, "person_medication": {
                "person_id": fixture.user_person_id.to_string(),
                "medication_id": fixture.dose_write_schedule_medication_id.to_string()
            }}),
            422,
        ),
        (
            json!({"person_medication": {
                "person_id": fixture.user_person_id.to_string(),
                "medication_id": fixture.dose_write_schedule_medication_id.to_string(),
                "dose_amount": 1.25
            }}),
            422,
        ),
        (
            json!({"person_medication": {
                "person_id": fixture.user_person_id.to_string()
            }}),
            422,
        ),
    ];
    for (invalid, status) in invalid_payloads {
        assert_error(
            create_request(&path, &fixture.access_token, &invalid, None),
            status,
        );
        assert_eq!(
            assignment_count(
                fixture.user_person_id,
                fixture.dose_write_schedule_medication_id
            ),
            0
        );
    }
    for (field_value, expected_status) in [("+1", 422), ("01", 422), ("not-a-uuid", 422)] {
        let invalid = json!({"person_medication": {
            "person_id": field_value,
            "medication_id": fixture.dose_write_schedule_medication_id.to_string()
        }});
        assert_error(
            create_request(&path, &fixture.access_token, &invalid, None),
            expected_status,
        );
    }
    assert_eq!(
        assignment_count(
            fixture.user_person_id,
            fixture.dose_write_schedule_medication_id
        ),
        0
    );
    for amount in ["+1", "1_0"] {
        let invalid = json!({"person_medication": {
            "person_id": fixture.user_person_id.to_string(),
            "medication_id": fixture.dose_write_schedule_medication_id.to_string(),
            "dose_amount": amount
        }});
        assert_error(
            create_request(&path, &fixture.access_token, &invalid, None),
            422,
        );
    }
    assert_eq!(
        assignment_count(
            fixture.user_person_id,
            fixture.dose_write_schedule_medication_id
        ),
        0
    );
    let unrepresentable = json!({"person_medication": {
        "person_id": fixture.user_person_id.to_string(),
        "medication_id": fixture.dose_write_schedule_medication_id.to_string(),
        "dose_amount": "1.234",
        "dose_unit": "ml",
        "administration_kind": "as_needed"
    }});
    assert_error(
        create_request(&path, &fixture.access_token, &unrepresentable, None),
        422,
    );
    assert_eq!(
        assignment_count(
            fixture.user_person_id,
            fixture.dose_write_schedule_medication_id
        ),
        0
    );
    let invalid_key = format!("person-medication-invalid-{}", fixture.household_id);
    let first_invalid = create_request(
        &path,
        &fixture.access_token,
        &unrepresentable,
        Some(&invalid_key),
    );
    assert_eq!(first_invalid.status().as_u16(), 422);
    let first_invalid_body: Value = first_invalid.json().expect("invalid create body");
    let replay_invalid = create_request(
        &path,
        &fixture.access_token,
        &unrepresentable,
        Some(&invalid_key),
    );
    assert_eq!(replay_invalid.status().as_u16(), 422);
    assert_eq!(replay_invalid.headers()["idempotency-replayed"], "true");
    assert_eq!(
        replay_invalid
            .json::<Value>()
            .expect("replayed invalid body"),
        first_invalid_body
    );
    assert_error(
        create_request(&path, &fixture.access_token, &payload, Some(&invalid_key)),
        409,
    );
    assert_eq!(
        assignment_count(
            fixture.user_person_id,
            fixture.dose_write_schedule_medication_id
        ),
        0
    );
    let fractional_interval = json!({"person_medication": {
        "person_id": fixture.user_person_id.to_string(),
        "medication_id": fixture.dose_write_schedule_medication_id.to_string(),
        "min_hours_between_doses": "1.5"
    }});
    assert_error(
        create_request(&path, &fixture.access_token, &fractional_interval, None),
        422,
    );
    assert_eq!(
        assignment_count(
            fixture.user_person_id,
            fixture.dose_write_schedule_medication_id
        ),
        0
    );

    let key = format!("person-medication-create-{}", fixture.household_id);
    let first = create_request(&path, &fixture.access_token, &payload, Some(&key));
    assert_eq!(first.status().as_u16(), 201);
    let first_tag = etag(&first);
    let first_request = request_id(&first);
    let first: Value = first.json().expect("idempotent create JSON");
    assert_assignment(&first["data"]);
    assert_decimal_equals(&first["data"]["dose_amount"], "1.25");
    assert_decimal_equals(&first["data"]["min_hours_between_doses"], "2");
    let id = first["data"]["id"].as_i64().expect("created assignment ID");
    assert_create_audit(&first_request, id);
    assert_eq!(
        assignment_count(
            fixture.user_person_id,
            fixture.dose_write_schedule_medication_id
        ),
        1
    );

    let replay = create_request(&path, &fixture.access_token, &payload, Some(&key));
    assert_eq!(replay.status().as_u16(), 201);
    assert_eq!(replay.headers()["idempotency-replayed"], "true");
    assert_eq!(etag(&replay), first_tag);
    let replay: Value = replay.json().expect("replayed create JSON");
    assert_eq!(replay, first);
    assert_create_audit(&first_request, id);
    assert_eq!(
        assignment_count(
            fixture.user_person_id,
            fixture.dose_write_schedule_medication_id
        ),
        1
    );

    assert_error(
        create_request(
            &path,
            &fixture.access_token,
            &json!({"person_medication": {
                "person_id": fixture.user_person_id.to_string(),
                "medication_id": fixture.dose_write_schedule_medication_id.to_string(),
                "dose_amount": "2.25",
                "dose_unit": "ml",
                "administration_kind": "as_needed"
            }}),
            Some(&key),
        ),
        409,
    );
    {
        let _manager_grant = AccessGrantGuard::new(
            fixture.household_id,
            fixture.manager_membership_id,
            fixture.user_person_id,
            "manage",
        );
        assert_error(
            create_request(&path, &fixture.manager_access_token, &payload, Some(&key)),
            409,
        );
    }
    assert_eq!(
        assignment_count(
            fixture.user_person_id,
            fixture.dose_write_schedule_medication_id
        ),
        1
    );
}

#[test]
fn create_checks_person_medication_and_household_authority() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = assignments_path(&fixture);
    let valid = create_payload(
        &fixture.managed_person_portable_id,
        &fixture.hidden_medication_portable_id,
    );
    assert_error(create_request_without_auth(&path, &valid), 401);
    assert_error(
        target.post_json_authorized(&path, "invalid-token", &valid),
        401,
    );
    assert_error(
        target.post_json_authorized(
            &path,
            &fixture.view_access_token,
            &create_payload(
                &fixture.managed_person_portable_id,
                &fixture.managed_medication_portable_id,
            ),
        ),
        403,
    );
    assert_error(
        target.post_json_authorized(
            &format!(
                "/api/v1/households/{}/person_medications",
                fixture.foreign_household_id
            ),
            &fixture.access_token,
            &valid,
        ),
        403,
    );
    assert_error(
        target.post_json_authorized(
            &path,
            &fixture.access_token,
            &create_payload(
                &fixture.foreign_person_portable_id,
                &fixture.managed_medication_portable_id,
            ),
        ),
        404,
    );
    assert_error(
        target.post_json_authorized(
            &path,
            &fixture.access_token,
            &create_payload(
                &fixture.managed_person_portable_id,
                &fixture.foreign_medication_portable_id,
            ),
        ),
        404,
    );
    assert_error(
        target.post_json_authorized(&path, &fixture.access_token, &json!({})),
        400,
    );
    assert_error(
        target.post_json_authorized(&path, &fixture.access_token, &json!({"unexpected": {}})),
        400,
    );
    assert_eq!(
        assignment_count(fixture.managed_person_id, fixture.managed_medication_id),
        1
    );
}

#[test]
fn owner_and_administrator_writes_require_current_manage_grants() {
    let fixture = fixture();
    let target = Target::from_env();
    let item = format!(
        "{}/{}",
        assignments_path(&fixture),
        fixture.managed_assignment_portable_id
    );
    let create = json!({"person_medication": {
        "person_id": fixture.managed_person_portable_id,
        "medication_id": fixture.managed_medication_portable_id
    }});
    for (token, membership_id) in [
        (&fixture.access_token, fixture.owner_membership_id),
        (&fixture.manager_access_token, fixture.manager_membership_id),
    ] {
        let _grant = AccessGrantGuard::new(
            fixture.household_id,
            membership_id,
            fixture.managed_person_id,
            "view",
        );
        let before = target.get(&item, Some(token));
        assert_eq!(before.status().as_u16(), 200);
        let before_tag = etag(&before);
        let before: Value = before.json().expect("assignment before grant checks");
        let count = assignment_count(fixture.managed_person_id, fixture.managed_medication_id);
        assert_error(
            target.post_json_authorized(&assignments_path(&fixture), token, &create),
            403,
        );
        assert_error(
            target.patch_json(
                &item,
                token,
                &json!({"person_medication": {"notes": "Denied without manage grant"}}),
            ),
            403,
        );
        assert_error(
            target.put_json(
                &item,
                token,
                &json!({"person_medication": {"notes": "Denied without manage grant"}}),
            ),
            403,
        );
        assert_eq!(
            assignment_count(fixture.managed_person_id, fixture.managed_medication_id),
            count
        );
        let after = target.get(&item, Some(token));
        assert_eq!(etag(&after), before_tag);
        assert_eq!(
            after
                .json::<Value>()
                .expect("assignment after grant checks"),
            before
        );
    }
}

#[test]
fn concurrent_same_etag_assignment_updates_have_one_winner() {
    let fixture = fixture();
    let _fields_guard = AssignmentFieldsGuard::new(fixture.dose_write_assignment_id);
    let target = Target::from_env();
    let path = format!(
        "{}/{}",
        assignments_path(&fixture),
        fixture.dose_write_assignment_portable_id
    );
    let initial = target.get(&path, Some(&fixture.access_token));
    assert_eq!(initial.status().as_u16(), 200);
    let tag = etag(&initial);
    let before_audits = assignment_update_count(fixture.dose_write_assignment_id);
    let barrier = Arc::new(Barrier::new(3));
    let handles: Vec<_> = ["concurrent update one", "concurrent update two"]
        .into_iter()
        .map(|amount| {
            let barrier = Arc::clone(&barrier);
            let path = path.clone();
            let token = fixture.access_token.clone();
            let tag = tag.clone();
            thread::spawn(move || {
                barrier.wait();
                let response = assignment_write_request(
                    "PATCH",
                    &path,
                    Some(&token),
                    &json!({"person_medication": {"notes": amount}}),
                    None,
                    Some(&tag),
                );
                (
                    response.status().as_u16(),
                    response.json::<Value>().expect("concurrent write JSON"),
                )
            })
        })
        .collect();
    barrier.wait();
    let outcomes: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().expect("concurrent assignment worker"))
        .collect();
    assert_eq!(
        outcomes.iter().filter(|outcome| outcome.0 == 200).count(),
        1
    );
    assert_eq!(
        outcomes.iter().filter(|outcome| outcome.0 == 409).count(),
        1
    );
    assert_eq!(
        assignment_update_count(fixture.dose_write_assignment_id),
        before_audits + 1
    );
    let current = target.get(&path, Some(&fixture.access_token));
    assert_eq!(current.status().as_u16(), 200);
    let current: Value = current.json().expect("assignment after concurrent update");
    assert!(outcomes
        .iter()
        .any(|outcome| outcome.0 == 200 && outcome.1["data"] == current["data"]));
}

#[test]
fn z_rate_limiter_wires_assignment_collection_and_item_writes() {
    let fixture = fixture();
    let base = env::var("CONTRACT_RATE_BASE_URL").expect("rate-limited API URL");
    let client = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("rate limit client");
    let started = std::time::Instant::now();
    let mut limited = false;
    for _ in 0..601 {
        let response = client
            .get(format!("{base}/api/v1/capabilities"))
            .send()
            .expect("rate limit request");
        if response.status().as_u16() == 429 {
            assert_eq!(response.headers()["ratelimit-limit"], "300");
            assert_eq!(response.headers()["ratelimit-remaining"], "0");
            assert!(response.headers().get("retry-after").is_some());
            assert_eq!(
                response.json::<Value>().expect("rate limit JSON")["error"]["code"],
                "rate_limited"
            );
            limited = true;
            break;
        }
        assert_eq!(response.status().as_u16(), 200);
    }
    assert!(limited && started.elapsed() < Duration::from_secs(60));
    let collection = format!("{base}{}", assignments_path(&fixture));
    let item = format!("{collection}/{}", fixture.managed_assignment_portable_id);
    let payload = json!({"person_medication": {"dose_amount": "4.25"}});
    let limited_responses = [
        client
            .get(&collection)
            .bearer_auth(&fixture.access_token)
            .send(),
        client
            .post(&collection)
            .bearer_auth(&fixture.access_token)
            .json(&create_payload(
                &fixture.managed_person_portable_id,
                &fixture.dose_write_schedule_medication_id.to_string(),
            ))
            .send(),
        client.get(&item).bearer_auth(&fixture.access_token).send(),
        client
            .patch(&item)
            .bearer_auth(&fixture.access_token)
            .json(&payload)
            .send(),
        client
            .put(&item)
            .bearer_auth(&fixture.access_token)
            .json(&payload)
            .send(),
    ];
    for response in limited_responses {
        assert_eq!(
            response
                .expect("rate-limited assignment operation")
                .status()
                .as_u16(),
            429
        );
    }
}
