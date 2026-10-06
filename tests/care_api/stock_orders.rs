use super::*;

fn order_body() -> Value {
    json!({"order_details":{"supplier":"Synthetic pharmacy","quantity":"20","expected_arrival_on":"2026-11-01"}})
}

#[tokio::test]
async fn stock_order_visible_member_can_order_and_receive_without_adding_stock() {
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001; UPDATE person_access_grants SET access_level='view' WHERE id=78001").await.unwrap();
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/medications/80001", app.origin);
    let ordered = app
        .client
        .patch(format!("{endpoint}/mark_as_ordered"))
        .bearer_auth(&token)
        .json(&order_body())
        .send()
        .await
        .unwrap();
    let received = app
        .client
        .patch(format!("{endpoint}/mark_as_received"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let statuses = (ordered.status().as_u16(), received.status().as_u16());
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT reorder_status,current_supply::text AS supply,order_supplier,order_quantity::text AS quantity,expected_arrival_on::text AS arrival,ordered_at IS NOT NULL AS ordered,reordered_at IS NOT NULL AS received,(SELECT count(*) FROM versions WHERE item_type='Medication' AND event IN ('mark_as_ordered','mark_as_received')) AS versions FROM medications WHERE id=80001")).await.unwrap().unwrap();
    let status: Option<i32> = row.try_get("", "reorder_status").unwrap();
    let values: Vec<Option<String>> = ["supply", "order_supplier", "quantity", "arrival"]
        .iter()
        .map(|field| row.try_get("", field).unwrap())
        .collect();
    let ordered: bool = row.try_get("", "ordered").unwrap();
    let received: bool = row.try_get("", "received").unwrap();
    let versions: i64 = row.try_get("", "versions").unwrap();
    let audits: i64 = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS n FROM security_audit_events WHERE event_type='api.request' AND actor_account_id=71001 AND actor_membership_id=74001 AND metadata->>'controller'='api/v1/medications' AND metadata->>'outcome'='success' AND metadata->>'status'='200' AND audit_context->>'policy_class'='MedicationPolicy' AND audit_context->>'active_role'='member' AND request_id=audit_context->>'request_id' AND ((metadata->>'action'='mark_as_ordered' AND audit_context->>'policy_query'='mark_as_ordered?') OR (metadata->>'action'='mark_as_received' AND audit_context->>'policy_query'='mark_as_received?'))")).await.unwrap().unwrap().try_get("","n").unwrap();
    app.close().await;
    assert_eq!(statuses, (200, 200));
    assert_eq!(status, Some(2));
    assert_eq!(
        values,
        vec![
            Some("10.00".into()),
            Some("Synthetic pharmacy".into()),
            Some("20.00".into()),
            Some("2026-11-01".into())
        ]
    );
    assert!(ordered && received);
    assert_eq!(versions, 2);
    assert_eq!(audits, 2);
}

#[tokio::test]
async fn stock_order_foreign_household_and_revoked_view_grant_cannot_mutate() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("UPDATE medications SET reorder_status=0 WHERE id=80001; UPDATE household_memberships SET role='member' WHERE id=74001; UPDATE person_access_grants SET access_level='view',revoked_at=now() WHERE id=78001").await.unwrap();
    let denied = app
        .client
        .patch(format!(
            "{}/api/v1/households/72001/medications/80001/mark_as_ordered",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&order_body())
        .send()
        .await
        .unwrap();
    let foreign = app
        .client
        .patch(format!(
            "{}/api/v1/households/72002/medications/80001/mark_as_received",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let statuses = (denied.status().as_u16(), foreign.status().as_u16());
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT reorder_status,current_supply::text AS supply FROM medications WHERE id=80001",
        ))
        .await
        .unwrap()
        .unwrap();
    let state: Option<i32> = row.try_get("", "reorder_status").unwrap();
    let supply: String = row.try_get("", "supply").unwrap();
    app.close().await;
    assert_eq!(statuses, (404, 403));
    assert_eq!((state, supply.as_str()), (Some(0), "10.00"));
}

#[tokio::test]
async fn stock_order_invalid_quantity_and_date_leave_order_fields_unchanged() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("UPDATE medications SET reorder_status=0,order_supplier=NULL,order_quantity=NULL,expected_arrival_on=NULL,ordered_at=NULL WHERE id=80001").await.unwrap();
    let endpoint = format!(
        "{}/api/v1/households/72001/medications/80001/mark_as_ordered",
        app.origin
    );
    let mut statuses = Vec::new();
    for body in [
        json!({"order_details":{"quantity":"-1"}}),
        json!({"order_details":{"quantity":"100000000"}}),
        json!({"order_details":{"expected_arrival_on":"2026-02-30"}}),
    ] {
        statuses.push(
            app.client
                .patch(&endpoint)
                .bearer_auth(&token)
                .json(&body)
                .send()
                .await
                .unwrap()
                .status()
                .as_u16(),
        );
    }
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT reorder_status=0 AND order_supplier IS NULL AND order_quantity IS NULL AND expected_arrival_on IS NULL AND ordered_at IS NULL AS unchanged,(SELECT count(*) FROM versions WHERE item_type='Medication' AND event='mark_as_ordered') AS versions FROM medications WHERE id=80001")).await.unwrap().unwrap();
    let unchanged: bool = row.try_get("", "unchanged").unwrap();
    let versions: i64 = row.try_get("", "versions").unwrap();
    app.close().await;
    assert_eq!(statuses, vec![422, 422, 422]);
    assert!(unchanged);
    assert_eq!(versions, 0);
}

#[tokio::test]
async fn stock_order_keyed_retry_replays_once_and_changed_body_conflicts() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/72001/medications/80001/mark_as_ordered",
        app.origin
    );
    let first = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-order-once")
        .json(&order_body())
        .send()
        .await
        .unwrap();
    let repeated = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-order-once")
        .json(&order_body())
        .send()
        .await
        .unwrap();
    let replayed = repeated
        .headers()
        .get("idempotency-replayed")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let changed = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-order-once")
        .json(&json!({"order_details":{"quantity":"21"}}))
        .send()
        .await
        .unwrap();
    let statuses = (
        first.status().as_u16(),
        repeated.status().as_u16(),
        changed.status().as_u16(),
    );
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT order_quantity::text AS quantity,(SELECT count(*) FROM versions WHERE item_type='Medication' AND event='mark_as_ordered') AS versions,(SELECT count(*) FROM api_change_events WHERE record_type='Medication' AND action='update') AS changes FROM medications WHERE id=80001")).await.unwrap().unwrap();
    let quantity: Option<String> = row.try_get("", "quantity").unwrap();
    let versions: i64 = row.try_get("", "versions").unwrap();
    let changes: i64 = row.try_get("", "changes").unwrap();
    app.close().await;
    assert_eq!(statuses, (200, 200, 409));
    assert_eq!(replayed, "true");
    assert_eq!(
        (quantity.as_deref(), versions, changes),
        (Some("20.00"), 1, 1)
    );
}

#[tokio::test]
async fn stock_order_audit_rejection_rolls_back_order_and_replay_ledger() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("UPDATE medications SET reorder_status=0,ordered_at=NULL WHERE id=80001; CREATE SEQUENCE synthetic_order_audit_reached; GRANT USAGE,SELECT ON SEQUENCE synthetic_order_audit_reached TO med_tracker_app; CREATE FUNCTION reject_order_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='Medication' AND NEW.event='mark_as_ordered' THEN IF (SELECT reorder_status FROM medications WHERE id=80001)=1 THEN PERFORM nextval('synthetic_order_audit_reached'); END IF; RAISE EXCEPTION 'Synthetic order audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_order_audit BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_order_audit()").await.unwrap();
    let response = app
        .client
        .patch(format!(
            "{}/api/v1/households/72001/medications/80001/mark_as_ordered",
            app.origin
        ))
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-order-rollback")
        .json(&order_body())
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT is_called FROM synthetic_order_audit_reached) AS reached,(SELECT reorder_status=0 AND ordered_at IS NULL FROM medications WHERE id=80001) AS unchanged,(SELECT count(*) FROM api_idempotency_keys WHERE key='synthetic-order-rollback') AS keys,(SELECT count(*) FROM api_change_events WHERE record_type='Medication') AS changes")).await.unwrap().unwrap();
    let reached: bool = row.try_get("", "reached").unwrap();
    let unchanged: bool = row.try_get("", "unchanged").unwrap();
    let keys: i64 = row.try_get("", "keys").unwrap();
    let changes: i64 = row.try_get("", "changes").unwrap();
    let attempts: i64 = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS n FROM security_audit_events WHERE event_type='api.request' AND metadata->>'action'='mark_as_ordered' AND metadata->>'status'='500' AND metadata->>'outcome'='failure'")).await.unwrap().unwrap().try_get("","n").unwrap();
    app.close().await;
    assert_eq!(status, 500);
    assert!(reached && unchanged);
    assert_eq!((keys, changes), (0, 0));
    assert_eq!(attempts, 1);
}
