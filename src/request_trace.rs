use axum::{
    extract::{MatchedPath, Request},
    http::HeaderValue,
    middleware::Next,
    response::Response,
};
use tracing::Instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::telemetry::ids;

/// Request server span middleware: one `"{METHOD} {route}"` server span per
/// request carrying semconv attributes, joining a valid inbound W3C
/// `traceparent` while ignoring its sampled flag, and returning the 32-hex
/// trace ID in `x-medtracker-trace-id`.
pub async fn boundary(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map_or("unmatched", MatchedPath::as_str)
        .to_owned();
    let name = format!("{method} {route}");
    let traceparent = request
        .headers()
        .get("traceparent")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let span = tracing::info_span!(
        "request",
        otel.name = %name,
        otel.kind = "server",
        http.request.method = %method,
        http.route = %route,
        http.response.status_code = tracing::field::Empty,
        "error.type" = tracing::field::Empty,
        trace_id = tracing::field::Empty,
        span_id = tracing::field::Empty,
        medtracker.db.query_count = tracing::field::Empty,
        medtracker.db.runtime_ms = tracing::field::Empty,
        medtracker.db.failed_query_count = tracing::field::Empty,
    );
    let trace_id = ids::attach(&span, traceparent.as_deref());
    let outbound = ids::outbound_traceparent(&span, &trace_id);
    let mut response = ids::hold(
        outbound,
        crate::telemetry::db::scope(&span, async {
            next.run(request).instrument(span.clone()).await
        }),
    )
    .await;
    let status = i64::from(response.status().as_u16());
    span.record("http.response.status_code", status);
    span.set_attribute("http.response.status_code", status);
    if status >= 500 {
        span.record("error.type", status.to_string().as_str());
        span.set_attribute("error.type", status.to_string());
        span.set_status(opentelemetry::trace::Status::Error {
            description: "".into(),
        });
    }
    if let Ok(header) = HeaderValue::from_str(&trace_id) {
        response
            .headers_mut()
            .insert("x-medtracker-trace-id", header);
    }
    response
}

#[cfg(test)]
#[path = "../tests/support/request_trace_capture.rs"]
mod capture;

#[cfg(test)]
mod tests {
    use super::{boundary, capture::Captured};
    use axum::{Router, http::StatusCode, routing::get};
    use tracing::instrument::WithSubscriber;

    #[tokio::test(flavor = "current_thread")]
    async fn database_failures_inherit_only_their_generated_request_span()
    -> Result<(), Box<dyn std::error::Error>> {
        let captured = Captured::default();
        let router = Router::new()
            .route(
                "/private",
                get(|| async {
                    tokio::task::yield_now().await;
                    let error = crate::models::errors::OperationError::from(
                        sea_orm::DbErr::Custom("ClinicalValueSentinel PasswordSentinel".into()),
                    );
                    (StatusCode::SERVICE_UNAVAILABLE, error.to_string())
                }),
            )
            .layer(axum::middleware::from_fn(boundary));
        let server = loco_rs::TestServer::builder()
            .mock_transport()
            .build(router)?;
        let (first, second) =
            async { tokio::join!(server.get("/private"), server.get("/private")) }
                .with_subscriber(captured.clone())
                .await;
        let events = captured.events();
        assert_eq!(events.len(), 2);
        let mut references = Vec::new();
        for response in [&first, &second] {
            assert_eq!(response.status_code(), 503);
            assert_eq!(response.text(), "Operation temporarily unavailable");
            let header = response.header("x-medtracker-trace-id");
            let reference = header.to_str()?;
            assert_eq!(reference.len(), 32);
            assert!(reference.chars().all(|c| c.is_ascii_hexdigit()));
            let matching = events
                .iter()
                .filter(|event| event.contains(reference))
                .collect::<Vec<_>>();
            assert_eq!(matching.len(), 1);
            let event = matching.first().ok_or("database event missing")?;
            assert!(event.contains("Database operation failed"));
            assert!(event.contains("error.type"));
            assert!(event.contains("medtracker.error_reference"));
            references.push(reference.to_owned());
        }
        assert_ne!(references.first(), references.last());
        assert!(!events.join("\n").contains("Sentinel"));
        Ok(())
    }
}

#[cfg(test)]
mod inbound_tests {
    use super::boundary;
    use axum::{Router, http::StatusCode, routing::get};

    #[tokio::test(flavor = "current_thread")]
    async fn inbound_unsampled_traceparent_is_joined_and_recorded()
    -> Result<(), Box<dyn std::error::Error>> {
        let router = Router::new()
            .route("/joined", get(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn(boundary));
        let server = loco_rs::TestServer::builder()
            .mock_transport()
            .build(router)?;
        let response = server
            .get("/joined")
            .add_header(
                "traceparent",
                "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-00",
            )
            .await;
        assert_eq!(response.status_code(), 200);
        let header = response.header("x-medtracker-trace-id");
        assert_eq!(header.to_str()?, "4bf92f3577b34da6a3ce929d0e0e4736");
        Ok(())
    }

    #[tokio::test(flavor = "current_thread")]
    async fn malformed_traceparent_gets_a_fresh_trace() -> Result<(), Box<dyn std::error::Error>> {
        let router = Router::new()
            .route("/fresh", get(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn(boundary));
        let server = loco_rs::TestServer::builder()
            .mock_transport()
            .build(router)?;
        let response = server
            .get("/fresh")
            .add_header("traceparent", "garbage")
            .await;
        assert_eq!(response.status_code(), 200);
        let header = response.header("x-medtracker-trace-id");
        let value = header.to_str()?;
        assert_eq!(value.len(), 32);
        assert!(value.chars().all(|c| c.is_ascii_hexdigit()));
        Ok(())
    }

    #[tokio::test(flavor = "current_thread")]
    async fn invalid_version_traceparent_gets_a_fresh_trace()
    -> Result<(), Box<dyn std::error::Error>> {
        let router = Router::new()
            .route("/fresh", get(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn(boundary));
        let server = loco_rs::TestServer::builder()
            .mock_transport()
            .build(router)?;
        for traceparent in [
            "ff-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
            "zz-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01-extra",
        ] {
            let response = server
                .get("/fresh")
                .add_header("traceparent", traceparent)
                .await;
            assert_eq!(response.status_code(), 200);
            let value = response
                .header("x-medtracker-trace-id")
                .to_str()?
                .to_owned();
            assert_ne!(value, "4bf92f3577b34da6a3ce929d0e0e4736", "{traceparent}");
            assert_eq!(value.len(), 32);
            assert!(value.chars().all(|c| c.is_ascii_hexdigit()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod exported_span_tests {
    use super::boundary;
    use axum::{Router, http::StatusCode, routing::get};
    use opentelemetry_sdk::trace::{InMemorySpanExporter, Sampler, SdkTracerProvider};
    use tracing::instrument::WithSubscriber;
    use tracing_subscriber::{layer::SubscriberExt, registry};

    #[tokio::test(flavor = "current_thread")]
    async fn server_span_carries_semconv_attributes_without_private_input()
    -> Result<(), Box<dyn std::error::Error>> {
        let exporter = InMemorySpanExporter::default();
        let sanitized = crate::telemetry::export::SanitizingExporter::new(
            exporter.clone(),
            crate::telemetry::export::Diagnostics::new(),
        );
        let provider = SdkTracerProvider::builder()
            .with_simple_exporter(sanitized)
            .with_sampler(Sampler::AlwaysOn)
            .build();
        let otel = crate::telemetry::otel_layer(&provider, "test");
        let subscriber = registry().with(otel);
        let router = Router::new()
            .route(
                "/households/{slug}/medications/{id}",
                get(|| async { (StatusCode::INTERNAL_SERVER_ERROR, "ClinicalBodySentinel") }),
            )
            .layer(axum::middleware::from_fn(boundary));
        let server = loco_rs::TestServer::builder()
            .mock_transport()
            .build(router)?;
        let response = async {
            server
                .get("/households/ClinicalPathSentinel/medications/123?token=ClinicalQuerySentinel")
                .add_header("authorization", "ClinicalHeaderSentinel")
                .await
        }
        .with_subscriber(subscriber)
        .await;
        assert_eq!(response.status_code(), 500);
        let header = response.header("x-medtracker-trace-id");
        let trace_id = header.to_str()?;
        assert_eq!(trace_id.len(), 32);
        provider.force_flush().expect("flush");
        let spans = exporter.get_finished_spans()?;
        assert_eq!(spans.len(), 1);
        let span = &spans[0];
        assert_eq!(
            span.name.as_ref(),
            "GET /households/{slug}/medications/{id}"
        );
        assert_eq!(span.span_context.trace_id().to_string(), trace_id);
        let attribute = |key: &str| {
            span.attributes
                .iter()
                .find(|attribute| attribute.key.as_str() == key)
                .map(|attribute| match &attribute.value {
                    opentelemetry::Value::String(value) => value.to_string(),
                    _ => format!("{:?}", attribute.value),
                })
        };
        assert_eq!(attribute("http.request.method").as_deref(), Some("GET"));
        assert_eq!(
            attribute("http.route").as_deref(),
            Some("/households/{slug}/medications/{id}")
        );
        assert!(attribute("http.response.status_code").is_some());
        assert!(
            attribute("medtracker.telemetry.dropped_keys").is_none(),
            "ordinary request span must not produce dropped_keys"
        );
        assert_eq!(attribute("error.type").as_deref(), Some("500"));
        assert!(matches!(
            span.status,
            opentelemetry::trace::Status::Error { .. }
        ));
        assert!(!format!("{spans:?}").contains("Sentinel"));
        provider.shutdown().expect("shutdown");
        Ok(())
    }

    #[tokio::test(flavor = "current_thread")]
    async fn exported_catch_all_route_uses_template_without_private_path()
    -> Result<(), Box<dyn std::error::Error>> {
        let exporter = InMemorySpanExporter::default();
        let sanitized = crate::telemetry::export::SanitizingExporter::new(
            exporter.clone(),
            crate::telemetry::export::Diagnostics::new(),
        );
        let provider = SdkTracerProvider::builder()
            .with_simple_exporter(sanitized)
            .with_sampler(Sampler::AlwaysOn)
            .build();
        let subscriber = registry().with(crate::telemetry::otel_layer(&provider, "test"));
        let router = Router::new()
            .route("/static/{*path}", get(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn(boundary));
        let server = loco_rs::TestServer::builder()
            .mock_transport()
            .build(router)?;
        let response = async { server.get("/static/ClinicalPathSentinel/file").await }
            .with_subscriber(subscriber)
            .await;
        assert_eq!(response.status_code(), 200);
        provider.force_flush()?;
        let spans = exporter.get_finished_spans()?;
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].name, "GET /static/{*path}");
        assert!(!format!("{spans:?}").contains("ClinicalPathSentinel"));
        provider.shutdown()?;
        Ok(())
    }

    #[tokio::test(flavor = "current_thread")]
    async fn inbound_unsampled_traceparent_joins_the_exported_trace()
    -> Result<(), Box<dyn std::error::Error>> {
        let exporter = InMemorySpanExporter::default();
        let provider = SdkTracerProvider::builder()
            .with_simple_exporter(exporter.clone())
            .with_sampler(Sampler::AlwaysOn)
            .build();
        let otel = crate::telemetry::otel_layer(&provider, "test");
        let subscriber = registry().with(otel);
        let router = Router::new()
            .route("/joined", get(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn(boundary));
        let server = loco_rs::TestServer::builder()
            .mock_transport()
            .build(router)?;
        let response = async {
            server
                .get("/joined")
                .add_header(
                    "traceparent",
                    "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-00",
                )
                .await
        }
        .with_subscriber(subscriber)
        .await;
        let header = response.header("x-medtracker-trace-id");
        assert_eq!(header.to_str()?, "4bf92f3577b34da6a3ce929d0e0e4736");
        provider.force_flush().expect("flush");
        let spans = exporter.get_finished_spans()?;
        assert_eq!(spans.len(), 1);
        assert_eq!(
            spans[0].span_context.trace_id().to_string(),
            "4bf92f3577b34da6a3ce929d0e0e4736"
        );
        assert!(spans[0].span_context.is_sampled());
        provider.shutdown().expect("shutdown");
        Ok(())
    }
}
