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
/// `medtracker.db.*` values on the current unit-of-work span.
pub async fn scope<F: Future>(future: F) -> F::Output {
    let acc = DbAccumulator::default();
    ACCUMULATOR
        .scope(acc.clone(), async move {
            let output = future.await;
            let span = tracing::Span::current();
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
}
