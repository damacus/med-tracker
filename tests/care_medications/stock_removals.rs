use super::*;
use medications::stock_removals::{self, RemoveStock};

fn input() -> RemoveStock {
    RemoveStock {
        medication_id: "90001".into(),
        quantity: "2".into(),
        reason: "dropped".into(),
        note: Some(" synthetic note ".into()),
        dosage_id: None,
        submission_id: "106b49d9-1da2-42f1-800b-8c80867aee1c".into(),
    }
}

async fn remove(
    fixture: &Fixture,
    input: RemoveStock,
) -> Result<serde_json::Value, OperationError> {
    let tenant = access::begin(&fixture.runtime, &scope()).await?;
    match stock_removals::create(&tenant, input, None).await {
        Ok(result) => {
            tenant.commit().await?;
            Ok(result)
        }
        Err(error) => {
            tenant.rollback().await?;
            Err(error)
        }
    }
}

#[tokio::test]
async fn stock_removal_scalar_replay_and_changed_facts() {
    let fixture = Fixture::new().await;
    let first = remove(&fixture, input()).await;
    let replay = remove(&fixture, input()).await;
    let mut changed = input();
    changed.quantity = "3".into();
    let conflict = remove(&fixture, changed).await;
    let effect = fixture.effect().await;
    fixture.close().await;
    assert!(first.is_ok());
    assert_eq!(first.unwrap(), replay.unwrap());
    assert!(matches!(conflict, Err(OperationError::Validation { .. })));
    assert_eq!(effect, ("8.00".into(), 1, 1));
}

#[tokio::test]
async fn stock_removal_concurrent_submission_has_one_effect() {
    let fixture = Fixture::new().await;
    let (first, second) = tokio::join!(remove(&fixture, input()), remove(&fixture, input()));
    let effect = fixture.effect().await;
    fixture.close().await;
    assert!(first.is_ok());
    assert_eq!(first.unwrap(), second.unwrap());
    assert_eq!(effect, ("8.00".into(), 1, 1));
}

#[tokio::test]
async fn stock_removal_insufficient_stock_and_nonmanager_are_rejected() {
    let fixture = Fixture::new().await;
    let mut too_much = input();
    too_much.quantity = "11".into();
    let insufficient = remove(&fixture, too_much).await;
    fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
    let denied = remove(&fixture, input()).await;
    let effect = fixture.effect().await;
    fixture.close().await;
    assert!(matches!(
        insufficient,
        Err(OperationError::Validation { .. })
    ));
    assert!(matches!(denied, Err(OperationError::Forbidden)));
    assert_eq!(effect, ("10.00".into(), 0, 0));
}

#[tokio::test]
async fn stock_removal_history_is_bounded_and_newest_first() {
    let fixture = Fixture::new().await;
    assert!(remove(&fixture, input()).await.is_ok());
    let mut second = input();
    second.submission_id = uuid::Uuid::new_v4().to_string();
    let newest = remove(&fixture, second).await.unwrap();
    let tenant = access::begin(&fixture.runtime, &scope()).await.unwrap();
    let history = stock_removals::history(&tenant, "90001", 1, 1).await;
    tenant.rollback().await.unwrap();
    fixture.close().await;
    let history = history.unwrap();
    assert_eq!(history["data"].as_array().unwrap().len(), 1);
    assert_eq!(history["data"][0], newest);
    assert_eq!(history["meta"]["total_count"], 2);
}

#[tokio::test]
async fn stock_removal_tracked_option_updates_parent_and_history() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("INSERT INTO dosages(id,household_id,medication_id,amount,unit,frequency,current_supply,default_dose_cycle,default_max_daily_doses,default_min_hours_between_doses,created_at,updated_at) VALUES(92001,72001,90001,2,'tablet','daily',6,0,4,0,'2020-01-01','2020-01-01'),(92002,72001,90001,1,'tablet','daily',4,0,4,0,'2020-01-01','2020-01-01')").await.unwrap();
    let mut command = input();
    command.dosage_id = Some("92001".into());
    let result = remove(&fixture, command).await;
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT current_supply::text AS supply,updated_at>'2020-01-01'::timestamp AS touched FROM dosages WHERE id=92001")).await.unwrap().unwrap();
    let supply: String = row.try_get("", "supply").unwrap();
    let touched: bool = row.try_get("", "touched").unwrap();
    let effect = fixture.effect().await;
    fixture.close().await;
    let result = result.unwrap();
    assert_eq!(result["remaining_quantity"], "4");
    assert_eq!(supply, "4.00");
    assert!(touched);
    assert_eq!(effect, ("8.00".into(), 1, 1));
}

#[tokio::test]
async fn stock_removal_audit_failure_rolls_back_stock_and_change() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("CREATE SEQUENCE synthetic_stock_removal_attempt; GRANT USAGE ON SEQUENCE synthetic_stock_removal_attempt TO med_tracker_app; CREATE FUNCTION synthetic_stock_removal_failure() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='MedicationStockRemoval' THEN IF (SELECT current_supply FROM medications WHERE id=90001)=8 THEN PERFORM nextval('synthetic_stock_removal_attempt'); END IF; RAISE EXCEPTION 'synthetic removal audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER synthetic_stock_removal_failure BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION synthetic_stock_removal_failure()").await.unwrap();
    let result = remove(&fixture, input()).await;
    let row = fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT is_called FROM synthetic_stock_removal_attempt",
        ))
        .await
        .unwrap()
        .unwrap();
    let reached: bool = row.try_get("", "is_called").unwrap();
    let effect = fixture.effect().await;
    fixture.close().await;
    assert!(matches!(result, Err(OperationError::Unavailable)));
    assert!(reached);
    assert_eq!(effect, ("10.00".into(), 0, 0));
}

#[tokio::test]
async fn stock_removal_current_role_controls_replay_and_history() {
    let fixture = Fixture::new().await;
    assert!(remove(&fixture, input()).await.is_ok());
    let tenant = access::begin(&fixture.runtime, &scope()).await.unwrap();
    fixture.admin.execute_unprepared("UPDATE household_memberships SET role='member',permissions_version=permissions_version+1 WHERE id=74001").await.unwrap();
    let replay = stock_removals::create(&tenant, input(), None).await;
    let history = stock_removals::history(&tenant, "90001", 1, 20).await;
    tenant.rollback().await.unwrap();
    let effect = fixture.effect().await;
    fixture.close().await;
    assert!(matches!(replay, Err(OperationError::Forbidden)));
    assert!(matches!(history, Err(OperationError::Forbidden)));
    assert_eq!(effect, ("8.00".into(), 1, 1));
}

#[tokio::test]
async fn stock_removal_rejects_invalid_contract_fields_without_effect() {
    let fixture = Fixture::new().await;
    for quantity in ["0", "-1", "0.001", "NaN", "1e1", "+1", " 1", "100000000"] {
        let mut command = input();
        command.quantity = quantity.into();
        assert!(matches!(
            remove(&fixture, command).await,
            Err(OperationError::Validation { .. })
        ));
    }
    let mut command = input();
    command.reason = "private-invalid".into();
    assert!(matches!(
        remove(&fixture, command).await,
        Err(OperationError::Validation { .. })
    ));
    let mut command = input();
    command.note = Some("x".repeat(1001));
    assert!(matches!(
        remove(&fixture, command).await,
        Err(OperationError::Validation { .. })
    ));
    let mut command = input();
    command.submission_id = "invalid".into();
    assert!(matches!(
        remove(&fixture, command).await,
        Err(OperationError::Validation { .. })
    ));
    let mut command = input();
    command.medication_id = "missing-medication".into();
    assert!(matches!(
        remove(&fixture, command).await,
        Err(OperationError::NotFound)
    ));
    let effect = fixture.effect().await;
    fixture.close().await;
    assert_eq!(effect, ("10.00".into(), 0, 0));
}
