use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::Response;
use serde_json::Value;
use std::env;

struct TemporaryGrantLevel {
    db: postgres::Client,
    id: i64,
    original: String,
}

impl TemporaryGrantLevel {
    fn record_for_viewer(fixture: &Fixture) -> Self {
        let mut db = database();
        let row = db
            .query_one(
                "SELECT id, access_level FROM person_access_grants WHERE household_membership_id = $1 AND person_id = $2 AND revoked_at IS NULL",
                &[&fixture.view_membership_id, &fixture.managed_person_id],
            )
            .expect("view-only grant");
        let id = row.get(0);
        let original: String = row.get(1);
        assert_eq!(original, "view");
        let mut guard = Self { db, id, original };
        guard
            .db
            .execute(
                "UPDATE person_access_grants SET access_level = 'record' WHERE id = $1",
                &[&guard.id],
            )
            .expect("temporary record grant");
        guard
    }
}

impl Drop for TemporaryGrantLevel {
    fn drop(&mut self) {
        let _ = self.db.execute(
            "UPDATE person_access_grants SET access_level = $1 WHERE id = $2",
            &[&self.original, &self.id],
        );
    }
}

struct TemporaryMedication {
    db: postgres::Client,
    id: i64,
    original_name: String,
    original_amount: Option<String>,
    original_unit: String,
    original_supply: Option<String>,
    original_location_id: i64,
}

impl TemporaryMedication {
    fn new(id: i64) -> Self {
        let mut db = database();
        let row = db
            .query_one(
                "SELECT name, dose_amount::text, dose_unit, current_supply::text, location_id FROM medications WHERE id = $1",
                &[&id],
            )
            .expect("temporary stock candidate");
        Self {
            db,
            id,
            original_name: row.get(0),
            original_amount: row.get(1),
            original_unit: row.get(2),
            original_supply: row.get(3),
            original_location_id: row.get(4),
        }
    }

    fn set(&mut self, name: &str, amount: &str, unit: &str, supply: &str) {
        self.db
            .execute(
                "UPDATE medications SET name = $1, dose_amount = $2::text::numeric, dose_unit = $3, current_supply = $4::text::numeric WHERE id = $5",
                &[&name, &amount, &unit, &supply, &self.id],
            )
            .expect("change temporary stock candidate");
    }

    fn set_location(&mut self, location_id: i64) {
        self.db
            .execute(
                "UPDATE medications SET location_id = $1 WHERE id = $2",
                &[&location_id, &self.id],
            )
            .expect("change temporary stock location");
    }
}

impl Drop for TemporaryMedication {
    fn drop(&mut self) {
        let _ = self.db.execute(
            "UPDATE medications SET name = $1, dose_amount = $2::text::numeric, dose_unit = $3, current_supply = $4::text::numeric, location_id = $5 WHERE id = $6",
            &[&self.original_name, &self.original_amount, &self.original_unit, &self.original_supply, &self.original_location_id, &self.id],
        );
    }
}

struct TemporaryTrackedSupply {
    db: postgres::Client,
    medication_id: i64,
    dosage_id: i64,
    original_medication: Option<String>,
    original_dosage: Option<String>,
}

impl TemporaryTrackedSupply {
    fn new(medication_id: i64, dosage_id: i64) -> Self {
        let mut db = database();
        let original_medication = db
            .query_one(
                "SELECT current_supply::text FROM medications WHERE id = $1",
                &[&medication_id],
            )
            .expect("tracked medication")
            .get(0);
        let original_dosage = db
            .query_one(
                "SELECT current_supply::text FROM dosages WHERE id = $1",
                &[&dosage_id],
            )
            .expect("tracked dosage")
            .get(0);
        Self {
            db,
            medication_id,
            dosage_id,
            original_medication,
            original_dosage,
        }
    }

    fn set(&mut self, supply: &str) {
        self.db
            .execute(
                "UPDATE dosages SET current_supply = $1::text::numeric WHERE id = $2",
                &[&supply, &self.dosage_id],
            )
            .expect("change tracked dosage supply");
        self.db
            .execute(
                "UPDATE medications SET current_supply = $1::text::numeric WHERE id = $2",
                &[&supply, &self.medication_id],
            )
            .expect("change tracked medication supply");
    }
}

impl Drop for TemporaryTrackedSupply {
    fn drop(&mut self) {
        let _ = self.db.execute(
            "UPDATE dosages SET current_supply = $1::text::numeric WHERE id = $2",
            &[&self.original_dosage, &self.dosage_id],
        );
        let _ = self.db.execute(
            "UPDATE medications SET current_supply = $1::text::numeric WHERE id = $2",
            &[&self.original_medication, &self.medication_id],
        );
    }
}

fn database() -> postgres::Client {
    let url = env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract database URL");
    postgres::Client::connect(&url, postgres::NoTls).expect("contract database")
}

fn collection(fixture: &Fixture, kind: &str) -> String {
    format!("/api/v1/households/{}/{kind}", fixture.household_id)
}

fn body(response: Response) -> Value {
    let status = response.status().as_u16();
    let text = response.text().expect("HTTP body");
    assert_eq!(status, 200, "{text}");
    serde_json::from_str(&text).expect("JSON response")
}

fn source(target: &Target, fixture: &Fixture, kind: &str, id: i64, token: &str) -> Value {
    let path = format!("{}/{id}", collection(fixture, kind));
    body(target.get(&path, Some(token)))["data"].clone()
}

fn source_in_collection(
    target: &Target,
    fixture: &Fixture,
    kind: &str,
    id: i64,
    token: &str,
) -> Value {
    let collection = body(target.get(&collection(fixture, kind), Some(token)));
    collection["data"]
        .as_array()
        .expect("source collection")
        .iter()
        .find(|row| row["id"] == id)
        .expect("visible source")
        .clone()
}

fn eligible_ids(row: &Value) -> Vec<i64> {
    row["eligible_stock_medication_ids"]
        .as_array()
        .expect("source stock eligibility")
        .iter()
        .map(|id| id.as_i64().expect("medication id"))
        .collect()
}

fn assignment_id(portable_id: &str) -> i64 {
    database()
        .query_one(
            "SELECT id FROM person_medications WHERE portable_id::text = $1",
            &[&portable_id],
        )
        .expect("seeded assignment")
        .get(0)
}

fn location_id(portable_id: &str) -> i64 {
    database()
        .query_one(
            "SELECT id FROM locations WHERE portable_id::text = $1",
            &[&portable_id],
        )
        .expect("seeded location")
        .get(0)
}

#[test]
fn source_list_and_detail_separate_record_from_manage_permission() {
    let fixture = fixture();
    let target = Target::from_env();
    for (kind, id, medication_id) in [
        (
            "schedules",
            fixture.dose_write_schedule_id,
            fixture.dose_write_schedule_medication_id,
        ),
        (
            "person_medications",
            fixture.dose_write_assignment_id,
            fixture.dose_write_medication_id,
        ),
    ] {
        let owner_list = source_in_collection(&target, &fixture, kind, id, &fixture.access_token);
        let owner_detail = source(&target, &fixture, kind, id, &fixture.access_token);
        assert_eq!(owner_list, owner_detail);
        assert_eq!(owner_list["can_record"], true);
        assert_eq!(eligible_ids(&owner_list), vec![medication_id]);

        let viewer_list =
            source_in_collection(&target, &fixture, kind, id, &fixture.view_access_token);
        let viewer_detail = source(&target, &fixture, kind, id, &fixture.view_access_token);
        assert_eq!(viewer_list, viewer_detail);
        assert_eq!(viewer_list["can_manage"], false);
        assert_eq!(viewer_list["can_record"], false);
        assert!(eligible_ids(&viewer_list).is_empty());
    }

    let _record_grant = TemporaryGrantLevel::record_for_viewer(&fixture);
    for (kind, id, medication_id) in [
        (
            "schedules",
            fixture.dose_write_schedule_id,
            fixture.dose_write_schedule_medication_id,
        ),
        (
            "person_medications",
            fixture.dose_write_assignment_id,
            fixture.dose_write_medication_id,
        ),
    ] {
        let row = source(&target, &fixture, kind, id, &fixture.view_access_token);
        assert_eq!(row["can_manage"], false);
        assert_eq!(row["can_record"], true);
        assert_eq!(eligible_ids(&row), vec![medication_id]);
    }

    let paused_id = assignment_id(&fixture.fhir_stopped_assignment_portable_id);
    let paused = source(
        &target,
        &fixture,
        "person_medications",
        paused_id,
        &fixture.access_token,
    );
    assert_eq!(paused["paused"], true);
    assert_eq!(paused["can_record"], true);
    assert!(eligible_ids(&paused).is_empty());
}

#[test]
fn eligible_stock_uses_source_signature_current_supply_and_selected_tracked_dosage() {
    let fixture = fixture();
    let target = Target::from_env();
    let assignment = "person_medications";
    let source_id = fixture.dose_write_assignment_id;
    let original = source(
        &target,
        &fixture,
        assignment,
        source_id,
        &fixture.access_token,
    );
    assert_eq!(
        eligible_ids(&original),
        vec![fixture.dose_write_medication_id]
    );

    let source_name: String = database()
        .query_one(
            "SELECT name FROM medications WHERE id = $1",
            &[&fixture.dose_write_medication_id],
        )
        .expect("source medication")
        .get(0);
    {
        let mut hidden = TemporaryMedication::new(fixture.hidden_medication_id);
        hidden.set(&source_name, "1.25", "ml", "5.00");
        let owner = source(
            &target,
            &fixture,
            assignment,
            source_id,
            &fixture.access_token,
        );
        assert_eq!(
            eligible_ids(&owner),
            vec![
                fixture.hidden_medication_id,
                fixture.dose_write_medication_id
            ]
        );
        let _record_grant = TemporaryGrantLevel::record_for_viewer(&fixture);
        let delegated = source(
            &target,
            &fixture,
            assignment,
            source_id,
            &fixture.view_access_token,
        );
        assert_eq!(
            eligible_ids(&delegated),
            vec![fixture.dose_write_medication_id]
        );
    }

    let mut alternate = TemporaryMedication::new(fixture.dose_write_other_medication_id);
    alternate.set(&source_name, "1.25", "ml", "5.00");
    let matching = source(
        &target,
        &fixture,
        assignment,
        source_id,
        &fixture.access_token,
    );
    assert_eq!(
        eligible_ids(&matching),
        vec![
            fixture.dose_write_medication_id,
            fixture.dose_write_other_medication_id
        ]
    );
    alternate.set_location(location_id(&fixture.hidden_location_portable_id));
    let by_location = source(
        &target,
        &fixture,
        assignment,
        source_id,
        &fixture.access_token,
    );
    assert_eq!(
        eligible_ids(&by_location),
        vec![
            fixture.dose_write_other_medication_id,
            fixture.dose_write_medication_id
        ]
    );
    alternate.set_location(fixture.primary_location_id);
    alternate.set(&source_name, "2.00", "ml", "5.00");
    let wrong_dose = source(
        &target,
        &fixture,
        assignment,
        source_id,
        &fixture.access_token,
    );
    assert_eq!(
        eligible_ids(&wrong_dose),
        vec![fixture.dose_write_medication_id]
    );
    alternate.set(&source_name, "1.25", "tablet", "5.00");
    let wrong_unit = source(
        &target,
        &fixture,
        assignment,
        source_id,
        &fixture.access_token,
    );
    assert_eq!(
        eligible_ids(&wrong_unit),
        vec![fixture.dose_write_medication_id]
    );
    alternate.set(&source_name, "1.25", "ml", "0.00");
    let depleted = source(
        &target,
        &fixture,
        assignment,
        source_id,
        &fixture.access_token,
    );
    assert_eq!(
        eligible_ids(&depleted),
        vec![fixture.dose_write_medication_id]
    );
    alternate.set(&source_name, "1.25", "ml", "1.00");
    let insufficient = source(
        &target,
        &fixture,
        assignment,
        source_id,
        &fixture.access_token,
    );
    assert_eq!(
        eligible_ids(&insufficient),
        vec![fixture.dose_write_medication_id]
    );

    let tracked_assignment_id = assignment_id(&fixture.dose_write_tracked_assignment_portable_id);
    let tracked = source(
        &target,
        &fixture,
        assignment,
        tracked_assignment_id,
        &fixture.access_token,
    );
    assert_eq!(
        eligible_ids(&tracked),
        vec![fixture.dose_write_tracked_medication_id]
    );
    let mut tracked_supply = TemporaryTrackedSupply::new(
        fixture.dose_write_tracked_medication_id,
        fixture.dose_write_tracked_option_id,
    );
    tracked_supply.set("1.00");
    let insufficient_tracked = source(
        &target,
        &fixture,
        assignment,
        tracked_assignment_id,
        &fixture.access_token,
    );
    assert!(eligible_ids(&insufficient_tracked).is_empty());
}
