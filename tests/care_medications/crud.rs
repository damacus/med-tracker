use super::*;
use medications::crud;
use sea_orm::prelude::Decimal;
use serde_json::{Value, json};

fn input(value: Value) -> crud::MedicationInput {
    crud::MedicationInput::from_json(value)
}

#[tokio::test]
async fn typed_medication_create_persists_exact_stock_and_remains_readable() {
    let fixture = Fixture::new().await;
    let tenant = access::begin(&fixture.runtime, &fixture.scope, None)
        .await
        .unwrap();
    let command = crud::MedicationCreate::new(
        "Synthetic typed tablets".into(),
        crud::LocationId::try_from(fixture.location_id).unwrap(),
        Decimal::new(5, 1),
    )
    .with_stock(Decimal::new(125, 1));
    let created = crud::create(&tenant, command, None).await.unwrap();
    let snapshot = medications::read_stock_snapshot(&tenant, &created.id.to_string())
        .await
        .unwrap();
    assert_eq!(
        snapshot.medication.current_supply,
        Some(Decimal::new(125, 1))
    );
    assert_eq!(snapshot.medication.reorder_threshold, Decimal::new(5, 1));
    assert_eq!(
        snapshot.medication.name.as_deref(),
        Some("Synthetic typed tablets")
    );
    assert_eq!(
        snapshot.representation.data.display_name.as_deref(),
        Some("Synthetic typed tablets")
    );
    assert_eq!(
        snapshot.representation.data.current_supply.as_deref(),
        Some("12.5")
    );
    assert_eq!(snapshot.representation.data.reorder_threshold, "0.5");
    tenant.commit().await.unwrap();
    fixture.close().await;
}

#[tokio::test]
async fn medication_crud_conformance_empty_barcodes_do_not_conflict() {
    let fixture = Fixture::new().await;
    let tenant = access::begin(&fixture.runtime, &fixture.scope, None)
        .await
        .unwrap();
    let first = crud::create(&tenant, input(json!({"name":"Synthetic blank barcode one","location_id":fixture.location_id,"dose_amount":"2","current_supply":"10","reorder_threshold":"2","barcode":""})), None).await.unwrap();
    let second = crud::create(&tenant, input(json!({"name":"Synthetic blank barcode two","location_id":fixture.location_id,"dose_amount":"2","current_supply":"10","reorder_threshold":"2","barcode":""})), None).await.unwrap();
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
    let tenant = access::begin(&fixture.runtime, &fixture.scope, None)
        .await
        .unwrap();
    let before = medications::read_stock_snapshot(&tenant, &fixture.medication_id.to_string())
        .await
        .unwrap();
    let result = crud::update(
        &tenant,
        &fixture.medication_id.to_string(),
        input(json!({"dose_amount":"9".repeat(309)})),
        None,
        None,
    )
    .await;
    assert!(matches!(result, Err(OperationError::Validation { .. })));
    let after = medications::read_stock_snapshot(&tenant, &fixture.medication_id.to_string())
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
    let tenant = access::begin(&fixture.runtime, &fixture.scope, None)
        .await
        .unwrap();
    let before = medications::read_stock_snapshot(&tenant, &fixture.medication_id.to_string())
        .await
        .unwrap();
    assert!(before.medication.dose_amount.is_some());
    let updated = crud::update(
        &tenant,
        &fixture.medication_id.to_string(),
        input(json!({"dose_amount":null})),
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(updated.dose_amount, None);
    assert_eq!(updated.current_supply, before.medication.current_supply);
    tenant.commit().await.unwrap();
    let tenant = access::begin(&fixture.runtime, &fixture.scope, None)
        .await
        .unwrap();
    let nullable = medications::read_stock_snapshot(&tenant, &fixture.medication_id.to_string())
        .await
        .unwrap();
    crud::update(
        &tenant,
        &fixture.medication_id.to_string(),
        input(json!({"dose_amount":null})),
        None,
        None,
    )
    .await
    .unwrap();
    let replay = medications::read_stock_snapshot(&tenant, &fixture.medication_id.to_string())
        .await
        .unwrap();
    assert_eq!(replay.etag, nullable.etag);
    tenant.commit().await.unwrap();
    let audits = fixture
        .audits()
        .filter(version::Column::Event.eq("api_update"))
        .count(&fixture.admin)
        .await
        .unwrap();
    assert_eq!(audits, 1);
    fixture.close().await;
}

#[tokio::test]
async fn medication_crud_create_is_immediately_usable_and_audited() {
    let fixture = Fixture::new().await;
    let tenant = access::begin(&fixture.runtime, &fixture.scope, None)
        .await
        .unwrap();
    let result = crud::create(&tenant, input(json!({"name":"Synthetic new tablets","location_id":fixture.location_id,"dose_amount":"2","dose_unit":"tablet","current_supply":"10","reorder_threshold":"2","barcode":"5901234123457"})), None).await;
    let created = result.unwrap();
    let snapshot = medications::read_stock_snapshot(&tenant, &created.id.to_string())
        .await
        .unwrap();
    assert_eq!(
        snapshot.medication.name.as_deref(),
        Some("Synthetic new tablets")
    );
    assert_eq!(
        snapshot.medication.created_by_membership_id,
        Some(fixture.membership.id)
    );
    assert_eq!(snapshot.medication.household_id, fixture.household.id);
    assert!(!snapshot.medication.portable_id.is_empty());
    tenant.commit().await.unwrap();
    let audits = version::Entity::find()
        .filter(version::Column::HouseholdId.eq(fixture.household.id))
        .filter(version::Column::ItemType.eq("Medication"))
        .filter(version::Column::ItemId.eq(created.id))
        .filter(version::Column::Event.eq("api_create"))
        .count(&fixture.admin)
        .await
        .unwrap();
    let changes = fixture
        .changes()
        .filter(api_change_event::Column::RecordType.eq("Medication"))
        .filter(api_change_event::Column::RecordId.eq(created.id))
        .filter(api_change_event::Column::Action.eq("create"))
        .count(&fixture.admin)
        .await
        .unwrap();
    assert_eq!(audits, 1);
    assert_eq!(changes, 1);
    fixture.close().await;
}

#[tokio::test]
async fn medication_crud_update_preserves_omitted_attributes_and_checks_etag() {
    let fixture = Fixture::new().await;
    let tenant = access::begin(&fixture.runtime, &fixture.scope, None)
        .await
        .unwrap();
    let old = medications::read_stock_snapshot(&tenant, &fixture.medication_id.to_string())
        .await
        .unwrap();
    let updated = crud::update(
        &tenant,
        &fixture.medication_id.to_string(),
        input(json!({"friendly_name":"Synthetic display name"})),
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
        &fixture.medication_id.to_string(),
        input(json!({"name":"Must not overwrite"})),
        Some(&old.etag),
        None,
    )
    .await;
    assert!(matches!(stale, Err(OperationError::Conflict { .. })));
    tenant.commit().await.unwrap();
    let row = medication::Entity::find_by_id(fixture.medication_id)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.name.unwrap(), old.medication.name.unwrap());
    fixture.close().await;
}

#[tokio::test]
async fn medication_crud_rejects_invalid_attributes_and_duplicate_barcode() {
    let fixture = Fixture::new().await;
    medication::Entity::update_many()
        .col_expr(
            medication::Column::Barcode,
            sea_orm::sea_query::Expr::val("5901234123458"),
        )
        .filter(medication::Column::Id.eq(fixture.medication_id))
        .exec(&fixture.admin)
        .await
        .unwrap();
    for mut attributes in [
        json!({"name":""}),
        json!({"name":"Synthetic","actor_account_id":fixture.account.id}),
        json!({"name":"Synthetic","dose_amount":2}),
        json!({"name":"Synthetic","current_supply":"-1"}),
        json!({"name":"Synthetic","barcode":"5901234123458"}),
    ] {
        attributes["location_id"] = json!(fixture.location_id);
        attributes["reorder_threshold"] = json!("2");
        let tenant = access::begin(&fixture.runtime, &fixture.scope, None)
            .await
            .unwrap();
        let result = crud::create(&tenant, input(attributes), None).await;
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
    let tenant = access::begin(&fixture.runtime, &fixture.scope, None)
        .await
        .unwrap();
    let snapshot = medications::read_stock_snapshot(&tenant, &fixture.medication_id.to_string())
        .await
        .unwrap();
    let result = crud::retire(
        &tenant,
        &fixture.medication_id.to_string(),
        Some(&snapshot.etag),
        None,
    )
    .await;
    let portable_id = snapshot.medication.portable_id;
    assert!(result.is_ok());
    tenant.commit().await.unwrap();
    let remaining = medication::Entity::find_by_id(fixture.medication_id)
        .count(&fixture.admin)
        .await
        .unwrap();
    let audits = fixture
        .audits()
        .filter(version::Column::Event.eq("api_destroy"))
        .count(&fixture.admin)
        .await
        .unwrap();
    let tombstones = api_tombstone::Entity::find()
        .filter(api_tombstone::Column::HouseholdId.eq(fixture.household.id))
        .filter(api_tombstone::Column::RecordType.eq("Medication"))
        .filter(api_tombstone::Column::RecordPortableId.eq(portable_id))
        .filter(api_tombstone::Column::Action.eq("delete"))
        .count(&fixture.admin)
        .await
        .unwrap();
    assert_eq!(remaining, 0);
    assert_eq!(audits, 1);
    assert_eq!(tombstones, 1);
    fixture.close().await;
}

#[tokio::test]
async fn medication_crud_retire_preserves_administration_history() {
    let fixture = Fixture::new().await;
    let now = chrono::Utc::now().naive_utc();
    let assignment = person_medication::ActiveModel {
        household_id: Set(fixture.household.id),
        person_id: Set(fixture.account.person_id),
        medication_id: Set(fixture.medication_id),
        dose_amount: Set(Some(Decimal::new(2, 0))),
        dose_unit: Set(Some("tablet".into())),
        position: Set(0),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
    let take = medication_take::ActiveModel {
        household_id: Set(fixture.household.id),
        person_medication_id: Set(Some(assignment.id)),
        dose_amount: Set(Some(Decimal::new(2, 0))),
        dose_unit: Set(Some("tablet".into())),
        taken_at: Set(Some(now)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
    let tenant = access::begin(&fixture.runtime, &fixture.scope, None)
        .await
        .unwrap();
    let result = crud::retire(&tenant, &fixture.medication_id.to_string(), None, None).await;
    tenant.rollback().await.unwrap();
    assert!(matches!(result, Err(OperationError::Validation { .. })));
    assert_eq!(
        medication_take::Entity::find_by_id(take.id)
            .count(&fixture.admin)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        person_medication::Entity::find_by_id(assignment.id)
            .count(&fixture.admin)
            .await
            .unwrap(),
        1
    );
    assert_eq!(fixture.effect().await, ("10.00".into(), 0, 0));
    fixture.close().await;
}

#[tokio::test]
async fn medication_crud_member_cannot_update_or_destroy_visible_medication() {
    let fixture = Fixture::new().await;
    fixture.demote().await;
    let tenant = access::begin(&fixture.runtime, &fixture.scope, None)
        .await
        .unwrap();
    let updated = crud::update(
        &tenant,
        &fixture.medication_id.to_string(),
        input(json!({"name":"Forbidden replacement"})),
        None,
        None,
    )
    .await;
    let retired = crud::retire(&tenant, &fixture.medication_id.to_string(), None, None).await;
    tenant.rollback().await.unwrap();
    assert!(matches!(updated, Err(OperationError::Forbidden)));
    assert!(matches!(retired, Err(OperationError::Forbidden)));
    assert_eq!(fixture.effect().await, ("10.00".into(), 0, 0));
    fixture.close().await;
}

#[tokio::test]
async fn medication_crud_create_audit_failure_rolls_back_row_and_change() {
    let fixture = Fixture::new().await;
    let failure = AuditFailure::install(&fixture, "Medication", Decimal::new(10, 0)).await;
    let tenant = access::begin(&fixture.runtime, &fixture.scope, None)
        .await
        .unwrap();
    let result = crud::create(
    &tenant,
    input(json!({"name":"Synthetic rollback medicine","location_id":fixture.location_id,"reorder_threshold":"2"})),
    None,
)
.await;
    tenant.rollback().await.unwrap();
    let reached = failure.reached_and_remove(&fixture).await;
    assert!(matches!(result, Err(OperationError::Unavailable)));
    let medicines = medication::Entity::find()
        .filter(medication::Column::HouseholdId.eq(fixture.household.id))
        .filter(medication::Column::Name.eq("Synthetic rollback medicine"))
        .count(&fixture.admin)
        .await
        .unwrap();
    let changes = fixture
        .changes()
        .filter(api_change_event::Column::RecordType.eq("Medication"))
        .count(&fixture.admin)
        .await
        .unwrap();
    assert_eq!(medicines, 0);
    assert!(reached);
    assert_eq!(changes, 0);

    let now = chrono::Utc::now().naive_utc();
    let utility = med_tracker::models::_entities::locations::ActiveModel {
        household_id: Set(fixture.household.id),
        name: Set("Utility".to_owned()),
        portable_id: Set(uuid::Uuid::new_v4().to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
    let kitchen = med_tracker::models::_entities::locations::ActiveModel {
        household_id: Set(fixture.household.id),
        name: Set("Kitchen".to_owned()),
        portable_id: Set(uuid::Uuid::new_v4().to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
    let tenant = access::begin(&fixture.runtime, &fixture.scope, None)
        .await
        .unwrap();
    let moved = crud::update(
        &tenant,
        &fixture.medication_id.to_string(),
        input(json!({"location_id": utility.id})),
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(moved.location_id, utility.id);
    tenant.commit().await.unwrap();

    let move_version = fixture
        .audits()
        .filter(version::Column::Event.eq("api_update"))
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        move_version.whodunnit.as_deref(),
        Some(fixture.account.id.to_string().as_str())
    );
    assert_eq!(
        move_version.actor_membership_id,
        Some(fixture.membership.id)
    );
    let move_changes: Value =
        serde_json::from_str(move_version.object_changes.as_deref().unwrap()).unwrap();
    assert_eq!(
        move_changes["location_id"],
        json!([fixture.location_id, utility.id])
    );

    let version_fault = AuditFailure::install(&fixture, "Medication", Decimal::new(10, 0)).await;
    let tenant = access::begin(&fixture.runtime, &fixture.scope, None)
        .await
        .unwrap();
    let version_failed = crud::update(
        &tenant,
        &fixture.medication_id.to_string(),
        input(json!({"location_id": kitchen.id})),
        None,
        None,
    )
    .await;
    tenant.rollback().await.unwrap();
    let version_reached = version_fault.reached_and_remove(&fixture).await;
    assert!(matches!(version_failed, Err(OperationError::Unavailable)));
    assert!(version_reached);
    assert_eq!(
        medication::Entity::find_by_id(fixture.medication_id)
            .one(&fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .location_id,
        utility.id
    );
    assert_eq!(
        fixture
            .audits()
            .filter(version::Column::Event.eq("api_update"))
            .count(&fixture.admin)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        fixture
            .changes()
            .filter(api_change_event::Column::RecordType.eq("Medication"))
            .filter(api_change_event::Column::Action.eq("update"))
            .count(&fixture.admin)
            .await
            .unwrap(),
        1
    );

    let sync_fault = format!("medication_sync_{}", uuid::Uuid::new_v4().simple());
    let household_id = fixture.household.id;
    let medication_id = fixture.medication_id;
    // SeaORM Schema/Table builders cannot express PostgreSQL trigger functions or
    // non-transactional sequence probes. This isolated DDL tests rollback when
    // api_change_events insertion fails after updating medications and versions.
    fixture
        .admin
        .execute_unprepared(&format!(
            "CREATE SEQUENCE {sync_fault};
             GRANT USAGE ON SEQUENCE {sync_fault} TO med_tracker_app;
             CREATE FUNCTION {sync_fault}() RETURNS trigger LANGUAGE plpgsql AS $$
             BEGIN
               IF NEW.household_id={household_id} AND NEW.record_type='Medication' AND NEW.action='update' THEN
                 IF (SELECT location_id FROM medications WHERE id={medication_id})={} THEN
                   PERFORM nextval('{sync_fault}');
                 END IF;
                 RAISE EXCEPTION 'synthetic medication sync rejection';
               END IF;
               RETURN NEW;
             END $$;
             CREATE TRIGGER {sync_fault} BEFORE INSERT ON api_change_events FOR EACH ROW EXECUTE FUNCTION {sync_fault}()",
            kitchen.id
        ))
        .await
        .unwrap();
    let tenant = access::begin(&fixture.runtime, &fixture.scope, None)
        .await
        .unwrap();
    let sync_failed = crud::update(
        &tenant,
        &fixture.medication_id.to_string(),
        input(json!({"location_id": kitchen.id})),
        None,
        None,
    )
    .await;
    tenant.rollback().await.unwrap();
    let sync_reached: bool = fixture
        .admin
        .query_one_raw(
            fixture.admin.get_database_backend().build(
                &sea_orm::sea_query::Query::select()
                    .column(sea_orm::sea_query::Alias::new("is_called"))
                    .from(sea_orm::sea_query::Alias::new(sync_fault.clone()))
                    .to_owned(),
            ),
        )
        .await
        .unwrap()
        .unwrap()
        .try_get("", "is_called")
        .unwrap();
    fixture
        .admin
        .execute_unprepared(&format!(
            "DROP TRIGGER {0} ON api_change_events; DROP FUNCTION {0}(); DROP SEQUENCE {0}",
            sync_fault
        ))
        .await
        .unwrap();
    assert!(matches!(sync_failed, Err(OperationError::Unavailable)));
    assert!(sync_reached);
    assert_eq!(
        medication::Entity::find_by_id(fixture.medication_id)
            .one(&fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .location_id,
        utility.id
    );
    assert_eq!(
        fixture
            .audits()
            .filter(version::Column::Event.eq("api_update"))
            .count(&fixture.admin)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        fixture
            .changes()
            .filter(api_change_event::Column::RecordType.eq("Medication"))
            .filter(api_change_event::Column::Action.eq("update"))
            .count(&fixture.admin)
            .await
            .unwrap(),
        1
    );
    fixture.close().await;
}
