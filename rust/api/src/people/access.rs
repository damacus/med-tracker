use crate::database_error;
use crate::entities::grant;
use crate::entities::membership;
use crate::read_entities::carer_relationship;
use crate::ApiError;
use crate::AuthContext;
use chrono::Utc;
use sea_orm::ColumnTrait;
use sea_orm::Condition;
use sea_orm::DatabaseTransaction;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::QuerySelect;

pub(crate) async fn may_create(
    db: &DatabaseTransaction,
    member: &membership::Model,
) -> Result<bool, ApiError> {
    if matches!(member.role.as_str(), "owner" | "administrator") {
        return Ok(true);
    }
    let active = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(member.household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(member.id))
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
    Ok(active.is_some())
}

pub(super) async fn manageable(
    db: &DatabaseTransaction,
    context: &AuthContext,
    person_id: i64,
) -> Result<bool, ApiError> {
    let active = manageable_scope(context)
        .filter(grant::Column::PersonId.eq(person_id))
        .one(db)
        .await
        .map_err(database_error)?;
    Ok(active.is_some())
}

fn manageable_scope(context: &AuthContext) -> sea_orm::Select<grant::Entity> {
    grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(context.membership.household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(context.membership.id))
        .filter(grant::Column::AccessLevel.eq("manage"))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(
            Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(grant::Column::ExpiresAt.gt(Utc::now().naive_utc())),
        )
}

pub(crate) async fn manageable_ids(
    db: &DatabaseTransaction,
    context: &AuthContext,
) -> Result<Vec<i64>, ApiError> {
    manageable_scope(context)
        .select_only()
        .column(grant::Column::PersonId)
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(database_error)
}

pub(super) async fn carer_exists(
    db: &DatabaseTransaction,
    person_id: i64,
) -> Result<bool, ApiError> {
    let relationship = carer_relationship::Entity::find()
        .filter(carer_relationship::Column::PatientId.eq(person_id))
        .filter(carer_relationship::Column::Active.eq(true))
        .one(db)
        .await
        .map_err(database_error)?;
    Ok(relationship.is_some())
}
