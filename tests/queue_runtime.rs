#[path = "support/scenario.rs"]
pub mod scenario;
use loco_rs::{
    bgworker::{Queue, pg},
    config::{Config, PostgresQueueConfig, QueueConfig},
    environment::Environment,
};
use scenario::Scenario;
use sea_orm::ConnectionTrait;
use sea_orm::sea_query::{
    Alias, ColumnDef, Expr, ExprTrait, Order, Query, Table, extension::postgres::PgExpr,
};
use serde_json::json;

struct Fixture {
    scenario: Scenario,
    config: PostgresQueueConfig,
}

impl std::ops::Deref for Fixture {
    type Target = Scenario;
    fn deref(&self) -> &Self::Target {
        &self.scenario
    }
}

impl Fixture {
    async fn new() -> Self {
        let scenario = Scenario::new().await;
        let config = serde_json::from_value(json!({
            "uri": std::env::var("DATABASE_URL").unwrap(),
            "min_connections": 1,
            "max_connections": 2,
            "connect_timeout": 5000,
            "num_workers": 1,
            "poll_interval_sec": 1
        }))
        .unwrap();
        Self { scenario, config }
    }

    async fn queue(&self) -> Queue {
        let queue = pg::create_provider(&self.config).await.unwrap();
        queue.setup().await.unwrap();
        queue
    }

    async fn status(&self, id: &str) -> String {
        let query = Query::select()
            .column("status")
            .from(("public", "pg_loco_queue"))
            .and_where(Expr::col("id").eq(id))
            .to_owned();
        self.runtime
            .query_one_raw(self.runtime.get_database_backend().build(&query))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "status")
            .unwrap()
    }

    async fn mark(&self, id: &str, status: &str, updated_at: chrono::DateTime<chrono::Utc>) {
        let query = Query::update()
            .table(("public", "pg_loco_queue"))
            .value("status", status)
            .value("updated_at", updated_at)
            .and_where(Expr::col("id").eq(id))
            .to_owned();
        let result = self
            .runtime
            .execute_raw(self.runtime.get_database_backend().build(&query))
            .await
            .unwrap();
        assert_eq!(result.rows_affected(), 1);
    }

    async fn close(self, ids: &[String]) {
        if !ids.is_empty() {
            let query = Query::delete()
                .from_table(("public", "pg_loco_queue"))
                .and_where(Expr::col("id").is_in(ids.iter().cloned()))
                .to_owned();
            self.admin
                .execute_raw(self.admin.get_database_backend().build(&query))
                .await
                .unwrap();
        }
        self.scenario.close().await;
    }
}

#[tokio::test]
async fn test_queue_starts_when_database_handshake_takes_over_half_a_second() {
    let fixture = Fixture::new().await;
    let mut config = Config::new(&Environment::Test).unwrap();
    let Some(QueueConfig::Postgres(queue_config)) = config.queue.as_mut() else {
        panic!("PostgreSQL queue required")
    };
    let mut uri = url::Url::parse(&fixture.config.uri).unwrap();
    let upstream_host = uri.host_str().expect("database URI host").to_owned();
    let upstream_port = uri.port().unwrap_or(5432);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    uri.set_host(Some("127.0.0.1")).unwrap();
    uri.set_port(Some(listener.local_addr().unwrap().port()))
        .unwrap();
    queue_config.uri = uri.to_string();
    let relay = tokio::spawn(async move {
        let (mut incoming, _) = listener.accept().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(750)).await;
        let mut outgoing = tokio::net::TcpStream::connect((upstream_host.as_str(), upstream_port))
            .await
            .unwrap();
        let _ = tokio::io::copy_bidirectional(&mut incoming, &mut outgoing).await;
    });
    let result = pg::create_provider(queue_config).await;
    relay.abort();
    let _ = relay.await;
    let queue = result.expect("test queue must tolerate a bounded slow database handshake");
    queue.shutdown().unwrap();
    drop(queue);
    fixture.close(&[]).await;
}

#[tokio::test]
async fn queue_setup_and_reconnection_preserve_jobs_without_runtime_ddl() {
    let fixture = Fixture::new().await;
    let table = Alias::new(format!("forbidden_queue_{}", uuid::Uuid::new_v4().simple()));
    let create = Table::create()
        .table((Alias::new("public"), table.clone()))
        .col(ColumnDef::new("id").big_integer())
        .to_owned();
    let result = fixture
        .runtime
        .execute_raw(fixture.runtime.get_database_backend().build(&create))
        .await;
    if result.is_ok() {
        let drop_table = Table::drop()
            .table((Alias::new("public"), table))
            .to_owned();
        fixture
            .admin
            .execute_raw(fixture.admin.get_database_backend().build(&drop_table))
            .await
            .unwrap();
    }
    assert!(result.is_err());
    let queue = fixture.queue().await;
    let id = queue
        .enqueue(
            "PersistedJob".into(),
            None,
            json!({"operation_key": uuid::Uuid::new_v4().to_string()}),
            None,
            None,
        )
        .await
        .unwrap()
        .unwrap();
    queue.shutdown().unwrap();
    drop(queue);
    let restarted = fixture.queue().await;
    restarted.ping().await.unwrap();
    assert_eq!(fixture.status(&id).await, "queued");
    restarted.shutdown().unwrap();
    drop(restarted);
    fixture.close(&[id]).await;
}

#[tokio::test]
async fn maintained_retry_and_requeue_preserve_payload_and_recover_only_eligible_jobs() {
    let fixture = Fixture::new().await;
    let queue = fixture.queue().await;
    let mut ids = Vec::new();
    let keys =
        ["failed", "abandoned", "active"].map(|key| format!("{key}-{}", uuid::Uuid::new_v4()));
    for key in &keys {
        ids.push(
            queue
                .enqueue(
                    "RecoveryJob".into(),
                    None,
                    json!({"operation_key": key}),
                    None,
                    None,
                )
                .await
                .unwrap()
                .unwrap(),
        );
    }
    let now = chrono::Utc::now();
    for (id, kind) in ids.iter().zip(["failed", "abandoned", "active"]) {
        let status = if kind == "failed" {
            "failed"
        } else {
            "processing"
        };
        let updated = if kind == "abandoned" {
            now - chrono::Duration::minutes(20)
        } else {
            now
        };
        fixture.mark(id, status, updated).await;
    }
    assert_eq!(queue.retry_failed(Some(&ids[0])).await.unwrap(), 1);
    assert_eq!(queue.retry_failed(Some(&ids[0])).await.unwrap(), 0);
    queue.requeue(&10).await.unwrap();
    assert_eq!(fixture.status(&ids[0]).await, "queued");
    assert_eq!(fixture.status(&ids[1]).await, "queued");
    assert_eq!(fixture.status(&ids[2]).await, "processing");
    let query = Query::select()
        .expr_as(
            Expr::col("task_data").cast_json_field("operation_key"),
            "operation_key",
        )
        .from(("public", "pg_loco_queue"))
        .and_where(Expr::col("id").is_in(ids.iter().cloned()))
        .order_by("operation_key", Order::Asc)
        .to_owned();
    let payloads = fixture
        .runtime
        .query_all_raw(fixture.runtime.get_database_backend().build(&query))
        .await
        .unwrap();
    assert_eq!(
        payloads
            .iter()
            .map(|row| row.try_get::<String>("", "operation_key").unwrap())
            .collect::<Vec<_>>(),
        [keys[1].clone(), keys[2].clone(), keys[0].clone()]
    );
    queue.shutdown().unwrap();
    drop(queue);
    fixture.close(&ids).await;
}

#[tokio::test]
async fn configured_postgres_workers_recover_abandoned_jobs_automatically() {
    for environment in [
        Environment::Development,
        Environment::Test,
        Environment::Production,
    ] {
        let config = Config::new(&environment).unwrap();
        let Some(QueueConfig::Postgres(queue)) = config.queue else {
            panic!("The application must use its standard PostgreSQL queue");
        };
        let reaper = queue
            .reaper
            .expect("An interrupted worker must recover abandoned jobs automatically");
        assert!(reaper.age_minutes > 0);
        assert!(reaper.interval_seconds > 0);
    }
    let mut fixture = Fixture::new().await;
    let Some(QueueConfig::Postgres(config)) = Config::new(&Environment::Test).unwrap().queue else {
        panic!("The application must use its standard PostgreSQL queue");
    };
    let mut reaper = config.reaper.unwrap();
    reaper.interval_seconds = 1;
    fixture.config.reaper = Some(reaper);
    let queue = std::sync::Arc::new(fixture.queue().await);
    let id = queue
        .enqueue(
            "InterruptedJob".into(),
            None,
            json!({"operation_key": uuid::Uuid::new_v4().to_string()}),
            Some(vec!["unassigned".into()]),
            None,
        )
        .await
        .unwrap()
        .unwrap();
    fixture
        .mark(
            &id,
            "processing",
            chrono::Utc::now() - chrono::Duration::minutes(20),
        )
        .await;
    let running_queue = std::sync::Arc::clone(&queue);
    let worker = tokio::spawn(async move { running_queue.run(vec![]).await });
    let recovered = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if fixture.status(&id).await == "queued" {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await;
    queue.shutdown().unwrap();
    let stopped = tokio::time::timeout(std::time::Duration::from_secs(5), worker).await;
    assert!(
        recovered.is_ok(),
        "The framework reaper must recover abandoned work"
    );
    stopped.unwrap().unwrap().unwrap();
    drop(queue);
    fixture.close(&[id]).await;
}
