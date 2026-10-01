use super::*;

pub(super) async fn find_person(
    db: &DatabaseTransaction,
    context: &AuthContext,
    identifier: &str,
) -> Result<Option<person::Model>, ApiError> {
    let query = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(context.membership.household_id))
        .filter(person::Column::Id.in_subquery(granted_people(&context.membership)));
    let query = match identifier.parse::<i64>() {
        Ok(id) => query.filter(person::Column::Id.eq(id)),
        Err(_) => query.filter(person::Column::PortableId.eq(identifier)),
    };
    query.one(db).await.map_err(database_error)
}

pub(super) async fn can_manage_person(
    db: &DatabaseTransaction,
    context: &AuthContext,
    person_id: i64,
) -> Result<bool, ApiError> {
    let grant = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(context.membership.household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(context.membership.id))
        .filter(grant::Column::PersonId.eq(person_id))
        .filter(grant::Column::AccessLevel.eq("manage"))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(
            Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(grant::Column::ExpiresAt.gt(Utc::now().naive_utc())),
        )
        .one(db)
        .await
        .map_err(database_error)?;
    Ok(grant.is_some())
}

pub(super) async fn find_medication(
    db: &DatabaseTransaction,
    context: &AuthContext,
    identifier: &str,
) -> Result<Option<medication::Model>, ApiError> {
    let query = scope(context.membership.household_id, &context.membership);
    let query = match identifier.parse::<i64>() {
        Ok(id) => query.filter(medication::Column::Id.eq(id)),
        Err(_) => query.filter(medication::Column::PortableId.eq(identifier)),
    };
    query.one(db).await.map_err(database_error)
}

pub(super) async fn find_visible_option(
    db: &DatabaseTransaction,
    context: &AuthContext,
    identifier: &str,
) -> Result<Option<dosage::Model>, ApiError> {
    let query = dosage::Entity::find()
        .filter(dosage::Column::HouseholdId.eq(context.membership.household_id));
    let query = match identifier.parse::<i64>() {
        Ok(id) => query.filter(dosage::Column::Id.eq(id)),
        Err(_) => query.filter(dosage::Column::PortableId.eq(identifier)),
    };
    let Some(option) = query.one(db).await.map_err(database_error)? else {
        return Ok(None);
    };
    if find_medication(db, context, &option.medication_id.to_string())
        .await?
        .is_none()
    {
        return Ok(None);
    }
    Ok(Some(option))
}

pub(super) async fn find_assignment(
    db: &DatabaseTransaction,
    context: &AuthContext,
    identifier: &str,
) -> Result<Option<person_medication::Model>, ApiError> {
    let query = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(context.membership.household_id))
        .filter(person_medication::Column::RetiredAt.is_null())
        .filter(
            person_medication::Column::PersonId.in_subquery(granted_people(&context.membership)),
        );
    let query = match identifier.parse::<i64>() {
        Ok(id) => query.filter(person_medication::Column::Id.eq(id)),
        Err(_) => query.filter(person_medication::Column::PortableId.eq(identifier)),
    };
    query.one(db).await.map_err(database_error)
}
