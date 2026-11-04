use sea_orm::sea_query::{Expr, ExprTrait, Func, Query};
use sea_orm::{ConnectOptions, ConnectionTrait, Database};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const APPLICATION_NAME: &str = "medtracker_cleanup_probe";
const SAMPLE_INTERVAL: Duration = Duration::from_millis(250);
const SAMPLE_WINDOW: Duration = Duration::from_secs(14);
const CLOSE_WINDOW: Duration = Duration::from_secs(1);
const BACKEND_LIMIT: u64 = 16;

#[tokio::main]
async fn main() {
    if observe().await.is_err() {
        eprintln!("cleanup waits probe failed: connection or activity metadata unavailable");
        std::process::exit(1);
    }
}

async fn observe() -> Result<(), ()> {
    if std::env::var("MEDTRACKER_OWNED_DATABASE").as_deref() != Ok("1") {
        return Err(());
    }
    uuid::Uuid::parse_str(&std::env::var("MEDTRACKER_TEST_RUN_ID").map_err(|_| ())?)
        .map_err(|_| ())?;

    let control_uri = std::env::var("DATABASE_URL").map_err(|_| ())?;
    if !is_owned_control_uri(&control_uri) {
        return Err(());
    }

    let mut options = ConnectOptions::new(control_uri);
    options
        .max_connections(1)
        .min_connections(0)
        .connect_timeout(Duration::from_secs(3))
        .sqlx_logging(false);
    let control = Database::connect(options).await.map_err(|_| ())?;
    let started = tokio::time::Instant::now();
    let sample_deadline = started + SAMPLE_WINDOW;
    let finish_deadline = sample_deadline + CLOSE_WINDOW;
    let mut sample_number = 0_u64;

    while tokio::time::Instant::now() < sample_deadline {
        let query = Query::select()
            .columns(["state", "wait_event_type", "wait_event", "backend_type"])
            .expr_as(
                Func::cust("cardinality").arg(Func::cust("pg_blocking_pids").arg(Expr::col("pid"))),
                "blocker_count",
            )
            .from(("pg_catalog", "pg_stat_activity"))
            .and_where(Expr::col("application_name").eq(APPLICATION_NAME))
            .limit(BACKEND_LIMIT)
            .to_owned();
        let statement = control.get_database_backend().build(&query);
        let rows = tokio::time::timeout_at(sample_deadline, control.query_all_raw(statement))
            .await
            .map_err(|_| ())?
            .map_err(|_| ())?;

        let mut backends = Vec::with_capacity(rows.len());
        for row in rows {
            backends.push(serde_json::json!({
                "state": row.try_get::<Option<String>>("", "state").map_err(|_| ())?,
                "wait_event_type": row.try_get::<Option<String>>("", "wait_event_type").map_err(|_| ())?,
                "wait_event": row.try_get::<Option<String>>("", "wait_event").map_err(|_| ())?,
                "backend_type": row.try_get::<Option<String>>("", "backend_type").map_err(|_| ())?,
                "blocker_count": row.try_get::<i32>("", "blocker_count").map_err(|_| ())?,
            }));
        }

        let observed_epoch_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ())?
            .as_nanos()
            .to_string();
        let sample = serde_json::json!({
            "sample": sample_number,
            "observed_epoch_ns": observed_epoch_ns,
            "elapsed_ms": started.elapsed().as_millis(),
            "backends": backends,
        });
        println!("{sample}");
        use std::io::Write;
        std::io::stdout().flush().map_err(|_| ())?;
        sample_number += 1;
        tokio::time::sleep(SAMPLE_INTERVAL).await;
    }

    tokio::time::timeout_at(finish_deadline, control.close())
        .await
        .map_err(|_| ())?
        .map_err(|_| ())
}

fn is_owned_control_uri(value: &str) -> bool {
    let Ok(uri) = url::Url::parse(value) else {
        return false;
    };
    let owned_host = match uri.host_str() {
        Some("127.0.0.1") => uri.port().is_some_and(|port| port != 0),
        Some("postgres") => uri.port().is_none_or(|port| port == 5432),
        _ => false,
    };

    owned_host
        && uri.scheme() == "postgres"
        && uri.username() == "medtracker"
        && uri.password() == Some("medtracker_password")
        && uri.path() == "/medtracker_loco"
        && uri.query().is_none()
        && uri.fragment().is_none()
}
