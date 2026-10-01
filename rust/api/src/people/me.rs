use crate::database_error;
use crate::entities::account;
use crate::entities::user;
use crate::medication_management::finish;
use crate::medication_management::request_context;
use crate::read_entities::person;
use crate::read_resources::serialize_people;
use crate::tenant_setting;
use crate::ApiError;
use crate::AppState;
use axum::extract::Path;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::Response;
use sea_orm::EntityTrait;
use serde_json::json;

pub(crate) async fn show_me(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let account = account::Entity::find_by_id(context.account_id)
        .one(&db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    let user = user::Entity::find_by_id(context.user_id)
        .one(&db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    let record = person::Entity::find_by_id(user.person_id)
        .one(&db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    tenant_setting(&db, "med_tracker.current_household_id", record.household_id)
        .await
        .map_err(database_error)?;
    let own = serialize_people(&db, vec![record]).await?.remove(0);
    tenant_setting(&db, "med_tracker.current_household_id", household_id)
        .await
        .map_err(database_error)?;
    let status = match account.status {
        1 => "unverified",
        2 => "verified",
        3 => "closed",
        _ => return Err(ApiError::internal()),
    };
    let body = json!({"data": {"id": user.id, "email_address": user.email_address, "membership_role": context.membership.role, "active": user.active, "person": own, "account": {"id": account.id, "email": account.email, "status": status}}});
    finish(
        db,
        &context,
        "GET",
        "api/v1/me",
        "MePolicy",
        "show",
        StatusCode::OK,
        true,
        body,
        None,
    )
    .await
}
