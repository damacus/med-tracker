use super::*;
use med_tracker::models::entities::{carer_relationship, grant, membership};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

const AUTHORITY_READS: [&str; 3] = [
    "mobile_snapshot",
    "portable_export?version=2",
    "data_exports/health_data_json",
];

async fn authority_payload(app: &Application, token: &str, path: &str) -> Value {
    let response = app
        .client
        .get(format!("{}/api/v1/households/72001/{path}", app.origin))
        .bearer_auth(token)
        .header(
            "x-medtracker-portable-passphrase",
            "synthetic authority secret",
        )
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200, "{path}");
    let body: Value = response.json().await.unwrap();
    if path.starts_with("portable_export") {
        med_tracker::models::care::portable_crypto::decrypt(
            &body["data"],
            "synthetic authority secret",
        )
        .unwrap()
    } else {
        body["data"].clone()
    }
}

async fn legacy_manage(app: &Application, person_id: i64) {
    let now = chrono::Utc::now().naive_utc();
    grant::ActiveModel {
        household_id: Set(72001),
        household_membership_id: Set(74001),
        person_id: Set(person_id),
        access_level: Set("manage".into()),
        relationship_type: Set("parent".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
}

#[tokio::test]
async fn mobile_snapshot_uses_self_and_live_guardian_authority_instead_of_manage_grants() {
    let app = Application::new().await;
    let token = app.token().await;
    let created = app.client.post(format!("{}/api/v1/households/72001/people", app.origin))
        .bearer_auth(&token)
        .json(&json!({"person":{"name":"Declared snapshot child","date_of_birth":"2020-01-01","person_type":"minor","has_capacity":false,"initial_guardian_declaration":true}}))
        .send().await.unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let child: Value = created.json().await.unwrap();
    let child_id = child["data"]["id"].as_i64().unwrap();
    let schedule = app.client.post(format!("{}/api/v1/households/72001/schedules", app.origin))
        .bearer_auth(&token)
        .json(&json!({"schedule":{"person_id":child_id.to_string(),"medication_id":"80001","dose_amount":"2","dose_unit":"tablet","start_date":"2026-01-01","end_date":"2027-01-01","schedule_type":"prn","schedule_config":{},"dose_cycle":"daily"}}))
        .send().await.unwrap();
    assert_eq!(schedule.status().as_u16(), 201);
    let schedule: Value = schedule.json().await.unwrap();
    let schedule_pid = &schedule["data"]["portable_id"];
    assert!(schedule_pid.is_string());
    membership::ActiveModel {
        id: Set(74001),
        role: Set("member".into()),
        ..Default::default()
    }
    .update(&app.fixture.admin)
    .await
    .unwrap();
    grant::ActiveModel {
        id: Set(78001),
        revoked_at: Set(Some(chrono::Utc::now().naive_utc())),
        ..Default::default()
    }
    .update(&app.fixture.admin)
    .await
    .unwrap();
    legacy_manage(&app, 73002).await;
    for path in AUTHORITY_READS {
        let payload = authority_payload(&app, &token, path).await;
        let people = payload["records"]["people"].as_array().unwrap();
        assert_eq!(people.len(), 2, "{path}");
        assert!(people.iter().any(|row| row["name"] == "Synthetic adult"));
        assert!(
            people
                .iter()
                .any(|row| row["name"] == "Declared snapshot child")
        );
        assert!(!people.iter().any(|row| row["name"] == "Synthetic minor"));
        assert!(
            payload["records"]["schedules"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| &row["portable_id"] == schedule_pid),
            "{path}"
        );
    }
    let declaration = carer_relationship::Entity::find()
        .filter(carer_relationship::Column::PatientId.eq(child_id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
        .guardian_declaration_id
        .unwrap();
    let withdrawn = app
        .client
        .patch(format!(
            "{}/api/v1/auth/guardian-declarations/{declaration}",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&json!({"guardian_declaration":{"withdrawn":true,"revision":"1"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(withdrawn.status().as_u16(), 200);
    legacy_manage(&app, child_id).await;
    for path in AUTHORITY_READS {
        let payload = authority_payload(&app, &token, path).await;
        let people = payload["records"]["people"].as_array().unwrap();
        assert_eq!(people.len(), 1, "{path}");
        assert_eq!(people[0]["name"], "Synthetic adult");
        assert!(
            !payload["records"]["schedules"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| &row["portable_id"] == schedule_pid),
            "{path}"
        );
    }
    app.close().await;
}

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
    app.fixture.seed_foreign_household(99999).await;
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
