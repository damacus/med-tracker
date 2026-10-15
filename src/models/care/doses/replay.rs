use super::*;
use loco_rs::model::TenantQueryExt;
use sea_orm::sea_query::{Expr, ExprTrait};

pub(super) async fn replay_matches(
    db: &DatabaseTransaction,
    context: &DoseContext<'_>,
    household_id: HouseholdId,
    take: &medication_take::Model,
    request: &PreparedRequest,
) -> Result<bool, ApiError> {
    let stored_source = if let Some(id) = take.schedule_id {
        schedule::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(database_error)?
            .filter(|value| value.household_id == household_id.get())
            .map(|value| {
                (
                    DoseSource::Schedule(ScheduleId::from_database(value.id)),
                    value.portable_id,
                    value.person_id,
                )
            })
    } else if let Some(id) = take.person_medication_id {
        person_medication::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(database_error)?
            .filter(|value| value.household_id == household_id.get())
            .map(|value| {
                (
                    DoseSource::Assignment(AssignmentId::from_database(value.id)),
                    value.portable_id,
                    value.person_id,
                )
            })
    } else {
        None
    };
    let (stored_source, stored_portable_id, stored_person) =
        stored_source.ok_or(OperationError::NotFound)?;
    if !allowed_person(db, context, PersonId::from_database(stored_person)).await? {
        return Err(OperationError::Forbidden);
    }
    let source_matches = request.source.matches(stored_source, &stored_portable_id);
    let time_matches = request.taken_at.as_ref().ok().copied() == take.taken_at;
    let amount_matches = match &request.amount {
        Ok(None) => true,
        Ok(Some(amount)) => Some(*amount) == take.dose_amount,
        Err(_) => false,
    };
    let unit_matches = request
        .unit
        .as_deref()
        .is_none_or(|unit| Some(unit) == take.dose_unit.as_deref());
    let stock_matches = request
        .selected
        .is_none_or(|id| Some(id.get()) == take.taken_from_medication_id);
    Ok(source_matches && time_matches && amount_matches && unit_matches && stock_matches)
}

pub(super) async fn lock_client_uuid(
    db: &DatabaseTransaction,
    client_uuid: Uuid,
) -> Result<(), ApiError> {
    let canonical = client_uuid.hyphenated().to_string();
    let digest = Sha256::digest(canonical.as_bytes());
    let lock_id = i64::from_be_bytes(
        digest
            .get(..8)
            .ok_or(OperationError::Unavailable)?
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
    household_id: HouseholdId,
    client_uuid: Uuid,
) -> Result<Option<medication_take::Model>, ApiError> {
    let parsed = client_uuid;
    let candidates = medication_take::Entity::find()
        .in_tenant(household_id.get())
        .filter(Expr::cust(migration::CANONICAL_CLIENT_UUID_SQL).eq(parsed.simple().to_string()))
        .all(db)
        .await
        .map_err(database_error)?;
    let mut matching = candidates.into_iter().filter(|record| {
        record
            .client_uuid
            .as_deref()
            .and_then(|value| Uuid::parse_str(value).ok())
            == Some(parsed)
    });
    let first = matching.next();
    if matching.next().is_some() {
        return Err(DoseConflict::IdempotencyKeyUnavailable.into_error());
    }
    Ok(first)
}
