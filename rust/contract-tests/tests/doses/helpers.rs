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
            "current_supply": "20.0",
            "reorder_threshold": "3"
        }}),
    );
    let status = response.status().as_u16();
    let created = body(response);
    assert_eq!(status, 201, "canonical dose medication setup: {created}");
    created["data"].clone()
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

fn zero_stock_clinical_state(fixture: &Fixture) -> Value {
    let mut db = postgres::Client::connect(
        &std::env::var("CONTRACT_AUDIT_DATABASE_URL").unwrap(),
        postgres::NoTls,
    ).unwrap();
    let text: String = db.query_one(
        "SELECT json_build_object('medications', (SELECT COALESCE(json_agg(m ORDER BY m.id), '[]'::json) FROM medications m WHERE household_id=$1), 'dosages', (SELECT COALESCE(json_agg(d ORDER BY d.id), '[]'::json) FROM dosages d WHERE household_id=$1), 'assignments', (SELECT COALESCE(json_agg(p ORDER BY p.id), '[]'::json) FROM person_medications p WHERE household_id=$1), 'schedules', (SELECT COALESCE(json_agg(s ORDER BY s.id), '[]'::json) FROM schedules s WHERE household_id=$1), 'occurrences', (SELECT COALESCE(json_agg(o ORDER BY o.id), '[]'::json) FROM medication_dose_occurrences o WHERE household_id=$1), 'takes', (SELECT COALESCE(json_agg(t ORDER BY t.id), '[]'::json) FROM medication_takes t WHERE household_id=$1), 'versions', (SELECT COALESCE(json_agg(v ORDER BY v.id), '[]'::json) FROM versions v WHERE household_id=$1), 'sync', (SELECT COALESCE(json_agg(e ORDER BY e.id), '[]'::json) FROM api_change_events e WHERE household_id=$1))::text",
        &[&fixture.household_id],
    ).unwrap().get(0);
    serde_json::from_str(&text).unwrap()
}
