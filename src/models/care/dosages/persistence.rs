use super::*;

pub(super) fn dosage_snapshot(record: &dosage::Model) -> Value {
    json!({
        "id": record.id,
        "portable_id": record.portable_id,
        "medication_id": record.medication_id,
        "amount": record.amount.to_string(),
        "unit": record.unit,
        "frequency": record.frequency,
        "description": record.description,
        "default_for_adults": record.default_for_adults,
        "default_for_children": record.default_for_children,
        "default_max_daily_doses": record.default_max_daily_doses,
        "default_min_hours_between_doses": record.default_min_hours_between_doses.to_string(),
        "default_dose_cycle": record.default_dose_cycle,
        "current_supply": record.current_supply.map(|value| value.to_string()),
        "reorder_threshold": record.reorder_threshold.map(|value| value.to_string())
    })
}

pub(super) async fn persist(
    tenant: &TenantTransaction,
    row: &dosage::Model,
    before: Option<Value>,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    let action = if before.is_some() { "update" } else { "create" };
    administration::persistence::record_version_as(
        tenant,
        "MedicationDosageOption",
        row.id,
        if before.is_some() {
            "api_update"
        } else {
            "api_create"
        },
        before,
        dosage_snapshot(row),
        provenance,
    )
    .await?;
    sync(
        tenant,
        "MedicationDosageOption",
        row.id,
        &row.portable_id,
        action,
    )
    .await
}

pub(super) async fn sync(
    tenant: &TenantTransaction,
    kind: &str,
    id: i64,
    portable: &str,
    action: &str,
) -> Result<(), OperationError> {
    let now = Utc::now().naive_utc();
    api_change_event::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        household_membership_id: Set(Some(tenant.membership().id)),
        account_id: Set(Some(tenant.scope().actor.account_id)),
        action: Set(action.into()),
        record_type: Set(kind.into()),
        record_id: Set(id),
        record_portable_id: Set(Some(portable.into())),
        request_id: Set(Some(tenant.scope().request_id.clone())),
        metadata: Set(json!({"record_type":kind,"record_id":id,"portable_id":portable})),
        occurred_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await?;
    Ok(())
}
