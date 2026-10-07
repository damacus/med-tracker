use std::net::SocketAddr;
use axum::{extract::{ConnectInfo, Request}, http::HeaderValue, middleware::Next, response::Response};

pub async fn boundary(mut request: Request, next: Next) -> Response {
    let peer = request.extensions().get::<ConnectInfo<SocketAddr>>().map(|peer| peer.0.ip().to_string());
    for name in ["x-forwarded-for", "x-real-ip", "x-medtracker-peer"] { request.headers_mut().remove(name); }
    let source = peer.as_deref().unwrap_or("unknown");
    if let Ok(value) = HeaderValue::from_str(source) {
        request.headers_mut().insert("x-forwarded-for", value.clone());
        request.headers_mut().insert("x-real-ip", value);
    }
    next.run(request).await
}
