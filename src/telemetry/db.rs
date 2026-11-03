//! Unit-of-work database accounting. A task-local accumulator collects query
//! count, total elapsed and failed count per request, job or task; a SeaORM
//! metric callback feeds it. The callback never allocates or logs.

use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Default)]
pub struct DbAccumulator {
    queries: std::sync::Arc<AtomicU64>,
    elapsed_us: std::sync::Arc<AtomicU64>,
    failed: std::sync::Arc<AtomicU64>,
}

tokio::task_local! {
    static ACCUMULATOR: DbAccumulator;
}

/// Registers the SeaORM metric callback on a connection. Runs even when
/// export is disabled because the counts also appear in log records.
pub fn install(connection: &mut sea_orm::DatabaseConnection) {
    connection.set_metric_callback(|info| {
        let _ = ACCUMULATOR.try_with(|acc| {
            acc.queries.fetch_add(1, Ordering::Relaxed);
            acc.elapsed_us
                .fetch_add(info.elapsed.as_micros() as u64, Ordering::Relaxed);
            if info.failed {
                acc.failed.fetch_add(1, Ordering::Relaxed);
            }
        });
    });
}

/// Reads (query_count, runtime_ms, failed_query_count) for log attributes.
pub fn snapshot() -> Option<(u64, f64, u64)> {
    ACCUMULATOR
        .try_with(|acc| {
            (
                acc.queries.load(Ordering::Relaxed),
                acc.elapsed_us.load(Ordering::Relaxed) as f64 / 1000.0,
                acc.failed.load(Ordering::Relaxed),
            )
        })
        .ok()
}

/// Runs `future` with a fresh accumulator; on completion records the three
/// `medtracker.db.*` values on the given unit-of-work span.
pub async fn scope<F: Future>(span: &tracing::Span, future: F) -> F::Output {
    let acc = DbAccumulator::default();
    ACCUMULATOR
        .scope(acc.clone(), async move {
            let output = future.await;
            span.record(
                "medtracker.db.query_count",
                acc.queries.load(Ordering::Relaxed),
            );
            span.record(
                "medtracker.db.runtime_ms",
                acc.elapsed_us.load(Ordering::Relaxed) as f64 / 1000.0,
            );
            span.record(
                "medtracker.db.failed_query_count",
                acc.failed.load(Ordering::Relaxed),
            );
            output
        })
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callback_outside_a_scope_does_nothing() {
        assert!(snapshot().is_none());
    }

    #[tokio::test]
    async fn scope_records_db_metrics_on_the_passed_span() {
        let exporter = opentelemetry_sdk::trace::InMemorySpanExporter::default();
        let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
            .with_simple_exporter(exporter.clone())
            .with_sampler(opentelemetry_sdk::trace::Sampler::AlwaysOn)
            .build();
        let layer = crate::telemetry::otel_layer(&provider, "test");
        use tracing_subscriber::layer::SubscriberExt;
        let subscriber = tracing_subscriber::registry().with(layer);
        use tracing::instrument::WithSubscriber;
        async {
            let span = tracing::info_span!(
                "job",
                otel.name = "job accounting",
                medtracker.db.query_count = tracing::field::Empty,
                medtracker.db.runtime_ms = tracing::field::Empty,
                medtracker.db.failed_query_count = tracing::field::Empty,
            );
            scope(&span, async {
                ACCUMULATOR.with(|acc| {
                    acc.queries.fetch_add(1, Ordering::Relaxed);
                    acc.elapsed_us.fetch_add(1500, Ordering::Relaxed);
                });
            })
            .await;
        }
        .with_subscriber(subscriber)
        .await;
        provider.force_flush().expect("flush");
        let spans = exporter.get_finished_spans().expect("exported spans");
        assert_eq!(spans.len(), 1);
        let attrs: Vec<_> = spans[0]
            .attributes
            .iter()
            .map(|kv| (kv.key.as_str(), format!("{:?}", kv.value)))
            .collect();
        assert!(
            attrs
                .iter()
                .any(|(k, v)| *k == "medtracker.db.query_count" && v.contains('1')),
            "{attrs:?}"
        );
        assert!(
            attrs
                .iter()
                .any(|(k, v)| *k == "medtracker.db.runtime_ms" && v.contains("1.5")),
            "{attrs:?}"
        );
    }
}
