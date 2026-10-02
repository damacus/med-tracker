use super::*;

pub(super) async fn find_person(
    db: &DatabaseTransaction,
    context: &AuthContext,
    id: &str,
) -> Result<Option<person::Model>, ApiError> {
    let mut query = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(context.membership.household_id));
    query = match id.parse::<i64>() {
        Ok(id) => query.filter(person::Column::Id.eq(id)),
        Err(_) => query.filter(person::Column::PortableId.eq(id)),
    };
    query.one(db).await.map_err(database_error)
}

pub(super) async fn person_access(
    db: &DatabaseTransaction,
    context: &AuthContext,
    person_id: i64,
    manage: bool,
) -> Result<bool, ApiError> {
    let mut query = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(context.membership.household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(context.membership.id))
        .filter(grant::Column::PersonId.eq(person_id))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(
            Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(grant::Column::ExpiresAt.gt(Utc::now().naive_utc())),
        );
    if manage {
        query = query.filter(grant::Column::AccessLevel.eq("manage"));
    } else {
        query = query.filter(grant::Column::AccessLevel.is_in(["view", "record", "manage"]));
    }
    Ok(query.one(db).await.map_err(database_error)?.is_some())
}

pub(super) async fn find_schedule(
    db: &DatabaseTransaction,
    context: &AuthContext,
    id: &str,
) -> Result<Option<schedule::Model>, ApiError> {
    let mut query = schedule::Entity::find()
        .filter(schedule::Column::HouseholdId.eq(context.membership.household_id))
        .filter(schedule::Column::RetiredAt.is_null());
    query = match id.parse::<i64>() {
        Ok(id) => query.filter(schedule::Column::Id.eq(id)),
        Err(_) => query.filter(schedule::Column::PortableId.eq(id)),
    };
    let found = query.one(db).await.map_err(database_error)?;
    if let Some(found) = found {
        if person_access(db, context, found.person_id, false).await? {
            return Ok(Some(found));
        }
    }
    Ok(None)
}

pub(super) async fn find_dosage(
    db: &DatabaseTransaction,
    context: &AuthContext,
    id: &str,
) -> Result<Option<dosage::Model>, ApiError> {
    let mut query = dosage::Entity::find()
        .filter(dosage::Column::HouseholdId.eq(context.membership.household_id));
    query = match id.parse::<i64>() {
        Ok(id) => query.filter(dosage::Column::Id.eq(id)),
        Err(_) => query.filter(dosage::Column::PortableId.eq(id)),
    };
    let found = query.one(db).await.map_err(database_error)?;
    if let Some(found) = found {
        if visible_medication(db, context, &found.medication_id.to_string())
            .await?
            .is_some()
        {
            return Ok(Some(found));
        }
    }
    Ok(None)
}
