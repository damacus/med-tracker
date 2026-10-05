use super::*;

pub(super) async fn replay_matches(
    db: &DatabaseTransaction,
    context: &DoseContext<'_>,
    household_id: i64,
    take: &medication_take::Model,
    attributes: &Value,
) -> Result<bool, ApiError> {
    let stored_source = if let Some(id) = take.schedule_id {
        schedule::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(database_error)?
            .filter(|value| value.household_id == household_id)
            .map(|value| ("schedule", value.id, value.portable_id, value.person_id))
    } else if let Some(id) = take.person_medication_id {
        person_medication::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(database_error)?
            .filter(|value| value.household_id == household_id)
            .map(|value| {
                (
                    "person_medication",
                    value.id,
                    value.portable_id,
                    value.person_id,
                )
            })
    } else {
        None
    };
    let (stored_kind, stored_id, stored_portable_id, stored_person) =
        stored_source.ok_or(OperationError::NotFound)?;
    if !allowed_person(db, context, stored_person, true).await? {
        return Err(OperationError::Forbidden);
    }
    let kind = attributes.get("source_type").and_then(Value::as_str);
    let source_id = attributes.get("source_id").and_then(Value::as_str);
    let source_matches = kind == Some(stored_kind)
        && source_id.is_some_and(|value| {
            value == stored_portable_id || value.parse::<i64>().ok() == Some(stored_id)
        });
    let time_matches = parse_input_time(attributes.get("taken_at")).ok() == take.taken_at;
    let amount_matches = match attributes.get("dose_amount") {
        None => true,
        Some(value) => decimal_from_json(Some(value)).ok().flatten() == take.dose_amount,
    };
    let unit_matches = attributes
        .get("dose_unit")
        .is_none_or(|value| value.is_null() || value.as_str() == take.dose_unit.as_deref());
    let stock_matches = attributes
        .get("taken_from_medication_id")
        .is_none_or(|value| value.as_i64() == take.taken_from_medication_id);
    Ok(source_matches && time_matches && amount_matches && unit_matches && stock_matches)
}

pub(super) async fn lock_client_uuid(
    db: &DatabaseTransaction,
    client_uuid: &str,
) -> Result<(), ApiError> {
    let digest = Sha256::digest(client_uuid.as_bytes());
    let lock_id = i64::from_be_bytes(
        digest[..8]
            .try_into()
            .map_err(|_| OperationError::Unavailable)?,
    );
    db.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT pg_advisory_xact_lock($1)",
        [lock_id.into()],
    ))
    .await
    .map_err(database_error)?;
    Ok(())
}
