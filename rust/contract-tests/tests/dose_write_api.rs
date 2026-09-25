use medtracker_contract_tests::{fixture, Fixture, Target};
use serde_json::{json, Value};
use std::env;
use std::sync::{Arc, Barrier};
use std::thread;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

fn path(household_id: i64) -> String {
    format!("/api/v1/households/{household_id}/medication_takes")
}

fn now() -> String {
    OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .unwrap()
        .format(&Rfc3339)
        .unwrap()
}

fn payload(source_type: &str, source_id: &str, medication_id: i64, uuid: &str) -> Value {
    json!({"medication_take": {
        "client_uuid": uuid,
        "source_type": source_type,
        "source_id": source_id,
        "taken_at": now(),
        "dose_amount": "1.25",
        "taken_from_medication_id": medication_id
    }})
}

fn database() -> postgres::Client {
    let url = env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract audit database URL");
    postgres::Client::connect(&url, postgres::NoTls).expect("contract audit database")
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

fn count_for_uuid(uuid: &str) -> i64 {
    database()
        .query_one(
            "SELECT count(*) FROM medication_takes WHERE client_uuid = $1",
            &[&uuid],
        )
        .expect("take count")
        .get(0)
}

fn take_id_for_portable_id(portable_id: &str) -> i64 {
    database()
        .query_one(
            "SELECT id FROM medication_takes WHERE portable_id::text = $1",
            &[&portable_id],
        )
        .expect("seeded take")
        .get(0)
}

fn take_version_count(id: i64) -> i64 {
    database()
        .query_one(
            "SELECT count(*) FROM versions WHERE item_type = 'MedicationTake' AND item_id = $1",
            &[&id],
        )
        .expect("take version count")
        .get(0)
}

fn assert_take_creation_version(id: i64, take: &Value, request_id: &str, fixture: &Fixture) {
    let row = database()
        .query_one(
            "SELECT row_to_json(versions)::text FROM versions WHERE item_type = 'MedicationTake' AND item_id = $1 AND event = 'create'",
            &[&id],
        )
        .expect("take creation version");
    let version: Value = serde_json::from_str(&row.get::<_, String>(0)).unwrap();
    assert_eq!(version["request_id"], request_id);
    assert_eq!(version["household_id"], fixture.household_id);
    assert_eq!(version["actor_membership_id"], fixture.owner_membership_id);
    assert_eq!(version["whodunnit"], fixture.user_id.to_string());
    assert_eq!(
        version["audit_context"]["actor_account_id"],
        fixture.account_id
    );
    assert_eq!(version["audit_context"]["actor_user_id"], fixture.user_id);
    assert_eq!(
        version["audit_context"]["actor_membership_id"],
        fixture.owner_membership_id
    );
    let snapshot: Value = serde_json::from_str(version["object"].as_str().unwrap()).unwrap();
    let changes: Value = serde_json::from_str(version["object_changes"].as_str().unwrap()).unwrap();
    for field in [
        "person_id",
        "medication_id",
        "person_medication_id",
        "taken_from_medication_id",
        "taken_from_location_id",
        "dose_amount",
        "dose_unit",
        "taken_at",
        "client_uuid",
    ] {
        assert_eq!(snapshot[field], take[field], "snapshot field {field}");
        assert_eq!(
            changes[field],
            json!([null, take[field]]),
            "change field {field}"
        );
    }
    assert!(!version.to_string().contains(&fixture.access_token));
}

fn all_take_version_count() -> i64 {
    database()
        .query_one(
            "SELECT count(*) FROM versions WHERE item_type = 'MedicationTake'",
            &[],
        )
        .expect("all take versions")
        .get(0)
}

fn take_change_count(id: i64) -> i64 {
    database()
        .query_one(
            "SELECT count(*) FROM api_change_events WHERE record_type = 'MedicationTake' AND record_id = $1",
            &[&id],
        )
        .expect("take change count")
        .get(0)
}

fn request_id(response: &reqwest::blocking::Response) -> String {
    response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_string()
}

fn assert_request_audit(
    id: &str,
    fixture: &Fixture,
    status: u16,
    actor_account_id: i64,
    actor_membership_id: i64,
) {
    let rows = database()
        .query(
            "SELECT row_to_json(security_audit_events)::text FROM security_audit_events WHERE request_id = $1",
            &[&id],
        )
        .expect("request audit");
    assert_eq!(rows.len(), 1, "one audit event for {id}");
    let event: Value = serde_json::from_str(&rows[0].get::<_, String>(0)).unwrap();
    assert_eq!(event["event_type"], "api.request");
    assert_eq!(event["household_id"], fixture.household_id);
    assert_eq!(event["actor_account_id"], actor_account_id);
    assert_eq!(event["actor_membership_id"], actor_membership_id);
    assert_eq!(event["metadata"]["http_method"], "POST");
    assert_eq!(event["metadata"]["action"], "create");
    assert_eq!(event["metadata"]["status"], status);
    assert!(!event.to_string().contains(&fixture.access_token));
}

fn history(target: &Target, fixture: &Fixture) -> Value {
    let response = target.get(&path(fixture.household_id), Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    response.json().expect("JSON dose history")
}

#[test]
fn authorized_household_can_list_direct_dose_history() {
    let fixture = fixture();
    let target = Target::from_env();
    let collection_path = path(fixture.household_id);
    let managed_take_id = take_id_for_portable_id(&fixture.managed_take_portable_id);
    let hidden_take_id = take_id_for_portable_id(&fixture.hidden_take_portable_id);
    let foreign_take_id = take_id_for_portable_id(&fixture.foreign_take_portable_id);
    let response = target.get(&collection_path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().expect("JSON dose history");
    let rows = body["data"].as_array().expect("dose rows");
    assert!(rows.iter().any(|row| row["id"] == managed_take_id));
    assert!(rows.iter().any(|row| row["id"] == hidden_take_id));
    assert!(!rows.iter().any(|row| row["id"] == foreign_take_id));
    let viewer = target.get(&collection_path, Some(&fixture.view_access_token));
    assert_eq!(viewer.status().as_u16(), 200);
    let viewer: Value = viewer.json().unwrap();
    assert!(viewer["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == managed_take_id));
    assert!(!viewer["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == hidden_take_id));
    assert!(body["meta"]["total_count"].as_u64().is_some());
    assert!(rows.len() >= 2);
    let first_page = target.get(
        &format!("{collection_path}?page=1&per_page=1"),
        Some(&fixture.access_token),
    );
    assert_eq!(first_page.status().as_u16(), 200);
    let first_page: Value = first_page.json().unwrap();
    let second_page = target.get(
        &format!("{collection_path}?page=2&per_page=1"),
        Some(&fixture.access_token),
    );
    assert_eq!(second_page.status().as_u16(), 200);
    let second_page: Value = second_page.json().unwrap();
    assert_eq!(first_page["data"][0]["id"], rows[0]["id"]);
    assert_eq!(second_page["data"][0]["id"], rows[1]["id"]);
    assert!(
        first_page["data"][0]["id"].as_i64().unwrap()
            < second_page["data"][0]["id"].as_i64().unwrap()
    );
    let repeated = target.get(&collection_path, Some(&fixture.access_token));
    assert_eq!(repeated.status().as_u16(), 200);
    let repeated: Value = repeated.json().unwrap();
    assert_eq!(repeated["data"], body["data"]);
    let paged = target.get(
        &format!("{collection_path}?page=0&per_page=500"),
        Some(&fixture.access_token),
    );
    assert_eq!(paged.status().as_u16(), 200);
    let paged: Value = paged.json().unwrap();
    assert_eq!(paged["meta"]["page"], 1);
    assert_eq!(paged["meta"]["per_page"], 100);
    let filtered = target.get(
        &format!("{collection_path}?updated_since=2000-01-01T00:00:00Z"),
        Some(&fixture.access_token),
    );
    assert_eq!(filtered.status().as_u16(), 200);
    let filtered: Value = filtered.json().unwrap();
    assert!(filtered["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == managed_take_id));
    let future = target.get(
        &format!("{collection_path}?updated_since=2100-01-01T00:00:00Z"),
        Some(&fixture.access_token),
    );
    assert_eq!(future.status().as_u16(), 200);
    let future: Value = future.json().unwrap();
    assert_eq!(future["meta"]["total_count"], 0);
    assert!(future["data"].as_array().unwrap().is_empty());
    let invalid = target.get(
        &format!("{collection_path}?updated_since=invalid"),
        Some(&fixture.access_token),
    );
    assert_eq!(invalid.status().as_u16(), 422);
    let invalid: Value = invalid.json().unwrap();
    assert_eq!(invalid["error"]["code"], "unprocessable_content");
    assert_eq!(
        target
            .get(
                &path(fixture.foreign_household_id),
                Some(&fixture.access_token)
            )
            .status()
            .as_u16(),
        403
    );
    assert_eq!(target.get(&collection_path, None).status().as_u16(), 401);
}

#[test]
fn direct_person_medication_take_updates_exact_stock_and_history_once() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = path(fixture.household_id);
    let uuid = format!(
        "11111111-1111-4111-8111-{:012x}",
        fixture.dose_write_medication_id
    );
    let request_payload = payload(
        "person_medication",
        &fixture.dose_write_assignment_portable_id,
        fixture.dose_write_medication_id,
        &uuid,
    );
    assert_eq!(stock(fixture.dose_write_medication_id), "20.00");
    let denied = target.post_json_authorized(&path, &fixture.view_access_token, &request_payload);
    assert_eq!(denied.status().as_u16(), 403);
    let denied_request_id = request_id(&denied);
    assert_eq!(count_for_uuid(&uuid), 0);
    assert_eq!(stock(fixture.dose_write_medication_id), "20.00");
    assert_request_audit(
        &denied_request_id,
        &fixture,
        403,
        fixture.view_account_id,
        fixture.view_membership_id,
    );

    let created = target.post_json_authorized(&path, &fixture.access_token, &request_payload);
    assert_eq!(created.status().as_u16(), 201);
    let created_request_id = request_id(&created);
    let etag = created.headers()["etag"].to_str().unwrap().to_string();
    let take: Value = created.json().expect("created dose JSON");
    let take = take["data"].clone();
    let id = take["id"].as_i64().expect("take ID");
    assert_eq!(take["client_uuid"], uuid);
    assert_eq!(
        take["person_medication_id"],
        fixture.dose_write_assignment_id
    );
    assert_eq!(take["person_id"], fixture.dose_write_person_id);
    assert_eq!(take["medication_id"], fixture.dose_write_medication_id);
    assert_eq!(
        take["taken_from_medication_id"],
        fixture.dose_write_medication_id
    );
    assert_eq!(take["dose_amount"], "1.25");
    assert_eq!(take["dose_unit"], "ml");
    assert_eq!(
        take["taken_at"],
        request_payload["medication_take"]["taken_at"]
    );
    assert_eq!(stock(fixture.dose_write_medication_id), "18.75");
    let dose_amount: String = database()
        .query_one(
            "SELECT dose_amount::text FROM medication_takes WHERE id = $1",
            &[&id],
        )
        .unwrap()
        .get(0);
    assert_eq!(dose_amount, "1.25");
    assert!(history(&target, &fixture)["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == id));
    assert_eq!(count_for_uuid(&uuid), 1);
    assert_eq!(take_version_count(id), 1);
    assert_take_creation_version(id, &take, &created_request_id, &fixture);
    assert_eq!(take_change_count(id), 1);
    assert_request_audit(
        &created_request_id,
        &fixture,
        201,
        fixture.account_id,
        fixture.owner_membership_id,
    );

    let replay = target.post_json_authorized(&path, &fixture.access_token, &request_payload);
    assert_eq!(replay.status().as_u16(), 200);
    let replay_request_id = request_id(&replay);
    assert_eq!(replay.headers()["etag"].to_str().unwrap(), etag);
    assert_eq!(replay.json::<Value>().unwrap()["data"], take);
    assert_eq!(stock(fixture.dose_write_medication_id), "18.75");
    assert_eq!(count_for_uuid(&uuid), 1);
    assert_eq!(take_version_count(id), 1);
    assert_eq!(take_change_count(id), 1);
    assert_request_audit(
        &replay_request_id,
        &fixture,
        200,
        fixture.account_id,
        fixture.owner_membership_id,
    );

    let mut changed = request_payload;
    changed["medication_take"]["dose_amount"] = json!("1.50");
    let conflict = target.post_json_authorized(&path, &fixture.access_token, &changed);
    assert_eq!(conflict.status().as_u16(), 409);
    let conflict_request_id = request_id(&conflict);
    assert_eq!(stock(fixture.dose_write_medication_id), "18.75");
    assert_eq!(count_for_uuid(&uuid), 1);
    assert_eq!(take_version_count(id), 1);
    assert_eq!(take_change_count(id), 1);
    assert_request_audit(
        &conflict_request_id,
        &fixture,
        409,
        fixture.account_id,
        fixture.owner_membership_id,
    );

    let foreign_attempt = target.post_json_authorized(
        &format!(
            "/api/v1/households/{}/medication_takes",
            fixture.foreign_household_id
        ),
        &fixture.foreign_access_token,
        &payload(
            "person_medication",
            &fixture.dose_write_foreign_assignment_portable_id,
            fixture.dose_write_foreign_medication_id,
            &uuid,
        ),
    );
    assert_eq!(foreign_attempt.status().as_u16(), 409);
    let foreign_request_id = request_id(&foreign_attempt);
    let foreign_body: Value = foreign_attempt.json().unwrap();
    assert_eq!(foreign_body["error"]["code"], "conflict");
    assert!(foreign_body.get("data").is_none());
    assert!(!foreign_body
        .to_string()
        .contains(take["portable_id"].as_str().unwrap()));
    assert_eq!(stock(fixture.dose_write_foreign_medication_id), "9.00");
    assert_eq!(stock(fixture.dose_write_medication_id), "18.75");
    assert_eq!(count_for_uuid(&uuid), 1);
    assert_eq!(take_version_count(id), 1);
    let foreign_audit = database()
        .query_one(
            "SELECT household_id, metadata->>'status' FROM security_audit_events WHERE request_id = $1",
            &[&foreign_request_id],
        )
        .expect("foreign conflict audit");
    assert_eq!(foreign_audit.get::<_, i64>(0), fixture.foreign_household_id);
    assert_eq!(foreign_audit.get::<_, String>(1), "409");
}

#[test]
fn direct_schedule_and_tracked_dosage_takes_reduce_the_selected_inventory() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = path(fixture.household_id);
    assert_eq!(stock(fixture.dose_write_schedule_medication_id), "7.50");
    let schedule_payload = payload(
        "schedule",
        &fixture.dose_write_schedule_portable_id,
        fixture.dose_write_schedule_medication_id,
        &format!(
            "22222222-2222-4222-8222-{:012x}",
            fixture.dose_write_schedule_id
        ),
    );
    let schedule = target.post_json_authorized(&path, &fixture.access_token, &schedule_payload);
    assert_eq!(schedule.status().as_u16(), 201);
    let schedule_request_id = request_id(&schedule);
    let schedule_take: Value = schedule.json().unwrap();
    let schedule_take = &schedule_take["data"];
    assert_eq!(schedule_take["schedule_id"], fixture.dose_write_schedule_id);
    assert_eq!(schedule_take["dose_amount"], "1.25");
    assert_eq!(stock(fixture.dose_write_schedule_medication_id), "6.25");
    assert_eq!(take_version_count(schedule_take["id"].as_i64().unwrap()), 1);
    assert_request_audit(
        &schedule_request_id,
        &fixture,
        201,
        fixture.account_id,
        fixture.owner_membership_id,
    );

    assert_eq!(dosage_stock(fixture.dose_write_tracked_option_id), "10.00");
    assert_eq!(stock(fixture.dose_write_tracked_medication_id), "10.00");
    let tracked_payload = payload(
        "person_medication",
        &fixture.dose_write_tracked_assignment_portable_id,
        fixture.dose_write_tracked_medication_id,
        &format!(
            "33333333-3333-4333-8333-{:012x}",
            fixture.dose_write_tracked_medication_id
        ),
    );
    let tracked = target.post_json_authorized(&path, &fixture.access_token, &tracked_payload);
    assert_eq!(tracked.status().as_u16(), 201);
    let tracked_request_id = request_id(&tracked);
    let tracked_take: Value = tracked.json().unwrap();
    let tracked_take = &tracked_take["data"];
    assert_eq!(tracked_take["dose_amount"], "1.25");
    assert_eq!(
        tracked_take["medication_id"],
        fixture.dose_write_tracked_medication_id
    );
    assert_eq!(dosage_stock(fixture.dose_write_tracked_option_id), "8.75");
    assert_eq!(stock(fixture.dose_write_tracked_medication_id), "8.75");
    assert_eq!(take_version_count(tracked_take["id"].as_i64().unwrap()), 1);
    assert_request_audit(
        &tracked_request_id,
        &fixture,
        201,
        fixture.account_id,
        fixture.owner_membership_id,
    );
    let listed = history(&target, &fixture);
    let rows = listed["data"].as_array().expect("dose rows");
    assert!(rows.iter().any(|row| row["id"] == schedule_take["id"]));
    assert!(rows.iter().any(|row| row["id"] == tracked_take["id"]));
}

#[test]
fn direct_take_respects_timing_without_a_second_stock_or_audit_mutation() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = path(fixture.household_id);
    let first = payload(
        "person_medication",
        &fixture.dose_write_timed_assignment_portable_id,
        fixture.dose_write_timed_medication_id,
        "44444444-4444-4444-8444-000000000001",
    );
    let created = target.post_json_authorized(&path, &fixture.access_token, &first);
    assert_eq!(created.status().as_u16(), 201);
    let created: Value = created.json().unwrap();
    let take_id = created["data"]["id"].as_i64().unwrap();
    assert_eq!(stock(fixture.dose_write_timed_medication_id), "8.75");
    let mut second = first;
    second["medication_take"]["client_uuid"] = json!("44444444-4444-4444-8444-000000000002");
    let version_count = all_take_version_count();
    let denied = target.post_json_authorized(&path, &fixture.access_token, &second);
    assert_eq!(denied.status().as_u16(), 422);
    let denial_id = request_id(&denied);
    let body: Value = denied.json().unwrap();
    assert_eq!(
        body["error"]["message"],
        "Cannot take medication: timing restrictions not met"
    );
    assert_eq!(stock(fixture.dose_write_timed_medication_id), "8.75");
    assert_eq!(count_for_uuid("44444444-4444-4444-8444-000000000002"), 0);
    assert_eq!(take_version_count(take_id), 1);
    assert_eq!(all_take_version_count(), version_count);
    assert_request_audit(
        &denial_id,
        &fixture,
        422,
        fixture.account_id,
        fixture.owner_membership_id,
    );
}

#[test]
fn invalid_time_source_unit_and_stock_selection_leave_no_partial_take() {
    let fixture = fixture();
    let target = Target::from_env();
    let collection_path = path(fixture.household_id);
    let before_stock = stock(fixture.dose_write_medication_id);
    let before_other_stock = stock(fixture.dose_write_other_medication_id);
    let before_versions = all_take_version_count();
    let future = (OffsetDateTime::now_utc() + time::Duration::hours(2))
        .format(&Rfc3339)
        .unwrap();
    for (sequence, source_type, source_id, time, medication_id, unit, status) in [
        (
            1,
            "person_medication",
            fixture.dose_write_assignment_portable_id.as_str(),
            "invalid",
            fixture.dose_write_medication_id,
            None,
            422,
        ),
        (
            2,
            "unknown",
            fixture.dose_write_assignment_portable_id.as_str(),
            "valid",
            fixture.dose_write_medication_id,
            None,
            404,
        ),
        (
            3,
            "person_medication",
            fixture.foreign_assignment_portable_id.as_str(),
            "valid",
            fixture.dose_write_medication_id,
            None,
            404,
        ),
        (
            4,
            "person_medication",
            fixture.hidden_assignment_portable_id.as_str(),
            "valid",
            fixture.dose_write_medication_id,
            None,
            404,
        ),
        (
            5,
            "person_medication",
            fixture.dose_write_assignment_portable_id.as_str(),
            "valid",
            fixture.dose_write_other_medication_id,
            None,
            422,
        ),
        (
            6,
            "person_medication",
            fixture.dose_write_assignment_portable_id.as_str(),
            "future",
            fixture.dose_write_medication_id,
            None,
            422,
        ),
        (
            7,
            "person_medication",
            fixture.dose_write_assignment_portable_id.as_str(),
            "valid",
            fixture.dose_write_medication_id,
            Some("tablet"),
            422,
        ),
    ] {
        let uuid = format!("55555555-5555-4555-8555-{sequence:012x}");
        let mut attempted = payload(source_type, source_id, medication_id, &uuid);
        if time == "invalid" {
            attempted["medication_take"]["taken_at"] = json!("invalid");
        } else if time == "future" {
            attempted["medication_take"]["taken_at"] = json!(future);
        }
        if let Some(unit) = unit {
            attempted["medication_take"]["dose_unit"] = json!(unit);
        }
        let denied =
            target.post_json_authorized(&collection_path, &fixture.access_token, &attempted);
        assert_eq!(denied.status().as_u16(), status, "case {sequence}");
        let id = request_id(&denied);
        let denial: Value = denied.json().expect("dose denial JSON");
        assert_eq!(denial["error"]["request_id"], id);
        assert_eq!(count_for_uuid(&uuid), 0);
        assert_eq!(stock(fixture.dose_write_medication_id), before_stock);
        assert_eq!(
            stock(fixture.dose_write_other_medication_id),
            before_other_stock
        );
        assert_eq!(all_take_version_count(), before_versions);
        assert_request_audit(
            &id,
            &fixture,
            status,
            fixture.account_id,
            fixture.owner_membership_id,
        );
    }
    let foreign = target.post_json_authorized(
        &path(fixture.foreign_household_id),
        &fixture.access_token,
        &payload(
            "person_medication",
            &fixture.dose_write_assignment_portable_id,
            fixture.dose_write_medication_id,
            "55555555-5555-4555-8555-000000000008",
        ),
    );
    assert_eq!(foreign.status().as_u16(), 403);
    assert_eq!(stock(fixture.dose_write_medication_id), before_stock);
    assert_eq!(all_take_version_count(), before_versions);
}

#[test]
fn contending_identical_dose_requests_converge_on_one_take_and_stock_change() {
    let fixture = fixture();
    let path = path(fixture.household_id);
    let uuid = format!(
        "66666666-6666-4666-8666-{:012x}",
        fixture.dose_write_other_medication_id
    );
    let payload = payload(
        "person_medication",
        &fixture.dose_write_concurrent_assignment_portable_id,
        fixture.dose_write_other_medication_id,
        &uuid,
    );
    assert_eq!(stock(fixture.dose_write_other_medication_id), "5.00");
    let barrier = Arc::new(Barrier::new(3));
    let mut workers = Vec::new();
    for _ in 0..2 {
        let path = path.clone();
        let token = fixture.access_token.clone();
        let payload = payload.clone();
        let barrier = Arc::clone(&barrier);
        workers.push(thread::spawn(move || {
            let target = Target::from_env();
            barrier.wait();
            let response = target.post_json_authorized(&path, &token, &payload);
            let status = response.status().as_u16();
            let id = request_id(&response);
            let body: Value = response.json().expect("concurrent dose response");
            (status, id, body)
        }));
    }
    barrier.wait();
    let outcomes: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().expect("dose worker"))
        .collect();
    let mut statuses: Vec<_> = outcomes.iter().map(|(status, _, _)| *status).collect();
    statuses.sort_unstable();
    assert_eq!(statuses, [200, 201]);
    let first_id = outcomes[0].2["data"]["id"].as_i64().expect("take ID");
    assert_eq!(outcomes[1].2["data"]["id"], first_id);
    assert_eq!(stock(fixture.dose_write_other_medication_id), "3.75");
    assert_eq!(count_for_uuid(&uuid), 1);
    assert_eq!(take_version_count(first_id), 1);
    assert_eq!(take_change_count(first_id), 1);
    for (status, id, _) in outcomes {
        assert_request_audit(
            &id,
            &fixture,
            status,
            fixture.account_id,
            fixture.owner_membership_id,
        );
    }
    let target = Target::from_env();
    for sequence in 1..=3 {
        let mut next = payload.clone();
        next["medication_take"]["client_uuid"] =
            json!(format!("77777777-7777-4777-8777-{sequence:012x}"));
        let response = target.post_json_authorized(&path, &fixture.access_token, &next);
        assert_eq!(response.status().as_u16(), 201);
    }
    assert_eq!(stock(fixture.dose_write_other_medication_id), "0.00");
    let replay = target.post_json_authorized(&path, &fixture.access_token, &payload);
    assert_eq!(replay.status().as_u16(), 200);
    let replay_request_id = request_id(&replay);
    let replay: Value = replay.json().unwrap();
    assert_eq!(replay["data"]["id"], first_id);
    assert_eq!(stock(fixture.dose_write_other_medication_id), "0.00");
    assert_eq!(count_for_uuid(&uuid), 1);
    assert_eq!(take_version_count(first_id), 1);
    assert_eq!(take_change_count(first_id), 1);
    assert_request_audit(
        &replay_request_id,
        &fixture,
        200,
        fixture.account_id,
        fixture.owner_membership_id,
    );
}
