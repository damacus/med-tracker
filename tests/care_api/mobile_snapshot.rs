use super::*;

fn snapshot_url(app: &Application) -> String {
    format!("{}/api/v1/households/72001/mobile_snapshot", app.origin)
}

async fn snapshot_events(app: &Application) -> i64 {
    app.fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM security_audit_events WHERE event_type='portable_data.mobile_snapshot_read' AND metadata->>'export_mode'='mobile_snapshot'",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "count")
        .unwrap()
}

#[tokio::test]
async fn mobile_snapshot_returns_a_v1_portable_payload() {
    let app = Application::new().await;
    let token = app.token().await;
    let response = app
        .client
        .get(snapshot_url(&app))
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
    let audits = snapshot_events(&app).await;
    let requests: i64 = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM security_audit_events WHERE event_type='api.request' AND metadata->>'controller'='api/v1/mobile_snapshots' AND metadata->>'action'='show' AND metadata->>'status'='200'",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "count")
        .unwrap();
    app.close().await;
    assert_eq!(status, 200);
    assert_ne!(cache_control.as_deref(), Some("no-store"));
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
    assert_eq!(data["format"], "medtracker.portable.v1");
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
    for field in [
        "dose_occurrences",
        "medication_pause_periods",
        "api_sessions",
        "security_audit_events",
    ] {
        assert!(
            records.get(field).is_none(),
            "unexpected collection {field}"
        );
    }
    assert!(!records["people"].as_array().unwrap().is_empty());
    assert!(!body.to_string().contains("password_hash"));
    assert_eq!(audits, 1);
    assert_eq!(requests, 1);
}

#[tokio::test]
async fn mobile_snapshot_rejects_anonymous_and_foreign_households() {
    let app = Application::new().await;
    let token = app.token().await;
    let anonymous = app.client.get(snapshot_url(&app)).send().await.unwrap();
    let anonymous_status = anonymous.status().as_u16();
    let authenticate = anonymous
        .headers()
        .get("www-authenticate")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/99999/mobile_snapshot",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let foreign_status = foreign.status().as_u16();
    let audits = snapshot_events(&app).await;
    app.close().await;
    assert_eq!(anonymous_status, 401);
    assert_eq!(authenticate.as_deref(), Some("Bearer"));
    assert_eq!(foreign_status, 403);
    assert_eq!(audits, 0);
}
