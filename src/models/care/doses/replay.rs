use super::*;
use loco_rs::model::TenantQueryExt;
use sea_orm::sea_query::{Expr, ExprTrait};

pub(super) async fn replay_matches(
    db: &DatabaseTransaction,
    context: &DoseContext<'_>,
    household_id: i64,
    take: &medication_take::Model,
    command: &Take,
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
    let source_matches = match (&command.source, stored_kind) {
        (DoseSource::Schedule(ScheduleId(id)), "schedule")
        | (DoseSource::Assignment(AssignmentId(id)), "person_medication") => {
            id.matches(stored_id, &stored_portable_id)
        }
        _ => false,
    };
    let time_matches = Some(command.taken_at) == take.taken_at;
    let amount_matches = command
        .dose_amount
        .is_none_or(|amount| Some(amount) == take.dose_amount);
    let unit_matches = command
        .dose_unit
        .as_deref()
        .is_none_or(|unit| Some(unit) == take.dose_unit.as_deref());
    let stock_matches = command
        .taken_from_medication_id
        .is_none_or(|id| Some(id.0) == take.taken_from_medication_id);
    Ok(source_matches && time_matches && amount_matches && unit_matches && stock_matches)
}

pub(super) async fn lock_client_uuid(
    db: &DatabaseTransaction,
    client_uuid: Uuid,
) -> Result<(), ApiError> {
    let canonical = client_uuid.hyphenated().to_string();
    let digest = Sha256::digest(canonical.as_bytes());
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

pub(super) async fn existing_take(
    db: &DatabaseTransaction,
    household_id: i64,
    client_uuid: Uuid,
) -> Result<Option<medication_take::Model>, ApiError> {
    let candidates = medication_take::Entity::find()
        .in_tenant(household_id)
        .filter(
            Expr::cust(migration::CANONICAL_CLIENT_UUID_SQL).eq(client_uuid.simple().to_string()),
        )
        .all(db)
        .await
        .map_err(database_error)?;
    let mut matching = candidates.into_iter().filter(|record| {
        record
            .client_uuid
            .as_deref()
            .and_then(|value| Uuid::parse_str(value).ok())
            == Some(client_uuid)
    });
    let first = matching.next();
    if matching.next().is_some() {
        return Err(OperationError::Conflict {
            code: "idempotency_key_unavailable".into(),
            details: json!({"message": "Medication take idempotency key is unavailable"}),
        });
    }
    Ok(first)
}
