use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use med_tracker::models::_entities as rows;
use std::io::{Cursor, Read};

const PASSPHRASE: &str = "synthetic portable secret";

fn export_url(app: &Application, mode: &str) -> String {
    let fixture_household_id = app.fixture.household.id;
    format!(
        "{}/api/v1/households/{fixture_household_id}/data_exports/{mode}",
        app.origin
    )
}

async fn exported_events(app: &Application, mode: &str) -> i64 {
    super::portable_imports::audit_fields(app, "portable_data.exported", &[("export_mode", mode)])
        .await
}

#[tokio::test]
async fn data_export_health_data_json_returns_the_documented_shape() {
    let app = Application::new().await;
    let now = chrono::Utc::now().naive_utc();
    let location = rows::locations::ActiveModel {
        household_id: Set(app.fixture.household.id),
        name: Set("Unreferenced shelf".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let stock = rows::medications::ActiveModel {
        household_id: Set(app.fixture.household.id),
        location_id: Set(location.id),
        name: Set(Some("Unreferenced stock".into())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let med_pid = stock.portable_id;
    let loc_pid = location.portable_id;
    let linked_med_pid = app.fixture.medication.portable_id.clone();
    let token = app.token().await;
    let response = app
        .client
        .get(export_url(&app, "health_data_json"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let cache_control = response
        .headers()
        .get("cache-control")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let body: Value = response.json().await.unwrap();
    let audits = exported_events(&app, "health_data_json").await;
    let requests = super::portable_imports::audit_fields(
        &app,
        "api.request",
        &[
            ("controller", "api/v1/data_exports"),
            ("action", "show"),
            ("status", "200"),
        ],
    )
    .await;
    app.close().await;
    assert_eq!(status, 200);
    assert_eq!(cache_control.as_deref(), Some("no-store"));
    let data = &body["data"];
    let mut keys: Vec<&str> = data
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
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
    assert!(
        data["source_instance_id"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );
    chrono::DateTime::parse_from_rfc3339(data["exported_at"].as_str().unwrap()).unwrap();
    let records = &data["records"];
    for field in [
        "dosage_options",
        "locations",
        "medication_takes",
        "medications",
        "notification_preferences",
        "people",
        "person_medications",
        "schedules",
    ] {
        assert!(records[field].is_array(), "missing records field {field}");
    }
    for (field, rows) in records.as_object().unwrap() {
        assert!(rows.is_array(), "collection {field} is not an array");
    }
    assert!(!records["people"].as_array().unwrap().is_empty());
    let medication_ids: Vec<&str> = records["medications"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|row| row["portable_id"].as_str())
        .collect();
    let location_ids: Vec<&str> = records["locations"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|row| row["portable_id"].as_str())
        .collect();
    assert!(medication_ids.contains(&linked_med_pid.as_str()));
    assert!(!medication_ids.contains(&med_pid.as_str()));
    assert!(!location_ids.contains(&loc_pid.as_str()));
    assert!(!body.to_string().contains("password_hash"));
    assert_eq!(audits, 1);
    assert_eq!(requests, 1);
}

#[tokio::test]
async fn data_export_backup_zip_returns_a_base64_archive() {
    let app = Application::new().await;
    let token = app.token().await;
    let response = app
        .client
        .get(export_url(&app, "backup_zip"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let cache_control = response
        .headers()
        .get("cache-control")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let body: Value = response.json().await.unwrap();
    let audits = exported_events(&app, "backup_zip").await;
    app.close().await;
    assert_eq!(status, 200);
    assert_eq!(cache_control.as_deref(), Some("no-store"));
    let data = &body["data"];
    let mut keys: Vec<&str> = data
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["base64", "content_type", "filename"]);
    assert_eq!(data["content_type"], "application/zip");
    assert!(
        data["filename"]
            .as_str()
            .is_some_and(|name| name.starts_with("medtracker-backup-") && name.ends_with(".zip"))
    );
    let archive = STANDARD.decode(data["base64"].as_str().unwrap()).unwrap();
    let mut zip = zip::ZipArchive::new(Cursor::new(archive)).unwrap();
    assert_eq!(zip.len(), 1);
    let mut json = String::new();
    zip.by_name("medtracker-backup.json")
        .unwrap()
        .read_to_string(&mut json)
        .unwrap();
    let payload: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(payload["format"], "medtracker.backup.v1");
    assert!(!payload["records"]["people"].as_array().unwrap().is_empty());
    assert!(!json.contains("password_hash"));
    assert_eq!(audits, 1);
}

#[tokio::test]
async fn data_export_encrypted_migration_bundle_requires_a_passphrase() {
    let app = Application::new().await;
    let token = app.token().await;
    let missing = app
        .client
        .get(export_url(&app, "encrypted_migration_bundle"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let missing_status = missing.status().as_u16();
    let missing_cache = missing
        .headers()
        .get("cache-control")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let missing_body: Value = missing.json().await.unwrap();
    let blank = app
        .client
        .get(export_url(&app, "encrypted_migration_bundle"))
        .bearer_auth(&token)
        .header("x-medtracker-portable-passphrase", "   ")
        .send()
        .await
        .unwrap();
    let blank_status = blank.status().as_u16();
    let response = app
        .client
        .get(export_url(&app, "encrypted_migration_bundle"))
        .bearer_auth(&token)
        .header("x-medtracker-portable-passphrase", PASSPHRASE)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let cache_control = response
        .headers()
        .get("cache-control")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let body_text = response.text().await.unwrap();
    let body: Value = serde_json::from_str(&body_text).unwrap();
    let audits = exported_events(&app, "encrypted_migration_bundle").await;
    app.close().await;
    assert_eq!(missing_status, 422);
    assert_eq!(missing_cache.as_deref(), Some("no-store"));
    assert_eq!(missing_body["error"]["code"], "unprocessable_content");
    assert_eq!(
        missing_body["error"]["message"],
        "Portable passphrase header is required"
    );
    assert_eq!(blank_status, 422);
    assert_eq!(status, 200);
    assert_eq!(cache_control.as_deref(), Some("no-store"));
    let data = &body["data"];
    let mut keys: Vec<&str> = data
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "checksum",
            "cipher",
            "ciphertext",
            "encrypted_at",
            "format",
            "kdf",
            "salt"
        ]
    );
    assert_eq!(data["format"], "medtracker.portable.encrypted.v1");
    assert_eq!(data["cipher"], "aes-256-gcm");
    assert_eq!(data["kdf"], "pbkdf2_sha256");
    for field in ["salt", "checksum"] {
        let value = data[field].as_str().unwrap();
        assert_eq!(value.len(), 64);
        assert!(value.chars().all(|c| c.is_ascii_hexdigit()));
    }
    assert!(!data["ciphertext"].as_str().unwrap().is_empty());
    chrono::DateTime::parse_from_rfc3339(data["encrypted_at"].as_str().unwrap()).unwrap();
    assert!(!body_text.contains(PASSPHRASE));
    assert!(!body_text.contains("password_hash"));
    assert_eq!(audits, 1);
}

#[tokio::test]
async fn data_export_rejects_unknown_modes_and_anonymous_requests() {
    let app = Application::new().await;
    let foreign = app.fixture.foreign_household().await;
    let foreign_id = foreign.household.id;
    foreign.close().await;
    let token = app.token().await;
    let invalid = app
        .client
        .get(export_url(&app, "unsupported"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let invalid_status = invalid.status().as_u16();
    let invalid_cache = invalid
        .headers()
        .get("cache-control")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let invalid_body: Value = invalid.json().await.unwrap();
    let anonymous = app
        .client
        .get(export_url(&app, "health_data_json"))
        .send()
        .await
        .unwrap();
    let anonymous_status = anonymous.status().as_u16();
    let authenticate = anonymous
        .headers()
        .get("www-authenticate")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/{foreign_id}/data_exports/health_data_json",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let foreign_status = foreign.status().as_u16();
    let audits = exported_events(&app, "unsupported").await;
    app.close().await;
    assert_eq!(invalid_status, 422);
    assert_eq!(invalid_cache.as_deref(), Some("no-store"));
    assert_eq!(invalid_body["error"]["code"], "unprocessable_content");
    assert_eq!(
        invalid_body["error"]["message"],
        "Export mode is unsupported"
    );
    assert_eq!(anonymous_status, 401);
    assert_eq!(authenticate.as_deref(), Some("Bearer"));
    assert_eq!(foreign_status, 403);
    assert_eq!(audits, 0);
}
