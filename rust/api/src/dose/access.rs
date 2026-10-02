use super::*;

pub(super) async fn allowed_person(
    db: &DatabaseTransaction,
    context: &AuthContext,
    person_id: i64,
    write: bool,
) -> Result<bool, ApiError> {
    let levels = if write {
        vec!["record", "manage"]
    } else {
        vec!["view", "record", "manage"]
    };
    Ok(grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(context.membership.household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(context.membership.id))
        .filter(grant::Column::PersonId.eq(person_id))
        .filter(grant::Column::AccessLevel.is_in(levels))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(
            Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(grant::Column::ExpiresAt.gt(Utc::now().naive_utc())),
        )
        .one(db)
        .await
        .map_err(database_error)?
        .is_some())
}

pub(crate) async fn authorize_sync_replay(
    db: &DatabaseTransaction,
    context: &AuthContext,
    operation: &crate::sync_batch::SyncOperation,
    saved: &Value,
) -> Result<(), ApiError> {
    let portable_id = saved
        .get("record_portable_id")
        .and_then(Value::as_str)
        .ok_or_else(ApiError::forbidden)?;
    let take = medication_take::Entity::find()
        .filter(medication_take::Column::HouseholdId.eq(context.membership.household_id))
        .filter(medication_take::Column::PortableId.eq(portable_id))
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::forbidden)?;
    let source = if let Some(id) = take.schedule_id {
        schedule::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(database_error)?
            .filter(|row| row.household_id == context.membership.household_id)
            .map(|row| ("schedule", row.id, row.portable_id, row.person_id))
    } else if let Some(id) = take.person_medication_id {
        person_medication::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(database_error)?
            .filter(|row| row.household_id == context.membership.household_id)
            .map(|row| ("person_medication", row.id, row.portable_id, row.person_id))
    } else {
        None
    }
    .ok_or_else(ApiError::forbidden)?;
    if !allowed_person(db, context, source.3, true).await?
        || operation
            .attributes
            .get("source_type")
            .and_then(Value::as_str)
            != Some(source.0)
        || !operation
            .attributes
            .get("source_id")
            .and_then(Value::as_str)
            .is_some_and(|id| id == source.2 || id.parse::<i64>().ok() == Some(source.1))
    {
        return Err(ApiError::forbidden());
    }
    Ok(())
}
