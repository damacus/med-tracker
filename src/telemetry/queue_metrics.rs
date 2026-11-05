//! Worker-owned queue snapshots. Collection callbacks only read atomics.

use opentelemetry::KeyValue;
use sea_orm::{
    ConnectionTrait, DatabaseConnection,
    sea_query::{Alias, Expr, ExprTrait, Func, Query},
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

const STATES: [&str; 6] = [
    "queued",
    "processing",
    "completed",
    "failed",
    "cancelled",
    "other",
];

struct Snapshot {
    counts: [AtomicU64; 6],
    oldest: AtomicI64,
    ready: AtomicBool,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            counts: std::array::from_fn(|_| AtomicU64::new(0)),
            oldest: AtomicI64::new(i64::MIN),
            ready: AtomicBool::new(false),
        }
    }
}

#[derive(Default)]
struct Health {
    failing: bool,
    last_warning: Option<Instant>,
}

impl Health {
    fn record(&mut self, success: bool, now: Instant) {
        if success {
            if self.failing {
                tracing::info!(
                    event.name = "medtracker.telemetry.queue_recovered",
                    "Queue metrics collection recovered"
                );
                self.failing = false;
            }
        } else {
            if self
                .last_warning
                .is_none_or(|last| now.duration_since(last) >= Duration::from_secs(60))
            {
                tracing::warn!(event.name = "medtracker.telemetry.queue_failed", error.type = "database", "Queue metrics collection failed");
                self.last_warning = Some(now);
            }
            self.failing = true;
        }
    }
}

fn publish(
    snapshot: &Snapshot,
    health: &mut Health,
    result: Result<([u64; 6], i64), ()>,
    now: Instant,
) {
    let success = result.is_ok();
    if let Ok((counts, oldest)) = result {
        for (value, count) in counts.into_iter().zip(&snapshot.counts) {
            count.store(value, Ordering::Relaxed);
        }
        snapshot.oldest.store(oldest, Ordering::Relaxed);
        snapshot.ready.store(true, Ordering::Release);
    }
    health.record(success, now);
}

struct RefreshGuard(tokio::task::JoinHandle<()>);
impl Drop for RefreshGuard {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub fn start(ctx: &loco_rs::app::AppContext) {
    let Some(meter) = super::metrics::meter() else {
        return;
    };
    if ctx.shared_store.get::<Arc<RefreshGuard>>().is_some() {
        return;
    }
    let snapshot = Arc::new(Snapshot::default());
    let counts = snapshot.clone();
    meter
        .u64_observable_gauge("medtracker.queue.jobs")
        .with_callback(move |observer| {
            if counts.ready.load(Ordering::Acquire) {
                for (index, name) in STATES.iter().enumerate() {
                    observer.observe(
                        counts.counts[index].load(Ordering::Relaxed),
                        &[KeyValue::new("medtracker.queue.state", *name)],
                    );
                }
            }
        })
        .build();
    let age = snapshot.clone();
    meter
        .u64_observable_gauge("medtracker.queue.oldest_queued_age_s")
        .with_unit("s")
        .with_callback(move |observer| {
            if age.ready.load(Ordering::Acquire) {
                let oldest = age.oldest.load(Ordering::Relaxed);
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;
                observer.observe(
                    if oldest == i64::MIN {
                        0
                    } else {
                        std::cmp::max(now.saturating_sub(oldest), 0) as u64
                    },
                    &[],
                );
            }
        })
        .build();
    let db = ctx.db.clone();
    let handle = tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(30));
        let mut health = Health::default();
        loop {
            ticker.tick().await;
            let result = tokio::time::timeout(Duration::from_secs(5), refresh(&db))
                .await
                .ok()
                .and_then(Result::ok)
                .ok_or(());
            publish(&snapshot, &mut health, result, Instant::now());
        }
    });
    ctx.shared_store.insert(Arc::new(RefreshGuard(handle)));
}

pub fn stop(ctx: &loco_rs::app::AppContext) {
    if let Some(guard) = ctx.shared_store.get::<Arc<RefreshGuard>>() {
        guard.0.abort();
    }
}

async fn refresh(db: &DatabaseConnection) -> Result<([u64; 6], i64), sea_orm::DbErr> {
    let state = Alias::new("status");
    let table = Alias::new("pg_loco_queue");
    let counts = Query::select()
        .column(state.clone())
        .expr_as(Expr::col(state.clone()).count(), Alias::new("total"))
        .from(table.clone())
        .group_by_col(state.clone())
        .to_owned();
    let rows = db
        .query_all_raw(db.get_database_backend().build(&counts))
        .await?;
    let now = chrono::Utc::now();
    let oldest = Query::select()
        .expr_as(
            Func::min(Expr::col(Alias::new("run_at"))),
            Alias::new("oldest"),
        )
        .from(table)
        .and_where(Expr::col(state).eq("queued"))
        .and_where(Expr::col(Alias::new("run_at")).lte(now))
        .to_owned();
    let row = db
        .query_one_raw(db.get_database_backend().build(&oldest))
        .await?
        .ok_or_else(|| sea_orm::DbErr::Custom("Queue aggregate unavailable".into()))?;
    let earliest: Option<chrono::DateTime<chrono::Utc>> = row.try_get("", "oldest")?;
    let mut next = [0_u64; 6];
    for row in rows {
        let name: String = row.try_get("", "status")?;
        let total: i64 = row.try_get("", "total")?;
        let index = STATES[..5]
            .iter()
            .position(|state| *state == name)
            .unwrap_or(5);
        let total = u64::try_from(total)
            .map_err(|_| sea_orm::DbErr::Custom("Invalid queue count".into()))?;
        next[index] = next[index]
            .checked_add(total)
            .ok_or_else(|| sea_orm::DbErr::Custom("Queue count overflow".into()))?;
    }
    Ok((next, earliest.map_or(i64::MIN, |time| time.timestamp())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::{Layer, layer::SubscriberExt};

    #[derive(Clone, Default)]
    struct Events(Arc<std::sync::Mutex<Vec<tracing::Level>>>);
    impl<S: tracing::Subscriber> Layer<S> for Events {
        fn on_event(
            &self,
            event: &tracing::Event<'_>,
            _: tracing_subscriber::layer::Context<'_, S>,
        ) {
            self.0.lock().unwrap().push(*event.metadata().level());
        }
    }

    #[test]
    fn failed_refresh_preserves_snapshot_and_reports_rate_limited_recovery() {
        let snapshot = Snapshot::default();
        let mut health = Health::default();
        let now = Instant::now();
        let events = Events::default();
        tracing::subscriber::with_default(
            tracing_subscriber::registry().with(events.clone()),
            || {
                publish(&snapshot, &mut health, Ok(([2, 0, 0, 0, 0, 0], 0)), now);
                publish(&snapshot, &mut health, Err(()), now);
                publish(
                    &snapshot,
                    &mut health,
                    Err(()),
                    now + Duration::from_secs(59),
                );
                assert_eq!(snapshot.counts[0].load(Ordering::Acquire), 2);
                assert_eq!(snapshot.oldest.load(Ordering::Acquire), 0);
                publish(
                    &snapshot,
                    &mut health,
                    Err(()),
                    now + Duration::from_secs(60),
                );
                publish(
                    &snapshot,
                    &mut health,
                    Ok(([3, 0, 0, 0, 0, 0], i64::MIN)),
                    now + Duration::from_secs(61),
                );
                publish(
                    &snapshot,
                    &mut health,
                    Ok(([3, 0, 0, 0, 0, 0], i64::MIN)),
                    now + Duration::from_secs(62),
                );
            },
        );
        assert_eq!(
            *events.0.lock().unwrap(),
            vec![
                tracing::Level::WARN,
                tracing::Level::WARN,
                tracing::Level::INFO
            ]
        );
        assert_eq!(snapshot.counts[0].load(Ordering::Acquire), 3);
        assert_eq!(snapshot.oldest.load(Ordering::Acquire), i64::MIN);
    }

    #[tokio::test]
    async fn dropping_guard_aborts_owned_refresh_task() {
        let (started, ready) = tokio::sync::oneshot::channel();
        let handle = tokio::spawn(async move {
            let _ = started.send(());
            std::future::pending::<()>().await;
        });
        let abort = handle.abort_handle();
        let guard = RefreshGuard(handle);
        ready.await.unwrap();
        drop(guard);
        tokio::task::yield_now().await;
        assert!(abort.is_finished());
    }
}
