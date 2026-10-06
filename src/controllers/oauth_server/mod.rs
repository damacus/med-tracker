use crate::models::identity::{self, ExchangeError, oauth};
use axum::{
    body::{Body, to_bytes},
    extract::Request,
    http::{StatusCode, header},
    response::IntoResponse,
};
use loco_rs::prelude::*;
mod authorization_response;
mod consent;
pub use consent::routes as browser_routes;

pub fn routes() -> Routes {
    Routes::new()
        .add("/.well-known/oauth-authorization-server", get(discovery))
        .add("/api/v1/capabilities", get(capabilities))
        .add("/token", post(token))
        .add("/revoke", post(revoke))
}

async fn discovery(State(ctx): State<AppContext>) -> Response {
    let issuer = ctx.config.server.full_url();
    axum::Json(serde_json::json!({"issuer":issuer,"authorization_endpoint":format!("{issuer}/authorize"),"token_endpoint":format!("{issuer}/token"),"revocation_endpoint":format!("{issuer}/revoke"),"response_types_supported":["code"],"response_modes_supported":["query","form_post"],"grant_types_supported":["authorization_code","refresh_token"],"code_challenge_methods_supported":["S256"],"token_endpoint_auth_methods_supported":["none","client_secret_basic","client_secret_post"]})).into_response()
}

async fn capabilities(State(ctx): State<AppContext>) -> Response {
    match oauth::mobile_clients(&ctx.db).await {
        Ok(clients) => axum::Json(serde_json::json!({"data":{"authentication":{"mobile_oauth":{"household_binding":"account","clients":clients}}}})).into_response(),
        Err(error) => failure(error),
    }
}

async fn token(State(ctx): State<AppContext>, request: Request) -> Response {
    match identity::exchange(&ctx.db, request).await {
        Ok(body) => no_cache(axum::Json(body).into_response()),
        Err(error) => failure(error),
    }
}

async fn revoke(State(ctx): State<AppContext>, request: Request) -> Response {
    let request = match revocation_request(request).await {
        Ok(request) => request,
        Err(error) => return failure(error),
    };
    match oauth::revoke(&ctx.db, request).await {
        Ok(()) => no_cache(StatusCode::OK.into_response()),
        Err(error) => failure(error),
    }
}

async fn revocation_request(request: Request) -> std::result::Result<Request, ExchangeError> {
    if request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        != Some("application/json")
    {
        return Ok(request);
    }
    let (mut parts, body) = request.into_parts();
    let bytes = to_bytes(body, 16_384)
        .await
        .map_err(|_| ExchangeError::InvalidRequest)?;
    let fields: std::collections::BTreeMap<String, String> =
        serde_json::from_slice(&bytes).map_err(|_| ExchangeError::InvalidRequest)?;
    let body = serde_urlencoded::to_string(fields).map_err(|_| ExchangeError::InvalidRequest)?;
    parts.headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/x-www-form-urlencoded"),
    );
    parts.headers.remove(header::CONTENT_LENGTH);
    Ok(Request::from_parts(parts, Body::from(body)))
}

pub(super) fn failure(error: ExchangeError) -> Response {
    let (status, body, authenticate) = match error {
        ExchangeError::InvalidRequest => (
            StatusCode::BAD_REQUEST,
            serde_json::json!({"error":"invalid_request"}),
            None,
        ),
        ExchangeError::InvalidClient => (
            StatusCode::UNAUTHORIZED,
            serde_json::json!({"error":"invalid_client"}),
            Some("Basic realm=\"OAuth\"".to_owned()),
        ),
        ExchangeError::InvalidGrant => (
            StatusCode::BAD_REQUEST,
            serde_json::json!({"error":"invalid_grant"}),
            None,
        ),
        ExchangeError::Unavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            serde_json::json!({"error":"temporarily_unavailable"}),
            None,
        ),
        ExchangeError::Protocol { body, authenticate } => (
            if authenticate.is_some() {
                StatusCode::UNAUTHORIZED
            } else {
                StatusCode::BAD_REQUEST
            },
            body,
            authenticate,
        ),
    };
    let mut response = (status, axum::Json(body)).into_response();
    if let Some(value) = authenticate.and_then(|value| header::HeaderValue::from_str(&value).ok()) {
        response
            .headers_mut()
            .insert(header::WWW_AUTHENTICATE, value);
    }
    no_cache(response)
}

fn no_cache(mut response: Response) -> Response {
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    response
        .headers_mut()
        .insert(header::PRAGMA, header::HeaderValue::from_static("no-cache"));
    response
}
