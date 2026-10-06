use super::*;

#[tokio::test]
async fn treatment_schedule_creation_representation_matches_read_and_update_precondition() {
    let app = Application::new().await;
    let token = app.token().await;
    let collection = format!("{}/api/v1/households/72001/schedules", app.origin);
    let created = app
        .client
        .post(&collection)
        .bearer_auth(&token)
        .json(&schedule_body())
        .send()
        .await
        .unwrap();
    let created_status = created.status().as_u16();
    let created_etag = created
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("missing")
        .to_owned();
    let created_body = created.json::<Value>().await.unwrap_or(Value::Null);
    let id = created_body["data"]["id"].as_i64().unwrap_or(-1);
    let endpoint = format!("{collection}/{id}");
    let read = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let read_status = read.status().as_u16();
    let read_etag = read
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("missing")
        .to_owned();
    let read_body = read.json::<Value>().await.unwrap_or(Value::Null);
    let updated = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("if-match", &created_etag)
        .json(&json!({"schedule":{"notes":"Created ETag remains usable"}}))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let version=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT object_changes::jsonb AS changes FROM versions WHERE item_type='Schedule' AND event='update' ORDER BY id DESC LIMIT 1")).await.unwrap();
    let changes: Value = version
        .map(|row| row.try_get("", "changes").unwrap())
        .unwrap_or(Value::Null);
    app.close().await;
    assert_eq!((created_status, read_status, updated), (201, 200, 200));
    assert_eq!(created_body, read_body);
    assert_eq!(created_etag, read_etag);
    assert_eq!(
        changes["notes"],
        json!([null, "Created ETag remains usable"])
    );
    for field in [
        "can_manage",
        "can_record",
        "eligible_stock_medication_ids",
        "person_portable_id",
        "medication_portable_id",
        "current_pause_period",
    ] {
        assert!(changes.get(field).is_none());
    }
}

#[tokio::test]
async fn treatment_schedule_create_audit_failure_rolls_back_and_records_request() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("CREATE SEQUENCE synthetic_schedule_create_audit_reached; GRANT USAGE,SELECT ON SEQUENCE synthetic_schedule_create_audit_reached TO med_tracker_app; CREATE FUNCTION reject_schedule_create_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='Schedule' AND NEW.event='create' THEN IF EXISTS(SELECT 1 FROM schedules WHERE id=NEW.item_id) THEN PERFORM nextval('synthetic_schedule_create_audit_reached'); END IF; RAISE EXCEPTION 'Synthetic schedule create audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_schedule_create_audit BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_schedule_create_audit()").await.unwrap();
    let response = app
        .client
        .post(format!("{}/api/v1/households/72001/schedules", app.origin))
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-create-audit-rollback")
        .json(&schedule_body())
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT is_called FROM synthetic_schedule_create_audit_reached) AS reached,(SELECT count(*) FROM schedules) AS schedules,(SELECT count(*) FROM versions WHERE item_type='Schedule') AS versions,(SELECT count(*) FROM api_change_events WHERE record_type='Schedule') AS changes,(SELECT count(*) FROM api_idempotency_keys WHERE key='synthetic-create-audit-rollback') AS keys,(SELECT count(*) FROM security_audit_events WHERE event_type='api.request' AND metadata->>'controller'='api/v1/schedules' AND metadata->>'action'='create' AND metadata->>'status'='500' AND metadata->>'outcome'='failure') AS attempts")).await.unwrap().unwrap();
    let reached: bool = row.try_get("", "reached").unwrap();
    let counts: Vec<i64> = ["schedules", "versions", "changes", "keys", "attempts"]
        .iter()
        .map(|field| row.try_get("", field).unwrap())
        .collect();
    app.close().await;
    assert_eq!(status, 500);
    assert!(reached);
    assert_eq!(counts, vec![0, 0, 0, 0, 1]);
}

fn schedule_body() -> Value {
    json!({"schedule":{"person_id":"73001","medication_id":"80001","dose_amount":"2","dose_unit":"tablet","frequency":"daily","start_date":"2026-01-01","end_date":"2027-01-01","schedule_type":"prn","schedule_config":{},"dose_cycle":"daily","max_daily_doses":3}})
}

#[tokio::test]
async fn treatment_schedule_create_returns_retained_decimal_representation() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/schedules", app.origin);
    let response = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&schedule_body())
        .send()
        .await
        .unwrap();
    let created = response.status().as_u16();
    let body: Value = response.json().await.unwrap_or(Value::Null);
    let audits: i64 = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS n FROM security_audit_events WHERE event_type='api.request' AND actor_account_id=71001 AND actor_membership_id=74001 AND metadata->>'controller'='api/v1/schedules' AND metadata->>'action'='create' AND metadata->>'outcome'='success' AND metadata->>'status'='201' AND audit_context->>'policy_class'='SchedulePolicy' AND audit_context->>'policy_query'='create?' AND request_id=audit_context->>'request_id'")).await.unwrap().unwrap().try_get("","n").unwrap();
    app.close().await;
    assert_eq!(created, 201);
    assert_eq!(body["data"]["dose_amount"], "2.0");
    assert_eq!(audits, 1);
}

#[tokio::test]
async fn treatment_schedule_invalid_date_and_dose_have_no_effect() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/schedules", app.origin);
    let before = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM schedules WHERE household_id=72001",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64>("", "count")
        .unwrap();
    let mut body = schedule_body();
    body["schedule"]["end_date"] = json!("2025-01-01");
    let dates = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    body = schedule_body();
    body["schedule"]["dose_amount"] = json!("0");
    let dose = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let after = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM schedules WHERE household_id=72001",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64>("", "count")
        .unwrap();
    let effects = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT (SELECT count(*) FROM versions WHERE item_type='Schedule') AS versions,(SELECT count(*) FROM api_change_events WHERE record_type='Schedule') AS changes,(SELECT current_supply::text FROM medications WHERE id=80001) AS supply")).await.unwrap().unwrap();
    let versions: i64 = effects.try_get("", "versions").unwrap();
    let changes: i64 = effects.try_get("", "changes").unwrap();
    let supply: String = effects.try_get("", "supply").unwrap();
    app.close().await;
    assert_eq!((dates, dose), (422, 422));
    assert_eq!(after, before);
    assert_eq!((versions, changes), (0, 0));
    assert_eq!(supply, "10.00");
}

#[tokio::test]
async fn treatment_schedule_requires_current_manage_grant() {
    let app = Application::new().await;
    app.fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
    let token = app.token().await;
    app.fixture
        .admin
        .execute_unprepared("UPDATE person_access_grants SET access_level='record' WHERE id=78001")
        .await
        .unwrap();
    let status = app
        .client
        .post(format!("{}/api/v1/households/72001/schedules", app.origin))
        .bearer_auth(&token)
        .json(&schedule_body())
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    app.close().await;
    assert_eq!(status, 403);
}
