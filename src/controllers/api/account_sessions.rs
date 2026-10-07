use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::IntoResponse,
};
use loco_rs::{controller::middleware::request_id::LocoRequestId, prelude::*};
use serde_json::{Value, json};

use crate::models::{
    errors::OperationError,
    identity::{
        account_sessions,
        resource::{self, AuthenticationError},
    },
};

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/v1/auth")
        .add("/households", get(households))
        .add("/sessions", get(sessions))
        .add("/sessions/{id}", delete(revoke))
        .add("/logout", delete(logout))
}

async fn households(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    request: Option<axum::Extension<LocoRequestId>>,
) -> Response {
    let request_id = request_id(request);
    let principal = match resource::authenticate(&ctx.db, &headers).await {
        Ok(principal) => principal,
        Err(error) => return authentication(error, &request_id),
    };
    match account_sessions::households(&ctx.db, principal.account_id()).await {
        Ok(body) => reply(StatusCode::OK, Some(body), &request_id),
        Err(error) => operation(error, &request_id),
    }
}

async fn sessions(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    request: Option<axum::Extension<LocoRequestId>>,
) -> Response {
    let request_id = request_id(request);
    let principal = match resource::authenticate(&ctx.db, &headers).await {
        Ok(principal) => principal,
        Err(error) => return authentication(error, &request_id),
    };
    match account_sessions::sessions(
        &ctx.db,
        principal.account_id(),
        principal.provenance().method,
    )
    .await
    {
        Ok(body) => reply(StatusCode::OK, Some(body), &request_id),
        Err(error) => operation(error, &request_id),
    }
}

async fn revoke(
    State(ctx): State<AppContext>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    request: Option<axum::Extension<LocoRequestId>>,
) -> Response {
    let request_id = request_id(request);
    let principal = match resource::authenticate(&ctx.db, &headers).await {
        Ok(principal) => principal,
        Err(error) => return authentication(error, &request_id),
    };
    match account_sessions::revoke(
        &ctx.db,
        principal.account_id(),
        principal.provenance().method,
        id,
        &request_id,
    )
    .await
    {
        Ok(()) => reply(StatusCode::NO_CONTENT, None, &request_id),
        Err(error) => operation(error, &request_id),
    }
}

async fn logout(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    request: Option<axum::Extension<LocoRequestId>>,
) -> Response {
    let request_id = request_id(request);
    match account_sessions::logout(&ctx.db, &headers, &request_id).await {
        Ok(()) => reply(StatusCode::NO_CONTENT, None, &request_id),
        Err(error) => operation(error, &request_id),
    }
}

fn request_id(request: Option<axum::Extension<LocoRequestId>>) -> String {
    request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |axum::Extension(id)| id.get().to_owned(),
    )
}

fn authentication(error: AuthenticationError, request_id: &str) -> Response {
    match error {
        AuthenticationError::Unauthenticated => failure(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "Authentication required",
            request_id,
        ),
        AuthenticationError::Forbidden => failure(
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
            request_id,
        ),
        AuthenticationError::InsufficientScope { authenticate } => {
            let mut response = failure(
                StatusCode::FORBIDDEN,
                "forbidden",
                "You are not authorized to perform this action.",
                request_id,
            );
            if let Ok(value) = HeaderValue::from_str(&authenticate) {
                response
                    .headers_mut()
                    .insert(header::WWW_AUTHENTICATE, value);
            }
            response
        }
        AuthenticationError::Unavailable => failure(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "Internal server error",
            request_id,
        ),
    }
}

fn operation(error: OperationError, request_id: &str) -> Response {
    match error {
        OperationError::NotFound => failure(
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
            request_id,
        ),
        OperationError::Unauthenticated => {
            authentication(AuthenticationError::Unauthenticated, request_id)
        }
        OperationError::Forbidden => authentication(AuthenticationError::Forbidden, request_id),
        _ => failure(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "Internal server error",
            request_id,
        ),
    }
}

fn failure(status: StatusCode, code: &str, message: &str, request_id: &str) -> Response {
    reply(
        status,
        Some(json!({"error":{"code":code,"message":message,"request_id":request_id}})),
        request_id,
    )
}

fn reply(status: StatusCode, body: Option<Value>, request_id: &str) -> Response {
    let mut response = match body {
        Some(body) => (status, Json(body)).into_response(),
        None => status.into_response(),
    };
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    if let Ok(value) = HeaderValue::from_str(request_id) {
        response.headers_mut().insert("x-request-id", value);
    }
    if status == StatusCode::UNAUTHORIZED {
        response
            .headers_mut()
            .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
    }
    response
}
