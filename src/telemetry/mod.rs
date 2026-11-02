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
    let json = json::layer(std::io::stdout, resource.clone());
    match build_provider(&settings, &resource) {
        Some(provider) => {
            let _ = TRACER_PROVIDER.set(provider.clone());
            let otel = tracing_opentelemetry::layer().with_tracer(provider.tracer("medtracker"));
            registry().with(json).with(otel).with(filter).init();
            let host = settings.endpoint_host().unwrap_or_default();
            tracing::info!(
                event.name = "medtracker.telemetry.export",
                server.address = %host,
                "Telemetry export enabled"
            );
        }
        None => {
            registry().with(json).with(filter).init();
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
) -> Option<SdkTracerProvider> {
    let mut attributes = vec![
        opentelemetry::KeyValue::new("service.version", resource.service_version.clone()),
        opentelemetry::KeyValue::new(
            "deployment.environment.name",
            resource.environment.clone(),
        ),
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
    let exporter = export::SanitizingExporter::new(exporter, export::Diagnostics::new());
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
    use super::{TelemetrySettings, build_provider, json::Resource};

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
        assert!(build_provider(&settings, &resource()).is_none());
        let settings = TelemetrySettings::from_settings(Some(&serde_json::json!({
            "telemetry": {"otlp_endpoint": "", "otlp_traces_endpoint": ""}
        })));
        assert!(build_provider(&settings, &resource()).is_none());
    }
}
