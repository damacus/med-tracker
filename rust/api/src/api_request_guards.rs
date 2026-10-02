use crate::{
    audit, authenticate, database_error, oauth, rate_limit, restricted_role, ApiError, AppState,
};
use axum::extract::{Request, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use sea_orm::TransactionTrait;
use serde_json::json;
use std::net::SocketAddr;

pub(super) async fn rate_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path();
    if path == "/up" || path == "/health" {
        return next.run(request).await;
    }
    let Some(peer) = request
        .extensions()
        .get::<axum::extract::connect_info::ConnectInfo<SocketAddr>>()
        .map(|connection| connection.0.ip())
    else {
        return next.run(request).await;
    };
    if rate_limit::direct_loopback(peer, &state.trusted_proxy_ips) {
        return next.run(request).await;
    }
    let ip = rate_limit::client_ip(peer, request.headers(), &state.trusted_proxy_ips);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if let Some(rejection) = state.rate_limiter.check(ip, request.method(), path, now) {
        let mut response = (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({"error": {
                "code": "rate_limited",
                "message": format!("Rate limit exceeded. Retry in {} seconds.", rejection.retry_after)
            }})),
        )
            .into_response();
        for (name, value) in [
            ("retry-after", rejection.retry_after),
            ("ratelimit-limit", u64::from(rejection.limit)),
            ("ratelimit-remaining", 0),
            ("ratelimit-reset", rejection.reset_at),
        ] {
            response.headers_mut().insert(
                name,
                HeaderValue::from_str(&value.to_string()).expect("numeric rate header"),
            );
        }
        return response;
    }
    next.run(request).await
}

pub(super) async fn private_api_cache(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    response
}

pub(super) async fn cookie_api_csrf(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let api = request.uri().path().starts_with("/api/v1/");
    let unsafe_api = !matches!(
        *request.method(),
        axum::http::Method::GET | axum::http::Method::HEAD | axum::http::Method::OPTIONS
    );
    if !api
        || request.headers().contains_key(header::AUTHORIZATION)
        || !request.headers().contains_key(header::COOKIE)
    {
        return next.run(request).await;
    }
    let trusted_origin = !unsafe_api || oauth::trusted_cookie_origin(&state, request.headers());
    let db = match state.db.begin().await {
        Ok(db) => db,
        Err(error) => return database_error(error).into_response(),
    };
    if let Err(error) = restricted_role(&db).await {
        return database_error(error).into_response();
    }
    let session = match oauth::browser_session(&state, &db, request.headers()).await {
        Ok(Some(session)) => session,
        Ok(None) => {
            return if !unsafe_api {
                let _ = db.rollback().await;
                next.run(request).await
            } else if trusted_origin {
                ApiError::unauthorized().into_response()
            } else {
                ApiError::forbidden().into_response()
            };
        }
        Err(_) => return ApiError::internal().into_response(),
    };
    let csrf = request
        .headers()
        .get("x-csrf-token")
        .and_then(|value| value.to_str().ok());
    if unsafe_api && (!trusted_origin || csrf != Some(session.csrf.as_str())) {
        let request_id = uuid::Uuid::new_v4().to_string();
        if *request.method() == axum::http::Method::POST {
            if let Some(household_id) = dose_household_path(request.uri().path()) {
                if let Ok(context) =
                    authenticate(&state, &db, request.headers(), household_id).await
                {
                    if let Err(error) = audit::record_cookie_write_denial(
                        &db,
                        &context,
                        &request_id,
                        request.method().as_str(),
                        StatusCode::FORBIDDEN,
                    )
                    .await
                    {
                        return database_error(error).into_response();
                    }
                }
            }
        }
        if let Err(error) = db.commit().await {
            return database_error(error).into_response();
        }
        let error = ApiError::forbidden();
        let mut response = (
            error.status,
            Json(json!({"error": {"code": error.code, "message": error.message, "request_id": request_id}})),
        )
            .into_response();
        if let Ok(value) = HeaderValue::from_str(&request_id) {
            response.headers_mut().insert("x-request-id", value);
        }
        return response;
    }
    if let Err(error) = db.commit().await {
        return database_error(error).into_response();
    }
    let mut response = next.run(request).await;
    if let Some(cookie) = oauth::renewed_session_cookie(&state, &session) {
        response.headers_mut().append(header::SET_COOKIE, cookie);
    }
    response
}

fn dose_household_path(path: &str) -> Option<i64> {
    let rest = path.strip_prefix("/api/v1/households/")?;
    let (id, tail) = rest.split_once('/')?;
    (tail == "medication_takes")
        .then(|| id.parse::<i64>().ok())
        .flatten()
}
