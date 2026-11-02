//! OpenTelemetry telemetry: JSON log layer, OTLP trace export, the generic
//! privacy key rule, and bounded shutdown. Disabled unless configured.

pub mod config;
pub mod export;
pub mod ids;
pub mod json;
pub mod keys;

use std::sync::OnceLock;
use std::time::Duration;

use loco_rs::app::Hooks;
use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::trace::{BatchSpanProcessor, Sampler, SdkTracerProvider};
use tracing_subscriber::{layer::SubscriberExt, registry, util::SubscriberInitExt};

use config::TelemetrySettings;
use opentelemetry_otlp::{WithExportConfig, WithHttpConfig};

static TRACER_PROVIDER: OnceLock<SdkTracerProvider> = OnceLock::new();

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
    match build_provider(&settings, &resource, diagnostics) {
        Some(provider) => {
            let _ = TRACER_PROVIDER.set(provider.clone());
            let otel = tracing_opentelemetry::layer().with_tracer(provider.tracer("medtracker"));
            let _ = registry().with(json).with(otel).with(filter).try_init();
            let host = settings.endpoint_host().unwrap_or_default();
            tracing::info!(
                event.name = "medtracker.telemetry.export",
                server.address = %host,
                "Telemetry export enabled"
            );
        }
        None => {
            let _ = registry().with(json).with(filter).try_init();
            tracing::info!(
                event.name = "medtracker.telemetry.export",
                "Telemetry export disabled"
            );
        }
    }
    true
}

fn build_provider(
    settings: &TelemetrySettings,
    resource: &json::Resource,
    diagnostics: std::sync::Arc<export::Diagnostics>,
) -> Option<SdkTracerProvider> {
    // The reqwest blocking exporter client must be constructed off the async
    // context: building it inside a Tokio runtime can panic.
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        if matches!(
            handle.runtime_flavor(),
            tokio::runtime::RuntimeFlavor::MultiThread
        ) {
            return tokio::task::block_in_place(|| {
                build_provider_inner(settings, resource, diagnostics)
            });
        }
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

/// Flushes and shuts down the tracer provider within a 2-second bound. Called
/// from `Hooks::on_shutdown`; a failed flush surfaces only through the
/// rate-limited diagnostics warning.
pub async fn shutdown() {
    let Some(provider) = TRACER_PROVIDER.get() else {
        return;
    };
    let provider = provider.clone();
    let _ = tokio::time::timeout(Duration::from_secs(2), async move {
        let _ = tokio::task::spawn_blocking(move || {
            let _ = provider.force_flush();
            provider.shutdown()
        })
        .await;
    })
    .await;
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
    struct AlwaysFails;
    impl SpanExporter for AlwaysFails {
        fn export(
            &self,
            _: Vec<SpanData>,
        ) -> impl std::future::Future<Output = OTelSdkResult> + Send {
            async { Err(OTelSdkError::InternalFailure("endpoint down".into())) }
        }
    }

    fn resource() -> json::Resource {
        json::Resource {
            service_version: "test".into(),
            environment: "test".into(),
            instance_id: String::new(),
        }
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
        let exporter = SanitizingExporter::new(AlwaysFails, diagnostics.clone());
        let processor = BatchSpanProcessor::builder(exporter).build();
        let provider = SdkTracerProvider::builder()
            .with_span_processor(processor)
            .with_sampler(Sampler::AlwaysOn)
            .build();

        let buffer = Buffer::default();
        let json = json::layer(buffer.clone(), resource(), diagnostics.clone());
        let filter = EnvFilter::new("info,opentelemetry=warn");
        let otel = tracing_opentelemetry::layer().with_tracer(provider.tracer("test"));
        let subscriber = registry().with(json).with(otel).with(filter);
        let _ = tracing::subscriber::set_global_default(subscriber);

        for _ in 0..3 {
            emit_span(&provider);
            let _ = provider.force_flush();
        }
        // Shutdown while failing: still no extra lines.
        let _ = provider.shutdown_with_timeout(Duration::from_millis(250));

        let output = buffer.text();
        let warnings = output
            .lines()
            .filter(|l| l.contains("medtracker.telemetry.export_failed"))
            .count();
        assert_eq!(warnings, 1, "{output}");
        assert!(
            !output.lines().any(|l| l.contains("\"severity_number\":1")
                && l.contains('7')
                && l.contains("ERROR")),
            "ERROR line present: {output}"
        );
        assert!(!output.contains("BatchSpanProcessor"), "{output}");
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
