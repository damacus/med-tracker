use crate::audit;
use crate::database_error;
use crate::medication_management::finish_with_request_id;
use crate::mutation_idempotency;
use crate::mutation_idempotency::Lookup;
use crate::mutation_idempotency::StoredResponse;
use crate::ApiError;
use crate::AuthContext;
use axum::http::HeaderMap;
use axum::http::HeaderValue;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::Json;
use sea_orm::DatabaseTransaction;
use serde_json::json;
use serde_json::Value;
use uuid::Uuid;

#[allow(clippy::too_many_arguments)]
pub(super) async fn keyed_replay(
    db: &DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    method: &str,
    path: &str,
    body: &Value,
    action: &str,
    controller: &str,
    policy: &str,
) -> Result<Option<Response>, ApiError> {
    let Some(key) = mutation_idempotency::key(headers) else {
        return Ok(None);
    };
    let digest = mutation_idempotency::digest(method, path, body);
    match mutation_idempotency::lookup(db, context, key, method, path, &digest).await? {
        Lookup::New => Ok(None),
        Lookup::Replay(saved) => {
            let mut saved = *saved;
            let status = StatusCode::from_u16(saved.response_status as u16)
                .map_err(|_| ApiError::internal())?;
            let request_id = audit::record_resource_request(
                db, context, method, controller, policy, action, status, true,
            )
            .await
            .map_err(database_error)?;
            if status.is_client_error() && saved.response_body.get("error").is_some() {
                saved.response_body["error"]["request_id"] = json!(request_id);
            }
            let mut response = mutation_idempotency::replay(saved)?;
            response.headers_mut().insert(
                "x-request-id",
                HeaderValue::from_str(&request_id).map_err(|_| ApiError::internal())?,
            );
            Ok(Some(response))
        }
        Lookup::Conflict => {
            let request_id = audit::record_resource_request(
                db,
                context,
                method,
                controller,
                policy,
                action,
                StatusCode::CONFLICT,
                true,
            )
            .await
            .map_err(database_error)?;
            let mut response = (StatusCode::CONFLICT, Json(json!({"error":{"code":"idempotency_key_reused","message":"Idempotency key has already been used for a different request","request_id":request_id}}))).into_response();
            response.headers_mut().insert(
                "x-request-id",
                HeaderValue::from_str(&request_id).map_err(|_| ApiError::internal())?,
            );
            Ok(Some(response))
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn store_keyed(
    db: &DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    method: &str,
    path: &str,
    request: &Value,
    status: StatusCode,
    body: &Value,
    request_id: &str,
    etag: Option<&str>,
) -> Result<(), ApiError> {
    if let Some(key) = mutation_idempotency::key(headers) {
        mutation_idempotency::store(
            db,
            context,
            StoredResponse {
                key,
                method,
                path,
                digest: &mutation_idempotency::digest(method, path, request),
                status,
                body: body.clone(),
                request_id,
                etag,
            },
        )
        .await?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn keyed_failure(
    db: DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    method: &str,
    path: &str,
    request: &Value,
    controller: &str,
    policy: &str,
    action: &str,
    status: StatusCode,
    code: &str,
    message: &str,
    errors: Option<Value>,
) -> Result<Response, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let mut body = json!({"error": {"code": code, "message": message, "request_id": request_id}});
    if let Some(errors) = errors {
        body["error"]["errors"] = errors;
    }
    store_keyed(
        &db,
        context,
        headers,
        method,
        path,
        request,
        status,
        &body,
        &request_id,
        None,
    )
    .await?;
    finish_with_request_id(
        db,
        context,
        &request_id,
        method,
        controller,
        policy,
        action,
        status,
        true,
        body,
        None,
    )
    .await
}
