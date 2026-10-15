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
    tracing::info!(method = %method, route, status = response.status().as_u16(),
        duration_ms = started.elapsed().as_secs_f64() * 1000.0,
        "HTTP request completed");
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, http::StatusCode, routing::get};
    use std::sync::{Arc, Mutex};
    use tracing::{
        Event, Metadata, Subscriber,
        field::{Field, Visit},
        instrument::WithSubscriber,
        span::{Attributes, Id, Record},
    };

    #[derive(Clone, Default)]
    struct RequestIdEvents(Arc<Mutex<Vec<String>>>);

    #[derive(Default)]
    struct RequestIdField(Option<String>);

    impl Visit for RequestIdField {
        fn record_str(&mut self, field: &Field, value: &str) {
            if field.name() == "request_id" {
                self.0 = Some(value.to_owned());
            }
        }

        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            if field.name() == "request_id" {
                self.0 = Some(format!("{value:?}").trim_matches('"').to_owned());
            }
        }
    }

    impl Subscriber for RequestIdEvents {
        fn enabled(&self, metadata: &Metadata<'_>) -> bool {
            metadata.fields().field("request_id").is_some()
        }

        fn new_span(&self, _: &Attributes<'_>) -> Id {
            Id::from_u64(1)
        }

        fn record(&self, _: &Id, _: &Record<'_>) {}

        fn record_follows_from(&self, _: &Id, _: &Id) {}

        fn event(&self, event: &Event<'_>) {
            let mut field = RequestIdField::default();
            event.record(&mut field);
            if let Some(request_id) = field.0 {
                self.0.lock().unwrap().push(request_id);
            }
        }

        fn enter(&self, _: &Id) {}

        fn exit(&self, _: &Id) {}
    }

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

    #[tokio::test(flavor = "current_thread")]
    async fn completion_log_omits_client_controlled_framework_request_id() {
        let events = RequestIdEvents::default();
        let _subscriber = tracing::subscriber::set_default(events.clone());
        let peer = ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 12345)));
        let router = Router::new()
            .route("/api/auth/get-session", get(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn(boundary))
            .layer(axum::middleware::from_fn(
                loco_rs::controller::middleware::request_id::request_id_middleware,
            ))
            .layer(axum::Extension(peer));
        let request = loco_rs::TestServer::builder()
            .mock_transport()
            .build(router)
            .unwrap();
        let (generated, supplied) = async {
            let generated = request.get("/api/auth/get-session").await;
            assert_eq!(generated.status_code(), 200);
            let generated_id = generated.header("x-request-id").to_owned();
            let supplied = request
                .get("/api/auth/get-session")
                .add_header("x-request-id", "client / 42")
                .await;
            assert_eq!(supplied.status_code(), 200);
            (generated_id, supplied.header("x-request-id").to_owned())
        }
        .with_subscriber(events.clone())
        .await;
        drop(request);
        assert!(!generated.is_empty());
        assert_eq!(supplied, "client42");
        assert!(events.0.lock().unwrap().is_empty());
    }
}
