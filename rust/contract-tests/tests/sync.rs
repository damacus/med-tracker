use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::Response;
use serde_json::Value;

fn body(response: Response) -> Value {
    response.json().expect("JSON response")
}

fn snapshot_path(fixture: &Fixture, kind: &str) -> String {
    format!("/api/v1/households/{}/{}", fixture.household_id, kind)
}

fn records(snapshot: &Value) -> &Value {
    &snapshot["data"]["records"]
}

fn has_portable_id(rows: &Value, portable_id: &str) -> bool {
    rows.as_array()
        .expect("record collection")
        .iter()
        .any(|row| row["portable_id"] == portable_id)
}

fn assert_portable_records(collections: &Value, require_nonempty: bool) {
    for category in [
        "people",
        "locations",
        "medications",
        "dosage_options",
        "schedules",
        "person_medications",
        "medication_takes",
        "notification_preferences",
        "health_events",
    ] {
        let rows = collections[category].as_array().expect(category);
        if require_nonempty {
            assert!(!rows.is_empty(), "{category} must have a fixture record");
        }
        for row in rows {
            assert!(
                row["portable_id"].as_str().is_some_and(|id| !id.is_empty()),
                "{category}"
            );
            assert!(
                row["etag"].as_str().is_some_and(|tag| !tag.is_empty()),
                "{category}"
            );
            assert!(row.get("id").is_none(), "numeric ID leaked in {category}");
        }
    }
}

fn assert_hidden_and_foreign_records_absent(collections: &Value, fixture: &Fixture) {
    for (category, portable_ids) in [
        (
            "people",
            [
                &fixture.hidden_person_portable_id,
                &fixture.foreign_person_portable_id,
            ],
        ),
        (
            "locations",
            [
                &fixture.hidden_location_portable_id,
                &fixture.foreign_location_portable_id,
            ],
        ),
        (
            "medications",
            [
                &fixture.hidden_medication_portable_id,
                &fixture.foreign_medication_portable_id,
            ],
        ),
        (
            "dosage_options",
            [
                &fixture.hidden_dosage_portable_id,
                &fixture.foreign_dosage_portable_id,
            ],
        ),
        (
            "schedules",
            [
                &fixture.hidden_schedule_portable_id,
                &fixture.foreign_schedule_portable_id,
            ],
        ),
        (
            "person_medications",
            [
                &fixture.hidden_assignment_portable_id,
                &fixture.foreign_assignment_portable_id,
            ],
        ),
        (
            "medication_takes",
            [
                &fixture.hidden_take_portable_id,
                &fixture.foreign_take_portable_id,
            ],
        ),
        (
            "notification_preferences",
            [
                &fixture.hidden_preference_portable_id,
                &fixture.foreign_preference_portable_id,
            ],
        ),
        (
            "health_events",
            [
                &fixture.hidden_health_event_portable_id,
                &fixture.foreign_health_event_portable_id,
            ],
        ),
    ] {
        for id in portable_ids {
            assert!(
                !has_portable_id(&collections[category], id),
                "{category} leaked {id}"
            );
        }
    }
    if let Some(rows) = collections.get("medication_pause_periods") {
        for id in [
            &fixture.hidden_pause_period_id,
            &fixture.foreign_pause_period_id,
        ] {
            assert!(!has_portable_id(rows, id));
        }
    }
    for (category, rows) in collections.as_object().unwrap() {
        for row in rows.as_array().unwrap() {
            for source in [
                &fixture.hidden_schedule_portable_id,
                &fixture.foreign_schedule_portable_id,
                &fixture.hidden_assignment_portable_id,
                &fixture.foreign_assignment_portable_id,
            ] {
                assert_ne!(
                    row["source_portable_id"],
                    source.as_str(),
                    "{category} leaked source {source}"
                );
            }
        }
    }
}

#[test]
fn sync_v2_uses_current_view_scope_and_exposes_portable_clinical_relations() {
    let target = Target::from_env();
    let fixture = fixture();
    let response = target.get(
        &snapshot_path(&fixture, "sync/snapshot"),
        Some(&fixture.view_access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let snapshot = body(response);
    let data = &snapshot["data"];
    assert_eq!(data["format"], "medtracker.portable.v2");
    assert_eq!(data["scope"], "single_person");
    assert!(data["cursor"]
        .as_str()
        .is_some_and(|cursor| !cursor.is_empty()));
    let collections = records(&snapshot);
    assert_portable_records(collections, true);
    assert_hidden_and_foreign_records_absent(collections, &fixture);
    assert!(has_portable_id(
        &collections["people"],
        &fixture.managed_person_portable_id
    ));
    assert!(!has_portable_id(
        &collections["people"],
        &fixture.hidden_person_portable_id
    ));
    assert!(!has_portable_id(
        &collections["people"],
        &fixture.foreign_person_portable_id
    ));
    assert!(has_portable_id(
        &collections["health_events"],
        &fixture.managed_health_event_portable_id
    ));
    assert!(!has_portable_id(
        &collections["health_events"],
        &fixture.hidden_health_event_portable_id
    ));
    assert!(!has_portable_id(
        &collections["health_events"],
        &fixture.foreign_health_event_portable_id
    ));
    assert!(has_portable_id(
        &collections["medications"],
        &fixture.managed_medication_portable_id
    ));
    assert!(has_portable_id(
        &collections["dosage_options"],
        &fixture.managed_dosage_portable_id
    ));
    assert!(!has_portable_id(
        &collections["medications"],
        &fixture.hidden_medication_portable_id
    ));
    assert!(!has_portable_id(
        &collections["medications"],
        &fixture.foreign_medication_portable_id
    ));
    assert!(has_portable_id(
        &collections["dose_occurrences"],
        &fixture.managed_occurrence_portable_id
    ));
    let occurrence = collections["dose_occurrences"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["portable_id"] == fixture.managed_occurrence_portable_id)
        .unwrap();
    assert_eq!(occurrence["source_type"], "schedule");
    assert_eq!(
        occurrence["source_portable_id"],
        fixture.historical_schedule_portable_id
    );
    assert!(occurrence["etag"]
        .as_str()
        .is_some_and(|tag| !tag.is_empty()));
    let period = collections["medication_pause_periods"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["portable_id"] == fixture.retired_assignment_period_id)
        .unwrap();
    assert_eq!(period["source_type"], "person_medication");
    assert_eq!(
        period["source_portable_id"],
        fixture.retired_assignment_portable_id
    );
    assert!(period["etag"].as_str().is_some_and(|tag| !tag.is_empty()));
    let schedule = collections["schedules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["portable_id"] == fixture.managed_schedule_portable_id)
        .unwrap();
    assert_eq!(
        schedule["person_portable_id"],
        fixture.managed_person_portable_id
    );
    assert_eq!(
        schedule["medication_portable_id"],
        fixture.managed_medication_portable_id
    );
    let medication = collections["medications"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["portable_id"] == fixture.managed_medication_portable_id)
        .unwrap();
    assert_eq!(
        medication["location_portable_id"],
        fixture.primary_location_portable_id
    );
    let dosage = collections["dosage_options"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["portable_id"] == fixture.managed_dosage_portable_id)
        .unwrap();
    assert_eq!(
        dosage["medication_portable_id"],
        fixture.managed_medication_portable_id
    );
    let event = collections["health_events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["portable_id"] == fixture.managed_health_event_portable_id)
        .unwrap();
    assert_eq!(
        event["medication_portable_ids"],
        serde_json::json!([fixture.managed_medication_portable_id])
    );
    let take = collections["medication_takes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["portable_id"] == fixture.managed_take_portable_id)
        .unwrap();
    assert_eq!(
        take["source_portable_id"],
        fixture.historical_schedule_portable_id
    );
    assert_eq!(
        take["taken_from_medication_portable_id"],
        fixture.historical_medication_portable_id
    );
    assert_eq!(
        take["taken_from_location_portable_id"],
        fixture.historical_location_portable_id
    );
    let preference = collections["notification_preferences"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["portable_id"] == fixture.managed_preference_portable_id)
        .unwrap();
    assert_eq!(
        preference["person_portable_id"],
        fixture.managed_person_portable_id
    );
    assert!(collections.get("api_sessions").is_none());
    assert!(collections.get("security_audit_events").is_none());
}

#[test]
fn mobile_v1_uses_manage_scope_and_records_one_correlated_read_audit() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = snapshot_path(&fixture, "mobile_snapshot");
    let response = target.get(&path, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 200);
    let request_id = response.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let snapshot = body(response);
    let data = &snapshot["data"];
    assert_eq!(data["format"], "medtracker.portable.v1");
    assert_eq!(data["scope"], "single_person");
    assert!(data.get("cursor").is_none());
    let collections = records(&snapshot);
    assert_portable_records(collections, false);
    assert_hidden_and_foreign_records_absent(collections, &fixture);
    assert!(!has_portable_id(
        &collections["people"],
        &fixture.managed_person_portable_id
    ));
    assert!(!has_portable_id(
        &collections["people"],
        &fixture.hidden_person_portable_id
    ));
    assert!(!has_portable_id(
        &collections["people"],
        &fixture.foreign_person_portable_id
    ));
    assert!(collections.get("dose_occurrences").is_none());
    assert!(collections.get("medication_pause_periods").is_none());
    let audit_response = target.get(
        &format!(
            "/api/v1/households/{}/admin/audit_logs",
            fixture.household_id
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(audit_response.status().as_u16(), 200);
    let audit = body(audit_response);
    let matching: Vec<_> = audit["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|event| event["request_id"] == request_id)
        .collect();
    assert_eq!(matching.len(), 2);
    let read = matching
        .iter()
        .find(|event| event["event_type"] == "portable_data.mobile_snapshot_read")
        .unwrap();
    assert_eq!(read["actor_account_id"], fixture.view_account_id);
    assert_eq!(read["actor_membership_id"], fixture.view_membership_id);
    assert_eq!(read["metadata"]["export_mode"], "mobile_snapshot");
    assert_eq!(read["metadata"]["encrypted"], false);
    for (category, rows) in collections.as_object().unwrap() {
        assert_eq!(
            read["metadata"]["record_counts"][category],
            rows.as_array().unwrap().len()
        );
    }
    let request = matching
        .iter()
        .find(|event| event["event_type"] == "api.request")
        .unwrap();
    assert_eq!(request["metadata"]["status"], 200);
}

#[test]
fn delegated_mobile_export_contains_manageable_person_and_relations() {
    let target = Target::from_env();
    let fixture = fixture();
    let response = target.get(
        &snapshot_path(&fixture, "mobile_snapshot"),
        Some(&fixture.delegated_access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let snapshot = body(response);
    assert_eq!(snapshot["data"]["format"], "medtracker.portable.v1");
    let collections = records(&snapshot);
    assert_portable_records(collections, true);
    assert_hidden_and_foreign_records_absent(collections, &fixture);
    assert!(has_portable_id(
        &collections["people"],
        &fixture.managed_person_portable_id
    ));
    assert!(has_portable_id(
        &collections["health_events"],
        &fixture.managed_health_event_portable_id
    ));
    assert!(!has_portable_id(
        &collections["people"],
        &fixture.hidden_person_portable_id
    ));
    assert!(!has_portable_id(
        &collections["people"],
        &fixture.foreign_person_portable_id
    ));
    assert_eq!(
        collections["health_events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["portable_id"] == fixture.managed_health_event_portable_id)
            .unwrap()["person_portable_id"],
        fixture.managed_person_portable_id
    );
    let medication = collections["medications"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["portable_id"] == fixture.managed_medication_portable_id)
        .unwrap();
    assert_eq!(
        medication["location_portable_id"],
        fixture.primary_location_portable_id
    );
}

#[test]
fn snapshot_routes_require_authentication_and_reject_foreign_households() {
    let target = Target::from_env();
    let fixture = fixture();
    for kind in ["sync/snapshot", "mobile_snapshot"] {
        let path = snapshot_path(&fixture, kind);
        assert_eq!(target.get(&path, None).status().as_u16(), 401);
        let foreign = format!(
            "/api/v1/households/{}/{}",
            fixture.foreign_household_id, kind
        );
        assert_eq!(
            target
                .get(&foreign, Some(&fixture.access_token))
                .status()
                .as_u16(),
            403
        );
    }
}
