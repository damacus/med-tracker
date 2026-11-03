//! Unit-of-work wrappers: background jobs, scheduled tasks, outbound client
//! spans and the panic hook.

use std::future::Future;

/// Fixed `error.type` category for a unit-of-work failure. Never error text.
pub trait ErrorCategory {
    fn error_type(&self) -> &'static str;
}

impl ErrorCategory for loco_rs::Error {
    fn error_type(&self) -> &'static str {
        "loco_error"
    }
}

impl ErrorCategory for crate::models::errors::OperationError {
    fn error_type(&self) -> &'static str {
        use crate::models::errors::OperationError as Operation;
        match self {
            Operation::Unauthenticated => "unauthenticated",
            Operation::Forbidden => "forbidden",
            Operation::NotFound => "not_found",
            Operation::Validation { .. } => "validation",
            Operation::Conflict { .. } => "conflict",
            Operation::Unavailable => "unavailable",
        }
    }
}

/// Runs a background job inside a root `job {name}` consumer span with DB
/// accounting; records `medtracker.outcome` and logs one ERROR on failure.
pub async fn job<F, T, E>(name: &'static str, future: F) -> Result<T, E>
where
    F: Future<Output = Result<T, E>>,
    E: ErrorCategory,
{
    job_with_parent(name, None, future).await
}

/// A background job whose span joins the caller's trace when a valid
/// inbound `traceparent` is supplied, in either export mode.
pub async fn job_with_parent<F, T, E>(
    name: &'static str,
    traceparent: Option<&str>,
    future: F,
) -> Result<T, E>
where
    F: Future<Output = Result<T, E>>,
    E: ErrorCategory,
{
    let span = tracing::info_span!(
        "job",
        otel.name = format!("job {name}"),
        otel.kind = "consumer",
        messaging.system = "loco",
        messaging.operation.type = "process",
        messaging.destination.name = name,
        medtracker.outcome = tracing::field::Empty,
        medtracker.db.query_count = tracing::field::Empty,
        medtracker.db.runtime_ms = tracing::field::Empty,
        medtracker.db.failed_query_count = tracing::field::Empty,
        error.type = tracing::field::Empty,
        trace_id = tracing::field::Empty,
        span_id = tracing::field::Empty,
    );
    let trace_id = super::ids::attach(&span, traceparent);
    let outbound = super::ids::outbound_traceparent(&span, &trace_id);
    use tracing::Instrument;
    let result = super::ids::hold(
        outbound,
        super::db::scope(&span, future.instrument(span.clone())),
    )
    .await;
    match &result {
        Ok(_) => {
            span.record("medtracker.outcome", "success");
        }
        Err(error) => {
            let category = error.error_type();
            span.record("medtracker.outcome", "failure");
            span.record("error.type", category);
            tracing::error!(
                parent: &span,
                messaging.destination.name = name,
                error.type = category,
                "Background job failed"
            );
        }
    }
    result
}

/// Runs a scheduled Loco task inside a root `task {name}` consumer span.
/// Scheduled jobs run in a child process where `on_shutdown` is never called,
/// so telemetry is flushed before returning, including on failure.
pub async fn task<F, T, E>(name: &'static str, future: F) -> Result<T, E>
where
    F: Future<Output = Result<T, E>>,
    E: ErrorCategory,
{
    let span = tracing::info_span!(
        "task",
        otel.name = format!("task {name}"),
        otel.kind = "consumer",
        messaging.system = "loco",
        messaging.operation.type = "process",
        medtracker.task.name = name,
        messaging.destination.name = name,
        medtracker.outcome = tracing::field::Empty,
        medtracker.db.query_count = tracing::field::Empty,
        medtracker.db.runtime_ms = tracing::field::Empty,
        medtracker.db.failed_query_count = tracing::field::Empty,
        error.type = tracing::field::Empty,
        trace_id = tracing::field::Empty,
        span_id = tracing::field::Empty,
    );
    let trace_id = super::ids::attach(&span, None);
    let outbound = super::ids::outbound_traceparent(&span, &trace_id);
    use tracing::Instrument;
    let result = super::ids::hold(
        outbound,
        super::db::scope(&span, future.instrument(span.clone())),
    )
    .await;
    match &result {
        Ok(_) => {
            span.record("medtracker.outcome", "success");
        }
        Err(error) => {
            let category = error.error_type();
            span.record("medtracker.outcome", "failure");
            span.record("error.type", category);
            tracing::error!(
                parent: &span,
                messaging.destination.name = name,
                error.type = category,
                "Scheduled task failed"
            );
        }
    }
    super::flush().await;
    result
}

/// An outbound HTTP call span: `{METHOD}` name, `client` kind, peer host only.
/// Never records paths, query strings, headers or bodies; no `traceparent`
/// injection — these receivers must not see W3C context.
pub fn client_span(host: &str, method: &str) -> tracing::Span {
    tracing::info_span!(
        "client",
        otel.name = method.to_string(),
        otel.kind = "client",
        server.address = host,
        server.port = tracing::field::Empty,
        http.request.method = method,
        http.response.status_code = tracing::field::Empty,
        error.type = tracing::field::Empty,
    )
}

/// An outbound storage operation span (`storage {operation}`).
pub fn storage_span(operation: &str) -> tracing::Span {
    tracing::info_span!(
        "storage",
        otel.name = format!("storage {operation}"),
        otel.kind = "client",
        error.type = tracing::field::Empty,
    )
}

/// Logs a fixed `Panic` error inside the current span. Never includes the
/// payload; re-entry is guarded.
fn safe_panic_event(info: &std::panic::PanicHookInfo<'_>) {
    static IN_HOOK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if IN_HOOK.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    let location = info.location();
    tracing::error!(
        error.type = "panic",
        code.file.path = location.map(|location| location.file()),
        code.line.number = location.map(|location| location.line()),
        "Panic"
    );
    IN_HOOK.store(false, std::sync::atomic::Ordering::SeqCst);
}

type PanicHook = Box<dyn Fn(&std::panic::PanicHookInfo<'_>) + Sync + Send + 'static>;

/// Builds the panic hook. When `chain_default` is true the previously
/// registered hook runs after the safe event (used outside production so
/// developers still see payload text); production never writes the payload
/// because stderr also reaches Loki.
fn panic_hook(previous: PanicHook, chain_default: bool) -> PanicHook {
    Box::new(move |info| {
        safe_panic_event(info);
        if chain_default {
            previous(info);
        }
    })
}

/// Replaces the default panic hook (which would write the payload to stderr)
/// with the safe event hook. Installed at most once per process.
pub fn install_panic_hook(chain_default: bool) {
    static INSTALLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if INSTALLED.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    let previous = std::panic::take_hook();
    std::panic::set_hook(panic_hook(previous, chain_default));
}

/// Records a response status on the client span; `error.type` is the status
/// code string on 4xx/5xx and a fixed `transport` category otherwise.
fn client_result(span: &tracing::Span, result: &reqwest::Result<reqwest::Response>) {
    match result {
        Ok(response) => {
            let status = response.status().as_u16();
            span.record("http.response.status_code", i64::from(status));
            if status >= 400 {
                span.record("error.type", status.to_string().as_str());
            }
        }
        Err(_) => {
            span.record("error.type", "transport");
        }
    }
}

/// Sends an outbound request inside a `client` span over the peer host.
pub async fn send(
    request: reqwest::RequestBuilder,
    host: &str,
    method: &str,
) -> reqwest::Result<reqwest::Response> {
    let span = client_span(host, method);
    use tracing::Instrument;
    let result = request.send().instrument(span.clone()).await;
    client_result(&span, &result);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::prelude::*;

    #[derive(Clone, Default)]
    pub(super) struct Capture(pub(super) Arc<Mutex<Vec<String>>>);

    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Capture {
        fn on_event(
            &self,
            event: &tracing::Event<'_>,
            _ctx: tracing_subscriber::layer::Context<'_, S>,
        ) {
            #[derive(Default)]
            struct Visitor(String);
            impl tracing::field::Visit for Visitor {
                fn record_debug(
                    &mut self,
                    field: &tracing::field::Field,
                    value: &dyn std::fmt::Debug,
                ) {
                    use std::fmt::Write;
                    write!(self.0, " {}={value:?}", field.name()).expect("visit");
                }
            }
            let mut visitor = Visitor::default();
            event.record(&mut visitor);
            self.0.lock().expect("capture lock").push(format!(
                "{}:{}",
                event.metadata().level(),
                visitor.0
            ));
        }
    }

    pub(super) fn test_otel() -> (
        opentelemetry_sdk::trace::InMemorySpanExporter,
        opentelemetry_sdk::trace::SdkTracerProvider,
        impl tracing_subscriber::Layer<tracing_subscriber::Registry>,
    ) {
        let exporter = opentelemetry_sdk::trace::InMemorySpanExporter::default();
        let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
            .with_simple_exporter(exporter.clone())
            .with_sampler(opentelemetry_sdk::trace::Sampler::AlwaysOn)
            .build();
        let layer = crate::telemetry::otel_layer(&provider, "test");
        (exporter, provider, layer)
    }

    #[tokio::test]
    async fn job_success_span_records_consumer_outcome_without_dropped_keys() {
        let (exporter, provider, layer) = test_otel();
        let capture = Capture::default();
        let subscriber = tracing_subscriber::registry()
            .with(layer)
            .with(capture.clone());
        use tracing::instrument::WithSubscriber;
        job("import", async { Ok::<u8, loco_rs::Error>(7) })
            .with_subscriber(subscriber)
            .await
            .unwrap();
        provider.force_flush().expect("flush");
        let spans = exporter.get_finished_spans().expect("exported spans");
        let span = &spans[0];
        assert_eq!(span.name, "job import");
        let attrs: Vec<_> = span
            .attributes
            .iter()
            .map(|kv| (kv.key.as_str(), format!("{:?}", kv.value)))
            .collect();
        assert!(attrs.iter().any(|(k, _)| *k == "medtracker.outcome"));
        assert!(
            attrs
                .iter()
                .any(|(k, v)| k.contains("outcome") && v.contains("success"))
        );
        assert!(!attrs.iter().any(|(k, _)| k.contains("dropped_keys")));
        assert!(
            attrs
                .iter()
                .any(|(k, v)| *k == "messaging.destination.name" && v.contains("import"))
        );
    }

    #[tokio::test]
    async fn job_failure_span_and_log_use_fixed_error_category() {
        let (exporter, provider, layer) = test_otel();
        let capture = Capture::default();
        let subscriber = tracing_subscriber::registry()
            .with(layer)
            .with(capture.clone());
        use tracing::instrument::WithSubscriber;
        let result = job("import", async {
            Err::<u8, _>(loco_rs::Error::string("ClinicalSentinelPayload"))
        })
        .with_subscriber(subscriber)
        .await;
        assert!(result.is_err());
        provider.force_flush().expect("flush");
        let spans = exporter.get_finished_spans().expect("exported spans");
        let attrs: Vec<_> = spans[0]
            .attributes
            .iter()
            .map(|kv| (kv.key.as_str(), format!("{:?}", kv.value)))
            .collect();
        assert!(
            attrs
                .iter()
                .any(|(k, v)| k.contains("outcome") && v.contains("failure"))
        );
        assert!(
            attrs
                .iter()
                .any(|(k, v)| *k == "error.type" && v.contains("loco_error"))
        );
        let events = capture.0.lock().expect("capture lock");
        let error = events
            .iter()
            .find(|event| event.starts_with("ERROR:"))
            .expect("failure event");
        assert!(error.contains("loco_error"));
        assert!(!error.contains("ClinicalSentinelPayload"));
    }

    #[tokio::test]
    async fn task_wrapper_with_no_provider_does_not_panic() {
        let result = task("canary-seed", async { Ok::<u8, loco_rs::Error>(1) }).await;
        assert_eq!(result.unwrap(), 1);
    }

    #[tokio::test]
    async fn client_span_records_status_and_error_type_without_path() {
        let router = axum::Router::new().fallback(axum::routing::any(|| async {
            axum::http::StatusCode::SERVICE_UNAVAILABLE
        }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let (exporter, provider, layer) = test_otel();
        let subscriber = tracing_subscriber::registry().with(layer);
        let sentinel = format!("http://{address}/ClinicalPathSentinel?token=ClinicalQuerySentinel");
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        use tracing::instrument::WithSubscriber;
        let outcome = send(reqwest::Client::new().get(sentinel), "127.0.0.1", "GET")
            .with_subscriber(subscriber)
            .await;
        assert!(outcome.is_ok());
        provider.force_flush().expect("flush");
        let spans = exporter.get_finished_spans().expect("exported spans");
        let attrs: Vec<_> = spans[0]
            .attributes
            .iter()
            .map(|kv| (kv.key.as_str(), format!("{:?}", kv.value)))
            .collect();
        assert!(
            attrs
                .iter()
                .any(|(k, v)| *k == "server.address" && v.contains("127.0.0.1"))
        );
        assert!(
            attrs
                .iter()
                .any(|(k, v)| *k == "http.response.status_code" && v.contains("503")),
            "attrs={attrs:?}"
        );
        assert!(
            attrs
                .iter()
                .any(|(k, v)| *k == "error.type" && v.contains("503"))
        );
        assert!(!attrs.iter().any(|(_, v)| v.contains("Sentinel")));
        assert!(!attrs.iter().any(|(k, _)| k.contains("dropped_keys")));
    }

    #[tokio::test]
    async fn job_with_parent_joins_inbound_trace_when_export_enabled() {
        let (exporter, provider, layer) = test_otel();
        let subscriber = tracing_subscriber::registry().with(layer);
        use tracing::instrument::WithSubscriber;
        let inbound = "00-7fa01885bf8086b9a009c00c9ca89e1b-6dd454d737734ffa-01";
        job_with_parent("import", Some(inbound), async {
            Ok::<u8, loco_rs::Error>(1)
        })
        .with_subscriber(subscriber)
        .await
        .unwrap();
        provider.force_flush().expect("flush");
        let spans = exporter.get_finished_spans().expect("exported spans");
        assert_eq!(
            spans[0].span_context.trace_id().to_string(),
            "7fa01885bf8086b9a009c00c9ca89e1b"
        );
    }

    #[tokio::test]
    async fn job_with_parent_joins_inbound_trace_when_export_disabled() {
        let inbound = "00-7fa01885bf8086b9a009c00c9ca89e1b-6dd454d737734ffa-01";
        let propagated = job_with_parent("import", Some(inbound), async {
            Ok::<String, loco_rs::Error>(
                super::super::ids::current_traceparent().expect("traceparent"),
            )
        })
        .await
        .unwrap();
        let trace_id = propagated.split('-').nth(1).expect("trace id");
        assert_eq!(trace_id, "7fa01885bf8086b9a009c00c9ca89e1b");
    }
}

#[cfg(test)]
mod panic_tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::prelude::*;

    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<Vec<String>>>);

    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Capture {
        fn on_event(
            &self,
            event: &tracing::Event<'_>,
            _ctx: tracing_subscriber::layer::Context<'_, S>,
        ) {
            struct Visitor(String);
            impl tracing::field::Visit for Visitor {
                fn record_debug(
                    &mut self,
                    field: &tracing::field::Field,
                    value: &dyn std::fmt::Debug,
                ) {
                    use std::fmt::Write;
                    write!(self.0, " {}={value:?}", field.name()).expect("visit");
                }
            }
            let mut visitor = Visitor(String::new());
            event.record(&mut visitor);
            let in_span = event.is_contextual();
            self.0.lock().expect("capture lock").push(format!(
                "{}:{}:ctx={in_span}",
                event.metadata().level(),
                visitor.0
            ));
        }
    }

    static HOOK_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn panic_hook_logs_one_error_inside_the_span_without_payload() {
        let _lock = HOOK_LOCK.lock().expect("hook lock");
        install_panic_hook(false);
        let capture = Capture::default();
        let subscriber = tracing_subscriber::registry().with(capture.clone());
        let handle = std::thread::spawn(move || {
            let _guard = tracing::subscriber::set_default(subscriber);
            let span = tracing::info_span!("job");
            let _entered = span.enter();
            std::panic::panic_any("ClinicalPanicSentinel".to_string());
        });
        let _ = handle.join();
        let events = capture.0.lock().expect("capture lock");
        let errors: Vec<_> = events
            .iter()
            .filter(|event| event.starts_with("ERROR:"))
            .collect();
        assert_eq!(errors.len(), 1);
        let error = errors[0];
        assert!(error.contains("error.type=\"panic\""));
        assert!(error.contains("code.file.path"));
        assert!(error.contains("ctx=true"));
        assert!(!error.contains("Sentinel"));
    }

    #[test]
    fn panic_hook_chains_the_previous_hook_only_when_enabled() {
        let _lock = HOOK_LOCK.lock().expect("hook lock");
        use std::sync::atomic::{AtomicBool, Ordering};
        for chain_default in [true, false] {
            let ran = Arc::new(AtomicBool::new(false));
            let marker = ran.clone();
            let previous: super::PanicHook = Box::new(move |_| {
                marker.store(true, Ordering::SeqCst);
            });
            let hook = super::panic_hook(previous, chain_default);
            let original = std::panic::take_hook();
            std::panic::set_hook(hook);
            let handle = std::thread::spawn(|| {
                let _ = std::panic::catch_unwind(|| {
                    std::panic::panic_any("chain test");
                });
            });
            let _ = handle.join();
            let _ = std::panic::take_hook();
            std::panic::set_hook(original);
            assert_eq!(ran.load(Ordering::SeqCst), chain_default, "{chain_default}");
        }
    }
}

#[cfg(test)]
mod task_flush_tests {
    use super::{task, tests::test_otel};
    use tracing_subscriber::prelude::*;

    #[tokio::test]
    async fn task_failure_logs_scheduled_task_failed_message() {
        let capture = super::tests::Capture::default();
        let subscriber = tracing_subscriber::registry().with(capture.clone());
        use tracing::instrument::WithSubscriber;
        let result: Result<u8, loco_rs::Error> = task("stale-sweep", async {
            Err(loco_rs::Error::string("ClinicalSentinelPayload"))
        })
        .with_subscriber(subscriber)
        .await;
        assert!(result.is_err());
        let events = capture.0.lock().unwrap().clone();
        let error = events
            .iter()
            .find(|event| event.starts_with("ERROR:"))
            .expect("failure event");
        assert!(error.contains("Scheduled task failed"), "{error}");
        assert!(!error.contains("ClinicalSentinelPayload"));
    }

    #[tokio::test]
    async fn task_wrapper_flushes_without_shutting_down_the_provider() {
        let (exporter, provider, layer) = test_otel();
        let _ = super::super::TRACER_PROVIDER.set(provider.clone());
        let subscriber = tracing_subscriber::registry().with(layer);
        use tracing::instrument::WithSubscriber;
        async {
            task("flush-check", async { Ok::<u8, loco_rs::Error>(1) })
                .await
                .unwrap();
            tracing::info_span!("after-task").in_scope(|| ());
        }
        .with_subscriber(subscriber)
        .await;
        let flushed = provider.clone();
        let _ = tokio::task::spawn_blocking(move || flushed.force_flush()).await;
        let spans = exporter.get_finished_spans().expect("exported spans");
        let names: Vec<_> = spans.iter().map(|span| span.name.to_string()).collect();
        assert!(
            names.iter().any(|name| name == "task flush-check"),
            "{names:?}"
        );
        assert!(names.iter().any(|name| name == "after-task"), "{names:?}");
    }
}
