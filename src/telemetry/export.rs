//! Span-exporting wrapper: applies the generic key rule to every exported
//! span surface and feeds self-diagnostics with a 5-minute rate limiter.

use std::borrow::Cow;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use opentelemetry::{KeyValue, trace::Status};
use opentelemetry_sdk::{Resource, error::OTelSdkResult, trace::SpanData, trace::SpanExporter};

use super::keys;

/// Rate window for repeated export-failure warnings.
const RATE_WINDOW: Duration = Duration::from_secs(300);

/// Shared self-diagnostics state. The span exporter wrapper records each
/// export outcome; the state machine emits the curated events.
#[derive(Debug)]
pub struct Diagnostics {
    state: Mutex<DiagnosticsState>,
    window: Duration,
}

#[derive(Debug, Default)]
struct DiagnosticsState {
    failing: bool,
    suppressed: u64,
    dropped_spans: u64,
    last_warning: Option<Instant>,
}

impl Diagnostics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(DiagnosticsState::default()),
            window: RATE_WINDOW,
        })
    }

    #[cfg(test)]
    pub fn with_window(window: Duration) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(DiagnosticsState::default()),
            window,
        })
    }

    /// Records one export outcome and emits the curated diagnostics events.
    /// `error_type` is a fixed category, never raw error text.
    pub fn record(&self, succeeded: bool, error_type: &'static str) {
        let mut emit_recovery = false;
        let mut emit_failure = false;
        let mut suppressed = 0_u64;
        {
            let Ok(mut state) = self.state.lock() else {
                return;
            };
            if succeeded {
                if state.failing {
                    state.failing = false;
                    state.suppressed = 0;
                    emit_recovery = true;
                }
            } else if !state.failing {
                state.failing = true;
                state.last_warning = Some(Instant::now());
                emit_failure = true;
            } else {
                state.suppressed += 1;
                let elapsed = state
                    .last_warning
                    .map(|instant| instant.elapsed())
                    .unwrap_or(self.window);
                if elapsed >= self.window {
                    suppressed = state.suppressed;
                    state.suppressed = 0;
                    state.last_warning = Some(Instant::now());
                    emit_failure = true;
                }
            }
        }
        if emit_recovery {
            tracing::info!(
                event.name = "medtracker.telemetry.export_recovered",
                "Telemetry export recovered"
            );
        } else if emit_failure {
            let dropped = self.dropped_spans();
            tracing::warn!(
                event.name = "medtracker.telemetry.export_failed",
                error.type = error_type,
                medtracker.telemetry.suppressed_count = suppressed,
                medtracker.telemetry.dropped_spans = dropped,
                "Telemetry export failed"
            );
        }
    }
}

/// Exporter wrapper applying the generic key rule to span attributes,
/// events, links, resource and status before delegating to `inner`.
#[derive(Debug)]
pub struct SanitizingExporter<E> {
    inner: E,
    diagnostics: Arc<Diagnostics>,
}

impl<E> SanitizingExporter<E> {
    pub fn new(inner: E, diagnostics: Arc<Diagnostics>) -> Self {
        Self { inner, diagnostics }
    }
}

fn sanitize_attributes(attributes: &mut Vec<KeyValue>, dropped: &mut Vec<String>) {
    let mut kept = Vec::with_capacity(attributes.len());
    for attribute in attributes.drain(..) {
        let key = attribute.key.as_str();
        if keys::allowed(key) {
            let value = match &attribute.value {
                opentelemetry::Value::String(text) => {
                    KeyValue::new(attribute.key.clone(), keys::cap(text.as_str()).to_owned())
                }
                _ => attribute,
            };
            kept.push(value);
        } else {
            dropped.push(key.to_owned());
        }
    }
    *attributes = kept;
}

fn sanitize_span(mut span: SpanData) -> SpanData {
    if keys::safe_span_name(&span.name) != span.name {
        span.name = Cow::Owned("medtracker.redacted".to_owned());
    }
    let mut dropped = Vec::new();
    sanitize_attributes(&mut span.attributes, &mut dropped);
    for event in span.events.events.iter_mut() {
        sanitize_attributes(&mut event.attributes, &mut dropped);
    }
    for link in span.links.links.iter_mut() {
        sanitize_attributes(&mut link.attributes, &mut dropped);
    }
    if !dropped.is_empty() {
        dropped.sort();
        dropped.dedup();
        span.attributes.push(KeyValue::new(
            "medtracker.telemetry.dropped_keys",
            dropped.join(","),
        ));
    }
    if let Status::Error { .. } = span.status {
        span.status = Status::Error {
            description: Cow::Borrowed(""),
        };
    }
    span
}

impl<E: SpanExporter> SpanExporter for SanitizingExporter<E> {
    fn export(
        &self,
        batch: Vec<SpanData>,
    ) -> impl std::future::Future<Output = OTelSdkResult> + Send {
        let batch = batch.into_iter().map(sanitize_span).collect();
        async move {
            let outcome = self.inner.export(batch).await;
            self.diagnostics
                .record(outcome.is_ok(), "export_request_failed");
            outcome
        }
    }

    fn shutdown_with_timeout(&self, timeout: Duration) -> OTelSdkResult {
        self.inner.shutdown_with_timeout(timeout)
    }

    fn force_flush(&self) -> OTelSdkResult {
        self.inner.force_flush()
    }

    fn set_resource(&mut self, resource: &Resource) {
        let mut attributes: Vec<KeyValue> = resource
            .iter()
            .map(|(k, v)| KeyValue::new(k.clone(), v.clone()))
            .collect();
        let mut dropped = Vec::new();
        sanitize_attributes(&mut attributes, &mut dropped);
        let resource = Resource::builder().with_attributes(attributes).build();
        self.inner.set_resource(&resource);
    }
}

#[cfg(test)]
mod tests {
    use super::{Diagnostics, SanitizingExporter, sanitize_span};
    use opentelemetry::{
        KeyValue, SpanId, TraceId,
        trace::{SpanContext, SpanKind, Status, TraceFlags, TraceState},
    };
    use opentelemetry_sdk::{
        error::{OTelSdkError, OTelSdkResult},
        trace::{SpanData, SpanEvents, SpanExporter, SpanLinks},
    };
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, SystemTime};
    use tracing::instrument::WithSubscriber;

    #[derive(Debug, Default)]
    struct FakeExporter {
        fail: Mutex<bool>,
        batches: Mutex<Vec<SpanData>>,
    }

    impl SpanExporter for FakeExporter {
        fn export(
            &self,
            batch: Vec<SpanData>,
        ) -> impl std::future::Future<Output = OTelSdkResult> + Send {
            let fail = *self.fail.lock().expect("fail flag");
            if !fail {
                self.batches.lock().expect("batches").extend(batch);
            }
            async move {
                if fail {
                    Err(OTelSdkError::InternalFailure("fake".into()))
                } else {
                    Ok(())
                }
            }
        }
    }

    fn span(attributes: Vec<KeyValue>, status: Status) -> SpanData {
        SpanData {
            span_context: SpanContext::new(
                TraceId::from_hex("4bf92f3577b34da6a3ce929d0e0e4736").expect("trace"),
                SpanId::from_hex("00f067aa0ba902b7").expect("span"),
                TraceFlags::SAMPLED,
                false,
                TraceState::default(),
            ),
            parent_span_id: SpanId::INVALID,
            parent_span_is_remote: false,
            span_kind: SpanKind::Server,
            name: "GET /health".into(),
            start_time: SystemTime::now(),
            end_time: SystemTime::now(),
            attributes,
            dropped_attributes_count: 0,
            events: SpanEvents::default(),
            links: SpanLinks::default(),
            status,
            instrumentation_scope: opentelemetry::InstrumentationScope::default(),
        }
    }

    #[test]
    fn sanitizer_drops_nonconforming_keys_and_lists_names() {
        let data = span(
            vec![
                KeyValue::new("http.route", "/health"),
                KeyValue::new("raw_password", "Sentinel"),
                KeyValue::new("medtracker.note", "x".repeat(300)),
            ],
            Status::Error {
                description: "raw upstream error".into(),
            },
        );
        let data = sanitize_span(data);
        let keys: Vec<&str> = data.attributes.iter().map(|a| a.key.as_str()).collect();
        assert!(keys.contains(&"http.route"));
        assert!(keys.contains(&"medtracker.note"));
        assert!(keys.contains(&"medtracker.telemetry.dropped_keys"));
        assert!(!keys.contains(&"raw_password"));
        let dropped = data
            .attributes
            .iter()
            .find(|a| a.key.as_str() == "medtracker.telemetry.dropped_keys")
            .and_then(|a| match &a.value {
                opentelemetry::Value::String(text) => Some(text.to_string()),
                _ => None,
            })
            .expect("dropped list");
        assert_eq!(dropped, "raw_password");
        let note = data
            .attributes
            .iter()
            .find(|a| a.key.as_str() == "medtracker.note")
            .and_then(|a| match &a.value {
                opentelemetry::Value::String(text) => Some(text.as_str().len()),
                _ => None,
            })
            .expect("note");
        assert_eq!(note, 256);
        match data.status {
            Status::Error { description } => assert_eq!(description.as_ref(), ""),
            _ => panic!("status changed"),
        }
    }

    #[derive(Clone, Default)]
    struct EventNames(Arc<Mutex<Vec<String>>>);

    impl tracing::Subscriber for EventNames {
        fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
            true
        }

        fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }

        fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}

        fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}

        fn event(&self, event: &tracing::Event<'_>) {
            struct Body(Option<String>);
            impl tracing::field::Visit for Body {
                fn record_debug(
                    &mut self,
                    field: &tracing::field::Field,
                    value: &dyn std::fmt::Debug,
                ) {
                    if field.name() == "message" {
                        self.0 = Some(format!("{value:?}"));
                    }
                }
            }
            let mut body = Body(None);
            event.record(&mut body);
            self.0.lock().expect("event names").push(format!(
                "{}:{}",
                event.metadata().level(),
                body.0.unwrap_or_default()
            ));
        }

        fn enter(&self, _: &tracing::span::Id) {}

        fn exit(&self, _: &tracing::span::Id) {}
    }

    #[tokio::test(flavor = "current_thread")]
    async fn failing_exporter_emits_one_warning_then_recovery() {
        let names = EventNames::default();
        let diagnostics = Diagnostics::with_window(Duration::from_secs(3600));
        let inner = Arc::new(FakeExporter::default());
        let exporter = SanitizingExporter::new(FakeHandle(inner.clone()), diagnostics);
        async {
            *inner.fail.lock().expect("fail") = true;
            for _ in 0..3 {
                assert!(
                    exporter
                        .export(vec![span(vec![], Status::Unset)])
                        .await
                        .is_err()
                );
            }
            *inner.fail.lock().expect("fail") = false;
            assert!(
                exporter
                    .export(vec![span(vec![], Status::Unset)])
                    .await
                    .is_ok()
            );
        }
        .with_subscriber(names.clone())
        .await;
        let recorded = names.0.lock().expect("events").clone();
        let failures = recorded
            .iter()
            .filter(|event| event.contains("WARN") && event.contains("Telemetry export failed"))
            .count();
        assert_eq!(failures, 1, "{recorded:?}");
        let recoveries = recorded
            .iter()
            .filter(|event| event.contains("INFO") && event.contains("Telemetry export recovered"))
            .count();
        assert_eq!(recoveries, 1, "{recorded:?}");
    }

    #[derive(Debug)]
    struct FakeHandle(Arc<FakeExporter>);

    impl SpanExporter for FakeHandle {
        fn export(
            &self,
            batch: Vec<SpanData>,
        ) -> impl std::future::Future<Output = OTelSdkResult> + Send {
            self.0.export(batch)
        }
    }
}

impl Diagnostics {
    /// Counts a suppressed OTel SDK internal diagnostic event. Called by the
    /// log-layer filter for every denied `opentelemetry*` event; `name` is the
    /// event metadata name (e.g. `BatchSpanProcessor.SpansDropped`).
    pub fn sdk_event(&self, name: &str) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if !state.failing {
            // Internal errors before any observed export failure still open a
            // failure window only via the exporter wrapper; count them here.
        }
        if name.contains("SpansDropped") || name.contains("SpanDroppingStarted") {
            state.dropped_spans += 1;
        }
    }

    /// Suppressed diagnostic count, included on the rate-limited warning.
    pub fn suppressed(&self) -> u64 {
        self.state.lock().map(|s| s.suppressed).unwrap_or(0)
    }

    /// Dropped-span notification count since the last reset.
    pub fn dropped_spans(&self) -> u64 {
        self.state.lock().map(|s| s.dropped_spans).unwrap_or(0)
    }

    /// Adds to the suppressed counter (used by the gate filter).
    pub fn suppress_one(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.suppressed += 1;
        }
    }
}

/// Per-layer filter that denies every OTel SDK internal diagnostic event
/// (`opentelemetry*` targets) while feeding the shared diagnostics limiter.
/// The curated `medtracker.telemetry.*` events come from the exporter wrapper,
/// so SDK lines never reach the log output.
pub struct DiagnosticsGate {
    diagnostics: Arc<Diagnostics>,
}

impl DiagnosticsGate {
    pub fn new(diagnostics: Arc<Diagnostics>) -> Self {
        Self { diagnostics }
    }
}

impl<S> tracing_subscriber::layer::Filter<S> for DiagnosticsGate {
    fn enabled(
        &self,
        meta: &tracing::Metadata<'_>,
        _ctx: &tracing_subscriber::layer::Context<'_, S>,
    ) -> bool {
        if !meta.target().starts_with("opentelemetry") {
            return true;
        }
        self.diagnostics.sdk_event(meta.name());
        self.diagnostics.suppress_one();
        false
    }

    fn callsite_enabled(
        &self,
        meta: &'static tracing::Metadata<'static>,
    ) -> tracing::subscriber::Interest {
        if meta.target().starts_with("opentelemetry") {
            // `sometimes` keeps `enabled` invoked so we can count the event.
            tracing::subscriber::Interest::sometimes()
        } else {
            tracing::subscriber::Interest::always()
        }
    }
}
