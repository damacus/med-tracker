//! Explicitly configured metrics with bounded, non-sensitive attributes.

use opentelemetry::{
    KeyValue,
    metrics::MeterProvider,
    metrics::{Counter, Histogram},
};
use opentelemetry_otlp::{WithExportConfig, WithHttpConfig};
use opentelemetry_sdk::{
    error::OTelSdkResult,
    metrics::{
        PeriodicReader, SdkMeterProvider, Temporality, data::ResourceMetrics,
        exporter::PushMetricExporter,
    },
};
use std::{sync::OnceLock, time::Duration};

use super::{config::TelemetrySettings, export::Diagnostics, json::Resource};

#[derive(Debug)]
struct Exporter {
    inner: opentelemetry_otlp::MetricExporter,
    diagnostics: std::sync::Arc<Diagnostics>,
}

impl PushMetricExporter for Exporter {
    async fn export(&self, metrics: &ResourceMetrics) -> OTelSdkResult {
        let result = self.inner.export(metrics).await;
        self.diagnostics.record(result.is_ok(), "metrics_export");
        result
    }
    fn force_flush(&self) -> OTelSdkResult {
        self.inner.force_flush()
    }
    fn shutdown_with_timeout(&self, timeout: Duration) -> OTelSdkResult {
        self.inner.shutdown_with_timeout(timeout)
    }
    fn temporality(&self) -> Temporality {
        self.inner.temporality()
    }
}

pub(super) fn build_provider(
    settings: &TelemetrySettings,
    resource: &Resource,
) -> Option<SdkMeterProvider> {
    if settings.otlp_metrics_endpoint.is_empty() {
        return None;
    }
    if let Ok(handle) = tokio::runtime::Handle::try_current()
        && matches!(
            handle.runtime_flavor(),
            tokio::runtime::RuntimeFlavor::MultiThread
        )
    {
        return tokio::task::block_in_place(|| build_inner(settings, resource));
    }
    if tokio::runtime::Handle::try_current().is_ok() {
        let settings = settings.clone();
        let resource = resource.clone();
        return std::thread::spawn(move || build_inner(&settings, &resource))
            .join()
            .ok()
            .flatten();
    }
    build_inner(settings, resource)
}

fn build_inner(settings: &TelemetrySettings, resource: &Resource) -> Option<SdkMeterProvider> {
    let exporter = opentelemetry_otlp::MetricExporter::builder().with_http()
        .with_endpoint(settings.otlp_metrics_endpoint.clone())
        .with_timeout(Duration::from_millis(settings.otlp_timeout_ms))
        .with_headers(settings.headers())
        .build().inspect_err(|_| {
            tracing::warn!(event.name = "medtracker.telemetry.export_failed", error.type = "metrics_configuration", "Metrics exporter could not be configured");
        }).ok()?;
    let reader = PeriodicReader::builder(Exporter {
        inner: exporter,
        diagnostics: Diagnostics::metrics(),
    })
    .with_interval(Duration::from_secs(60))
    .build();
    let mut attributes = vec![
        KeyValue::new("service.name", "medtracker"),
        KeyValue::new(
            "service.version",
            super::keys::cap(&resource.service_version).to_owned(),
        ),
        KeyValue::new(
            "deployment.environment.name",
            super::keys::cap(&resource.environment).to_owned(),
        ),
    ];
    if !resource.instance_id.is_empty() {
        attributes.push(KeyValue::new(
            "service.instance.id",
            super::keys::cap(&resource.instance_id).to_owned(),
        ));
    }
    Some(
        SdkMeterProvider::builder()
            .with_reader(reader)
            .with_resource(
                opentelemetry_sdk::Resource::builder_empty()
                    .with_attributes(attributes)
                    .build(),
            )
            .build(),
    )
}

struct Instruments {
    server: Histogram<f64>,
    client: Histogram<f64>,
    job: Histogram<f64>,
    imports: Counter<u64>,
}

static INSTRUMENTS: OnceLock<Instruments> = OnceLock::new();

pub(crate) fn meter() -> Option<opentelemetry::metrics::Meter> {
    super::METER_PROVIDER
        .get()
        .map(|provider| provider.meter("medtracker"))
}

/// SeaORM 2.0.4 exposes the underlying PostgreSQL pool but no aggregate pool
/// statistics. Read only SQLx pool metadata; all queries still use SeaORM.
pub fn observe_pool(db: &sea_orm::DatabaseConnection) {
    let Some(meter) = meter() else {
        return;
    };
    if db.get_database_backend() != sea_orm::DbBackend::Postgres {
        return;
    }
    let pool = db.get_postgres_connection_pool().clone();
    let count_pool = pool.clone();
    meter
        .u64_observable_gauge("db.client.connection.count")
        .with_callback(move |observer| {
            let total = u64::from(count_pool.size());
            let idle = count_pool.num_idle() as u64;
            observer.observe(idle, &[KeyValue::new("state", "idle")]);
            observer.observe(
                total.saturating_sub(idle),
                &[KeyValue::new("state", "used")],
            );
        })
        .build();
    meter
        .u64_observable_gauge("db.client.connection.max")
        .with_callback(move |observer| {
            observer.observe(u64::from(pool.options().get_max_connections()), &[]);
        })
        .build();
}

pub(super) fn install(provider: &SdkMeterProvider) {
    let meter = provider.meter("medtracker");
    let _ = INSTRUMENTS.set(Instruments {
        server: meter
            .f64_histogram("http.server.request.duration")
            .with_unit("s")
            .build(),
        client: meter
            .f64_histogram("http.client.request.duration")
            .with_unit("s")
            .build(),
        job: meter
            .f64_histogram("medtracker.job.duration")
            .with_unit("s")
            .build(),
        imports: meter.u64_counter("medtracker.import.runs").build(),
    });
}

fn method(method: &str) -> &'static str {
    match method {
        "GET" => "GET",
        "POST" => "POST",
        "PUT" => "PUT",
        "PATCH" => "PATCH",
        "DELETE" => "DELETE",
        "HEAD" => "HEAD",
        "OPTIONS" => "OPTIONS",
        "CONNECT" => "CONNECT",
        "TRACE" => "TRACE",
        _ => "_OTHER",
    }
}

pub fn server_request(duration: Duration, request_method: &str, route: &str, status: u16) {
    if let Some(i) = INSTRUMENTS.get() {
        i.server.record(
            duration.as_secs_f64(),
            &[
                KeyValue::new("http.request.method", method(request_method)),
                KeyValue::new("http.route", super::keys::cap(route).to_owned()),
                KeyValue::new("http.response.status_code", i64::from(status)),
            ],
        );
    }
}

pub fn client_request(duration: Duration, request_method: &str, status: Option<u16>) {
    if let Some(i) = INSTRUMENTS.get() {
        let mut labels = vec![KeyValue::new("http.request.method", method(request_method))];
        if let Some(status) = status {
            labels.push(KeyValue::new(
                "http.response.status_code",
                i64::from(status),
            ));
        } else {
            labels.push(KeyValue::new("error.type", "transport"));
        }
        i.client.record(duration.as_secs_f64(), &labels);
    }
}

pub fn job(duration: Duration, name: &'static str, succeeded: bool) {
    if let Some(i) = INSTRUMENTS.get() {
        i.job.record(
            duration.as_secs_f64(),
            &[
                KeyValue::new(
                    "messaging.destination.name",
                    super::keys::cap(name).to_owned(),
                ),
                KeyValue::new(
                    "medtracker.outcome",
                    if succeeded { "success" } else { "failure" },
                ),
            ],
        );
    }
}

pub struct JobAttempt {
    name: &'static str,
    started: std::time::Instant,
    succeeded: bool,
}

impl JobAttempt {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            started: std::time::Instant::now(),
            succeeded: false,
        }
    }
    pub fn finish(&mut self, succeeded: bool) {
        self.succeeded = succeeded;
    }
}

impl Drop for JobAttempt {
    fn drop(&mut self) {
        job(self.started.elapsed(), self.name, self.succeeded);
    }
}

pub struct ImportAttempt {
    kind: &'static str,
    succeeded: bool,
}

impl ImportAttempt {
    pub fn new(kind: &'static str) -> Self {
        Self {
            kind,
            succeeded: false,
        }
    }
    pub fn succeeded(&mut self) {
        self.succeeded = true;
    }
}

impl Drop for ImportAttempt {
    fn drop(&mut self) {
        import(self.kind, self.succeeded);
    }
}

pub fn import(kind: &'static str, succeeded: bool) {
    if let Some(i) = INSTRUMENTS.get() {
        i.imports.add(
            1,
            &[
                KeyValue::new("medtracker.import.kind", kind),
                KeyValue::new(
                    "medtracker.outcome",
                    if succeeded { "success" } else { "failure" },
                ),
            ],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry_sdk::metrics::InMemoryMetricExporter;

    #[test]
    fn instruments_record_safe_labels_and_terminal_outcomes() {
        let exporter = InMemoryMetricExporter::default();
        let provider = SdkMeterProvider::builder()
            .with_reader(PeriodicReader::builder(exporter.clone()).build())
            .build();
        install(&provider);
        server_request(
            Duration::from_millis(250),
            "PRIVATE_METHOD",
            "/people/{id}",
            200,
        );
        client_request(Duration::from_millis(100), "GET", None);
        {
            let mut job = JobAttempt::new("synthetic-job");
            job.finish(true);
        }
        {
            let _failed = ImportAttempt::new("nhs_dmd");
        }
        {
            let mut success = ImportAttempt::new("nhs_dmd");
            success.succeeded();
        }
        provider.force_flush().unwrap();
        let data = format!("{:?}", exporter.get_finished_metrics().unwrap());
        for expected in [
            "http.server.request.duration",
            "http.client.request.duration",
            "medtracker.job.duration",
            "medtracker.import.runs",
            "_OTHER",
            "/people/{id}",
            "transport",
            "success",
            "failure",
        ] {
            assert!(data.contains(expected), "missing {expected}");
        }
        assert!(!data.contains("PRIVATE_METHOD"));
        provider.shutdown().unwrap();
    }
}
