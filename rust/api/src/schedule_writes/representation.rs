use super::*;

pub(super) async fn schedule_body(
    db: &DatabaseTransaction,
    context: &AuthContext,
    id: i64,
) -> Result<(Value, String), ApiError> {
    let record = read_schedule::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?;
    let row = serialize_schedules(db, context, vec![record])
        .await?
        .remove(0);
    let body = json!({"data": row});
    let etag = representation_etag(&body);
    Ok((body, etag))
}

pub(super) fn snapshot(record: &schedule::Model) -> Value {
    json!({
        "household_id": record.household_id,
        "portable_id": record.portable_id,
        "person_id": record.person_id,
        "medication_id": record.medication_id,
        "source_dosage_option_id": record.source_dosage_option_id,
        "dose_amount": record.dose_amount.map(|amount| amount.to_string()),
        "dose_unit": record.dose_unit,
        "frequency": record.frequency,
        "start_date": record.start_date,
        "end_date": record.end_date,
        "notes": record.notes,
        "max_daily_doses": record.max_daily_doses,
        "min_hours_between_doses": record.min_hours_between_doses,
        "dose_cycle": record.dose_cycle,
        "schedule_type": record.schedule_type,
        "schedule_config": record.schedule_config,
    })
}
