use super::*;
use medications::crud;
use serde_json::json;

#[tokio::test]
async fn medication_crud_conformance_empty_barcodes_do_not_conflict() {
    let fixture = Fixture::new().await;
    let tenant = access::begin(&fixture.runtime, &scope()).await.unwrap();
    let first = crud::create(&tenant, json!({"name":"Synthetic blank barcode one","location_id":89001,"dose_amount":"2","current_supply":"10","reorder_threshold":"2","barcode":""}), None).await.unwrap();
    let second = crud::create(&tenant, json!({"name":"Synthetic blank barcode two","location_id":89001,"dose_amount":"2","current_supply":"10","reorder_threshold":"2","barcode":""}), None).await.unwrap();
    let snapshot = medications::read_stock_snapshot(&tenant, &second.id.to_string())
        .await
        .unwrap();
    assert_ne!(first.id, second.id);
    assert_eq!(first.barcode.as_deref(), Some(""));
    assert_eq!(second.barcode.as_deref(), Some(""));
    assert_eq!(snapshot.medication.id, second.id);
    tenant.commit().await.unwrap();
    fixture.close().await;
}

#[tokio::test]
async fn medication_crud_conformance_oversized_dose_is_rejected_before_mutation() {
    let fixture = Fixture::new().await;
    let tenant = access::begin(&fixture.runtime, &scope()).await.unwrap();
    let before = medications::read_stock_snapshot(&tenant, "90001")
        .await
        .unwrap();
    let result = crud::update(
        &tenant,
        "90001",
        json!({"dose_amount":"9".repeat(309)}),
        None,
        None,
    )
    .await;
    assert!(matches!(result, Err(OperationError::Validation { .. })));
    let after = medications::read_stock_snapshot(&tenant, "90001")
        .await
        .unwrap();
    assert_eq!(after.medication.dose_amount, before.medication.dose_amount);
    assert_eq!(after.etag, before.etag);
    tenant.commit().await.unwrap();
    assert_eq!(fixture.effect().await, ("10.00".into(), 0, 0));
    fixture.close().await;
}

#[tokio::test]
async fn medication_crud_conformance_nullable_dose_preserves_retained_api_semantics() {
    let fixture = Fixture::new().await;
    let tenant = access::begin(&fixture.runtime, &scope()).await.unwrap();
    let before = medications::read_stock_snapshot(&tenant, "90001")
        .await
        .unwrap();
    assert!(before.medication.dose_amount.is_some());
    let updated = crud::update(&tenant, "90001", json!({"dose_amount":null}), None, None)
        .await
        .unwrap();
    assert_eq!(updated.dose_amount, None);
    assert_eq!(updated.current_supply, before.medication.current_supply);
    tenant.commit().await.unwrap();
    let tenant = access::begin(&fixture.runtime, &scope()).await.unwrap();
    let nullable = medications::read_stock_snapshot(&tenant, "90001")
        .await
        .unwrap();
    crud::update(&tenant, "90001", json!({"dose_amount":null}), None, None)
        .await
        .unwrap();
    let replay = medications::read_stock_snapshot(&tenant, "90001")
        .await
        .unwrap();
    assert_eq!(replay.etag, nullable.etag);
    tenant.commit().await.unwrap();
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS count FROM versions WHERE item_type='Medication' AND item_id=90001 AND event='api_update'")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
    fixture.close().await;
}

#[tokio::test]
async fn medication_crud_create_is_immediately_usable_and_audited() {
    let fixture = Fixture::new().await;
    let tenant = access::begin(&fixture.runtime, &scope()).await.unwrap();
    let result = crud::create(&tenant, json!({"name":"Synthetic new tablets","location_id":89001,"dose_amount":"2","dose_unit":"tablet","current_supply":"10","reorder_threshold":"2","barcode":"5901234123457"}), None).await;
    let created = result.unwrap();
    let snapshot = medications::read_stock_snapshot(&tenant, &created.id.to_string())
        .await
        .unwrap();
    assert_eq!(
        snapshot.medication.name.as_deref(),
        Some("Synthetic new tablets")
    );
    assert_eq!(snapshot.medication.created_by_membership_id, Some(74001));
    assert_eq!(snapshot.medication.household_id, 72001);
    assert!(!snapshot.medication.portable_id.is_empty());
    tenant.commit().await.unwrap();
    let row = fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,"SELECT (SELECT count(*) FROM versions WHERE item_type='Medication' AND item_id=$1 AND event='api_create') AS audits,(SELECT count(*) FROM api_change_events WHERE record_type='Medication' AND record_id=$1 AND action='create') AS changes",[created.id.into()])).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "audits").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "changes").unwrap(), 1);
    fixture.close().await;
}

#[tokio::test]
async fn medication_crud_update_preserves_omitted_attributes_and_checks_etag() {
    let fixture = Fixture::new().await;
    let tenant = access::begin(&fixture.runtime, &scope()).await.unwrap();
    let old = medications::read_stock_snapshot(&tenant, "90001")
        .await
        .unwrap();
    let updated = crud::update(
        &tenant,
        "90001",
        json!({"friendly_name":"Synthetic display name"}),
        Some(&old.etag),
        None,
    )
    .await
    .unwrap();
    assert_eq!(updated.name, old.medication.name);
    assert_eq!(updated.current_supply, old.medication.current_supply);
    assert_eq!(
        updated.friendly_name.as_deref(),
        Some("Synthetic display name")
    );
    let stale = crud::update(
        &tenant,
        "90001",
        json!({"name":"Must not overwrite"}),
        Some(&old.etag),
        None,
    )
    .await;
    assert!(matches!(stale, Err(OperationError::Conflict { .. })));
    tenant.commit().await.unwrap();
    let row = fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT name FROM medications WHERE id=90001",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.try_get::<String>("", "name").unwrap(),
        old.medication.name.unwrap()
    );
    fixture.close().await;
}

#[tokio::test]
async fn medication_crud_rejects_invalid_attributes_and_duplicate_barcode() {
    let fixture = Fixture::new().await;
    fixture
        .admin
        .execute_unprepared("UPDATE medications SET barcode='5901234123458' WHERE id=90001")
        .await
        .unwrap();
    for mut attributes in [
        json!({"name":""}),
        json!({"name":"Synthetic","actor_account_id":71001}),
        json!({"name":"Synthetic","dose_amount":2}),
        json!({"name":"Synthetic","current_supply":"-1"}),
        json!({"name":"Synthetic","barcode":"5901234123458"}),
    ] {
        attributes["location_id"] = json!(89001);
        attributes["reorder_threshold"] = json!("2");
        let tenant = access::begin(&fixture.runtime, &scope()).await.unwrap();
        let result = crud::create(&tenant, attributes, None).await;
        tenant.rollback().await.unwrap();
        assert!(matches!(result, Err(OperationError::Validation { .. })));
    }
    let effect = fixture.effect().await;
    fixture.close().await;
    assert_eq!(effect, ("10.00".into(), 0, 0));
}

#[tokio::test]
async fn medication_crud_retire_unused_medication_records_audit_and_tombstone() {
    let fixture = Fixture::new().await;
    let tenant = access::begin(&fixture.runtime, &scope()).await.unwrap();
    let snapshot = medications::read_stock_snapshot(&tenant, "90001")
        .await
        .unwrap();
    let result = crud::retire(&tenant, "90001", Some(&snapshot.etag), None).await;
    let portable_id = snapshot.medication.portable_id;
    assert!(result.is_ok());
    tenant.commit().await.unwrap();
    let row = fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,"SELECT (SELECT count(*) FROM medications WHERE id=90001) AS remaining,(SELECT count(*) FROM versions WHERE item_type='Medication' AND item_id=90001 AND event='api_destroy') AS audits,(SELECT count(*) FROM api_tombstones WHERE record_type='Medication' AND record_portable_id=$1 AND action='delete') AS tombstones",[portable_id.into()])).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "remaining").unwrap(), 0);
    assert_eq!(row.try_get::<i64>("", "audits").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "tombstones").unwrap(), 1);
    fixture.close().await;
}

#[tokio::test]
async fn medication_crud_retire_preserves_administration_history() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("INSERT INTO person_medications(id,household_id,person_id,medication_id,dose_amount,dose_unit,position,created_at,updated_at) VALUES(97001,72001,73001,90001,2,'tablet',0,now(),now()); INSERT INTO medication_takes(id,household_id,person_medication_id,dose_amount,dose_unit,taken_at,created_at,updated_at) VALUES(97002,72001,97001,2,'tablet',now(),now(),now())").await.unwrap();
    let tenant = access::begin(&fixture.runtime, &scope()).await.unwrap();
    let result = crud::retire(&tenant, "90001", None, None).await;
    tenant.rollback().await.unwrap();
    assert!(matches!(result, Err(OperationError::Validation { .. })));
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM medication_takes WHERE id=97002) AS takes,(SELECT count(*) FROM person_medications WHERE id=97001) AS assignments")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "takes").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "assignments").unwrap(), 1);
    assert_eq!(fixture.effect().await, ("10.00".into(), 0, 0));
    fixture.close().await;
}

#[tokio::test]
async fn medication_crud_member_cannot_update_or_destroy_visible_medication() {
    let fixture = Fixture::new().await;
    fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
    let tenant = access::begin(&fixture.runtime, &scope()).await.unwrap();
    let updated = crud::update(
        &tenant,
        "90001",
        json!({"name":"Forbidden replacement"}),
        None,
        None,
    )
    .await;
    let retired = crud::retire(&tenant, "90001", None, None).await;
    tenant.rollback().await.unwrap();
    assert!(matches!(updated, Err(OperationError::Forbidden)));
    assert!(matches!(retired, Err(OperationError::Forbidden)));
    assert_eq!(fixture.effect().await, ("10.00".into(), 0, 0));
    fixture.close().await;
}

#[tokio::test]
async fn medication_crud_create_audit_failure_rolls_back_row_and_change() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("CREATE SEQUENCE synthetic_crud_attempt; GRANT USAGE ON SEQUENCE synthetic_crud_attempt TO med_tracker_app; CREATE FUNCTION synthetic_crud_failure() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='Medication' AND NEW.event='api_create' THEN PERFORM nextval('synthetic_crud_attempt'); RAISE EXCEPTION 'synthetic create audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER synthetic_crud_failure BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION synthetic_crud_failure()").await.unwrap();
    let tenant = access::begin(&fixture.runtime, &scope()).await.unwrap();
    let result = crud::create(
        &tenant,
        json!({"name":"Synthetic rollback medicine","location_id":89001,"reorder_threshold":"2"}),
        None,
    )
    .await;
    tenant.rollback().await.unwrap();
    assert!(matches!(result, Err(OperationError::Unavailable)));
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM medications WHERE name='Synthetic rollback medicine') AS medicines,(SELECT is_called FROM synthetic_crud_attempt) AS reached,(SELECT count(*) FROM api_change_events WHERE record_type='Medication') AS changes")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "medicines").unwrap(), 0);
    assert!(row.try_get::<bool>("", "reached").unwrap());
    assert_eq!(row.try_get::<i64>("", "changes").unwrap(), 0);
    fixture.close().await;
}
