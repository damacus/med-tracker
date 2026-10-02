use crate::authenticate;
use crate::database_error;
use crate::entities::grant;
use crate::entities::medication;
use crate::scope;
use crate::ApiError;
use crate::AppState;
use crate::AuthContext;
use axum::http::HeaderMap;
use chrono::Utc;
use sea_orm::ColumnTrait;
use sea_orm::Condition;
use sea_orm::ConnectionTrait;
use sea_orm::DatabaseTransaction;
use sea_orm::DbBackend;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::Statement;
use sea_orm::TransactionTrait;

pub(crate) async fn request_context(
    state: &AppState,
    headers: &HeaderMap,
    household_id: i64,
) -> Result<(DatabaseTransaction, AuthContext), ApiError> {
    let db = state.db.begin().await.map_err(database_error)?;
    let context = match authenticate(state, &db, headers, household_id).await {
        Ok(context) => context,
        Err(error) => {
            if error.preserve_activity {
                db.commit().await.map_err(database_error)?;
            }
            return Err(error);
        }
    };
    Ok((db, context))
}

pub(crate) fn household_manager(context: &AuthContext) -> bool {
    matches!(context.membership.role.as_str(), "owner" | "administrator")
}

pub(crate) async fn visible_medication(
    db: &DatabaseTransaction,
    context: &AuthContext,
    id: &str,
) -> Result<Option<medication::Model>, ApiError> {
    let mut query = scope(context.membership.household_id, &context.membership);
    query = match id.parse::<i64>() {
        Ok(id) => query.filter(medication::Column::Id.eq(id)),
        Err(_) => query.filter(medication::Column::PortableId.eq(id)),
    };
    query.one(db).await.map_err(database_error)
}

pub(crate) async fn lock_medication(db: &DatabaseTransaction, id: i64) -> Result<(), ApiError> {
    db.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT id FROM medications WHERE id = $1 FOR UPDATE",
        [id.into()],
    ))
    .await
    .map_err(database_error)?
    .ok_or_else(ApiError::not_found)?;
    Ok(())
}

pub(crate) async fn may_create(
    db: &DatabaseTransaction,
    context: &AuthContext,
) -> Result<bool, ApiError> {
    if household_manager(context) {
        return Ok(true);
    }
    let grant = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(context.membership.household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(context.membership.id))
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
