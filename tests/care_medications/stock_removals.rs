use super::*;
use medications::stock_removals::{self, RemoveStock};

fn input(fixture: &Fixture) -> RemoveStock {
    RemoveStock {
        medication_id: fixture.medication_id.to_string(),
        quantity: "2".into(),
        reason: "dropped".into(),
        note: Some(" synthetic note ".into()),
        dosage_id: None,
        submission_id: fixture.submission_id.clone(),
    }
}

async fn remove(
    fixture: &Fixture,
    input: RemoveStock,
) -> Result<serde_json::Value, OperationError> {
    let tenant = access::begin(&fixture.runtime, &fixture.scope, None).await?;
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
    let first = remove(&fixture, input(&fixture)).await;
    let replay = remove(&fixture, input(&fixture)).await;
    let mut changed = input(&fixture);
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
    let (first, second) = tokio::join!(
        remove(&fixture, input(&fixture)),
        remove(&fixture, input(&fixture))
    );
    let effect = fixture.effect().await;
    fixture.close().await;
    assert!(first.is_ok());
    assert_eq!(first.unwrap(), second.unwrap());
    assert_eq!(effect, ("8.00".into(), 1, 1));
}

#[tokio::test]
async fn stock_removal_insufficient_stock_and_nonmanager_are_rejected() {
    let fixture = Fixture::new().await;
    let mut too_much = input(&fixture);
    too_much.quantity = "11".into();
    let insufficient = remove(&fixture, too_much).await;
    fixture.demote().await;
    let denied = remove(&fixture, input(&fixture)).await;
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
    assert!(remove(&fixture, input(&fixture)).await.is_ok());
    let mut second = input(&fixture);
    second.submission_id = uuid::Uuid::new_v4().to_string();
    let newest = remove(&fixture, second).await.unwrap();
    let tenant = access::begin(&fixture.runtime, &fixture.scope, None)
        .await
        .unwrap();
    let history = stock_removals::history(&tenant, &fixture.medication_id.to_string(), 1, 1).await;
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
    let option = fixture.option(2, 6).await;
    fixture.option(1, 4).await;
    let mut command = input(&fixture);
    command.dosage_id = Some(option.id.to_string());
    let result = remove(&fixture, command).await;
    let row = dosage::Entity::find_by_id(option.id)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let supply = row.current_supply.unwrap().to_string();
    let touched = row.updated_at > option.updated_at;
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
    let failure =
        AuditFailure::install(&fixture, "MedicationStockRemoval", Decimal::new(8, 0)).await;
    let result = remove(&fixture, input(&fixture)).await;
    let reached = failure.reached_and_remove(&fixture).await;
    let effect = fixture.effect().await;
    fixture.close().await;
    assert!(matches!(result, Err(OperationError::Unavailable)));
    assert!(reached);
    assert_eq!(effect, ("10.00".into(), 0, 0));
}

#[tokio::test]
async fn stock_removal_current_role_controls_replay_and_history() {
    let fixture = Fixture::new().await;
    assert!(remove(&fixture, input(&fixture)).await.is_ok());
    let tenant = access::begin(&fixture.runtime, &fixture.scope, None)
        .await
        .unwrap();
    fixture.demote().await;
    let replay = stock_removals::create(&tenant, input(&fixture), None).await;
    let history = stock_removals::history(&tenant, &fixture.medication_id.to_string(), 1, 20).await;
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
        let mut command = input(&fixture);
        command.quantity = quantity.into();
        assert!(matches!(
            remove(&fixture, command).await,
            Err(OperationError::Validation { .. })
        ));
    }
    let mut command = input(&fixture);
    command.reason = "private-invalid".into();
    assert!(matches!(
        remove(&fixture, command).await,
        Err(OperationError::Validation { .. })
    ));
    let mut command = input(&fixture);
    command.note = Some("x".repeat(1001));
    assert!(matches!(
        remove(&fixture, command).await,
        Err(OperationError::Validation { .. })
    ));
    let mut command = input(&fixture);
    command.submission_id = "invalid".into();
    assert!(matches!(
        remove(&fixture, command).await,
        Err(OperationError::Validation { .. })
    ));
    let mut command = input(&fixture);
    command.medication_id = "missing-medication".into();
    assert!(matches!(
        remove(&fixture, command).await,
        Err(OperationError::NotFound)
    ));
    let effect = fixture.effect().await;
    fixture.close().await;
    assert_eq!(effect, ("10.00".into(), 0, 0));
}
