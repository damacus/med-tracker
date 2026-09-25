use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::Response;
use serde_json::{json, Value};
use std::env;
use std::sync::{Arc, Barrier};
use std::thread;

fn body(response: Response) -> Value {
    response.json().expect("JSON response")
}

fn database() -> postgres::Client {
    let url = env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract audit database URL");
    postgres::Client::connect(&url, postgres::NoTls).expect("contract database")
}

fn medications_path(fixture: &Fixture) -> String {
    format!("/api/v1/households/{}/medications", fixture.household_id)
}

fn create_medication(target: &Target, fixture: &Fixture, name: &str, supply: &str) -> i64 {
    let response = target.post_json_authorized(
        &medications_path(fixture),
        &fixture.access_token,
        &json!({"medication": {
            "name": name,
            "location_id": fixture.primary_location_id,
            "dose_amount": "1.25",
            "dose_unit": "ml",
            "current_supply": supply,
            "reorder_threshold": "0"
        }}),
    );
    assert_eq!(response.status().as_u16(), 201);
    body(response)["data"]["id"]
        .as_i64()
        .expect("medication ID")
}

fn removal_path(fixture: &Fixture, medication_id: i64) -> String {
    format!(
        "{}/{medication_id}/stock_removals",
        medications_path(fixture)
    )
}

fn removal_payload(quantity: &str, submission_id: &str, dosage_id: Option<i64>) -> Value {
    let mut payload = json!({"stock_removal": {
        "quantity": quantity,
        "reason": "dropped",
        "submission_id": submission_id
    }});
    if let Some(id) = dosage_id {
        payload["stock_removal"]["dosage_id"] = json!(id.to_string());
    }
    payload
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

fn versions(item_type: &str, item_id: i64, event: &str) -> i64 {
    database()
        .query_one(
            "SELECT count(*) FROM versions WHERE item_type = $1 AND item_id = $2 AND event = $3",
            &[&item_type, &item_id, &event],
        )
        .expect("domain versions")
        .get(0)
}

fn removal_versions(medication_id: i64) -> i64 {
    database()
        .query_one(
            "SELECT count(*) FROM versions WHERE item_type = 'MedicationStockRemoval' AND item_id = $1 AND event = 'stock_removal'",
            &[&medication_id],
        )
        .expect("stock removal versions")
        .get(0)
}

fn sync_updates(record_type: &str, record_id: i64) -> i64 {
    database()
        .query_one(
            "SELECT count(*) FROM api_change_events WHERE record_type = $1 AND record_id = $2 AND action = 'update'",
            &[&record_type, &record_id],
        )
        .expect("sync update events")
        .get(0)
}

struct MembershipRoleGuard {
    db: postgres::Client,
    membership_id: i64,
    original: String,
}

impl MembershipRoleGuard {
    fn set(membership_id: i64, role: &str) -> Self {
        let mut db = database();
        let original = db
            .query_one(
                "SELECT role FROM household_memberships WHERE id = $1",
                &[&membership_id],
            )
            .expect("membership role")
            .get(0);
        let mut guard = Self {
            db,
            membership_id,
            original,
        };
        guard
            .db
            .execute(
                "UPDATE household_memberships SET role = $1 WHERE id = $2",
                &[&role, &membership_id],
            )
            .expect("temporary role");
        guard
    }
}

impl Drop for MembershipRoleGuard {
    fn drop(&mut self) {
        let _ = self.db.execute(
            "UPDATE household_memberships SET role = $1 WHERE id = $2",
            &[&self.original, &self.membership_id],
        );
    }
}

#[test]
fn tracked_removal_requires_own_option_and_syncs_parent_supply() {
    let target = Target::from_env();
    let fixture = fixture();
    let medication_id = create_medication(&target, &fixture, "Contract tracked removal", "10.00");
    let mut db = database();
    let insert = "INSERT INTO dosages (household_id, medication_id, amount, unit, frequency, current_supply, created_at, updated_at) VALUES ($1, $2, '1.25', 'ml', 'daily', $3::text::numeric, NOW(), NOW()) RETURNING id";
    let first_id: i64 = db
        .query_one(insert, &[&fixture.household_id, &medication_id, &"6.00"])
        .expect("first tracked option")
        .get(0);
    let second_id: i64 = db
        .query_one(insert, &[&fixture.household_id, &medication_id, &"4.00"])
        .expect("second tracked option")
        .get(0);
    db.execute(
        "UPDATE medications SET dose_amount = NULL WHERE id = $1",
        &[&medication_id],
    )
    .expect("tracked dosage mode");
    let foreign_id: i64 = db
        .query_one(
            "SELECT id FROM dosages WHERE medication_id <> $1 ORDER BY id LIMIT 1",
            &[&medication_id],
        )
        .expect("unrelated dosage option")
        .get(0);
    let path = removal_path(&fixture, medication_id);
    let option_events_before = sync_updates("MedicationDosageOption", first_id);
    let medication_events_before = sync_updates("Medication", medication_id);

    let missing = removal_payload("1.25", "b1000000-0000-4000-8000-000000000001", None);
    let response = target.post_json_authorized(&path, &fixture.access_token, &missing);
    assert_eq!(response.status().as_u16(), 422);
    let foreign = removal_payload(
        "1.25",
        "b1000000-0000-4000-8000-000000000002",
        Some(foreign_id),
    );
    let response = target.post_json_authorized(&path, &fixture.access_token, &foreign);
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(stock(medication_id), "10.00");
    assert_eq!(dosage_stock(first_id), "6.00");
    assert_eq!(dosage_stock(second_id), "4.00");
    assert_eq!(removal_versions(medication_id), 0);
    assert_eq!(
        sync_updates("MedicationDosageOption", first_id),
        option_events_before
    );
    assert_eq!(
        sync_updates("Medication", medication_id),
        medication_events_before
    );

    let valid = removal_payload(
        "1.25",
        "b1000000-0000-4000-8000-000000000003",
        Some(first_id),
    );
    let response = target.post_json_authorized(&path, &fixture.access_token, &valid);
    assert_eq!(response.status().as_u16(), 201);
    let result = body(response);
    assert_eq!(result["data"]["dosage_id"], first_id.to_string());
    assert_eq!(result["data"]["remaining_quantity"], "4.75");
    assert_eq!(dosage_stock(first_id), "4.75");
    assert_eq!(dosage_stock(second_id), "4.00");
    assert_eq!(stock(medication_id), "8.75");
    assert_eq!(removal_versions(medication_id), 1);
    assert_eq!(
        sync_updates("MedicationDosageOption", first_id),
        option_events_before + 1
    );
    assert_eq!(
        sync_updates("Medication", medication_id),
        medication_events_before + 1
    );
}

#[test]
fn contending_stock_removals_are_idempotent_and_never_overdraw() {
    let target = Target::from_env();
    let fixture = fixture();
    let medication_id = create_medication(&target, &fixture, "Contract duplicate removal", "2.50");
    let path = removal_path(&fixture, medication_id);
    let payload = removal_payload("1.25", "b2000000-0000-4000-8000-000000000001", None);
    let barrier = Arc::new(Barrier::new(3));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            let path = path.clone();
            let payload = payload.clone();
            let token = fixture.access_token.clone();
            thread::spawn(move || {
                let target = Target::from_env();
                barrier.wait();
                let response = target.post_json_authorized(&path, &token, &payload);
                (response.status().as_u16(), body(response))
            })
        })
        .collect();
    barrier.wait();
    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert!(results.iter().all(|result| result.0 == 201), "{results:?}");
    assert_eq!(results[0].1["data"], results[1].1["data"]);
    assert_eq!(stock(medication_id), "1.25");
    assert_eq!(removal_versions(medication_id), 1);
    let changed = removal_payload("1", "b2000000-0000-4000-8000-000000000001", None);
    let response = target.post_json_authorized(&path, &fixture.access_token, &changed);
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(stock(medication_id), "1.25");
    assert_eq!(removal_versions(medication_id), 1);

    let limited_id = create_medication(&target, &fixture, "Contract contended removal", "1.50");
    let limited_path = removal_path(&fixture, limited_id);
    let barrier = Arc::new(Barrier::new(3));
    let handles: Vec<_> = (1..=2)
        .map(|index| {
            let barrier = Arc::clone(&barrier);
            let path = limited_path.clone();
            let token = fixture.access_token.clone();
            thread::spawn(move || {
                let target = Target::from_env();
                let submission_id = format!("b2000000-0000-4000-8000-00000000000{index}");
                let payload = removal_payload("1.25", &submission_id, None);
                barrier.wait();
                target
                    .post_json_authorized(&path, &token, &payload)
                    .status()
                    .as_u16()
            })
        })
        .collect();
    barrier.wait();
    let mut statuses: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    statuses.sort_unstable();
    assert_eq!(statuses, [201, 422]);
    assert_eq!(stock(limited_id), "0.25");
    assert_eq!(removal_versions(limited_id), 1);
}

#[test]
fn replay_rechecks_current_household_manager_permission() {
    let target = Target::from_env();
    let fixture = fixture();
    let medication_id = create_medication(&target, &fixture, "Contract revoked replay", "5.00");
    let path = removal_path(&fixture, medication_id);
    let payload = removal_payload("1.25", "b3000000-0000-4000-8000-000000000001", None);
    let response = target.post_json_authorized(&path, &fixture.access_token, &payload);
    assert_eq!(response.status().as_u16(), 201);
    let original = body(response)["data"].clone();
    assert_eq!(stock(medication_id), "3.75");
    assert_eq!(removal_versions(medication_id), 1);

    let _role = MembershipRoleGuard::set(fixture.owner_membership_id, "member");
    let response = target.post_json_authorized(&path, &fixture.access_token, &payload);
    assert!(matches!(response.status().as_u16(), 401 | 403 | 404));
    let denial = body(response);
    assert_ne!(denial["data"], original);
    assert_eq!(stock(medication_id), "3.75");
    assert_eq!(removal_versions(medication_id), 1);
}

#[test]
fn concurrent_same_etag_patch_has_one_update_and_one_conflict() {
    let target = Target::from_env();
    let fixture = fixture();
    let medication_id = create_medication(&target, &fixture, "Contract contended edit", "5.00");
    let path = format!("{}/{medication_id}", medications_path(&fixture));
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let initial_etag = response.headers()["etag"].to_str().unwrap().to_owned();
    let before = versions("Medication", medication_id, "api_update");
    let barrier = Arc::new(Barrier::new(3));
    let handles: Vec<_> = (1..=2)
        .map(|index| {
            let barrier = Arc::clone(&barrier);
            let path = path.clone();
            let token = fixture.access_token.clone();
            let etag = initial_etag.clone();
            thread::spawn(move || {
                let target = Target::from_env();
                barrier.wait();
                let response = target.patch_json_if_match(
                    &path,
                    &token,
                    &json!({"medication": {"friendly_name": format!("Contender {index}")}}),
                    &etag,
                );
                (response.status().as_u16(), response)
            })
        })
        .collect();
    barrier.wait();
    let mut results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    results.sort_by_key(|result| result.0);
    assert_eq!([results[0].0, results[1].0], [200, 409]);
    let winner = results.remove(0).1;
    let winner_etag = winner.headers()["etag"].to_str().unwrap().to_owned();
    let winner_name = body(winner)["data"]["display_name"].clone();
    assert!(winner_name == "Contender 1" || winner_name == "Contender 2");
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(response.headers()["etag"], winner_etag);
    assert_eq!(body(response)["data"]["display_name"], winner_name);
    assert_eq!(
        versions("Medication", medication_id, "api_update"),
        before + 1
    );
}

#[test]
fn audit_log_list_requires_current_household_manager_role() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = format!(
        "/api/v1/households/{}/admin/audit_logs",
        fixture.household_id
    );
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert!(body(response)["data"].is_array());

    for token in [&fixture.view_access_token, &fixture.delegated_access_token] {
        let response = target.get(&path, Some(token));
        assert_eq!(response.status().as_u16(), 403);
        let denial = body(response);
        assert!(denial["data"].is_null());
    }
    let foreign = format!(
        "/api/v1/households/{}/admin/audit_logs",
        fixture.foreign_household_id
    );
    let response = target.get(&foreign, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 403);
    assert!(body(response)["data"].is_null());

    let _role = MembershipRoleGuard::set(fixture.view_membership_id, "administrator");
    let response = target.get(&path, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert!(body(response)["data"].is_array());
}
