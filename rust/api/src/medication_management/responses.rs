use crate::audit;
use crate::database_error;
use crate::entities::medication;
use crate::representation_etag;
use crate::serialize_many;
use crate::ApiError;
use crate::AuthContext;
use axum::http::header;
use axum::http::HeaderValue;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::Json;
use sea_orm::DatabaseTransaction;
use serde_json::json;
use serde_json::Value;
use uuid::Uuid;

pub(crate) async fn medication_body(
    db: &DatabaseTransaction,
    medication: medication::Model,
) -> Result<(Value, String), ApiError> {
    let row = serialize_many(db, vec![medication]).await?.remove(0);
    let body = json!({"data": row});
    let etag = representation_etag(&body);
    Ok((body, etag))
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn finish(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    controller: &str,
    policy: &str,
    action: &str,
    status: StatusCode,
    authorized: bool,
    body: Value,
    etag: Option<&str>,
) -> Result<Response, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    finish_with_request_id(
        db,
        context,
        &request_id,
        method,
        controller,
        policy,
        action,
        status,
        authorized,
        body,
        etag,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn finish_with_request_id(
    db: DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    method: &str,
    controller: &str,
    policy: &str,
    action: &str,
    status: StatusCode,
    authorized: bool,
    mut body: Value,
    etag: Option<&str>,
) -> Result<Response, ApiError> {
    audit::record_resource_request_with_id(
        &db, context, request_id, method, controller, policy, action, status, authorized,
    )
    .await
    .map_err(database_error)?;
    if status.as_u16() >= 400 && body.get("error").is_some() {
        body["error"]["request_id"] = json!(request_id);
    }
    db.commit().await.map_err(database_error)?;
    let mut response = (status, Json(body)).into_response();
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(request_id).map_err(|_| ApiError::internal())?,
    );
    if let Some(etag) = etag {
        response.headers_mut().insert(
            header::ETAG,
            HeaderValue::from_str(etag).map_err(|_| ApiError::internal())?,
        );
    }
    Ok(response)
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn error_response(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    controller: &str,
    policy: &str,
    action: &str,
    status: StatusCode,
    code: &str,
    message: &str,
    errors: Option<Value>,
) -> Result<Response, ApiError> {
    let mut body = json!({"error": {"code": code, "message": message}});
    if let Some(errors) = errors {
        body["error"]["errors"] = errors;
    }
    finish(
        db, context, method, controller, policy, action, status, false, body, None,
    )
    .await
}

pub(super) async fn validation_response(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    action: &str,
    field: &str,
    message: &str,
) -> Result<Response, ApiError> {
    error_response(
        db,
        context,
        method,
        "api/v1/medications",
        "MedicationPolicy",
        action,
        StatusCode::UNPROCESSABLE_ENTITY,
        "validation_failed",
        "Validation failed",
        Some(json!({field: [message]})),
    )
    .await
}
