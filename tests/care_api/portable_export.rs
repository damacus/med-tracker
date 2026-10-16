use super::*;
use med_tracker::models::care::portable_crypto;

const PASSPHRASE: &str = "synthetic portable secret";

fn export_url(app: &Application, query: &str) -> String {
    format!(
        "{}/api/v1/households/72001/portable_export{query}",
        app.origin
    )
}

async fn export_events(app: &Application) -> i64 {
    app.fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM security_audit_events WHERE event_type='portable_data.exported' AND metadata->>'export_mode'='encrypted_migration_bundle' AND metadata->>'encrypted'='true'",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "count")
        .unwrap()
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
    let requests: i64 = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM security_audit_events WHERE event_type='api.request' AND metadata->>'controller'='api/v1/portable_exports' AND metadata->>'action'='show' AND metadata->>'status'='200'",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "count")
        .unwrap();
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
    app.fixture.seed_foreign_household(99999).await;
    let token = app.token().await;
    let mut statuses = Vec::new();
    let mut messages = Vec::new();
    for query in [
        "?version=3",
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
            "{}/api/v1/households/99999/portable_export",
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
