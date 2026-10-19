use super::*;
use med_tracker::models::care::portable_crypto;

const PASSPHRASE: &str = "synthetic portable secret";
const PASSPHRASE_HEADER: &str = "x-medtracker-portable-passphrase";

fn import_url(app: &Application, endpoint: &str) -> String {
    format!("{}/api/v1/households/72001/{endpoint}", app.origin)
}

async fn export_bundle(app: &Application, token: &str) -> Value {
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/data_exports/encrypted_migration_bundle",
            app.origin
        ))
        .bearer_auth(token)
        .header(PASSPHRASE_HEADER, PASSPHRASE)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    response.json::<Value>().await.unwrap()["data"].clone()
}

async fn audit_count(app: &Application, event: &str, clause: &str) -> i64 {
    app.fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            format!(
                "SELECT count(*) AS count FROM security_audit_events WHERE event_type='{event}' AND {clause}"
            ),
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "count")
        .unwrap()
}

async fn version_count(app: &Application) -> i64 {
    app.fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM versions WHERE household_id = 72001",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "count")
        .unwrap()
}

#[tokio::test]
async fn portable_imports_validate_the_request_shape_and_credentials() {
    let app = Application::new().await;
    app.fixture.seed_foreign_household(99999).await;
    let token = app.token().await;
    let mut statuses = Vec::new();
    let mut messages = Vec::new();
    for (endpoint, body, passphrase) in [
        ("portable_imports/dry_run", json!({}), Some(PASSPHRASE)),
        ("portable_imports", json!({}), Some(PASSPHRASE)),
        (
            "portable_imports/dry_run",
            json!({"bundle": {"format": "x"}, "unexpected": true}),
            Some(PASSPHRASE),
        ),
        (
            "portable_imports/dry_run",
            json!({"bundle": {"format": "x"}}),
            None,
        ),
        (
            "portable_imports/dry_run",
            json!({"bundle": "not an envelope"}),
            Some(PASSPHRASE),
        ),
        (
            "portable_imports/dry_run",
            json!("not an object"),
            Some(PASSPHRASE),
        ),
    ] {
        let mut request = app
            .client
            .post(import_url(&app, endpoint))
            .bearer_auth(&token)
            .json(&body);
        if let Some(passphrase) = passphrase {
            request = request.header(PASSPHRASE_HEADER, passphrase);
        }
        let response = request.send().await.unwrap();
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
        .post(import_url(&app, "portable_imports/dry_run"))
        .header(PASSPHRASE_HEADER, PASSPHRASE)
        .json(&json!({"bundle": {}}))
        .send()
        .await
        .unwrap();
    let anonymous_status = anonymous.status().as_u16();
    let foreign = app
        .client
        .post(format!(
            "{}/api/v1/households/99999/portable_imports/dry_run",
            app.origin
        ))
        .bearer_auth(&token)
        .header(PASSPHRASE_HEADER, PASSPHRASE)
        .json(&json!({"bundle": {}}))
        .send()
        .await
        .unwrap();
    let foreign_status = foreign.status().as_u16();
    app.close().await;
    assert_eq!(statuses, vec![400, 400, 422, 422, 422, 400]);
    assert_eq!(
        messages,
        vec![
            "Portable bundle is required",
            "Portable bundle is required",
            "Unsupported import request fields",
            "Portable passphrase header is required",
            "Portable bundle could not be decrypted",
            "Portable bundle is required",
        ]
    );
    assert_eq!(anonymous_status, 401);
    assert_eq!(foreign_status, 403);
}

fn importable_payload() -> Value {
    json!({
        "format": "medtracker.portable.v1",
        "scope": "household",
        "exported_at": "2026-10-11T10:00:00Z",
        "source_instance_id": "synthetic:test",
        "records": {
            "locations": [{
                "portable_id": "imported-location-0001",
                "name": "Imported Pharmacy",
                "description": "restored from a portable bundle"
            }],
            "people": [{
                "portable_id": "imported-person-0001",
                "name": "Imported Adult",
                "email": "imported.adult@example.com",
                "date_of_birth": "1990-01-15",
                "person_type": "adult",
                "has_capacity": true,
                "location_portable_ids": ["imported-location-0001"]
            }]
        }
    })
}

async fn table_count(app: &Application, table: &str) -> i64 {
    app.fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            format!("SELECT count(*) AS count FROM {table} WHERE household_id = 72001"),
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "count")
        .unwrap()
}

#[tokio::test]
async fn portable_import_dry_run_reports_counts_without_writing() {
    let app = Application::new().await;
    let token = app.token().await;
    let bundle = portable_crypto::encrypt(&importable_payload(), PASSPHRASE).unwrap();
    let versions_before = version_count(&app).await;
    let locations_before = table_count(&app, "locations").await;
    let response = app
        .client
        .post(import_url(&app, "portable_imports/dry_run"))
        .bearer_auth(&token)
        .header(PASSPHRASE_HEADER, PASSPHRASE)
        .json(&json!({"bundle": bundle}))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    let versions_after = version_count(&app).await;
    let locations_after = table_count(&app, "locations").await;
    let exported = export_bundle(&app, &token).await;
    let roundtrip = app
        .client
        .post(import_url(&app, "portable_imports/dry_run"))
        .bearer_auth(&token)
        .header(PASSPHRASE_HEADER, PASSPHRASE)
        .json(&json!({"bundle": exported}))
        .send()
        .await
        .unwrap();
    let roundtrip_status = roundtrip.status().as_u16();
    let roundtrip_body: Value = roundtrip.json().await.unwrap();
    app.close().await;
    assert_eq!(status, 200);
    let data = &body["data"];
    assert_eq!(data["applied"], false);
    assert_eq!(data["counts"]["locations"], 1);
    assert_eq!(data["counts"]["people"], 1);
    assert_eq!(data["conflicts"].as_array().unwrap().len(), 0);
    assert_eq!(data["errors"].as_array().unwrap().len(), 0);
    assert_eq!(versions_before, versions_after);
    assert_eq!(locations_before, locations_after);
    assert_eq!(roundtrip_status, 200, "roundtrip body: {roundtrip_body}");
    let roundtrip_data = &roundtrip_body["data"];
    assert_eq!(roundtrip_data["applied"], false);
    assert_eq!(
        roundtrip_data["errors"].as_array().unwrap().len(),
        0,
        "roundtrip errors: {roundtrip_data}"
    );
    assert_eq!(
        roundtrip_data["conflicts"].as_array().unwrap().len(),
        0,
        "roundtrip conflicts: {roundtrip_data}"
    );
}

#[tokio::test]
async fn portable_import_apply_writes_records_and_audits_the_import() {
    let app = Application::new().await;
    let token = app.token().await;
    let bundle = portable_crypto::encrypt(&importable_payload(), PASSPHRASE).unwrap();
    let imported = audit_count(&app, "portable_data.imported", "1=1").await;
    let versions_before = version_count(&app).await;
    let response = app
        .client
        .post(import_url(&app, "portable_imports"))
        .bearer_auth(&token)
        .header(PASSPHRASE_HEADER, PASSPHRASE)
        .json(&json!({"bundle": bundle}))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    let audits = audit_count(&app, "portable_data.imported", "1=1").await;
    let requests = audit_count(
        &app,
        "api.request",
        "metadata->>'controller'='api/v1/portable_imports' AND metadata->>'action'='create' AND metadata->>'status'='201'",
    )
    .await;
    let locations: i64 = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM locations WHERE household_id = 72001 AND portable_id = 'imported-location-0001'",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "count")
        .unwrap();
    let people: i64 = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM people WHERE id IN (SELECT person_id FROM household_memberships WHERE household_id=72001) AND portable_id = 'imported-person-0001'",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "count")
        .unwrap();
    let memberships: i64 = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM location_memberships membership JOIN people person ON person.id = membership.person_id JOIN locations location ON location.id = membership.location_id WHERE membership.household_id = 72001 AND location.household_id=72001 AND person.portable_id = 'imported-person-0001' AND location.portable_id = 'imported-location-0001'",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "count")
        .unwrap();
    let grants: i64 = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM person_access_grants g JOIN people person ON person.id = g.person_id WHERE g.household_id = 72001 AND g.household_membership_id = 74001 AND person.portable_id = 'imported-person-0001' AND g.access_level = 'manage' AND g.revoked_at IS NULL",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "count")
        .unwrap();
    let versions = version_count(&app).await - versions_before;
    let changes: i64 = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM api_change_events WHERE household_id = 72001 AND record_portable_id IN ('imported-location-0001', 'imported-person-0001')",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "count")
        .unwrap();
    app.close().await;
    assert_eq!(status, 201);
    assert_eq!(body["data"]["applied"], true);
    assert_eq!(body["data"]["counts"]["locations"], 1);
    assert_eq!(body["data"]["counts"]["people"], 1);
    assert_eq!(locations, 1);
    assert_eq!(people, 1);
    assert_eq!(memberships, 1);
    assert_eq!(grants, 1);
    assert_eq!(versions, 2);
    assert_eq!(changes, 2);
    assert_eq!(audits - imported, 1);
    assert_eq!(requests, 1);
}

async fn portable_id(app: &Application, table: &str, id: i64) -> String {
    app.fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            format!("SELECT portable_id AS pid FROM {table} WHERE id = {id}"),
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "pid")
        .unwrap()
}

#[tokio::test]
async fn portable_import_applies_nullable_and_sparse_rows() {
    let app = Application::new().await;
    let token = app.token().await;
    let person_pid = portable_id(&app, "people", 73001).await;
    let medication_pid = portable_id(&app, "medications", 80001).await;
    let payload = json!({
        "format": "medtracker.portable.v1",
        "scope": "household",
        "records": {
            "schedules": [{
                "portable_id": "imported-schedule-001",
                "person_portable_id": person_pid,
                "medication_portable_id": medication_pid,
                "schedule_type": "daily",
                "schedule_config": {"times": ["08:00"]},
                "start_date": null,
                "end_date": null,
                "dose_amount": null,
                "dose_unit": null,
                "active": true
            }],
            "notification_preferences": [{
                "portable_id": "imported-prefs-001",
                "person_portable_id": person_pid
            }]
        }
    });
    let bundle = portable_crypto::encrypt(&payload, PASSPHRASE).unwrap();
    let dry_run = app
        .client
        .post(import_url(&app, "portable_imports/dry_run"))
        .bearer_auth(&token)
        .header(PASSPHRASE_HEADER, PASSPHRASE)
        .json(&json!({"bundle": bundle}))
        .send()
        .await
        .unwrap();
    let dry_status = dry_run.status().as_u16();
    let dry_body: Value = dry_run.json().await.unwrap();
    let response = app
        .client
        .post(import_url(&app, "portable_imports"))
        .bearer_auth(&token)
        .header(PASSPHRASE_HEADER, PASSPHRASE)
        .json(&json!({"bundle": bundle}))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    let prefs: i64 = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM notification_preferences WHERE household_id = 72001 AND portable_id = 'imported-prefs-001' AND enabled AND dose_due_enabled",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "count")
        .unwrap();
    app.close().await;
    assert_eq!(dry_status, 200, "dry-run body: {dry_body}");
    assert_eq!(dry_body["data"]["errors"].as_array().unwrap().len(), 0);
    assert_eq!(status, 201, "apply body: {body}");
    assert_eq!(prefs, 1, "sparse preference row must apply column defaults");
}

#[tokio::test]
async fn portable_import_accepts_derived_occurrence_window_ends() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture
        .admin
        .execute_raw(Statement::from_string(
            DbBackend::Postgres,
            "INSERT INTO schedules (household_id, person_id, medication_id, portable_id, schedule_type, schedule_config, created_at, updated_at) VALUES (72001, 73001, 80001, 'seeded-schedule-001', 0, '{}'::jsonb, NOW(), NOW())",
        ))
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_raw(Statement::from_string(
            DbBackend::Postgres,
            "INSERT INTO medication_dose_occurrences (household_id, schedule_id, portable_id, window_starts_on, position, outcome, created_at, updated_at) VALUES (72001, (SELECT id FROM schedules WHERE household_id = 72001 AND portable_id = 'seeded-schedule-001'), 'seeded-occurrence-001', '2026-10-06', 1, 'open', NOW(), NOW())",
        ))
        .await
        .unwrap();
    let payload = json!({
        "format": "medtracker.portable.v2",
        "scope": "household",
        "records": {
            "dose_occurrences": [{
                "portable_id": "seeded-occurrence-001",
                "source_type": "schedule",
                "source_portable_id": "seeded-schedule-001",
                "window_starts_on": "2026-10-06",
                "window_ends_on": "2026-10-06",
                "position": 1,
                "scheduled_at": null,
                "outcome": "open",
                "reason": null,
                "note": null,
                "resolved_at": null,
                "medication_take_portable_id": null
            }]
        }
    });
    let bundle = portable_crypto::encrypt(&payload, PASSPHRASE).unwrap();
    let response = app
        .client
        .post(import_url(&app, "portable_imports/dry_run"))
        .bearer_auth(&token)
        .header(PASSPHRASE_HEADER, PASSPHRASE)
        .json(&json!({"bundle": bundle}))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    app.close().await;
    assert_eq!(status, 200, "dry-run body: {body}");
    assert_eq!(
        body["data"]["errors"].as_array().unwrap().len(),
        0,
        "derived window_ends_on must not conflict: {body}"
    );
}

#[tokio::test]
async fn portable_import_preflight_projects_pause_closures() {
    let app = Application::new().await;
    let token = app.token().await;
    let person_pid = portable_id(&app, "people", 73001).await;
    let medication_pid = portable_id(&app, "medications", 80001).await;
    app.fixture
        .admin
        .execute_raw(Statement::from_string(
            DbBackend::Postgres,
            "INSERT INTO schedules (household_id, person_id, medication_id, portable_id, schedule_type, schedule_config, created_at, updated_at) VALUES (72001, 73001, 80001, 'seeded-schedule-002', 0, '{}'::jsonb, NOW(), NOW())",
        ))
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_raw(Statement::from_string(
            DbBackend::Postgres,
            "INSERT INTO medication_pause_periods (household_id, schedule_id, portable_id, reason, started_at, imported_context, legacy_context, created_at, updated_at) VALUES (72001, (SELECT id FROM schedules WHERE household_id = 72001 AND portable_id = 'seeded-schedule-002'), 'seeded-pause-001', 'side_effects', '2026-10-01T09:00:00Z', true, false, NOW(), NOW())",
        ))
        .await
        .unwrap();
    let payload = json!({
        "format": "medtracker.portable.v2",
        "scope": "household",
        "records": {
            "schedules": [{
                "portable_id": "seeded-schedule-002",
                "person_portable_id": person_pid,
                "medication_portable_id": medication_pid,
                "schedule_type": "daily",
                "schedule_config": {"times": ["08:00"]},
                "active": false
            }],
            "medication_pause_periods": [{
                "portable_id": "seeded-pause-001",
                "source_type": "schedule",
                "source_portable_id": "seeded-schedule-002",
                "reason": "side_effects",
                "note": null,
                "started_at": "2026-10-01T09:00:00Z",
                "ended_at": "2026-10-05T09:00:00Z",
                "legacy_context": false
            }]
        }
    });
    let bundle = portable_crypto::encrypt(&payload, PASSPHRASE).unwrap();
    let response = app
        .client
        .post(import_url(&app, "portable_imports/dry_run"))
        .bearer_auth(&token)
        .header(PASSPHRASE_HEADER, PASSPHRASE)
        .json(&json!({"bundle": bundle}))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    app.close().await;
    assert_eq!(status, 200, "dry-run body: {body}");
    assert!(
        body["data"]["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|error| error.as_str().is_some_and(
                |message| message.contains("inactive source requires an open pause period")
            )),
        "closing a source's only open pause must surface the inactive-source error: {body}"
    );
}

#[tokio::test]
async fn portable_import_rejects_unsupported_and_inconsistent_payloads() {
    let app = Application::new().await;
    let token = app.token().await;
    let mut statuses = Vec::new();
    let mut messages = Vec::new();
    for payload in [
        json!({"format": "medtracker.portable.v3", "records": {}}),
        json!({"format": "medtracker.portable.v1"}),
        json!({"format": "medtracker.portable.v1", "records": {"widgets": []}, "unexpected": true}),
        json!({"format": "medtracker.portable.v1", "records": {"widgets": []}}),
        json!({"format": "medtracker.portable.v1", "records": {"people": [{"name": "No ID"}]}}),
        json!({"format": "medtracker.portable.v1", "records": {"people": [{"portable_id": "p-1", "bogus": 1}]}}),
        json!({"format": "medtracker.portable.v1", "records": {"people": [{"portable_id": "p-1", "id": 5, "name": "Person", "date_of_birth": "1990-01-01", "person_type": "adult", "has_capacity": true}]}}),
    ] {
        let bundle = portable_crypto::encrypt(&payload, PASSPHRASE).unwrap();
        let response = app
            .client
            .post(import_url(&app, "portable_imports/dry_run"))
            .bearer_auth(&token)
            .header(PASSPHRASE_HEADER, PASSPHRASE)
            .json(&json!({"bundle": bundle}))
            .send()
            .await
            .unwrap();
        statuses.push(response.status().as_u16());
        let body: Value = response.json().await.unwrap();
        messages.push(
            body["error"]["message"]
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| serde_json::to_string(&body["data"]["errors"]).unwrap()),
        );
    }
    app.close().await;
    assert_eq!(statuses, vec![422, 422, 422, 422, 422, 422, 200]);
    assert_eq!(
        messages[..6],
        [
            "Unsupported portable data format",
            "Portable data records are required",
            "Unsupported portable payload fields",
            "Unsupported portable record type: widgets",
            "people[0].portable_id is required",
            "people[0] has unsupported fields",
        ]
    );
    assert!(messages[6].contains("includes Rails numeric IDs: id"));
}
