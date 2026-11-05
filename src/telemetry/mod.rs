//! OpenTelemetry telemetry: JSON log layer, OTLP trace export, the generic
//! privacy key rule, and bounded shutdown. Disabled unless configured.

pub mod config;
pub mod db;
pub mod export;
pub mod ids;
pub mod json;
pub mod keys;
pub mod metrics;
pub mod queue_metrics;
pub mod units;

#[cfg(test)]
mod lifecycle_tests;

use std::sync::OnceLock;
use std::time::Duration;

use loco_rs::app::Hooks;
use opentelemetry_sdk::trace::{BatchSpanProcessor, Sampler, SdkTracerProvider};
use tracing_subscriber::{layer::SubscriberExt, registry, util::SubscriberInitExt};

use config::TelemetrySettings;
use opentelemetry_otlp::{WithExportConfig, WithHttpConfig};

static TRACER_PROVIDER: OnceLock<SdkTracerProvider> = OnceLock::new();
static METER_PROVIDER: OnceLock<opentelemetry_sdk::metrics::SdkMeterProvider> = OnceLock::new();

/// Implements `Hooks::init_logger`: builds the registry with Loco's env
/// filter, the MedTracker JSON layer, and — only when a traces endpoint is
/// configured — the OTLP span pipeline. Returns `Ok(true)` so Loco does not
/// install its own subscriber. Safe to call exactly once per process; tests
/// that never call it get no telemetry and no output contract change.
pub fn init<H: Hooks>(ctx: &loco_rs::app::AppContext) -> bool {
    let settings = TelemetrySettings::from_settings(ctx.config.settings.as_ref());
    let resource = json::Resource {
        service_version: H::app_version(),
        environment: ctx.environment.to_string(),
        instance_id: settings.service_instance_id.clone(),
    };
    let mut filter = loco_rs::logger::init_env_filter::<H>(
        ctx.config.logger.override_filter.as_ref(),
        &ctx.config.logger.level,
    );
    filter = filter.add_directive(
        "opentelemetry=warn"
            .parse()
            .expect("static filter directive is valid"),
    );
    let diagnostics = export::Diagnostics::new();
    let json = json::layer(std::io::stdout, resource.clone(), diagnostics.clone());
    if let Some(provider) = metrics::build_provider(&settings, &resource) {
        metrics::install(&provider);
        let _ = METER_PROVIDER.set(provider);
    }
    match build_provider(&settings, &resource, diagnostics) {
        Some(provider) => {
            let _ = TRACER_PROVIDER.set(provider.clone());
            let otel = otel_layer(&provider, "medtracker");
            match registry().with(json).with(otel).with(filter).try_init() {
                Ok(()) => {
                    let host = settings.endpoint_host().unwrap_or_default();
                    log_signal_state(true, METER_PROVIDER.get().is_some(), Some(&host));
                }
                Err(_) => {
                    eprintln!(
                        "MedTracker telemetry subscriber could not be installed; continuing without it"
                    );
                }
            }
        }
        None => match registry().with(json).with(filter).try_init() {
            Ok(()) => {
                let metrics_host = url::Url::parse(&settings.otlp_metrics_endpoint)
                    .ok()
                    .and_then(|url| url.host_str().map(str::to_owned));
                log_signal_state(
                    false,
                    METER_PROVIDER.get().is_some(),
                    metrics_host.as_deref(),
                );
            }
            Err(_) => {
                eprintln!(
                    "MedTracker telemetry subscriber could not be installed; continuing without it"
                );
            }
        },
    }
    units::install_panic_hook(!matches!(
        ctx.environment,
        loco_rs::environment::Environment::Production
    ));
    true
}

#[cfg(test)]
fn log_export_state(host: Option<&str>) {
    log_signal_state(host.is_some(), false, host);
}

fn log_signal_state(traces: bool, metrics: bool, host: Option<&str>) {
    if let Some(host) = host {
        tracing::info!(event.name = "medtracker.telemetry.export", medtracker.telemetry.export.enabled = traces || metrics, medtracker.telemetry.traces.enabled = traces, medtracker.telemetry.metrics.enabled = metrics, server.address = %host, "Telemetry export configured");
    } else {
        tracing::info!(
            event.name = "medtracker.telemetry.export",
            medtracker.telemetry.export.enabled = traces || metrics,
            medtracker.telemetry.traces.enabled = traces,
            medtracker.telemetry.metrics.enabled = metrics,
            "Telemetry export configured"
        );
    }
}

fn build_provider(
    settings: &TelemetrySettings,
    resource: &json::Resource,
    diagnostics: std::sync::Arc<export::Diagnostics>,
) -> Option<SdkTracerProvider> {
    // The reqwest blocking exporter client must be constructed off the async
    // context: building it inside a Tokio runtime can panic.
    if let Ok(handle) = tokio::runtime::Handle::try_current()
        && matches!(
            handle.runtime_flavor(),
            tokio::runtime::RuntimeFlavor::MultiThread
        )
    {
        return tokio::task::block_in_place(|| {
            build_provider_inner(settings, resource, diagnostics)
        });
    }
    build_provider_inner(settings, resource, diagnostics)
}

fn build_provider_inner(
    settings: &TelemetrySettings,
    resource: &json::Resource,
    diagnostics: std::sync::Arc<export::Diagnostics>,
) -> Option<SdkTracerProvider> {
    let mut attributes = vec![
        opentelemetry::KeyValue::new("service.version", resource.service_version.clone()),
        opentelemetry::KeyValue::new("deployment.environment.name", resource.environment.clone()),
    ];
    if !resource.instance_id.is_empty() {
        attributes.push(opentelemetry::KeyValue::new(
            "service.instance.id",
            resource.instance_id.clone(),
        ));
    }
    let endpoint = settings.traces_endpoint()?;
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_endpoint(endpoint)
        .with_timeout(Duration::from_millis(settings.otlp_timeout_ms))
        .with_headers(settings.headers())
        .build()
        .inspect_err(|_| {
            tracing::warn!(
                event.name = "medtracker.telemetry.export_failed",
                error.type = "exporter_configuration",
                "Telemetry exporter could not be configured"
            );
        })
        .ok()?;
    let exporter = export::SanitizingExporter::new(exporter, diagnostics);
    let batch = opentelemetry_sdk::trace::BatchConfigBuilder::default()
        .with_max_queue_size(2048)
        .with_scheduled_delay(Duration::from_secs(5))
        .with_max_export_batch_size(512)
        .build();
    let processor = BatchSpanProcessor::builder(exporter)
        .with_batch_config(batch)
        .build();
    let provider = SdkTracerProvider::builder()
        .with_span_processor(processor)
        .with_sampler(Sampler::TraceIdRatioBased(settings.trace_sample_ratio))
        .with_resource(
            opentelemetry_sdk::Resource::builder()
                .with_service_name("medtracker")
                .with_attributes(attributes)
                .build(),
        )
        .build();
    Some(provider)
}

#[derive(Default)]
struct LifecycleState {
    running: bool,
    pending_flush: bool,
    shutdown_requested: bool,
    shutdown_started: bool,
    shutdown_complete: bool,
}

struct Lifecycle {
    state: std::sync::Mutex<LifecycleState>,
    completed: tokio::sync::watch::Sender<u64>,
}

impl Lifecycle {
    fn new() -> Self {
        let (completed, _) = tokio::sync::watch::channel(0);
        Self {
            state: std::sync::Mutex::new(LifecycleState::default()),
            completed,
        }
    }

    fn finish(&self, state: &mut LifecycleState) {
        state.running = false;
        self.completed
            .send_modify(|generation| *generation = generation.saturating_add(1));
    }

    fn run<P: LifecycleProvider>(&'static self, provider: P) {
        loop {
            let _ =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| provider.force_flush()));
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if state.pending_flush {
                state.pending_flush = false;
                continue;
            }
            if state.shutdown_requested {
                state.shutdown_started = true;
                drop(state);
                let _ =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| provider.shutdown()));
                let mut state = self
                    .state
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                state.shutdown_complete = true;
                self.finish(&mut state);
                return;
            }
            self.finish(&mut state);
            return;
        }
    }

    async fn request<P: LifecycleProvider>(&'static self, provider: P, shutdown: bool) {
        let (mut completed, start) = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if state.shutdown_complete {
                return;
            }
            let completed = self.completed.subscribe();
            let start = !state.running;
            if start {
                state.running = true;
            } else if !state.shutdown_requested && !state.shutdown_started {
                state.pending_flush = true;
            }
            if shutdown {
                state.shutdown_requested = true;
            }
            (completed, start)
        };
        if start
            && std::thread::Builder::new()
                .name("telemetry-lifecycle".into())
                .spawn(move || self.run(provider))
                .is_err()
        {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            self.finish(&mut state);
            return;
        }
        let _ = tokio::time::timeout(Duration::from_secs(2), completed.changed()).await;
    }
}

static LIFECYCLE: OnceLock<Lifecycle> = OnceLock::new();
static METRICS_LIFECYCLE: OnceLock<Lifecycle> = OnceLock::new();

trait LifecycleProvider: Clone + Send + 'static {
    fn force_flush(&self) -> opentelemetry_sdk::error::OTelSdkResult;
    fn shutdown(&self) -> opentelemetry_sdk::error::OTelSdkResult;
}

impl LifecycleProvider for SdkTracerProvider {
    fn force_flush(&self) -> opentelemetry_sdk::error::OTelSdkResult {
        SdkTracerProvider::force_flush(self)
    }
    fn shutdown(&self) -> opentelemetry_sdk::error::OTelSdkResult {
        SdkTracerProvider::shutdown(self)
    }
}

impl LifecycleProvider for opentelemetry_sdk::metrics::SdkMeterProvider {
    fn force_flush(&self) -> opentelemetry_sdk::error::OTelSdkResult {
        opentelemetry_sdk::metrics::SdkMeterProvider::force_flush(self)
    }
    fn shutdown(&self) -> opentelemetry_sdk::error::OTelSdkResult {
        opentelemetry_sdk::metrics::SdkMeterProvider::shutdown(self)
    }
}

async fn lifecycle(shutdown: bool) {
    let traces = async {
        if let Some(provider) = TRACER_PROVIDER.get() {
            LIFECYCLE
                .get_or_init(Lifecycle::new)
                .request(provider.clone(), shutdown)
                .await;
        }
    };
    let metrics = async {
        if let Some(provider) = METER_PROVIDER.get() {
            METRICS_LIFECYCLE
                .get_or_init(Lifecycle::new)
                .request(provider.clone(), shutdown)
                .await;
        }
    };
    tokio::join!(traces, metrics);
}

/// Flushes the tracer provider without shutting it down, within a 2-second
/// bound. Used by scheduled tasks running in a child process where
/// `on_shutdown` is never called.
pub async fn flush() {
    lifecycle(false).await;
}

/// Flushes and shuts down the tracer provider within a 2-second bound. Called
/// from `Hooks::on_shutdown`; a failed flush surfaces only through the
/// rate-limited diagnostics warning.
pub async fn shutdown() {
    lifecycle(true).await;
}

/// Build identity: `version+commit`, preferring the `MEDTRACKER_VERSION` /
/// `MEDTRACKER_COMMIT` image build args over git metadata (see `build.rs`).
pub fn build_version() -> String {
    env!("MEDTRACKER_BUILD_VERSION").to_owned()
}

#[cfg(test)]
mod tests {
    use super::{TelemetrySettings, build_provider, export, json::Resource};

    fn resource() -> Resource {
        Resource {
            service_version: "test".to_owned(),
            environment: "test".to_owned(),
            instance_id: String::new(),
        }
    }

    #[test]
    fn unconfigured_settings_create_no_provider() {
        let settings = TelemetrySettings::from_settings(None);
        assert!(build_provider(&settings, &resource(), export::Diagnostics::new()).is_none());
        let settings = TelemetrySettings::from_settings(Some(&serde_json::json!({
            "telemetry": {"otlp_endpoint": "", "otlp_traces_endpoint": ""}
        })));
        assert!(build_provider(&settings, &resource(), export::Diagnostics::new()).is_none());
    }
}

#[cfg(test)]
mod pipeline_tests {
    use super::export::{Diagnostics, SanitizingExporter};
    use super::json;
    use opentelemetry::trace::{Tracer, TracerProvider};
    use opentelemetry_sdk::{
        error::{OTelSdkError, OTelSdkResult},
        trace::{BatchSpanProcessor, Sampler, SdkTracerProvider, SpanData, SpanExporter},
    };
    use std::io::Write;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tracing_subscriber::{EnvFilter, layer::SubscriberExt, registry};

    #[derive(Clone, Default)]
    struct Buffer(Arc<Mutex<Vec<u8>>>);
    impl Buffer {
        fn text(&self) -> String {
            String::from_utf8_lossy(&self.0.lock().expect("buffer")).to_string()
        }
    }
    struct BufferWriter(Arc<Mutex<Vec<u8>>>);
    impl Write for BufferWriter {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().expect("buffer").extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Buffer {
        type Writer = BufferWriter;
        fn make_writer(&'a self) -> Self::Writer {
            BufferWriter(self.0.clone())
        }
    }

    #[derive(Debug)]
    struct Toggleable {
        fail: Arc<std::sync::atomic::AtomicBool>,
    }
    impl SpanExporter for Toggleable {
        async fn export(&self, _: Vec<SpanData>) -> OTelSdkResult {
            if self.fail.load(std::sync::atomic::Ordering::Relaxed) {
                Err(OTelSdkError::InternalFailure("endpoint down".into()))
            } else {
                Ok(())
            }
        }
    }

    fn resource() -> json::Resource {
        json::Resource {
            service_version: "test".into(),
            environment: "test".into(),
            instance_id: String::new(),
        }
    }

    #[test]
    fn metrics_only_startup_and_independent_signal_recovery_are_explicit() {
        let buffer = Buffer::default();
        let subscriber =
            registry().with(json::layer(buffer.clone(), resource(), Diagnostics::new()));
        let traces = Diagnostics::new();
        let metrics = Diagnostics::metrics();
        tracing::subscriber::with_default(subscriber, || {
            super::log_signal_state(false, true, Some("collector.example"));
            traces.record(false, "trace_export");
            metrics.record(false, "metrics_export");
            traces.record(true, "trace_export");
            metrics.record(true, "metrics_export");
        });
        let records: Vec<serde_json::Value> = buffer
            .text()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(
            records[0]["attributes"]["medtracker.telemetry.export.enabled"],
            true
        );
        assert_eq!(
            records[0]["attributes"]["medtracker.telemetry.traces.enabled"],
            false
        );
        assert_eq!(
            records[0]["attributes"]["medtracker.telemetry.metrics.enabled"],
            true
        );
        let recovery: Vec<_> = records
            .iter()
            .filter(|record| {
                record["attributes"]["event.name"] == "medtracker.telemetry.export_recovered"
            })
            .collect();
        assert_eq!(recovery.len(), 2);
        assert_eq!(
            recovery[0]["attributes"]["medtracker.telemetry.signal"],
            "traces"
        );
        assert_eq!(
            recovery[1]["attributes"]["medtracker.telemetry.signal"],
            "metrics"
        );
    }

    #[test]
    fn startup_output_reports_export_state_without_message_prose() {
        let buffer = Buffer::default();
        let subscriber =
            registry().with(json::layer(buffer.clone(), resource(), Diagnostics::new()));
        tracing::subscriber::with_default(subscriber, || {
            super::log_export_state(Some("collector.example"));
            super::log_export_state(None);
        });
        let output = buffer.text();
        let records: Vec<serde_json::Value> = output
            .lines()
            .map(|line| serde_json::from_str(line).expect("JSON record"))
            .collect();
        assert_eq!(records.len(), 2);
        for (record, enabled) in records.iter().zip([true, false]) {
            assert_eq!(
                record["attributes"]["event.name"],
                "medtracker.telemetry.export"
            );
            assert_eq!(
                record["attributes"]["medtracker.telemetry.export.enabled"],
                enabled
            );
        }
        assert_eq!(
            records[0]["attributes"]["server.address"],
            "collector.example"
        );
        assert!(records[1]["attributes"].get("server.address").is_none());
        assert!(!output.contains("Telemetry export enabled"));
        assert!(!output.contains("Telemetry export disabled"));
    }

    fn emit_span(provider: &SdkTracerProvider) {
        use opentelemetry::trace::Span as _;
        let tracer = provider.tracer("test");
        let mut span = tracer.start("GET /health");
        span.end();
    }

    #[test]
    fn failing_pipeline_emits_one_warn_no_errors_and_recovers() {
        let diagnostics = Diagnostics::new();
        let fail = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let exporter =
            SanitizingExporter::new(Toggleable { fail: fail.clone() }, diagnostics.clone());
        let processor = BatchSpanProcessor::builder(exporter).build();
        let provider = SdkTracerProvider::builder()
            .with_span_processor(processor)
            .with_sampler(Sampler::AlwaysOn)
            .build();

        let buffer = Buffer::default();
        let json = json::layer(buffer.clone(), resource(), diagnostics.clone());
        let filter = EnvFilter::new("info,opentelemetry=warn");
        let otel = crate::telemetry::otel_layer(&provider, "test");
        let subscriber = registry().with(json).with(otel).with(filter);
        let _ = tracing::subscriber::set_global_default(subscriber);

        for _ in 0..3 {
            emit_span(&provider);
            let _ = provider.force_flush();
        }
        // Recovery: a successful export emits one INFO line.
        fail.store(false, std::sync::atomic::Ordering::Relaxed);
        emit_span(&provider);
        let _ = provider.force_flush();
        // Shutdown while failing earlier: still no extra lines.
        let _ = provider.shutdown_with_timeout(Duration::from_millis(250));

        let output = buffer.text();
        let warnings = output
            .lines()
            .filter(|l| l.contains("medtracker.telemetry.export_failed"))
            .count();
        assert_eq!(warnings, 1, "{output}");
        assert!(
            !output.contains("\"severity_text\":\"ERROR\""),
            "ERROR line present: {output}"
        );
        assert!(
            output
                .lines()
                .any(|l| l.contains("medtracker.telemetry.export_recovered")),
            "no recovery line: {output}"
        );
        assert!(!output.contains("BatchSpanProcessor"), "{output}");
        assert!(
            !output.contains("medtracker.telemetry.dropped_keys"),
            "ordinary telemetry must not produce dropped_keys: {output}"
        );
        assert!(!output.contains("endpoint down"), "{output}");
    }

    #[test]
    fn dropped_span_warnings_never_reach_output() {
        let diagnostics = Diagnostics::new();
        let buffer = Buffer::default();
        let json = json::layer(buffer.clone(), resource(), diagnostics.clone());
        let subscriber = registry().with(json);
        let _default = tracing::subscriber::set_default(subscriber);
        // Simulate SDK internal events at warn/error targets.
        tracing::warn!(name: "BatchSpanProcessor.SpansDropped", target: "opentelemetry_sdk", name = "BatchSpanProcessor.SpansDropped", "dropped");
        tracing::error!(name: "BatchSpanProcessor.ExportError", target: "opentelemetry_sdk", name = "BatchSpanProcessor.ExportError", "failed");
        assert_eq!(diagnostics.dropped_spans(), 1);
        assert_eq!(diagnostics.suppressed(), 2);
        assert!(buffer.text().is_empty(), "{}", buffer.text());
    }
}

/// The OpenTelemetry tracing layer used by `init` and tests: location, thread,
/// inactivity and level/target metadata attributes are disabled so ordinary
/// telemetry never produces `medtracker.telemetry.dropped_keys` noise.
pub(crate) fn otel_layer<S>(
    provider: &SdkTracerProvider,
    name: &'static str,
) -> tracing_opentelemetry::OpenTelemetryLayer<S, opentelemetry_sdk::trace::Tracer>
where
    S: tracing::Subscriber + for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
{
    use opentelemetry::trace::TracerProvider;
    tracing_opentelemetry::layer()
        .with_tracer(provider.tracer(name))
        .with_location(false)
        .with_threads(false)
        .with_tracked_inactivity(false)
        .with_level(false)
        .with_target(false)
}
