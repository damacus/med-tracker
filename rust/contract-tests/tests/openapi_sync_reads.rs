use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::Response;
use serde_json::Value;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

fn body(response: Response) -> Value {
    response.json().expect("JSON response")
}

fn path(fixture: &Fixture, endpoint: &str) -> String {
    format!("/api/v1/households/{}/{endpoint}", fixture.household_id)
}

fn assert_keys(value: &Value, required: &[&str], allowed: &[&str]) {
    let object = value.as_object().expect("JSON object");
    for key in required {
        assert!(object.contains_key(*key), "missing key {key}");
    }
    for key in object.keys() {
        assert!(allowed.contains(&key.as_str()), "unexpected key {key}");
    }
}

fn assert_timestamp(value: &Value) {
    OffsetDateTime::parse(value.as_str().expect("date-time string"), &Rfc3339)
        .expect("RFC3339 timestamp");
}

fn assert_uuid(value: &Value) {
    let id = value.as_str().expect("portable UUID");
    assert_eq!(id.len(), 36);
    assert!(id.chars().enumerate().all(|(index, character)| {
        if [8, 13, 18, 23].contains(&index) {
            character == '-'
        } else {
            character.is_ascii_hexdigit()
        }
    }));
}

#[test]
fn snapshot_and_changes_match_the_closed_openapi_envelopes() {
    let target = Target::from_env();
    let fixture = fixture();
    let snapshot_response = target.get(
        &path(&fixture, "sync/snapshot"),
        Some(&fixture.access_token),
    );
    assert_eq!(snapshot_response.status().as_u16(), 200);
    let snapshot = body(snapshot_response);
    assert_keys(&snapshot, &["data"], &["data"]);
    let data = &snapshot["data"];
    let snapshot_fields = [
        "format",
        "scope",
        "exported_at",
        "source_instance_id",
        "records",
        "cursor",
    ];
    assert_keys(data, &snapshot_fields, &snapshot_fields);
    assert_eq!(data["format"], "medtracker.portable.v2");
    assert!(matches!(
        data["scope"].as_str(),
        Some("single_person" | "household")
    ));
    assert_timestamp(&data["exported_at"]);
    assert_timestamp(&data["cursor"]);
    assert!(data["source_instance_id"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));

    let record_required = [
        "people",
        "locations",
        "medications",
        "dosage_options",
        "schedules",
        "person_medications",
        "medication_takes",
        "notification_preferences",
    ];
    let record_allowed = [
        "medication_pause_periods",
        "people",
        "locations",
        "medications",
        "dosage_options",
        "schedules",
        "person_medications",
        "medication_takes",
        "dose_occurrences",
        "notification_preferences",
        "health_events",
    ];
    let records = &data["records"];
    assert_keys(records, &record_required, &record_allowed);
    for (name, rows) in records.as_object().expect("portable record collections") {
        assert!(
            rows.as_array().is_some(),
            "portable collection {name} must be an array"
        );
    }

    let feed_response = target.get(
        &path(&fixture, "sync/changes?cursor=1970-01-01T00:00:00Z"),
        Some(&fixture.access_token),
    );
    assert_eq!(feed_response.status().as_u16(), 200);
    let feed = body(feed_response);
    assert_keys(&feed, &["data"], &["data"]);
    let data = &feed["data"];
    let feed_fields = ["cursor", "changes", "tombstones"];
    assert_keys(data, &feed_fields, &feed_fields);
    assert_timestamp(&data["cursor"]);
    let changes = data["changes"].as_array().expect("change rows");
    let tombstones = data["tombstones"].as_array().expect("tombstone rows");
    assert!(
        !changes.is_empty(),
        "fixture should exercise change row schema"
    );
    assert!(
        !tombstones.is_empty(),
        "fixture should exercise tombstone row schema"
    );
    let change_required = [
        "id",
        "record_type",
        "record_id",
        "record_portable_id",
        "action",
        "occurred_at",
        "metadata",
    ];
    let change_allowed = [
        "id",
        "record_type",
        "record_id",
        "record_portable_id",
        "action",
        "occurred_at",
        "metadata",
        "record",
    ];
    for event in changes {
        assert_keys(event, &change_required, &change_allowed);
        assert!(event["id"].as_i64().is_some_and(|id| id > 0));
        assert!(event["record_id"].as_i64().is_some_and(|id| id > 0));
        assert!(event["record_type"]
            .as_str()
            .is_some_and(|kind| !kind.is_empty()));
        assert_uuid(&event["record_portable_id"]);
        assert!(matches!(
            event["action"].as_str(),
            Some("create" | "update" | "delete")
        ));
        assert_timestamp(&event["occurred_at"]);
        assert!(event["metadata"].is_object());
        if let Some(record) = event.get("record") {
            assert!(record.is_object());
        }
    }
    let tombstone_fields = [
        "id",
        "record_type",
        "record_portable_id",
        "action",
        "deleted_at",
        "metadata",
    ];
    for tombstone in tombstones {
        assert_keys(tombstone, &tombstone_fields, &tombstone_fields);
        assert!(tombstone["id"].as_i64().is_some_and(|id| id > 0));
        assert!(tombstone["record_type"]
            .as_str()
            .is_some_and(|kind| !kind.is_empty()));
        assert_uuid(&tombstone["record_portable_id"]);
        assert_eq!(tombstone["action"], "delete");
        assert_timestamp(&tombstone["deleted_at"]);
        assert!(tombstone["metadata"].is_object());
    }
}

#[test]
fn sync_reads_return_structured_not_found_for_an_absent_household() {
    let target = Target::from_env();
    let fixture = fixture();
    let absent_household_id = i64::MAX;
    for endpoint in [
        "sync/snapshot".to_owned(),
        "sync/changes?cursor=1970-01-01T00:00:00Z".to_owned(),
    ] {
        let response = target.get(
            &format!("/api/v1/households/{absent_household_id}/{endpoint}"),
            Some(&fixture.access_token),
        );
        assert_eq!(response.status().as_u16(), 404, "{endpoint}");
        let value = body(response);
        assert_keys(&value, &["error"], &["error"]);
        let error = &value["error"];
        assert_keys(
            error,
            &["code", "message", "request_id"],
            &["code", "message", "request_id", "errors"],
        );
        assert_eq!(error["code"], "not_found");
        assert!(error["message"]
            .as_str()
            .is_some_and(|message| !message.is_empty()));
        assert!(error["request_id"]
            .as_str()
            .is_some_and(|id| !id.is_empty()));
    }
}
