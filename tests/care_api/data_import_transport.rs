use super::Application;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::json;
use std::io::{Cursor, Write};
use zip::{ZipWriter, write::SimpleFileOptions};

#[tokio::test]
async fn inspection_returns_the_prepared_inventory() {
    let app = Application::new().await;
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in [
        (
            "portable.json",
            include_bytes!("../fixtures/portable_rails/prepared-history-v2.json").as_slice(),
        ),
        (
            "manifest.json",
            include_bytes!("../fixtures/portable_rails/prepared-history-v2.manifest.json")
                .as_slice(),
        ),
    ] {
        zip.start_file(name, SimpleFileOptions::default()).unwrap();
        zip.write_all(bytes).unwrap();
    }
    let archive = zip.finish().unwrap().into_inner();
    let token = app.token().await;
    let response = app
        .client
        .post(format!(
            "{}/api/v1/households/{}/data_imports/inspections",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(token)
        .json(&json!({
            "source": "med_tracker05", "archive_base64": STANDARD.encode(archive),
            "passphrase": "password", "source_timezone": "Europe/London", "timezone_confirmed": true
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 201);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["data"]["id"].as_str().is_some());
    assert_eq!(body["data"]["archive_sha256"].as_str().unwrap().len(), 64);
    assert_eq!(body["data"]["timezone"]["name"], "Europe/London");
    assert!(
        !body["data"]["inventory"]["records"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    app.close().await;
}

#[tokio::test]
async fn inspection_authenticates_before_reading_invalid_or_oversized_body() {
    let app = Application::new().await;
    let endpoint = format!(
        "{}/api/v1/households/{}/data_imports/inspections",
        app.origin, app.fixture.household.id
    );
    let malformed = app
        .client
        .post(&endpoint)
        .header("content-type", "application/json")
        .body("{")
        .send()
        .await
        .unwrap();
    assert_eq!(malformed.status().as_u16(), 401);

    let service = app
        .context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .unwrap();
    let key = service
        .api_keys()
        .unwrap()
        .create(
            &app.fixture.account.id.to_string(),
            better_auth::server_api::CreateKeyOptions {
                config_id: Some("medtracker-personal".into()),
                name: Some("Read only import test".into()),
                expires_in: Some(chrono::Duration::days(1).num_seconds() as f64),
                metadata: Some(json!({"households": [app.fixture.household.id]})),
                permissions: Some(std::collections::HashMap::from([(
                    "medtracker".into(),
                    vec!["care:read".into()],
                )])),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .key;
    for body in ["{".to_owned(), "x".repeat(90 * 1024 * 1024)] {
        let response = app
            .client
            .post(&endpoint)
            .bearer_auth(&key)
            .header("content-type", "application/json")
            .body(body)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 403);
    }
    app.close().await;
}

#[tokio::test]
async fn inspection_rejects_unconfirmed_and_invalid_timezones() {
    let app = Application::new().await;
    let endpoint = format!(
        "{}/api/v1/households/{}/data_imports/inspections",
        app.origin, app.fixture.household.id
    );
    let token = app.token().await;
    for (zone, confirmed) in [("Europe/London", false), ("not/a-zone", true)] {
        let response = app
            .client
            .post(&endpoint)
            .bearer_auth(&token)
            .json(&json!({
                "source": "med_tracker05", "archive_base64": "",
                "passphrase": "synthetic", "source_timezone": zone,
                "timezone_confirmed": confirmed
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 422);
    }
    app.close().await;
}
