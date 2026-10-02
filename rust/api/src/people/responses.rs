use crate::medication_management::error_response;
use crate::ApiError;
use crate::AuthContext;
use axum::http::StatusCode;
use axum::response::Response;
use sea_orm::DatabaseTransaction;
use serde_json::json;
use serde_json::Value;

pub(super) async fn failure(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    action: &str,
    status: StatusCode,
) -> Result<Response, ApiError> {
    failure_with_errors(db, context, method, action, status, None).await
}

pub(super) async fn failure_with_errors(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    action: &str,
    status: StatusCode,
    field_errors: Option<Value>,
) -> Result<Response, ApiError> {
    let (code, message) = match status {
        StatusCode::BAD_REQUEST => ("bad_request", "Invalid request body"),
        StatusCode::FORBIDDEN => (
            "forbidden",
            "You are not authorized to perform this action.",
        ),
        StatusCode::NOT_FOUND => ("not_found", "Record not found"),
        _ => ("validation_failed", "Validation failed"),
    };
    let errors = (status == StatusCode::UNPROCESSABLE_ENTITY)
        .then(|| field_errors.unwrap_or_else(|| json!({"person": ["is invalid"]})));
    error_response(
        db,
        context,
        method,
        "api/v1/people",
        "PersonPolicy",
        action,
        status,
        code,
        message,
        errors,
    )
    .await
}
