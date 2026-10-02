use super::*;

pub(super) async fn failure(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    action: &str,
    status: StatusCode,
    code: &str,
    message: &str,
) -> Result<Response, ApiError> {
    error_response(
        db,
        context,
        method,
        "api/v1/dosage_options",
        "DosageOptionPolicy",
        action,
        status,
        code,
        message,
        None,
    )
    .await
}

pub(super) async fn validation(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    action: &str,
) -> Result<Response, ApiError> {
    error_response(
        db,
        context,
        method,
        "api/v1/dosage_options",
        "DosageOptionPolicy",
        action,
        StatusCode::UNPROCESSABLE_ENTITY,
        "validation_failed",
        "Validation failed",
        Some(json!({"dosage_option": ["is invalid"]})),
    )
    .await
}

pub(crate) fn dosage_value(
    record: dosage::Model,
    medication_portable_id: &str,
) -> Result<Value, ApiError> {
    let amount = record.amount;
    let unit = record.unit;
    let frequency = record.frequency;
    let max_doses = record.default_max_daily_doses;
    let min_hours = record.default_min_hours_between_doses;
    let cycle = match record.default_dose_cycle {
        0 => "daily",
        1 => "weekly",
        2 => "monthly",
        _ => return Err(ApiError::internal()),
    };
    Ok(json!({
        "id": record.id, "portable_id": record.portable_id,
        "medication_id": record.medication_id, "medication_portable_id": medication_portable_id,
        "amount": decimal_string(amount.to_string()), "unit": unit, "frequency": frequency,
        "description": record.description, "default_for_adults": record.default_for_adults,
        "default_for_children": record.default_for_children,
        "default_max_daily_doses": max_doses,
        "default_min_hours_between_doses": decimal_string(min_hours.to_string()),
        "default_dose_cycle": cycle,
        "current_supply": record.current_supply.map(|value| decimal_string(value.to_string())),
        "reorder_threshold": record.reorder_threshold.map(|value| decimal_string(value.to_string())),
        "updated_at": record.updated_at.and_utc().to_rfc3339()
    }))
}

pub(crate) async fn representation(
    db: &DatabaseTransaction,
    record: dosage::Model,
) -> Result<(Value, String), ApiError> {
    let medication = medication::Entity::find_by_id(record.medication_id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::internal)?;
    let body = json!({"data": dosage_value(record, &medication.portable_id)?});
    let etag = representation_etag(&body);
    Ok((body, etag))
}
