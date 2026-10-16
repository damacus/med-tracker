use crate::models::rate_limit::{RateLimitState, Rejection};
use crate::views::rate_limit::RateLimitResponse;
use axum::{
    extract::{ConnectInfo, Request, State},
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
};
use std::{net::SocketAddr, sync::Arc};

pub async fn boundary(
    State(state): State<Arc<RateLimitState>>,
    request: Request,
    next: Next,
) -> Response {
    if !request.uri().path().starts_with("/api/v1/") {
        return next.run(request).await;
    }
    let Some(peer) = request.extensions().get::<ConnectInfo<SocketAddr>>() else {
        return json_error(
            StatusCode::SERVICE_UNAVAILABLE,
            RateLimitResponse::source_unavailable(),
        );
    };
    let Some(rejection) = state.check(
        peer.0.ip(),
        request.headers(),
        request.method(),
        request.uri().path(),
    ) else {
        return next.run(request).await;
    };
    rejected(rejection)
}

fn rejected(rejection: Rejection) -> Response {
    let mut response = json_error(StatusCode::TOO_MANY_REQUESTS, RateLimitResponse::default());
    let headers = response.headers_mut();
    for (name, value) in [
        ("retry-after", rejection.retry_after),
        ("ratelimit-limit", u64::from(rejection.limit)),
        ("ratelimit-remaining", 0),
        ("ratelimit-reset", rejection.reset_at),
    ] {
        headers.insert(
            name,
            HeaderValue::from_str(&value.to_string()).expect("numeric rate header"),
        );
    }
    response
}

fn json_error(status: StatusCode, body: RateLimitResponse) -> Response {
    let mut response = loco_rs::controller::format::render()
        .status(status)
        .json(body)
        .expect("static API error response");
    response
        .headers_mut()
        .insert("cache-control", HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    response
}
