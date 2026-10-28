use super::*;

use med_tracker::models::care::portable_crypto;
use med_tracker::models::entities::{location, medication, membership};
use sea_orm::{ActiveModelTrait, Set};

const PASSPHRASE: &str = "synthetic portable secret";

#[tokio::test]
async fn portable_v3_export_preserves_inventory_and_immutable_dose_locations_without_person_links()
{
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    membership::ActiveModel {
        id: Set(app.fixture.membership.id),
        role: Set("member".into()),
        ..Default::default()
    }
    .update(&app.fixture.admin)
    .await
    .unwrap();
    let token = app.token().await;
    let take = app
        .client
        .post(format!(
            "{}/api/v1/households/{fixture_household_id}/medication_takes",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&shared_take_body(&app.fixture))
        .send()
        .await
        .unwrap();
    assert_eq!(take.status().as_u16(), 201);
    let now = chrono::Utc::now().naive_utc();
    let moved = location::ActiveModel {
        household_id: Set(app.fixture.household.id),
        portable_id: Set(uuid::Uuid::new_v4().to_string()),
        name: Set("Relocated inventory".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    medication::ActiveModel {
        id: Set(app.fixture.medication.id),
        location_id: Set(moved.id),
        ..Default::default()
    }
    .update(&app.fixture.admin)
    .await
    .unwrap();
    let mut payloads = Vec::new();
    for query in [
        "?version=2",
        "?version=3",
        "?portable_format=medtracker.portable.v3",
    ] {
        let response = app
            .client
            .get(export_url(&app, query))
            .bearer_auth(&token)
            .header("x-medtracker-portable-passphrase", PASSPHRASE)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 200, "{query}");
        let body: Value = response.json().await.unwrap();
        payloads.push(portable_crypto::decrypt(&body["data"], PASSPHRASE).unwrap());
    }
    let legacy = &payloads[0];
    assert!(
        !legacy["records"]["medication_takes"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        legacy["records"]["medication_takes"][0]["taken_from_location_portable_id"].is_string()
    );
    assert!(legacy["records"]["medications"][0]["location_portable_id"].is_string());
    assert_ne!(
        legacy["records"]["medications"][0]["location_portable_id"],
        legacy["records"]["medication_takes"][0]["taken_from_location_portable_id"]
    );
    let before = super::portable_imports::import_effects(&app).await;
    let bundle = portable_crypto::encrypt(&payloads[1], PASSPHRASE).unwrap();
    let denied = app
        .client
        .post(format!(
            "{}/api/v1/households/{fixture_household_id}/portable_imports",
            app.origin
        ))
        .bearer_auth(&token)
        .header("x-medtracker-portable-passphrase", PASSPHRASE)
        .json(&json!({"bundle":bundle}))
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status().as_u16(), 403);
    assert_eq!(super::portable_imports::import_effects(&app).await, before);
    membership::ActiveModel {
        id: Set(app.fixture.membership.id),
        role: Set("administrator".into()),
        ..Default::default()
    }
    .update(&app.fixture.admin)
    .await
    .unwrap();
    let before = super::portable_imports::import_effects(&app).await;
    let mut altered = payloads[1].clone();
    altered["records"]["medication_takes"][0]["taken_from_location_portable_id"] =
        altered["records"]["medications"][0]["location_portable_id"].clone();
    let bundle = portable_crypto::encrypt(&altered, PASSPHRASE).unwrap();
    let rejected = app
        .client
        .post(format!(
            "{}/api/v1/households/{fixture_household_id}/portable_imports",
            app.origin
        ))
        .bearer_auth(&token)
        .header("x-medtracker-portable-passphrase", PASSPHRASE)
        .json(&json!({"bundle":bundle}))
        .send()
        .await
        .unwrap();
    assert_eq!(rejected.status().as_u16(), 422);
    let rejected: Value = rejected.json().await.unwrap();
    assert!(
        rejected["data"]["errors"]
            .to_string()
            .contains("conflicts with existing history"),
        "{rejected}"
    );
    assert_eq!(super::portable_imports::import_effects(&app).await, before);
    for payload in &payloads[1..] {
        assert_eq!(payload["format"], "medtracker.portable.v3");
        for person in payload["records"]["people"].as_array().unwrap() {
            assert!(person.get("location_portable_ids").is_none());
        }
        for collection in [
            "medication_takes",
            "medications",
            "locations",
            "medication_pause_periods",
            "dose_occurrences",
        ] {
            assert_eq!(
                payload["records"][collection], legacy["records"][collection],
                "{collection}"
            );
        }
        let before = super::portable_imports::import_effects(&app).await;
        let bundle = portable_crypto::encrypt(payload, PASSPHRASE).unwrap();
        let imported = app
            .client
            .post(format!(
                "{}/api/v1/households/{fixture_household_id}/portable_imports",
                app.origin
            ))
            .bearer_auth(&token)
            .header("x-medtracker-portable-passphrase", PASSPHRASE)
            .json(&json!({"bundle":bundle}))
            .send()
            .await
            .unwrap();
        let status = imported.status().as_u16();
        let body: Value = imported.json().await.unwrap();
        assert_eq!(status, 201, "{body}");
        assert_eq!(super::portable_imports::import_effects(&app).await, before);
    }
    app.close().await;
}

fn export_url(app: &Application, query: &str) -> String {
    let fixture_household_id = app.fixture.household.id;
    format!(
        "{}/api/v1/households/{fixture_household_id}/portable_export{query}",
        app.origin
    )
}

async fn export_events(app: &Application) -> i64 {
    super::portable_imports::audit_fields(
        app,
        "portable_data.exported",
        &[
            ("export_mode", "encrypted_migration_bundle"),
            ("encrypted", "true"),
        ],
    )
    .await
}

#[tokio::test]
async fn portable_export_requires_a_passphrase_and_returns_an_encrypted_envelope() {
    let app = Application::new().await;
    let token = app.token().await;
    let missing = app
        .client
        .get(export_url(&app, ""))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let missing_status = missing.status().as_u16();
    let missing_body: Value = missing.json().await.unwrap();
    let response = app
        .client
        .get(export_url(&app, ""))
        .bearer_auth(&token)
        .header("x-medtracker-portable-passphrase", PASSPHRASE)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body_text = response.text().await.unwrap();
    let body: Value = serde_json::from_str(&body_text).unwrap();
    let audits = export_events(&app).await;
    let requests = super::portable_imports::audit_fields(
        &app,
        "api.request",
        &[
            ("controller", "api/v1/portable_exports"),
            ("action", "show"),
            ("status", "200"),
        ],
    )
    .await;
    app.close().await;
    assert_eq!(missing_status, 422);
    assert_eq!(missing_body["error"]["code"], "unprocessable_content");
    assert_eq!(
        missing_body["error"]["message"],
        "Portable passphrase header is required"
    );
    assert_eq!(status, 200);
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
    assert!(!body_text.contains(PASSPHRASE));
    assert!(!body_text.contains("password_hash"));
    assert_eq!(audits, 1);
    assert_eq!(requests, 1);
}

#[tokio::test]
async fn portable_export_selects_the_requested_portable_format() {
    let app = Application::new().await;
    let token = app.token().await;
    let mut bodies = Vec::new();
    for query in ["", "?version=2", "?portable_format=medtracker.portable.v2"] {
        let response = app
            .client
            .get(export_url(&app, query))
            .bearer_auth(&token)
            .header("x-medtracker-portable-passphrase", PASSPHRASE)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 200, "{query}");
        bodies.push(response.json::<Value>().await.unwrap());
    }
    app.close().await;
    let v1 = portable_crypto::decrypt(&bodies[0]["data"], PASSPHRASE).unwrap();
    assert_eq!(v1["format"], "medtracker.portable.v1");
    assert!(v1["records"]["dose_occurrences"].is_null());
    assert!(v1["records"]["medication_pause_periods"].is_null());
    assert!(!v1["records"]["people"].as_array().unwrap().is_empty());
    for (index, body) in bodies.iter().enumerate().skip(1) {
        let v2 = portable_crypto::decrypt(&body["data"], PASSPHRASE).unwrap();
        assert_eq!(v2["format"], "medtracker.portable.v2", "bundle {index}");
        assert!(v2["records"]["dose_occurrences"].is_array());
        assert!(v2["records"]["medication_pause_periods"].is_array());
    }
}

#[tokio::test]
async fn portable_export_rejects_invalid_options_and_foreign_households() {
    let app = Application::new().await;
    let foreign = app.fixture.foreign_household().await;
    let foreign_id = foreign.household.id;
    foreign.close().await;
    let token = app.token().await;
    let mut statuses = Vec::new();
    let mut messages = Vec::new();
    for query in [
        "?version=4",
        "?portable_format=medtracker.portable.invalid",
        "?version=1&version=2",
    ] {
        let response = app
            .client
            .get(export_url(&app, query))
            .bearer_auth(&token)
            .header("x-medtracker-portable-passphrase", PASSPHRASE)
            .send()
            .await
            .unwrap();
        statuses.push(response.status().as_u16());
        messages.push(
            response.json::<Value>().await.unwrap()["error"]["message"]
                .as_str()
                .unwrap()
                .to_owned(),
        );
    }
    let anonymous = app
        .client
        .get(export_url(&app, ""))
        .header("x-medtracker-portable-passphrase", PASSPHRASE)
        .send()
        .await
        .unwrap();
    let anonymous_status = anonymous.status().as_u16();
    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/{foreign_id}/portable_export",
            app.origin
        ))
        .bearer_auth(&token)
        .header("x-medtracker-portable-passphrase", PASSPHRASE)
        .send()
        .await
        .unwrap();
    let foreign_status = foreign.status().as_u16();
    let audits = export_events(&app).await;
    app.close().await;
    assert_eq!(statuses, vec![422, 422, 422]);
    assert_eq!(
        messages,
        vec![
            "Unsupported portable data version",
            "Unsupported portable data format",
            "Export query is invalid"
        ]
    );
    assert_eq!(anonymous_status, 401);
    assert_eq!(foreign_status, 403);
    assert_eq!(audits, 0);
}
