use crate::entities::security_audit_event;
use crate::medication_management::{error_response, finish, household_manager, request_context};
use crate::{database_error, ApiError, AppState};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde_json::{json, Value};

pub(super) async fn index(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    if !household_manager(&context) {
        return error_response(
            db,
            &context,
            "GET",
            "api/v1/admin/audit_logs",
            "AuditLogPolicy",
            "index",
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
            None,
        )
        .await;
    }
    let events = security_audit_event::Entity::find()
        .filter(security_audit_event::Column::HouseholdId.eq(household_id))
        .order_by_desc(security_audit_event::Column::CreatedAt)
        .order_by_desc(security_audit_event::Column::Id)
        .limit(100)
        .all(&db)
        .await
        .map_err(database_error)?;
    let rows: Vec<Value> = events
        .into_iter()
        .map(|event| {
            json!({
                "id": event.id,
                "event_type": event.event_type,
                "actor_account_id": event.actor_account_id,
                "actor_membership_id": event.actor_membership_id,
                "request_id": event.request_id,
                "metadata": event.metadata,
                "created_at": event.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string()
            })
        })
        .collect();
    finish(
        db,
        &context,
        "GET",
        "api/v1/admin/audit_logs",
        "AuditLogPolicy",
        "index",
        StatusCode::OK,
        true,
        json!({"data": rows}),
        None,
    )
    .await
}
