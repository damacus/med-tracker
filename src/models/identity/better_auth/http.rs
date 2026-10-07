use axum::{
    extract::{ConnectInfo, MatchedPath, Request},
    http::HeaderValue,
    middleware::Next,
    response::Response,
};
use std::{net::SocketAddr, time::Instant};

pub async fn boundary(mut request: Request, next: Next) -> Response {
    let permission = request
        .extensions()
        .get::<axum::extract::MatchedPath>()
        .and_then(|route| super::personal_keys::permission(request.method(), route.as_str()));
    request.headers_mut().remove("x-medtracker-key-permission");
    if let Some(permission) = permission {
        request.headers_mut().insert(
            "x-medtracker-key-permission",
            HeaderValue::from_static(permission),
        );
    }
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|peer| peer.0.ip().to_string());
    for name in ["x-forwarded-for", "x-real-ip", "x-medtracker-peer"] {
        request.headers_mut().remove(name);
    }
    let source = peer.as_deref().unwrap_or("unknown");
    if let Ok(value) = HeaderValue::from_str(source) {
        request
            .headers_mut()
            .insert("x-forwarded-for", value.clone());
        request.headers_mut().insert("x-real-ip", value);
    }
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map_or("unmatched", MatchedPath::as_str)
        .to_owned();
    let method = request.method().clone();
    let started = Instant::now();
    let response = next.run(request).await;
    let request_id = response
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unavailable");
    tracing::info!(method = %method, route, status = response.status().as_u16(),
        duration_ms = started.elapsed().as_secs_f64() * 1000.0, request_id,
        "HTTP request completed");
    response
}
