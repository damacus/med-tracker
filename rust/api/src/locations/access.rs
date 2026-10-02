use super::responses::failure;
use crate::database_error;
use crate::entities::grant;
use crate::medication_management::request_context;
use crate::mutation_idempotency;
use crate::ApiError;
use crate::AppState;
use crate::AuthContext;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::Response;
use chrono::Utc;
use sea_orm::ColumnTrait;
use sea_orm::Condition;
use sea_orm::DatabaseTransaction;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;

pub(crate) fn manager(context: &AuthContext) -> bool {
    matches!(context.membership.role.as_str(), "owner" | "administrator")
}

pub(super) async fn person_manage_access(
    db: &DatabaseTransaction,
    context: &AuthContext,
    person_id: i64,
) -> Result<Option<bool>, ApiError> {
    let grant = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(context.membership.household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(context.membership.id))
        .filter(grant::Column::PersonId.eq(person_id))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(
            Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(grant::Column::ExpiresAt.gt(Utc::now().naive_utc())),
        )
        .one(db)
        .await
        .map_err(database_error)?;
    Ok(grant.map(|grant| grant.access_level == "manage"))
}

pub(super) async fn manager_context(
    state: &AppState,
    headers: &HeaderMap,
    household_id: i64,
    method: &str,
    action: &str,
) -> Result<Result<(DatabaseTransaction, AuthContext), Response>, ApiError> {
    let (db, _) = request_context(state, headers, household_id).await?;
    let (_, context) =
        mutation_idempotency::lock_household_and_reauthenticate(state, &db, headers, household_id)
            .await?;
    if manager(&context) {
        Ok(Ok((db, context)))
    } else {
        let response = failure(
            db,
            &context,
            method,
            action,
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
        )
        .await?;
        Ok(Err(response))
    }
}
