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
mod tests {
    use super::boundary;
    use axum::{Router, http::StatusCode, routing::get};
    use std::sync::{Arc, Mutex};
    use tracing::{
        Event, Metadata, Subscriber,
        field::{Field, Visit},
        instrument::WithSubscriber,
        span::{Attributes, Id, Record},
    };

    #[derive(Clone, Default)]
    struct Captured(Arc<Mutex<CaptureState>>);

    #[derive(Default)]
    struct CaptureState {
        next_id: u64,
        request_spans: Vec<(u64, String)>,
        entered: Vec<u64>,
        database_events: Vec<(String, Vec<u64>)>,
    }

    #[derive(Default)]
    struct Fields(Vec<(String, String)>);

    impl Visit for Fields {
        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            self.0.push((field.name().to_owned(), format!("{value:?}")));
        }

        fn record_str(&mut self, field: &Field, value: &str) {
            self.0.push((field.name().to_owned(), value.to_owned()));
        }
    }

    impl Subscriber for Captured {
        fn enabled(&self, _: &Metadata<'_>) -> bool {
            true
        }

        fn new_span(&self, attributes: &Attributes<'_>) -> Id {
            let Ok(mut state) = self.0.lock() else {
                panic!("request trace capture mutex poisoned")
            };
            state.next_id += 1;
            let id = state.next_id;
            if attributes.metadata().name() == "http_request" {
                let mut fields = Fields::default();
                attributes.record(&mut fields);
                let trace_id = fields
                    .0
                    .into_iter()
                    .find(|(name, _)| name == "request_trace_id")
                    .map(|(_, value)| value)
                    .unwrap_or_default();
                state.request_spans.push((id, trace_id));
            }
            Id::from_u64(id)
        }

        fn record(&self, _: &Id, _: &Record<'_>) {}

        fn record_follows_from(&self, _: &Id, _: &Id) {}

        fn event(&self, event: &Event<'_>) {
            let mut fields = Fields::default();
            event.record(&mut fields);
            if fields
                .0
                .iter()
                .any(|(name, value)| name == "message" && value == "Database operation failed")
            {
                let Ok(mut state) = self.0.lock() else {
                    panic!("request trace capture mutex poisoned")
                };
                let entered = state.entered.clone();
                state
                    .database_events
                    .push((format!("{:?}", fields.0), entered));
            }
        }

        fn enter(&self, id: &Id) {
            let Ok(mut state) = self.0.lock() else {
                panic!("request trace capture mutex poisoned")
            };
            state.entered.push(id.into_u64());
        }

        fn exit(&self, id: &Id) {
            let Ok(mut state) = self.0.lock() else {
                panic!("request trace capture mutex poisoned")
            };
            assert_eq!(state.entered.pop(), Some(id.into_u64()));
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn database_failure_event_inherits_generated_request_span()
    -> Result<(), Box<dyn std::error::Error>> {
        let subscriber = Captured::default();
        let _default = tracing::subscriber::set_default(subscriber.clone());
        let router = Router::new()
            .route(
                "/private",
                get(|| async {
                    let error = crate::models::errors::OperationError::from(
                        sea_orm::DbErr::Custom("ClinicalValueSentinel PasswordSentinel".to_owned()),
                    );
                    (StatusCode::SERVICE_UNAVAILABLE, error.to_string())
                }),
            )
            .layer(axum::middleware::from_fn(boundary));
        let server = loco_rs::TestServer::builder()
            .mock_transport()
            .build(router)?;
        let response = async { server.get("/private").await }
            .with_subscriber(subscriber.clone())
            .await;
        assert_eq!(response.status_code(), 503);
        assert_eq!(response.text(), "Operation temporarily unavailable");
        let header = response.header("x-medtracker-trace-id");
        let trace_id = header.to_str()?;
        assert!(uuid::Uuid::parse_str(trace_id).is_ok());
        let Ok(state) = subscriber.0.lock() else {
            panic!("request trace capture mutex poisoned")
        };
        assert_eq!(state.request_spans.len(), 1);
        let (span_id, span_trace_id) = &state.request_spans[0];
        assert_eq!(span_trace_id, trace_id);
        assert_eq!(state.database_events.len(), 1);
        let (event, entered) = &state.database_events[0];
        assert!(entered.contains(span_id));
        assert!(event.contains("category"));
        assert!(event.contains("error_trace_id"));
        assert!(!event.contains("ClinicalValueSentinel"));
        assert!(!event.contains("PasswordSentinel"));
        Ok(())
    }
}
