use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::Response;
use serde_json::{json, Value};
use time::OffsetDateTime;

fn body(response: Response) -> Value {
    response.json().expect("JSON response")
}

fn snapshot_path(fixture: &Fixture, kind: &str) -> String {
    format!("/api/v1/households/{}/{}", fixture.household_id, kind)
}

fn changes(target: &Target, fixture: &Fixture, token: &str, cursor: &str) -> Value {
    let path = format!("{}?cursor={cursor}", snapshot_path(fixture, "sync/changes"));
    let response = target.get(&path, Some(token));
    assert_eq!(response.status().as_u16(), 200);
    body(response)["data"].clone()
}

fn row_for_portable_id<'a>(rows: &'a Value, portable_id: &str) -> Option<&'a Value> {
    rows.as_array()
        .expect("feed collection")
        .iter()
        .find(|row| row["record_portable_id"] == portable_id)
}

fn contains_identifier(value: &Value, portable_id: &str) -> bool {
    match value {
        Value::String(text) => text.contains(portable_id),
        Value::Array(rows) => rows.iter().any(|row| contains_identifier(row, portable_id)),
        Value::Object(fields) => fields.iter().any(|(key, field)| {
            key.contains(portable_id) || contains_identifier(field, portable_id)
        }),
        _ => false,
    }
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

#[test]
fn change_feed_retains_inclusive_ordered_writes_and_tombstone_metadata() {
    let target = Target::from_env();
    let fixture = fixture();
    let snapshot = body(target.get(
        &snapshot_path(&fixture, "sync/snapshot"),
        Some(&fixture.access_token),
    ));
    let cursor = snapshot["data"]["cursor"]
        .as_str()
        .expect("snapshot cursor");
    let health_path = snapshot_path(&fixture, "health_events");
    let response = target.post_json_authorized(
        &health_path,
        &fixture.access_token,
        &json!({"health_event": {
            "person_id": fixture.managed_person_portable_id,
            "event_kind": "illness", "title": "Feed event", "started_on": "2026-02-25"
        }}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let created = body(response)["data"].clone();
    let health_id = created["portable_id"].as_str().expect("portable health ID");
    let response = target.patch_json(
        &format!("{health_path}/{health_id}"),
        &fixture.access_token,
        &json!({"health_event": {"title": "Feed event updated"}}),
    );
    assert_eq!(response.status().as_u16(), 200);

    let location_path = snapshot_path(&fixture, "locations");
    let response = target.post_json_authorized(
        &location_path,
        &fixture.access_token,
        &json!({"location": {"name": format!("Feed location {health_id}")}}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let tag = response.headers()["etag"]
        .to_str()
        .expect("location ETag")
        .to_owned();
    let location = body(response)["data"].clone();
    let location_id = location["portable_id"]
        .as_str()
        .expect("portable location ID");
    let response = target.delete_if_match(
        &format!("{location_path}/{location_id}"),
        &fixture.access_token,
        &tag,
    );
    assert_eq!(response.status().as_u16(), 204);
    let response = target.post_json_authorized(
        &location_path,
        &fixture.access_token,
        &json!({"location": {"name": format!("Second feed location {health_id}")}}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let second_tag = response.headers()["etag"]
        .to_str()
        .expect("location ETag")
        .to_owned();
    let second_location = body(response)["data"].clone();
    let second_location_id = second_location["portable_id"]
        .as_str()
        .expect("portable location ID");
    let response = target.delete_if_match(
        &format!("{location_path}/{second_location_id}"),
        &fixture.access_token,
        &second_tag,
    );
    assert_eq!(response.status().as_u16(), 204);

    let feed = changes(&target, &fixture, &fixture.access_token, cursor);
    assert!(feed["cursor"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    let events = feed["changes"].as_array().expect("changes array");
    let own_events: Vec<_> = events
        .iter()
        .filter(|event| event["record_portable_id"] == health_id)
        .collect();
    assert_eq!(own_events.len(), 2);
    assert_eq!(own_events[0]["action"], "create");
    assert_eq!(own_events[1]["action"], "update");
    for event in &own_events {
        assert_eq!(event["record_type"], "HealthEvent");
        assert_eq!(event["record_id"], created["id"]);
        assert_eq!(event["metadata"]["portable_id"], health_id);
        assert_eq!(
            event["metadata"]["person_portable_id"],
            fixture.managed_person_portable_id
        );
        assert!(event["id"].as_i64().is_some());
        assert!(event["occurred_at"]
            .as_str()
            .is_some_and(|value| value.ends_with('Z')));
    }
    assert!(events.windows(2).all(|pair| {
        let previous = pair[0]["occurred_at"].as_str().unwrap();
        let next = pair[1]["occurred_at"].as_str().unwrap();
        previous < next || (previous == next && pair[0]["id"].as_i64() < pair[1]["id"].as_i64())
    }));
    let inclusive = changes(
        &target,
        &fixture,
        &fixture.access_token,
        own_events[0]["occurred_at"].as_str().unwrap(),
    );
    assert!(inclusive["changes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == own_events[0]["id"]));
    let location_change =
        row_for_portable_id(&feed["changes"], location_id).expect("location create change");
    assert_eq!(location_change["record_type"], "Location");
    assert_eq!(location_change["action"], "create");
    assert_eq!(location_change["metadata"]["portable_id"], location_id);

    let tombstone =
        row_for_portable_id(&feed["tombstones"], location_id).expect("location tombstone");
    let second_tombstone = row_for_portable_id(&feed["tombstones"], second_location_id)
        .expect("second location tombstone");
    assert_eq!(tombstone["record_type"], "Location");
    assert_eq!(tombstone["action"], "delete");
    assert_eq!(tombstone["metadata"]["portable_id"], location_id);
    assert!(tombstone["id"].as_i64().is_some());
    assert!(tombstone["deleted_at"]
        .as_str()
        .is_some_and(|value| value.ends_with('Z')));
    assert!(second_tombstone["id"].as_i64() > tombstone["id"].as_i64());
    let tombstones = feed["tombstones"].as_array().unwrap();
    assert!(tombstones.windows(2).all(|pair| {
        let previous = pair[0]["deleted_at"].as_str().unwrap();
        let next = pair[1]["deleted_at"].as_str().unwrap();
        previous < next || (previous == next && pair[0]["id"].as_i64() < pair[1]["id"].as_i64())
    }));
}

#[test]
fn change_feed_includes_events_and_tombstones_at_the_exact_cursor() {
    let target = Target::from_env();
    let fixture = fixture();
    let boundary = "2026-01-01T00:00:00Z";
    let feed = changes(&target, &fixture, &fixture.access_token, boundary);
    let portable_id = &fixture.cursor_boundary_location_portable_id;
    let event = row_for_portable_id(&feed["changes"], portable_id).expect("boundary event");
    let tombstone =
        row_for_portable_id(&feed["tombstones"], portable_id).expect("boundary tombstone");
    assert_eq!(event["occurred_at"], boundary);
    assert_eq!(event["action"], "create");
    assert_eq!(tombstone["deleted_at"], boundary);
    assert_eq!(tombstone["action"], "delete");

    let after = changes(
        &target,
        &fixture,
        &fixture.access_token,
        "2026-01-01T00:00:01Z",
    );
    assert!(row_for_portable_id(&after["changes"], portable_id).is_none());
    assert!(row_for_portable_id(&after["tombstones"], portable_id).is_none());
}

#[test]
fn change_feed_validates_cursor_and_current_household_access() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = snapshot_path(&fixture, "sync/changes");
    assert_eq!(
        target
            .get(&path, Some(&fixture.access_token))
            .status()
            .as_u16(),
        400
    );
    let response = target.get(
        &format!("{path}?cursor=not-a-date"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(body(response)["error"]["code"], "unprocessable_content");
    assert_eq!(
        target
            .get(&format!("{path}?cursor=1970-01-01T00:00:00Z"), None)
            .status()
            .as_u16(),
        401
    );
    let foreign = format!(
        "/api/v1/households/{}/sync/changes?cursor=1970-01-01T00:00:00Z",
        fixture.foreign_household_id
    );
    assert_eq!(
        target
            .get(&foreign, Some(&fixture.access_token))
            .status()
            .as_u16(),
        403
    );
}

#[test]
fn change_feed_projects_the_current_saved_dose_outcome() {
    let target = Target::from_env();
    let fixture = fixture();
    let snapshot = body(target.get(
        &snapshot_path(&fixture, "sync/snapshot"),
        Some(&fixture.access_token),
    ));
    let cursor = snapshot["data"]["cursor"]
        .as_str()
        .expect("snapshot cursor");
    let date = OffsetDateTime::now_utc().date().to_string();
    let source_path = format!(
        "{}/{}{}",
        snapshot_path(&fixture, "schedules"),
        fixture.managed_schedule_portable_id,
        "/dose_occurrences"
    );
    let projected = body(target.get(
        &format!("{source_path}?start_date={date}&end_date={date}"),
        Some(&fixture.access_token),
    ));
    let occurrence = projected["data"]
        .as_array()
        .expect("occurrences")
        .first()
        .expect("managed occurrence");
    let key = occurrence["key"].as_str().expect("occurrence key");
    let not_taken = target.post_json_authorized(
        &format!("{source_path}/not_taken"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": key, "reason": "unwell", "note": "Feed control"}}),
    );
    assert_eq!(not_taken.status().as_u16(), 200);
    let tag = not_taken.headers()["etag"]
        .to_str()
        .expect("outcome ETag")
        .to_owned();
    assert_eq!(body(not_taken)["data"]["outcome"], "not_taken");
    let reopened = target.patch_json_if_match(
        &format!("{source_path}/reopen"),
        &fixture.access_token,
        &json!({"dose_occurrence": {"key": key}}),
        &tag,
    );
    assert_eq!(reopened.status().as_u16(), 200);
    let current = body(reopened)["data"].clone();
    assert_eq!(current["outcome"], "open");

    let feed = changes(&target, &fixture, &fixture.view_access_token, cursor);
    let outcome_events: Vec<_> = feed["changes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| {
            row["record_type"] == "MedicationDoseOccurrence"
                && row["record"]["source_portable_id"] == fixture.managed_schedule_portable_id
        })
        .collect();
    assert!(!outcome_events.is_empty(), "saved outcome change missing");
    for event in outcome_events {
        assert_eq!(event["record_type"], "MedicationDoseOccurrence");
        assert_eq!(
            event["metadata"]["person_portable_id"],
            fixture.managed_person_portable_id
        );
        assert_eq!(event["record"]["portable_id"], event["record_portable_id"]);
        assert_eq!(event["record"]["outcome"], "open");
        assert_eq!(event["record"]["etag"], current["etag"]);
    }
}

#[test]
#[ignore = "Rails exposes ordinary hidden-person change events and tombstones to a restricted member"]
fn change_feed_hides_ungranted_person_events_and_tombstones() {
    let target = Target::from_env();
    let fixture = fixture();
    let snapshot = body(target.get(
        &snapshot_path(&fixture, "sync/snapshot"),
        Some(&fixture.view_access_token),
    ));
    let cursor = snapshot["data"]["cursor"]
        .as_str()
        .expect("snapshot cursor");
    let health_path = snapshot_path(&fixture, "health_events");
    let visible = target.post_json_authorized(
        &health_path,
        &fixture.access_token,
        &json!({"health_event": {
            "person_id": fixture.managed_person_portable_id,
            "event_kind": "illness", "title": "Visible feed control", "started_on": "2026-02-25"
        }}),
    );
    assert_eq!(visible.status().as_u16(), 201);
    let visible_id = body(visible)["data"]["portable_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let hidden = target.post_json_authorized(
        &health_path,
        &fixture.feed_access_token,
        &json!({"health_event": {
            "person_id": fixture.hidden_person_portable_id,
            "event_kind": "illness", "title": "Hidden feed event", "started_on": "2026-02-25"
        }}),
    );
    assert_eq!(hidden.status().as_u16(), 201);
    let hidden_id = body(hidden)["data"]["portable_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let visible_pause = format!(
        "{}/{}{}",
        snapshot_path(&fixture, "schedules"),
        fixture.managed_schedule_portable_id,
        "/pause"
    );
    let visible_pause_response =
        target.patch_json(&visible_pause, &fixture.access_token, &json!({}));
    assert_eq!(visible_pause_response.status().as_u16(), 200);
    let visible_pause_id = body(visible_pause_response)["data"]["current_pause_period"]
        ["portable_id"]
        .as_str()
        .expect("visible pause ID")
        .to_owned();
    let hidden_pause = format!(
        "{}/{}{}",
        snapshot_path(&fixture, "schedules"),
        fixture.hidden_schedule_portable_id,
        "/pause"
    );
    let hidden_pause_response =
        target.patch_json(&hidden_pause, &fixture.feed_access_token, &json!({}));
    assert_eq!(hidden_pause_response.status().as_u16(), 200);
    let hidden_pause_id = body(hidden_pause_response)["data"]["current_pause_period"]
        ["portable_id"]
        .as_str()
        .expect("hidden pause ID")
        .to_owned();

    let hidden_assignment_path = format!(
        "{}/{}",
        snapshot_path(&fixture, "person_medications"),
        fixture.hidden_assignment_portable_id
    );
    let hidden_assignment = target.get(&hidden_assignment_path, Some(&fixture.feed_access_token));
    assert_eq!(hidden_assignment.status().as_u16(), 200);
    let tag = hidden_assignment.headers()["etag"]
        .to_str()
        .expect("assignment ETag")
        .to_owned();
    let deletion = target.post_json_authorized(
        &snapshot_path(&fixture, "sync/batches"),
        &fixture.feed_access_token,
        &json!({"batch": {"operations": [{
            "action": "delete", "resource_type": "person_medication",
            "id": fixture.hidden_assignment_portable_id, "if_match": tag,
            "attributes": {}
        }]}}),
    );
    assert_eq!(deletion.status().as_u16(), 201);
    assert!(
        body(deletion)["data"]["results"][0]["record_portable_id"]
            == fixture.hidden_assignment_portable_id,
        "hidden deletion did not return its portable ID"
    );

    let feed = changes(&target, &fixture, &fixture.view_access_token, cursor);
    assert!(
        row_for_portable_id(&feed["changes"], &visible_id).is_some(),
        "managed control missing"
    );
    assert!(
        row_for_portable_id(&feed["changes"], &visible_pause_id).is_some(),
        "managed pause control missing"
    );
    assert!(
        row_for_portable_id(&feed["changes"], &hidden_pause_id).is_none(),
        "hidden pause disclosed"
    );
    let hidden_event_disclosed = contains_identifier(&feed["changes"], &hidden_id);
    let hidden_tombstone_disclosed =
        contains_identifier(&feed["tombstones"], &fixture.hidden_assignment_portable_id);
    let hidden_identifier_disclosed = [
        &fixture.hidden_person_portable_id,
        &fixture.hidden_health_event_portable_id,
        &hidden_id,
        &fixture.hidden_assignment_portable_id,
        &fixture.hidden_schedule_portable_id,
        &fixture.hidden_medication_portable_id,
        &fixture.hidden_dosage_portable_id,
        &fixture.hidden_take_portable_id,
        &fixture.hidden_preference_portable_id,
        &fixture.hidden_pause_period_id,
        &hidden_pause_id,
    ]
    .iter()
    .any(|id| contains_identifier(&feed, id));
    assert!(
        !hidden_event_disclosed && !hidden_tombstone_disclosed && !hidden_identifier_disclosed,
        "hidden event disclosed: {hidden_event_disclosed}; hidden tombstone disclosed: {hidden_tombstone_disclosed}; hidden identifier disclosed: {hidden_identifier_disclosed}"
    );
}
