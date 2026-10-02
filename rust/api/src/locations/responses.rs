use crate::medication_management::error_response;
use crate::ApiError;
use crate::AuthContext;
use axum::http::StatusCode;
use axum::response::Response;
use sea_orm::DatabaseTransaction;

pub(super) async fn failure(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    action: &str,
    status: StatusCode,
    code: &str,
    message: &str,
) -> Result<Response, ApiError> {
    error_response(
        db,
        context,
        method,
        "api/v1/locations",
        "LocationPolicy",
        action,
        status,
        code,
        message,
        None,
    )
    .await
}
