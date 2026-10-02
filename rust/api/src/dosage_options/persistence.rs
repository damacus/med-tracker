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

pub(super) async fn record_sync(
    db: &DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    record_type: &str,
    id: i64,
    portable_id: &str,
    action: &str,
) -> Result<(), ApiError> {
    record_change(
        db,
        context,
        request_id,
        SyncRecord {
            record_type,
            record_id: id,
            portable_id,
            action,
            person_portable_id: None,
        },
    )
    .await
}
