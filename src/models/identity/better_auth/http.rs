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
    let missing_auth_peer = peer.is_none() && request.uri().path().starts_with("/api/auth/");
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
    let response = if missing_auth_peer {
        use axum::response::IntoResponse;
        axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response()
    } else {
        next.run(request).await
    };
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, http::StatusCode, routing::get};

    fn application() -> Router {
        Router::new()
            .route(
                "/api/auth/get-session",
                get(|headers: axum::http::HeaderMap| async move {
                    headers
                        .get("x-forwarded-for")
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_owned()
                }),
            )
            .layer(axum::middleware::from_fn(boundary))
    }

    async fn request(router: Router) -> reqwest::Response {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let response = reqwest::Client::new()
            .get(format!("http://{address}/api/auth/get-session"))
            .header("x-forwarded-for", "198.51.100.77")
            .send()
            .await
            .unwrap();
        server.abort();
        response
    }

    #[tokio::test]
    async fn authentication_requires_connection_metadata() {
        let response = request(application()).await;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn authentication_sources_remain_distinct_and_ignore_spoofed_headers() {
        for address in ["192.0.2.1:1234", "192.0.2.2:1234"] {
            let peer: SocketAddr = address.parse().unwrap();
            let response = request(application().layer(axum::Extension(ConnectInfo(peer)))).await;
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.text().await.unwrap(), peer.ip().to_string());
        }
    }
}
