use medtracker_contract_tests::{Target, fixture};
use serde_json::Value;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

const PASSPHRASE: &str = "contract portable secret";
const PASSPHRASE_HEADER: &str = "X-MedTracker-Portable-Passphrase";

fn path(household_id: i64, action: &str) -> String {
    format!("/api/v1/households/{household_id}/{action}")
}

fn keys(value: &Value) -> Vec<&str> {
    value
        .as_object()
        .expect("JSON object")
        .keys()
        .map(String::as_str)
        .collect()
}

fn body(response: reqwest::blocking::Response) -> Value {
    response.json().expect("JSON response")
}

fn assert_timestamp(value: &Value) {
    OffsetDateTime::parse(value.as_str().expect("RFC3339 timestamp"), &Rfc3339)
        .expect("valid timestamp");
}

fn assert_export_records(records: &Value) {
    let required = [
        "dosage_options",
        "locations",
        "medication_takes",
        "medications",
        "notification_preferences",
        "people",
        "person_medications",
        "schedules",
    ];
    let allowed = [
        "dose_occurrences",
        "dosage_options",
        "health_events",
        "locations",
        "medication_pause_periods",
        "medication_takes",
        "medications",
        "notification_preferences",
        "people",
        "person_medications",
        "schedules",
    ];
    let actual = keys(records);
    for field in required {
        assert!(
            actual.contains(&field),
            "missing portable records field {field}"
        );
    }
    assert!(actual.iter().all(|field| allowed.contains(field)));
    for field in actual {
        assert!(records[field].is_array(), "portable collection {field}");
    }
}

fn assert_encrypted_envelope(envelope: &Value) {
    assert_eq!(
        keys(envelope),
        [
            "checksum",
            "cipher",
            "ciphertext",
            "encrypted_at",
            "format",
            "kdf",
            "salt",
        ]
    );
    assert_eq!(envelope["format"], "medtracker.portable.encrypted.v1");
    assert_eq!(envelope["cipher"], "aes-256-gcm");
    assert_eq!(envelope["kdf"], "pbkdf2_sha256");
    assert!(
        envelope["salt"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );
    assert!(
        envelope["ciphertext"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );
    let checksum = envelope["checksum"].as_str().expect("checksum");
    assert_eq!(checksum.len(), 64);
    assert!(
        checksum
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    );
    assert_timestamp(&envelope["encrypted_at"]);
}

#[test]
fn portable_bundle_response_has_exact_encrypted_envelope_shape() {
    let target = Target::from_env();
    let fixture = fixture();
    let endpoint = path(fixture.portable_source_household_id, "portable_export");
    let response = target.get_with_header(
        &endpoint,
        &fixture.portable_source_access_token,
        PASSPHRASE_HEADER,
        PASSPHRASE,
    );
    assert_eq!(response.status().as_u16(), 200);
    let envelope = body(response);
    assert_eq!(keys(&envelope), ["data"]);
    assert_encrypted_envelope(&envelope["data"]);
    assert!(!envelope.to_string().contains(PASSPHRASE));
    assert!(
        !envelope
            .to_string()
            .contains(&fixture.portable_source_person_name)
    );

    for query in ["version=3", "portable_format=medtracker.portable.invalid"] {
        let invalid = target.get_with_header(
            &format!("{endpoint}?{query}"),
            &fixture.portable_source_access_token,
            PASSPHRASE_HEADER,
            PASSPHRASE,
        );
        assert_eq!(invalid.status().as_u16(), 422, "{query}");
        let error = body(invalid);
        assert_eq!(keys(&error), ["error"]);
        assert!(error["error"]["code"].is_string());
        assert!(!error.to_string().contains(PASSPHRASE));
    }
}

#[test]
fn mobile_snapshot_response_has_exact_snapshot_and_collection_shapes() {
    let target = Target::from_env();
    let fixture = fixture();
    let response = target.get(
        &path(fixture.household_id, "mobile_snapshot"),
        Some(&fixture.view_access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let snapshot = body(response);
    assert_eq!(keys(&snapshot), ["data"]);
    let data = &snapshot["data"];
    assert_eq!(
        keys(data),
        [
            "exported_at",
            "format",
            "records",
            "scope",
            "source_instance_id"
        ]
    );
    assert_eq!(data["format"], "medtracker.portable.v1");
    assert_eq!(data["scope"], "single_person");
    assert!(
        data["source_instance_id"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );
    assert_timestamp(&data["exported_at"]);

    let records = &data["records"];
    assert_export_records(records);
    assert!(records.get("dose_occurrences").is_none());
    assert!(records.get("medication_pause_periods").is_none());
    assert!(records.get("api_sessions").is_none());
    assert!(records.get("security_audit_events").is_none());
}

#[test]
fn data_export_modes_match_the_documented_response_shapes_and_cache_policy() {
    let target = Target::from_env();
    let fixture = fixture();

    let health_response = target.get(
        &path(fixture.household_id, "data_exports/health_data_json"),
        Some(&fixture.access_token),
    );
    assert_eq!(health_response.status().as_u16(), 200);
    assert_eq!(health_response.headers()["cache-control"], "no-store");
    let health = body(health_response);
    assert_eq!(keys(&health), ["data"]);
    let data = &health["data"];
    assert_eq!(
        keys(data),
        [
            "exported_at",
            "format",
            "records",
            "scope",
            "source_instance_id"
        ]
    );
    assert_eq!(data["format"], "medtracker.health_data.v1");
    assert_eq!(data["scope"], "single_person");
    assert_export_records(&data["records"]);
    assert_timestamp(&data["exported_at"]);

    let backup_response = target.get(
        &path(fixture.household_id, "data_exports/backup_zip"),
        Some(&fixture.access_token),
    );
    assert_eq!(backup_response.status().as_u16(), 200);
    assert_eq!(backup_response.headers()["cache-control"], "no-store");
    let backup = body(backup_response);
    assert_eq!(keys(&backup), ["data"]);
    assert_eq!(
        keys(&backup["data"]),
        ["base64", "content_type", "filename"]
    );
    assert_eq!(backup["data"]["content_type"], "application/zip");
    assert!(backup["data"]["filename"]
        .as_str()
        .is_some_and(|value| value.starts_with("medtracker-backup-") && value.ends_with(".zip")));
    assert!(
        backup["data"]["base64"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );

    let encrypted_response = target.get_with_header(
        &path(
            fixture.household_id,
            "data_exports/encrypted_migration_bundle",
        ),
        &fixture.access_token,
        PASSPHRASE_HEADER,
        PASSPHRASE,
    );
    assert_eq!(encrypted_response.status().as_u16(), 200);
    assert_eq!(encrypted_response.headers()["cache-control"], "no-store");
    let encrypted = body(encrypted_response);
    assert_eq!(keys(&encrypted), ["data"]);
    assert_encrypted_envelope(&encrypted["data"]);
    assert!(!encrypted.to_string().contains(PASSPHRASE));
}
