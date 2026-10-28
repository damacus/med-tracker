use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};
use tracing::Instrument;

pub async fn boundary(request: Request, next: Next) -> Response {
    let request_trace_id = uuid::Uuid::new_v4().to_string();
    let span = tracing::info_span!("http_request", request_trace_id = %request_trace_id);
    let mut response = next.run(request).instrument(span).await;
    if let Ok(header) = HeaderValue::from_bytes(request_trace_id.as_bytes()) {
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
            assert!(uuid::Uuid::parse_str(reference).is_ok());
            let matching = events
                .iter()
                .filter(|event| event.contains(reference))
                .collect::<Vec<_>>();
            assert_eq!(matching.len(), 1);
            let event = matching.first().ok_or("database event missing")?;
            assert!(event.contains("Database operation failed"));
            assert!(event.contains("category"));
            assert!(event.contains("error_trace_id"));
            references.push(reference.to_owned());
        }
        assert_ne!(references.first(), references.last());
        assert!(!events.join("\n").contains("Sentinel"));
        Ok(())
    }
}
