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
    signal: &'static str,
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
            signal: "traces",
        })
    }

    pub fn metrics() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(DiagnosticsState::default()),
            window: RATE_WINDOW,
            signal: "metrics",
        })
    }

    #[cfg(test)]
    pub fn with_window(window: Duration) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(DiagnosticsState::default()),
            window,
            signal: "traces",
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
                medtracker.telemetry.signal = self.signal,
                "Telemetry export recovered"
            );
        } else if emit_failure {
            let dropped = self.dropped_spans();
            tracing::warn!(
                event.name = "medtracker.telemetry.export_failed",
                medtracker.telemetry.signal = self.signal,
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
    let method = span
        .attributes
        .iter()
        .find(|attribute| attribute.key.as_str() == "http.request.method");
    let route = span
        .attributes
        .iter()
        .find(|attribute| attribute.key.as_str() == "http.route");
    let matched_route_name = match (method, route) {
        (Some(method), Some(route)) => match (&method.value, &route.value) {
            (opentelemetry::Value::String(method), opentelemetry::Value::String(route)) => {
                span.name == format!("{} {}", method.as_str(), route.as_str())
                    && route.as_str().starts_with('/')
                    && route.as_str().len() <= 128
                    && route.as_str().chars().all(|c| {
                        c.is_ascii_alphanumeric()
                            || matches!(c, '/' | '_' | '-' | '.' | '{' | '}' | '*')
                    })
            }
            _ => false,
        },
        _ => false,
    };
    if !matched_route_name && keys::safe_span_name(&span.name) != span.name {
        span.name = Cow::Owned("medtracker.redacted".to_owned());
    }
    let mut dropped = Vec::new();
    sanitize_attributes(&mut span.attributes, &mut dropped);
    for event in span.events.events.iter_mut() {
        if !matches!(
            event.name.as_ref(),
            "medtracker.dose.attempted"
                | "medtracker.dose.committed"
                | "medtracker.dose.rolled_back"
                | "medtracker.dose.blocked"
                | "medtracker.dose.replayed"
                | "medtracker.reminder.scheduled"
                | "medtracker.reminder.suppressed"
                | "medtracker.reminder.sent"
                | "medtracker.reminder.provider_failed"
                | "medtracker.mail.enqueued"
                | "medtracker.mail.delivered"
                | "medtracker.mail.retried"
                | "medtracker.mail.failed"
                | "medtracker.auth.sign_in_failed"
                | "medtracker.auth.throttled"
                | "medtracker.support.session_started"
                | "medtracker.support.session_ended"
                | "medtracker.support.session_expired"
        ) {
            event.name = Cow::Borrowed("medtracker.redacted");
        }
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
        let resource = Resource::builder_empty()
            .with_attributes(attributes)
            .build();
        self.inner.set_resource(&resource);
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

    pub(super) fn span(attributes: Vec<KeyValue>, status: Status) -> SpanData {
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

    #[test]
    fn resource_filter_does_not_redetect_environment_attributes() {
        const CHILD: &str = "MEDTRACKER_RESOURCE_FILTER_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "telemetry::export::tests::resource_filter_does_not_redetect_environment_attributes", "--nocapture"])
                .env(CHILD, "1")
                .env("OTEL_RESOURCE_ATTRIBUTES", "raw_password=SecretSentinel,service.name=allowed-service")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }

        #[derive(Debug, Clone, Default)]
        struct ResourceCapture(Arc<Mutex<Vec<KeyValue>>>, Arc<Mutex<usize>>);
        impl SpanExporter for ResourceCapture {
            async fn export(&self, batch: Vec<SpanData>) -> OTelSdkResult {
                *self.1.lock().unwrap() += batch.len();
                Ok(())
            }
            fn set_resource(&mut self, resource: &opentelemetry_sdk::Resource) {
                *self.0.lock().unwrap() = resource
                    .iter()
                    .map(|(k, v)| KeyValue::new(k.clone(), v.clone()))
                    .collect();
            }
        }
        let capture = ResourceCapture::default();
        let exporter = SanitizingExporter::new(
            capture.clone(),
            Diagnostics::with_window(Duration::from_secs(1)),
        );
        let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
            .with_sampler(opentelemetry_sdk::trace::Sampler::AlwaysOn)
            .with_resource(opentelemetry_sdk::Resource::builder().build())
            .with_simple_exporter(exporter)
            .build();
        use opentelemetry::trace::{Tracer, TracerProvider};
        drop(provider.tracer("test").start("test-span"));
        provider.force_flush().unwrap();
        assert_eq!(*capture.1.lock().unwrap(), 1);
        let attrs = capture.0.lock().unwrap();
        assert!(attrs.iter().any(
            |kv| kv.key.as_str() == "service.name" && kv.value.to_string() == "allowed-service"
        ));
        assert!(
            !attrs.iter().any(|kv| kv.key.as_str() == "raw_password"),
            "{attrs:?}"
        );
    }
}

#[cfg(test)]
mod event_name_tests {
    use super::{sanitize_span, tests::span};
    use opentelemetry::trace::Event;
    use opentelemetry_sdk::trace::SpanEvents;
    use std::borrow::Cow;

    #[test]
    fn named_domain_event_survives_export() {
        let mut data = span(vec![], opentelemetry::trace::Status::Unset);
        data.events.events.push(Event::new(
            Cow::Borrowed("medtracker.dose.committed"),
            std::time::SystemTime::now(),
            vec![],
            0,
        ));
        let data = sanitize_span(data);
        assert_eq!(data.events.events[0].name, "medtracker.dose.committed");
    }

    #[test]
    fn named_tracing_event_survives_the_real_export_pipeline() {
        use opentelemetry_sdk::trace::{InMemorySpanExporter, Sampler, SdkTracerProvider};
        use tracing_subscriber::layer::SubscriberExt;

        let exporter = InMemorySpanExporter::default();
        let provider = SdkTracerProvider::builder()
            .with_simple_exporter(super::SanitizingExporter::new(
                exporter.clone(),
                super::Diagnostics::new(),
            ))
            .with_sampler(Sampler::AlwaysOn)
            .build();
        let subscriber =
            tracing_subscriber::registry().with(crate::telemetry::otel_layer(&provider, "test"));
        tracing::subscriber::with_default(subscriber, || {
            let span = tracing::info_span!("request");
            let guard = span.enter();
            tracing::event!(name: "medtracker.dose.committed", tracing::Level::INFO,
                event.name = "medtracker.dose.committed", medtracker.outcome = "success");
            drop(guard);
            drop(span);
        });
        provider.force_flush().expect("flush");
        let spans = exporter.get_finished_spans().expect("exported spans");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].events.events.len(), 1);
        let event = &spans[0].events.events[0];
        assert_eq!(event.name, "medtracker.dose.committed");
        assert!(
            event
                .attributes
                .iter()
                .any(|attribute| attribute.key.as_str() == "event.name"
                    && attribute.value.to_string() == "medtracker.dose.committed")
        );
    }

    #[test]
    fn dynamic_span_event_names_are_replaced() {
        let mut data = span(vec![], opentelemetry::trace::Status::Unset);
        let mut events = SpanEvents::default();
        events.events.push(Event::new(
            Cow::Borrowed("/tmp/sentinel/archive.zip?token=abc"),
            std::time::SystemTime::now(),
            vec![],
            0,
        ));
        data.events = events;
        let data = sanitize_span(data);
        assert_eq!(data.events.events[0].name, "medtracker.redacted");
    }

    #[test]
    fn formatted_span_event_name_is_redacted() {
        let mut data = span(vec![], opentelemetry::trace::Status::Unset);
        data.events.events.push(Event::new(
            Cow::Borrowed("ClinicalValueSentinel"),
            std::time::SystemTime::now(),
            vec![],
            0,
        ));
        let data = sanitize_span(data);
        assert_eq!(data.events.events[0].name, "medtracker.redacted");
    }

    #[test]
    fn matched_parameterized_request_name_survives_export() {
        let route = "/households/{slug}/medications/{id}";
        let mut data = span(
            vec![
                opentelemetry::KeyValue::new("http.request.method", "GET"),
                opentelemetry::KeyValue::new("http.route", route),
            ],
            opentelemetry::trace::Status::Unset,
        );
        data.name = format!("GET {route}").into();
        let data = sanitize_span(data);
        assert_eq!(data.name, format!("GET {route}"));
    }
}
