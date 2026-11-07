use super::Application;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::json;
use std::io::{Cursor, Write};
use zip::{ZipWriter, write::SimpleFileOptions};

async fn read_status_line(socket: &mut tokio::net::TcpStream) -> String {
    use tokio::io::AsyncReadExt;
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        let mut line = Vec::new();
        while line.len() < 1024 {
            let byte = socket.read_u8().await.unwrap();
            line.push(byte);
            if line.ends_with(b"\r\n") {
                return String::from_utf8(line).unwrap();
            }
        }
        panic!("HTTP status line exceeded limit");
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn revoked_key_during_upload_cannot_persist_inspection_or_preview() {
    use med_tracker::models::{
        _entities::{data_import_inspections, data_import_runs},
        entities::identity_api_key,
    };
    use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, sea_query::Expr};
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpStream;
    use tokio::time::Duration;

    let app = Application::new().await;
    let household = app.fixture.household.id;
    let token = app.token().await;
    let base = format!("{}/api/v1/households/{household}/data_imports", app.origin);
    let payload: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../fixtures/portable_rails/prepared-history-v2.json"
    ))
    .unwrap();
    let upload = json!({"source":"med_tracker05", "archive_base64":archive_with_payload(&payload),
        "passphrase":"password", "source_timezone":"Europe/London", "timezone_confirmed":true});
    let response = app
        .client
        .post(format!("{base}/inspections"))
        .bearer_auth(&token)
        .json(&upload)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 201);
    let inspection: serde_json::Value = response.json().await.unwrap();
    let source = inspection["data"]["inventory"]["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["identity"]["kind"] == "person")
        .unwrap()["identity"]
        .clone();
    let omissions: Vec<_> = inspection["data"]["inventory"]["omissions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["acknowledgement_id"].clone())
        .collect();
    let mut preview = upload.clone();
    preview["inspection_id"] = inspection["data"]["id"].clone();
    preview["mappings"] = json!([{"source":source,"destination":"NewDependent"}]);
    preview["declarations"] = json!([]);
    preview["recipient_consents"] = json!([]);
    preview["acknowledged_omissions"] = json!(omissions);

    for (route, input) in [("inspections", upload), ("previews", preview)] {
        let before_inspections = data_import_inspections::Entity::find()
            .filter(data_import_inspections::Column::HouseholdId.eq(household))
            .count(&app.fixture.admin)
            .await
            .unwrap();
        let before_runs = data_import_runs::Entity::find()
            .filter(data_import_runs::Column::HouseholdId.eq(household))
            .count(&app.fixture.admin)
            .await
            .unwrap();
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
                    name: Some("Upload revocation test".into()),
                    expires_in: Some(chrono::Duration::days(1).num_seconds() as f64),
                    metadata: Some(json!({"households":[household]})),
                    permissions: Some(std::collections::HashMap::from([(
                        "medtracker".into(),
                        vec!["care:write".into()],
                    )])),
                    ..Default::default()
                },
            )
            .await
            .unwrap()
            .key;
        let bytes = serde_json::to_vec(&input).unwrap();
        let address = app.origin.strip_prefix("http://").unwrap();
        let mut socket = TcpStream::connect(address).await.unwrap();
        socket.write_all(format!("POST /api/v1/households/{household}/data_imports/{route} HTTP/1.1\r\nHost: {address}\r\nAuthorization: Bearer {key}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n", bytes.len()).as_bytes()).await.unwrap();
        socket.write_all(&bytes[..1]).await.unwrap();
        let mut acquired = false;
        for _ in 0..100 {
            let check = app
                .client
                .post(format!("{base}/inspections"))
                .bearer_auth(&token)
                .header("content-type", "application/json")
                .body("{")
                .send()
                .await
                .unwrap();
            if check.status().as_u16() == 503 {
                acquired = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(
            acquired,
            "the original request must reach upload buffering before revocation"
        );
        identity_api_key::Entity::update_many()
            .col_expr(identity_api_key::Column::Enabled, Expr::value(false))
            .filter(identity_api_key::Column::ReferenceId.eq(app.fixture.account.id))
            .exec(&app.fixture.admin)
            .await
            .unwrap();
        socket.write_all(&bytes[1..]).await.unwrap();
        let status = read_status_line(&mut socket).await;
        assert!(status.starts_with("HTTP/1.1 401") || status.starts_with("HTTP/1.1 403"));
        assert_eq!(
            data_import_inspections::Entity::find()
                .filter(data_import_inspections::Column::HouseholdId.eq(household))
                .count(&app.fixture.admin)
                .await
                .unwrap(),
            before_inspections
        );
        assert_eq!(
            data_import_runs::Entity::find()
                .filter(data_import_runs::Column::HouseholdId.eq(household))
                .count(&app.fixture.admin)
                .await
                .unwrap(),
            before_runs
        );
    }
    app.close().await;
}

fn archive_with_payload(payload: &serde_json::Value) -> String {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file("portable.json", SimpleFileOptions::default())
        .unwrap();
    zip.write_all(&serde_json::to_vec(payload).unwrap())
        .unwrap();
    zip.start_file("manifest.json", SimpleFileOptions::default())
        .unwrap();
    zip.write_all(include_bytes!(
        "../fixtures/portable_rails/prepared-history-v2.manifest.json"
    ))
    .unwrap();
    STANDARD.encode(zip.finish().unwrap().into_inner())
}

#[tokio::test]
async fn inspection_reports_bounded_safe_item_diagnostics() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{}/data_imports/inspections",
        app.origin, app.fixture.household.id
    );
    let original: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../fixtures/portable_rails/prepared-history-v2.json"
    ))
    .unwrap();
    let mut unknown = original.clone();
    for index in 0..101 {
        unknown["records"]["people"][0][format!("secret-field-{index}")] = json!("clinical-secret");
    }
    let mut invalid = original;
    invalid["records"]["schedules"][0]["dose_amount"] = json!("0.001");
    for (payload, path, reason, truncated) in [
        (
            unknown,
            "/records/people/0/[unsupported]",
            "unsupported_field",
            true,
        ),
        (
            invalid,
            "/records/schedules/0/dose_amount",
            "invalid_field",
            false,
        ),
    ] {
        let response = app.client.post(&endpoint).bearer_auth(&token).json(&json!({
            "source":"med_tracker05", "archive_base64":archive_with_payload(&payload),
            "passphrase":"password", "source_timezone":"Europe/London", "timezone_confirmed":true
        })).send().await.unwrap();
        assert_eq!(response.status().as_u16(), 422);
        let body: serde_json::Value = response.json().await.unwrap();
        let errors = &body["error"]["errors"];
        let items = errors["items"].as_array().unwrap();
        assert!(
            items
                .iter()
                .any(|item| item["path"] == path && item["reason"] == reason)
        );
        assert!(items.len() <= 100);
        assert_eq!(errors["truncated"], truncated);
        assert!(!body.to_string().contains("clinical-secret"));
        assert!(!body.to_string().contains("secret-field"));
        assert!(!body.to_string().contains("password"));
    }
    app.close().await;
}

#[tokio::test]
async fn preview_and_reports_follow_inspection_and_household_boundaries() {
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
    let archive = STANDARD.encode(zip.finish().unwrap().into_inner());
    let token = app.token().await;
    let base = format!(
        "{}/api/v1/households/{}/data_imports",
        app.origin, app.fixture.household.id
    );
    let upload = json!({"source":"med_tracker05", "archive_base64":archive,
        "passphrase":"password", "source_timezone":"Europe/London", "timezone_confirmed":true});
    let inspected = app
        .client
        .post(format!("{base}/inspections"))
        .bearer_auth(&token)
        .json(&upload)
        .send()
        .await
        .unwrap();
    assert_eq!(inspected.status().as_u16(), 201);
    let inspected: serde_json::Value = inspected.json().await.unwrap();
    let source = inspected["data"]["inventory"]["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["identity"]["kind"] == "person")
        .unwrap()["identity"]
        .clone();
    let omissions: Vec<_> = inspected["data"]["inventory"]["omissions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["acknowledgement_id"].clone())
        .collect();
    let mut input = upload;
    input["inspection_id"] = inspected["data"]["id"].clone();
    input["mappings"] = json!([{"source":source,"destination":"NewDependent"}]);
    input["declarations"] = json!([]);
    input["recipient_consents"] = json!([]);
    input["acknowledged_omissions"] = json!(omissions);
    let preview = app
        .client
        .post(format!("{base}/previews"))
        .bearer_auth(&token)
        .json(&input)
        .send()
        .await
        .unwrap();
    assert_eq!(preview.status().as_u16(), 201);
    let preview: serde_json::Value = preview.json().await.unwrap();
    assert_eq!(preview["data"]["report"]["state"], "blocked");
    assert!(preview["data"]["confirmation"].is_null());
    let id = preview["data"]["report"]["id"].as_str().unwrap();
    let listed = app
        .client
        .get(format!("{base}?limit=1"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(listed.status().as_u16(), 200);
    let listed: serde_json::Value = listed.json().await.unwrap();
    assert_eq!(listed["data"]["reports"][0]["id"], id);
    let detail = app
        .client
        .get(format!("{base}/{id}"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(detail.status().as_u16(), 200);
    let detail: serde_json::Value = detail.json().await.unwrap();
    assert!(detail["data"]["confirmation"].is_null());
    let invalid_limit = app
        .client
        .get(format!("{base}?limit=101"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(invalid_limit.status().as_u16(), 422);
    let foreign = app.fixture.foreign_household().await;
    let denied = app
        .client
        .get(format!(
            "{}/api/v1/households/{}/data_imports/{id}",
            app.origin, foreign.household.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert!(matches!(denied.status().as_u16(), 403 | 404));
    app.close().await;
}

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
    for body in ["{".to_owned(), "x".repeat(90 * 1024 * 1024 + 1)] {
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
    let preview = app
        .client
        .post(format!(
            "{}/api/v1/households/{}/data_imports/previews",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(&key)
        .header("content-type", "application/json")
        .body("{")
        .send()
        .await
        .unwrap();
    assert_eq!(preview.status().as_u16(), 403);
    app.close().await;
}

#[tokio::test]
async fn authorised_inspection_rejects_invalid_base64_and_body_over_limit() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{}/data_imports/inspections",
        app.origin, app.fixture.household.id
    );
    let invalid = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&json!({
            "source":"med_tracker05", "archive_base64":"%%%", "passphrase":"password",
            "source_timezone":"Europe/London", "timezone_confirmed":true
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(invalid.status().as_u16(), 422);
    let oversized = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("content-type", "application/json")
        .body(vec![b'x'; 90 * 1024 * 1024 + 1])
        .send()
        .await
        .unwrap();
    assert_eq!(oversized.status().as_u16(), 422);
    app.close().await;
}

#[tokio::test]
async fn inspection_enforces_decoded_archive_boundary() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{}/data_imports/inspections",
        app.origin, app.fixture.household.id
    );
    for (size, message) in [
        (64 * 1024 * 1024, "Import archive could not be prepared"),
        (64 * 1024 * 1024 + 1, "Import archive is too large"),
    ] {
        let response = app.client.post(&endpoint).bearer_auth(&token).json(&json!({
            "source":"med_tracker05", "archive_base64":STANDARD.encode(vec![0_u8; size]),
            "passphrase":"password", "source_timezone":"Europe/London", "timezone_confirmed":true
        })).send().await.unwrap();
        assert_eq!(response.status().as_u16(), 422);
        let body: serde_json::Value = response.json().await.unwrap();
        assert_eq!(body["error"]["message"], message);
    }
    app.close().await;
}

#[tokio::test]
async fn inspection_rejects_before_body_and_limits_concurrent_uploads() {
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpStream;
    use tokio::time::Duration;

    let app = Application::new().await;
    let address = app.origin.strip_prefix("http://").unwrap();
    let path = format!(
        "/api/v1/households/{}/data_imports/inspections",
        app.fixture.household.id
    );
    let mut anonymous = TcpStream::connect(address).await.unwrap();
    anonymous.write_all(format!("POST {path} HTTP/1.1\r\nHost: {address}\r\nContent-Length: 1024\r\nContent-Type: application/json\r\n\r\n").as_bytes()).await.unwrap();
    assert!(
        read_status_line(&mut anonymous)
            .await
            .starts_with("HTTP/1.1 401")
    );
    drop(anonymous);

    let service = app
        .context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .unwrap();
    let read_only = service
        .api_keys()
        .unwrap()
        .create(
            &app.fixture.account.id.to_string(),
            better_auth::server_api::CreateKeyOptions {
                config_id: Some("medtracker-personal".into()),
                name: Some("Stalled read-only key".into()),
                expires_in: Some(chrono::Duration::days(1).num_seconds() as f64),
                metadata: Some(json!({"households":[app.fixture.household.id]})),
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
    let mut rejected = TcpStream::connect(address).await.unwrap();
    rejected.write_all(format!("POST {path} HTTP/1.1\r\nHost: {address}\r\nAuthorization: Bearer {read_only}\r\nContent-Length: 1024\r\nContent-Type: application/json\r\n\r\n").as_bytes()).await.unwrap();
    assert!(
        read_status_line(&mut rejected)
            .await
            .starts_with("HTTP/1.1 403")
    );
    drop(rejected);

    let token = app.token().await;
    let mut stalled = TcpStream::connect(address).await.unwrap();
    stalled.write_all(format!("POST {path} HTTP/1.1\r\nHost: {address}\r\nAuthorization: Bearer {token}\r\nContent-Length: 1024\r\nContent-Type: application/json\r\n\r\n").as_bytes()).await.unwrap();
    let endpoint = format!("{}{path}", app.origin);
    let mut busy = false;
    for _ in 0..100 {
        let response = app
            .client
            .post(&endpoint)
            .bearer_auth(&token)
            .header("content-type", "application/json")
            .body("{")
            .send()
            .await
            .unwrap();
        if response.status().as_u16() == 503 {
            busy = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(busy, "a stalled upload must hold the shared permit");
    drop(stalled);
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
