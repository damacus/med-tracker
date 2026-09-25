use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::Response;
use serde_json::{json, Value};
use std::env;

fn body(response: Response) -> Value {
    response.json().expect("JSON response")
}

fn database() -> postgres::Client {
    let url = env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract audit database URL");
    postgres::Client::connect(&url, postgres::NoTls).expect("contract database")
}

fn medication_path(fixture: &Fixture, medication_id: i64) -> String {
    format!(
        "/api/v1/households/{}/medications/{medication_id}",
        fixture.household_id
    )
}

fn create_multi_dose_medication(target: &Target, fixture: &Fixture, name: &str) -> i64 {
    let response = target.post_json_authorized(
        &format!("/api/v1/households/{}/medications", fixture.household_id),
        &fixture.access_token,
        &json!({"medication": {
            "name": name,
            "location_id": fixture.primary_location_id,
            "current_supply": "10.00",
            "reorder_threshold": "1.00"
        }}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let medication_id = body(response)["data"]["id"].as_i64().unwrap();
    let mut db = database();
    for amount in ["5", "10"] {
        db.execute(
            "INSERT INTO dosages (household_id, medication_id, amount, unit, frequency, created_at, updated_at) VALUES ($1, $2, $3::text::numeric, 'mg', 'daily', NOW(), NOW())",
            &[&fixture.household_id, &medication_id, &amount],
        )
        .expect("multi-dose option");
    }
    medication_id
}

fn first_dosage(medication_id: i64) -> (i64, String) {
    let row = database()
        .query_one(
            "SELECT id, portable_id FROM dosages WHERE medication_id = $1 ORDER BY id LIMIT 1",
            &[&medication_id],
        )
        .expect("first dosage option");
    (row.get(0), row.get(1))
}

fn dosage_portable_ids(medication_id: i64) -> Vec<String> {
    database()
        .query(
            "SELECT portable_id FROM dosages WHERE medication_id = $1 ORDER BY id",
            &[&medication_id],
        )
        .expect("medication dosages")
        .into_iter()
        .map(|row| row.get(0))
        .collect()
}

fn medication_versions(medication_id: i64) -> i64 {
    database()
        .query_one(
            "SELECT count(*) FROM versions WHERE item_type = 'Medication' AND item_id = $1 AND event = 'api_update'",
            &[&medication_id],
        )
        .expect("medication update versions")
        .get(0)
}

fn tombstones(household_id: i64, portable_ids: &[String]) -> i64 {
    database()
        .query_one(
            "SELECT count(*) FROM api_tombstones WHERE household_id = $1 AND record_type = 'MedicationDosageOption' AND record_portable_id = ANY($2)",
            &[&household_id, &portable_ids],
        )
        .expect("dosage tombstones")
        .get(0)
}

fn source_changes(source_id: i64) -> i64 {
    database()
        .query_one(
            "SELECT count(*) FROM api_change_events WHERE record_type = 'PersonMedication' AND record_id = $1 AND action = 'update'",
            &[&source_id],
        )
        .expect("source sync changes")
        .get(0)
}

fn medication_changes(medication_id: i64) -> i64 {
    database()
        .query_one(
            "SELECT count(*) FROM api_change_events WHERE record_type = 'Medication' AND record_id = $1 AND action = 'update'",
            &[&medication_id],
        )
        .expect("medication sync changes")
        .get(0)
}

fn etag(target: &Target, fixture: &Fixture, medication_id: i64) -> String {
    let response = target.get(
        &medication_path(fixture, medication_id),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    response.headers()["etag"].to_str().unwrap().to_owned()
}

#[test]
fn switching_to_single_dose_clears_options_and_preserves_direct_source() {
    let target = Target::from_env();
    let fixture = fixture();
    let medication_id = create_multi_dose_medication(
        &target,
        &fixture,
        "Contract successful dose mode transition",
    );
    let (dosage_id, _) = first_dosage(medication_id);
    let options = dosage_portable_ids(medication_id);
    assert_eq!(options.len(), 2);
    let mut db = database();
    let source_id: i64 = db
        .query_one(
            "INSERT INTO person_medications (household_id, person_id, medication_id, source_dosage_option_id, dose_amount, dose_unit, position, created_at, updated_at) VALUES ($1, $2, $3, $4, '5', 'mg', (SELECT COALESCE(MAX(position), 0) + 1 FROM person_medications WHERE person_id = $2), NOW(), NOW()) RETURNING id",
            &[&fixture.household_id, &fixture.managed_person_id, &medication_id, &dosage_id],
        )
        .expect("direct source")
        .get(0);
    let before_changes = source_changes(source_id);
    let before_medication_changes = medication_changes(medication_id);
    let before_versions = medication_versions(medication_id);
    let before_tombstones = tombstones(fixture.household_id, &options);
    let response = target.patch_json_if_match(
        &medication_path(&fixture, medication_id),
        &fixture.access_token,
        &json!({"medication": {"dose_amount": "500", "dose_unit": "mg"}}),
        &etag(&target, &fixture, medication_id),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(body(response)["data"]["dose_unit"], "mg");
    let row = db
        .query_one(
            "SELECT dose_amount, dose_unit FROM medications WHERE id = $1",
            &[&medication_id],
        )
        .expect("single dose state");
    assert_eq!(row.get::<_, f64>(0), 500.0);
    assert_eq!(row.get::<_, String>(1), "mg");
    assert!(dosage_portable_ids(medication_id).is_empty());
    let source = db
        .query_one(
            "SELECT source_dosage_option_id, medication_id FROM person_medications WHERE id = $1",
            &[&source_id],
        )
        .expect("preserved direct source");
    assert_eq!(source.get::<_, Option<i64>>(0), None);
    assert_eq!(source.get::<_, i64>(1), medication_id);
    assert_eq!(
        tombstones(fixture.household_id, &options),
        before_tombstones + 2
    );
    assert_eq!(source_changes(source_id), before_changes + 1);
    assert_eq!(
        medication_changes(medication_id),
        before_medication_changes + 1
    );
    assert_eq!(medication_versions(medication_id), before_versions + 1);
}

#[test]
fn linked_schedule_blocks_single_dose_switch_without_sync_effects() {
    let target = Target::from_env();
    let fixture = fixture();
    let medication_id =
        create_multi_dose_medication(&target, &fixture, "Contract blocked dose mode transition");
    let (dosage_id, _) = first_dosage(medication_id);
    let options = dosage_portable_ids(medication_id);
    let mut db = database();
    let schedule_id: i64 = db
        .query_one(
            "INSERT INTO schedules (household_id, person_id, medication_id, source_dosage_option_id, dose_amount, dose_unit, frequency, start_date, created_at, updated_at) VALUES ($1, $2, $3, $4, '5', 'mg', 'Once daily', CURRENT_DATE, NOW(), NOW()) RETURNING id",
            &[&fixture.household_id, &fixture.managed_person_id, &medication_id, &dosage_id],
        )
        .expect("schedule using dosage")
        .get(0);
    let before_versions = medication_versions(medication_id);
    let before_medication_changes = medication_changes(medication_id);
    let before_tombstones = tombstones(fixture.household_id, &options);
    let response = target.patch_json_if_match(
        &medication_path(&fixture, medication_id),
        &fixture.access_token,
        &json!({"medication": {"dose_amount": "500", "dose_unit": "mg"}}),
        &etag(&target, &fixture, medication_id),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert!(body(response)["error"]["errors"]["dose_amount"].is_array());
    let row = db
        .query_one(
            "SELECT dose_amount FROM medications WHERE id = $1",
            &[&medication_id],
        )
        .expect("unchanged dose mode");
    assert_eq!(row.get::<_, Option<f64>>(0), None);
    assert_eq!(dosage_portable_ids(medication_id), options);
    let schedule = db
        .query_one(
            "SELECT source_dosage_option_id FROM schedules WHERE id = $1",
            &[&schedule_id],
        )
        .expect("unchanged schedule source");
    assert_eq!(schedule.get::<_, i64>(0), dosage_id);
    assert_eq!(
        tombstones(fixture.household_id, &options),
        before_tombstones
    );
    assert_eq!(medication_versions(medication_id), before_versions);
    assert_eq!(medication_changes(medication_id), before_medication_changes);

    let response = target.patch_json_if_match(
        &medication_path(&fixture, medication_id),
        &fixture.access_token,
        &json!({"medication": {"dose_amount": null}}),
        &etag(&target, &fixture, medication_id),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert!(body(response)["data"]["dose_amount"].is_null());
    assert_eq!(dosage_portable_ids(medication_id), options);
    assert_eq!(
        tombstones(fixture.household_id, &options),
        before_tombstones
    );
    assert_eq!(medication_versions(medication_id), before_versions);
    assert_eq!(medication_changes(medication_id), before_medication_changes);
}
