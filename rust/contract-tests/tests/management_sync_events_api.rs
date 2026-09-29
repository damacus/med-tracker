use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::Response;
use serde_json::{json, Value};
use std::env;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

fn body(response: Response) -> Value {
    response.json().expect("JSON response")
}

fn database() -> postgres::Client {
    let url = env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract audit database URL");
    postgres::Client::connect(&url, postgres::NoTls).expect("contract database")
}

fn request_id(response: &Response) -> String {
    response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned()
}

fn medications_path(fixture: &Fixture) -> String {
    format!("/api/v1/households/{}/medications", fixture.household_id)
}

fn create_medication(
    target: &Target,
    fixture: &Fixture,
    name: &str,
    supply: &str,
) -> (i64, String) {
    let response = target.post_json_authorized(
        &medications_path(fixture),
        &fixture.access_token,
        &json!({"medication": {
            "name": name,
            "location_id": fixture.primary_location_id,
            "current_supply": supply,
            "reorder_threshold": "1.00"
        }}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let id = request_id(&response);
    let medication_id = body(response)["data"]["id"].as_i64().unwrap();
    (medication_id, id)
}

fn portable_id(table: &str, id: i64) -> String {
    let query = match table {
        "medications" => "SELECT portable_id FROM medications WHERE id = $1",
        "dosages" => "SELECT portable_id FROM dosages WHERE id = $1",
        "medication_takes" => "SELECT portable_id FROM medication_takes WHERE id = $1",
        _ => panic!("unsupported portable table"),
    };
    database().query_one(query, &[&id]).unwrap().get(0)
}

fn event_count(record_type: &str, record_id: i64) -> i64 {
    database()
        .query_one(
            "SELECT count(*) FROM api_change_events WHERE record_type = $1 AND record_id = $2",
            &[&record_type, &record_id],
        )
        .expect("sync event count")
        .get(0)
}

fn assert_event(
    fixture: &Fixture,
    record_type: &str,
    record_id: i64,
    portable: &str,
    action: &str,
    request: &str,
    person_portable: Option<&str>,
) {
    let rows = database()
        .query(
            "SELECT row_to_json(api_change_events)::text FROM api_change_events WHERE record_type = $1 AND record_id = $2 AND action = $3 AND request_id = $4",
            &[&record_type, &record_id, &action, &request],
        )
        .expect("matching sync events");
    assert_eq!(
        rows.len(),
        1,
        "one {record_type} {action} event for {request}"
    );
    let event: Value = serde_json::from_str(&rows[0].get::<_, String>(0)).unwrap();
    assert_eq!(event["household_id"], fixture.household_id);
    assert_eq!(event["account_id"], fixture.account_id);
    assert_eq!(
        event["household_membership_id"],
        fixture.owner_membership_id
    );
    assert_eq!(event["request_id"], request);
    assert_eq!(event["record_type"], record_type);
    assert_eq!(event["record_id"], record_id);
    assert_eq!(event["record_portable_id"], portable);
    assert_eq!(event["action"], action);
    assert_eq!(event["metadata"]["record_type"], record_type);
    assert_eq!(event["metadata"]["record_id"], record_id);
    assert_eq!(event["metadata"]["portable_id"], portable);
    if let Some(person) = person_portable {
        assert_eq!(event["metadata"]["person_portable_id"], person);
    }
    assert!(!event.to_string().contains(&fixture.access_token));
}

fn assert_request_audit(request: &str, status: i32) {
    let row = database()
        .query_one(
            "SELECT metadata::text FROM security_audit_events WHERE request_id = $1 AND event_type = 'api.request'",
            &[&request],
        )
        .expect("request audit");
    let metadata: Value = serde_json::from_str(&row.get::<_, String>(0)).unwrap();
    assert_eq!(metadata["status"], status);
}

fn tracked_option(fixture: &Fixture, medication_id: i64, stock: &str) -> i64 {
    database()
        .query_one(
            "INSERT INTO dosages (household_id, medication_id, amount, unit, frequency, current_supply, created_at, updated_at) VALUES ($1, $2, '1.25', 'ml', 'daily', $3::text::numeric, NOW(), NOW()) RETURNING id",
            &[&fixture.household_id, &medication_id, &stock],
        )
        .expect("tracked option")
        .get(0)
}

fn stock(medication_id: i64) -> String {
    database()
        .query_one(
            "SELECT current_supply::text FROM medications WHERE id = $1",
            &[&medication_id],
        )
        .expect("medication stock")
        .get(0)
}

fn dosage_stock(dosage_id: i64) -> String {
    database()
        .query_one(
            "SELECT current_supply::text FROM dosages WHERE id = $1",
            &[&dosage_id],
        )
        .expect("dosage stock")
        .get(0)
}

#[test]
fn ordinary_management_writes_emit_attributed_sync_updates_only_when_committed() {
    let fixture = fixture();
    let target = Target::from_env();
    let (medication_id, created_request) = create_medication(
        &target,
        &fixture,
        "Contract management feed medicine",
        "10.00",
    );
    let medication_portable = portable_id("medications", medication_id);
    let path = format!("{}/{medication_id}", medications_path(&fixture));
    assert_event(
        &fixture,
        "Medication",
        medication_id,
        &medication_portable,
        "create",
        &created_request,
        None,
    );
    assert_request_audit(&created_request, 201);

    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let etag = response.headers()["etag"].to_str().unwrap().to_owned();
    let response = target.patch_json_if_match(
        &path,
        &fixture.access_token,
        &json!({"medication": {"description": "Current household edit"}}),
        &etag,
    );
    assert_eq!(response.status().as_u16(), 200);
    let edit_request = request_id(&response);
    assert_event(
        &fixture,
        "Medication",
        medication_id,
        &medication_portable,
        "update",
        &edit_request,
        None,
    );

    let response = target.patch_json(
        &format!("{path}/adjust_inventory"),
        &fixture.access_token,
        &json!({"adjustment": {"new_quantity": "9.75", "reason": "counted"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let adjust_request = request_id(&response);
    assert_event(
        &fixture,
        "Medication",
        medication_id,
        &medication_portable,
        "update",
        &adjust_request,
        None,
    );

    let response = target.patch_json(
        &format!("{path}/mark_as_ordered"),
        &fixture.access_token,
        &json!({"order_details": {"quantity": "5.00"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let order_request = request_id(&response);
    assert_event(
        &fixture,
        "Medication",
        medication_id,
        &medication_portable,
        "update",
        &order_request,
        None,
    );

    let removal_path = format!("{path}/stock_removals");
    let removal = json!({"stock_removal": {
        "quantity": "1.25", "reason": "dropped",
        "submission_id": "d1000000-0000-4000-8000-000000000001"
    }});
    let response = target.post_json_authorized(&removal_path, &fixture.access_token, &removal);
    assert_eq!(response.status().as_u16(), 201);
    let removal_request = request_id(&response);
    assert_event(
        &fixture,
        "Medication",
        medication_id,
        &medication_portable,
        "update",
        &removal_request,
        None,
    );
    assert_eq!(stock(medication_id), "8.50");
    let before = event_count("Medication", medication_id);
    let response = target.post_json_authorized(&removal_path, &fixture.access_token, &removal);
    assert_eq!(response.status().as_u16(), 201);
    assert_request_audit(&request_id(&response), 201);
    assert_eq!(event_count("Medication", medication_id), before);
    let bad = json!({"stock_removal": {
        "quantity": "50", "reason": "dropped",
        "submission_id": "d1000000-0000-4000-8000-000000000002"
    }});
    let response = target.post_json_authorized(&removal_path, &fixture.access_token, &bad);
    assert_eq!(response.status().as_u16(), 422);
    assert_request_audit(&request_id(&response), 422);
    assert_eq!(event_count("Medication", medication_id), before);
    assert_eq!(stock(medication_id), "8.50");
}

#[test]
fn tracked_removal_emits_option_and_parent_updates_without_replay_events() {
    let fixture = fixture();
    let target = Target::from_env();
    let (medication_id, _) = create_medication(
        &target,
        &fixture,
        "Contract tracked removal feed medicine",
        "6.00",
    );
    let option_id = tracked_option(&fixture, medication_id, "6.00");
    let medication_portable = portable_id("medications", medication_id);
    let option_portable = portable_id("dosages", option_id);
    let path = format!(
        "{}/{medication_id}/stock_removals",
        medications_path(&fixture)
    );
    let payload = json!({"stock_removal": {
        "quantity": "1.25", "reason": "dropped", "dosage_id": option_id.to_string(),
        "submission_id": "d2000000-0000-4000-8000-000000000001"
    }});
    let response = target.post_json_authorized(&path, &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 201);
    let request = request_id(&response);
    assert_event(
        &fixture,
        "MedicationDosageOption",
        option_id,
        &option_portable,
        "update",
        &request,
        None,
    );
    assert_event(
        &fixture,
        "Medication",
        medication_id,
        &medication_portable,
        "update",
        &request,
        None,
    );
    assert_eq!(dosage_stock(option_id), "4.75");
    assert_eq!(stock(medication_id), "4.75");
    let option_before = event_count("MedicationDosageOption", option_id);
    let medication_before = event_count("Medication", medication_id);
    let response = target.post_json_authorized(&path, &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 201);
    assert_eq!(
        event_count("MedicationDosageOption", option_id),
        option_before
    );
    assert_eq!(event_count("Medication", medication_id), medication_before);
    let changed = json!({"stock_removal": {
        "quantity": "2.00", "reason": "dropped", "dosage_id": option_id.to_string(),
        "submission_id": "d2000000-0000-4000-8000-000000000001"
    }});
    let response = target.post_json_authorized(&path, &fixture.access_token, &changed);
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(
        event_count("MedicationDosageOption", option_id),
        option_before
    );
    assert_eq!(event_count("Medication", medication_id), medication_before);
}

#[test]
fn tracked_dose_emits_take_person_metadata_and_stock_updates_once() {
    let fixture = fixture();
    let target = Target::from_env();
    let (medication_id, _) = create_medication(
        &target,
        &fixture,
        "Contract tracked take feed medicine",
        "6.00",
    );
    let option_id = tracked_option(&fixture, medication_id, "6.00");
    let medication_portable = portable_id("medications", medication_id);
    let option_portable = portable_id("dosages", option_id);
    let mut db = database();
    let source = db
        .query_one(
            "INSERT INTO person_medications (household_id, person_id, medication_id, source_dosage_option_id, dose_amount, dose_unit, position, created_at, updated_at) VALUES ($1, $2, $3, $4, '1.25', 'ml', (SELECT COALESCE(MAX(position), 0) + 1 FROM person_medications WHERE person_id = $2), NOW(), NOW()) RETURNING portable_id",
            &[&fixture.household_id, &fixture.managed_person_id, &medication_id, &option_id],
        )
        .expect("tracked direct source");
    let source_portable: String = source.get(0);
    let person_portable: String = db
        .query_one(
            "SELECT portable_id FROM people WHERE id = $1",
            &[&fixture.managed_person_id],
        )
        .expect("source person")
        .get(0);
    let taken_at = OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .unwrap()
        .format(&Rfc3339)
        .unwrap();
    let payload = json!({"medication_take": {
        "client_uuid": "d3000000-0000-4000-8000-000000000001",
        "source_type": "person_medication",
        "source_id": source_portable,
        "taken_at": taken_at,
        "dose_amount": "1.25",
        "taken_from_medication_id": medication_id
    }});
    let path = format!(
        "/api/v1/households/{}/medication_takes",
        fixture.household_id
    );
    let response = target.post_json_authorized(&path, &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 201);
    let request = request_id(&response);
    let take = body(response)["data"].clone();
    let take_id = take["id"].as_i64().unwrap();
    let take_portable = portable_id("medication_takes", take_id);
    assert_event(
        &fixture,
        "MedicationTake",
        take_id,
        &take_portable,
        "create",
        &request,
        Some(&person_portable),
    );
    assert_event(
        &fixture,
        "MedicationDosageOption",
        option_id,
        &option_portable,
        "update",
        &request,
        None,
    );
    assert_event(
        &fixture,
        "Medication",
        medication_id,
        &medication_portable,
        "update",
        &request,
        None,
    );
    assert_eq!(dosage_stock(option_id), "4.75");
    assert_eq!(stock(medication_id), "4.75");
    let take_before = event_count("MedicationTake", take_id);
    let option_before = event_count("MedicationDosageOption", option_id);
    let medication_before = event_count("Medication", medication_id);
    let response = target.post_json_authorized(&path, &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(body(response)["data"]["id"], take_id);
    assert_eq!(event_count("MedicationTake", take_id), take_before);
    assert_eq!(
        event_count("MedicationDosageOption", option_id),
        option_before
    );
    assert_eq!(event_count("Medication", medication_id), medication_before);
}
