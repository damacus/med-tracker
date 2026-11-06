#[path = "support/scenario.rs"]
pub mod scenario;
use loco_rs::{
    bgworker::{Queue, pg},
    config::{Config, PostgresQueueConfig, QueueConfig},
    environment::Environment,
};
use scenario::Scenario;
use sea_orm::sea_query::{
    Alias, ColumnDef, Expr, ExprTrait, Index, Order, Query, Table, extension::postgres::PgExpr,
};
use sea_orm::{ConditionalStatement, ConnectionTrait, TransactionTrait};
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
        pg::create_provider(&self.config).await.unwrap()
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
async fn restricted_worker_binary_starts_with_preprovisioned_queue() {
    let fixture = Fixture::new().await;
    let queue = fixture.queue().await;
    let id = queue
        .enqueue(
            "MailerWorker".into(),
            None,
            json!({"invalid": true}),
            Some(vec!["synthetic-no-jobs".into()]),
            None,
        )
        .await
        .unwrap()
        .unwrap();
    let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_med-tracker"));
    command
        .args([
            "start",
            "--worker=synthetic-no-jobs",
            "--environment",
            "test",
        ])
        .env("DATABASE_URL", &fixture.config.uri)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    let mut child = command.spawn().unwrap();
    let result = tokio::time::timeout(std::time::Duration::from_secs(15), async {
        loop {
            if fixture.status(&id).await == "failed" {
                return true;
            }
            if child.try_wait().unwrap().is_some() {
                return false;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap_or(false);
    let _ = child.kill().await;
    queue.shutdown().unwrap();
    fixture.close(&[id]).await;
    assert!(result, "worker did not process the synthetic job");
}

#[cfg(unix)]
#[tokio::test]
async fn restricted_worker_drains_inflight_job_on_sigterm() {
    let fixture = Fixture::new().await;
    let queue = fixture.queue().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let email = loco_rs::mailer::Email {
        from: Some("sender@example.test".into()),
        to: "recipient@example.test".into(),
        subject: "Synthetic shutdown test".into(),
        text: "Synthetic message".into(),
        html: "Synthetic message".into(),
        ..Default::default()
    };
    let id = queue
        .enqueue(
            "MailerWorker".into(),
            None,
            serde_json::to_value(email).unwrap(),
            Some(vec!["shutdown-test".into()]),
            None,
        )
        .await
        .unwrap()
        .unwrap();
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_med-tracker"))
        .args(["start", "--worker=shutdown-test", "--environment", "test"])
        .env("DATABASE_URL", &fixture.config.uri)
        .env("MEDTRACKER_CAPTURE_MAIL", "true")
        .env("MEDTRACKER_SMTP_PORT", port.to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let (socket, _) = tokio::time::timeout(std::time::Duration::from_secs(15), listener.accept())
        .await
        .expect("worker did not reach synthetic SMTP server")
        .unwrap();
    assert_eq!(fixture.status(&id).await, "processing");
    let signal = tokio::process::Command::new("kill")
        .args(["-TERM", &child.id().unwrap().to_string()])
        .status()
        .await
        .unwrap();
    assert!(signal.success());
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert!(
        child.try_wait().unwrap().is_none(),
        "worker exited with a job in flight"
    );
    drop(socket);
    let status = tokio::time::timeout(std::time::Duration::from_secs(10), child.wait())
        .await
        .expect("worker did not stop after the job finished")
        .unwrap();
    assert!(status.success(), "worker exited with {status}");
    assert_eq!(fixture.status(&id).await, "failed");
    queue.shutdown().unwrap();
    fixture.close(&[id]).await;
}

#[tokio::test]
async fn restricted_server_binary_starts_with_preprovisioned_queue() {
    let fixture = Fixture::new().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_med-tracker"));
    command
        .args([
            "start",
            "--environment",
            "test",
            "--binding",
            "127.0.0.1",
            "--port",
            &port.to_string(),
        ])
        .env("DATABASE_URL", &fixture.config.uri)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    let mut child = command.spawn().unwrap();
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{port}/up");
    let mut diagnostic = String::new();
    let result = tokio::time::timeout(std::time::Duration::from_secs(15), async {
        loop {
            match client.get(&url).send().await {
                Ok(response) if response.status().is_success() => return true,
                Ok(response) => diagnostic = format!("HTTP {}", response.status()),
                Err(error) => diagnostic = error.to_string(),
            }
            if let Some(status) = child.try_wait().unwrap() {
                diagnostic = format!("exit {status}: {diagnostic}");
                return false;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap_or(false);
    let _ = child.start_kill();
    child.wait().await.unwrap();
    fixture.close(&[]).await;
    assert!(result, "server did not become healthy: {diagnostic}");
}

#[tokio::test]
async fn boot_rejects_incorrect_unique_queue_index() {
    use loco_rs::{app::Hooks, boot::StartMode};
    let fixture = Fixture::new().await;
    let backend = fixture.admin.get_database_backend();
    let drop_index = Index::drop().name("pg_loco_queue_unique_queued").to_owned();
    for wrong_predicate in [false, true] {
        fixture
            .admin
            .execute_raw(backend.build(&drop_index))
            .await
            .unwrap();
        let mut wrong = Index::create();
        wrong
            .name("pg_loco_queue_unique_queued")
            .table("pg_loco_queue")
            .col("name");
        if wrong_predicate {
            wrong
                .unique()
                .col("unique_key")
                .and_where(Expr::col("status").eq("failed"))
                .and_where(Expr::col("unique_key").is_not_null());
        }
        fixture
            .admin
            .execute_raw(backend.build(&wrong.to_owned()))
            .await
            .unwrap();
        let mut config = Config::new(&Environment::Test).unwrap();
        config.database.uri = fixture.config.uri.clone();
        let Some(QueueConfig::Postgres(queue)) = config.queue.as_mut() else {
            panic!("PostgreSQL queue required")
        };
        queue.uri = fixture.config.uri.clone();
        let result =
            med_tracker::app::App::boot(StartMode::ServerOnly, &Environment::Test, config).await;
        if let Ok(boot) = &result
            && let Some(queue) = &boot.app_context.queue_provider
        {
            queue.shutdown().unwrap();
        }
        fixture
            .admin
            .execute_raw(backend.build(&drop_index))
            .await
            .unwrap();
        let correct = Index::create()
            .name("pg_loco_queue_unique_queued")
            .table("pg_loco_queue")
            .unique()
            .col("name")
            .col("unique_key")
            .and_where(Expr::col("status").eq("queued"))
            .and_where(Expr::col("unique_key").is_not_null())
            .to_owned();
        fixture
            .admin
            .execute_raw(backend.build(&correct))
            .await
            .unwrap();
        let error = result
            .err()
            .expect("boot accepted an incorrectly defined queue index");
        assert!(
            str::contains(&error.to_string(), "Queue schema is not prepared"),
            "unexpected boot error: {error}"
        );
    }
    fixture.close(&[]).await;
}

#[tokio::test]
async fn boot_rejects_queue_index_without_primary_key_constraint() {
    use loco_rs::{app::Hooks, boot::StartMode};
    let fixture = Fixture::new().await;
    let backend = fixture.admin.get_database_backend();
    let drop_primary = Table::alter()
        .table("pg_loco_queue")
        .drop_constraint("pg_loco_queue_pkey")
        .to_owned();
    fixture
        .admin
        .execute_raw(backend.build(&drop_primary))
        .await
        .unwrap();
    let index = Index::create()
        .name("pg_loco_queue_pkey")
        .table("pg_loco_queue")
        .unique()
        .col("id")
        .to_owned();
    fixture
        .admin
        .execute_raw(backend.build(&index))
        .await
        .unwrap();
    let mut config = Config::new(&Environment::Test).unwrap();
    config.database.uri = fixture.config.uri.clone();
    let Some(QueueConfig::Postgres(queue)) = config.queue.as_mut() else {
        panic!("PostgreSQL queue required")
    };
    queue.uri = fixture.config.uri.clone();
    let result =
        med_tracker::app::App::boot(StartMode::ServerOnly, &Environment::Test, config).await;
    if let Ok(boot) = &result
        && let Some(queue) = &boot.app_context.queue_provider
    {
        queue.shutdown().unwrap();
    }
    let drop_index = Index::drop().name("pg_loco_queue_pkey").to_owned();
    fixture
        .admin
        .execute_raw(backend.build(&drop_index))
        .await
        .unwrap();
    // SeaQuery cannot add a primary-key constraint to an existing table.
    fixture
        .admin
        .execute_unprepared("ALTER TABLE public.pg_loco_queue ADD PRIMARY KEY (id)")
        .await
        .unwrap();
    fixture.close(&[]).await;
    assert!(
        result.is_err(),
        "boot accepted a unique index without a primary-key constraint"
    );
}

#[tokio::test]
async fn boot_rejects_wrong_queue_column_type() {
    use loco_rs::{app::Hooks, boot::StartMode};
    let fixture = Fixture::new().await;
    let backend = fixture.admin.get_database_backend();
    let wrong = Table::alter()
        .table("pg_loco_queue")
        .modify_column(
            ColumnDef::new("attempts")
                .big_integer()
                .not_null()
                .default(0),
        )
        .to_owned();
    fixture
        .admin
        .execute_raw(backend.build(&wrong))
        .await
        .unwrap();
    let mut config = Config::new(&Environment::Test).unwrap();
    config.database.uri = fixture.config.uri.clone();
    let Some(QueueConfig::Postgres(queue)) = config.queue.as_mut() else {
        panic!("PostgreSQL queue required")
    };
    queue.uri = fixture.config.uri.clone();
    let result =
        med_tracker::app::App::boot(StartMode::ServerOnly, &Environment::Test, config).await;
    if let Ok(boot) = &result
        && let Some(queue) = &boot.app_context.queue_provider
    {
        queue.shutdown().unwrap();
    }
    let correct = Table::alter()
        .table("pg_loco_queue")
        .modify_column(ColumnDef::new("attempts").integer().not_null().default(0))
        .to_owned();
    fixture
        .admin
        .execute_raw(backend.build(&correct))
        .await
        .unwrap();
    fixture.close(&[]).await;
    assert!(
        result.is_err(),
        "boot accepted the wrong queue attempts type"
    );
}

#[tokio::test]
async fn boot_rejects_wrong_queue_column_default() {
    use loco_rs::{app::Hooks, boot::StartMode};
    let fixture = Fixture::new().await;
    let backend = fixture.admin.get_database_backend();
    let wrong = Table::alter()
        .table("pg_loco_queue")
        .modify_column(ColumnDef::new("attempts").integer().not_null().default(7))
        .to_owned();
    fixture
        .admin
        .execute_raw(backend.build(&wrong))
        .await
        .unwrap();
    let mut config = Config::new(&Environment::Test).unwrap();
    config.database.uri = fixture.config.uri.clone();
    let Some(QueueConfig::Postgres(queue)) = config.queue.as_mut() else {
        panic!("PostgreSQL queue required")
    };
    queue.uri = fixture.config.uri.clone();
    let result =
        med_tracker::app::App::boot(StartMode::ServerOnly, &Environment::Test, config).await;
    if let Ok(boot) = &result
        && let Some(queue) = &boot.app_context.queue_provider
    {
        queue.shutdown().unwrap();
    }
    let correct = Table::alter()
        .table("pg_loco_queue")
        .modify_column(ColumnDef::new("attempts").integer().not_null().default(0))
        .to_owned();
    fixture
        .admin
        .execute_raw(backend.build(&correct))
        .await
        .unwrap();
    fixture.close(&[]).await;
    let error = result
        .err()
        .expect("boot accepted the wrong queue attempts default");
    assert!(str::contains(
        &error.to_string(),
        "Queue schema is not prepared"
    ));
}

async fn prepare_old_queue_schema(transaction: &sea_orm::DatabaseTransaction) {
    let backend = transaction.get_database_backend();
    for name in [
        "pg_loco_queue_unique_queued",
        "pg_loco_queue_dequeue",
        "pg_loco_queue_status_updated_at",
    ] {
        let drop_index = Index::drop().name(name).to_owned();
        transaction
            .execute_raw(backend.build(&drop_index))
            .await
            .unwrap();
    }
    let old_schema = Table::alter()
        .table("pg_loco_queue")
        .drop_constraint("pg_loco_queue_pkey")
        .drop_column("attempts")
        .drop_column("unique_key")
        .to_owned();
    transaction
        .execute_raw(backend.build(&old_schema))
        .await
        .unwrap();
}

#[tokio::test]
async fn queue_upgrade_preserves_existing_jobs() {
    use migration::{MigrationTrait, SchemaManager};
    let fixture = Fixture::new().await;
    let transaction = fixture.admin.begin().await.unwrap();
    let backend = transaction.get_database_backend();
    prepare_old_queue_schema(&transaction).await;
    let ids = ["queued", "processing", "failed"]
        .map(|status| format!("{}-{status}", uuid::Uuid::new_v4()));
    for (id, status) in ids.iter().zip(["queued", "processing", "failed"]) {
        let insert = Query::insert()
            .into_table("pg_loco_queue")
            .columns(["id", "name", "task_data", "status", "run_at"])
            .values_panic([
                id.as_str().into(),
                "MigrationFixture".into(),
                json!({"fixture": true}).into(),
                status.into(),
                chrono::Utc::now().into(),
            ])
            .to_owned();
        transaction
            .execute_raw(backend.build(&insert))
            .await
            .unwrap();
    }
    let manager = SchemaManager::new(&transaction);
    migration::m20261106_000035_loco_queue::Migration
        .up(&manager)
        .await
        .unwrap();
    let select = Query::select()
        .columns(["id", "status", "task_data", "attempts", "unique_key"])
        .from("pg_loco_queue")
        .and_where(Expr::col("id").is_in(ids.iter().cloned()))
        .order_by("status", Order::Asc)
        .to_owned();
    let rows = transaction
        .query_all_raw(backend.build(&select))
        .await
        .unwrap();
    assert_eq!(rows.len(), 3);
    for row in rows {
        assert_eq!(row.try_get::<i32>("", "attempts").unwrap(), 0);
        assert_eq!(
            row.try_get::<Option<String>>("", "unique_key").unwrap(),
            None
        );
        let id = row.try_get::<String>("", "id").unwrap();
        let status = row.try_get::<String>("", "status").unwrap();
        assert!(ids.contains(&id));
        assert!(id.ends_with(&format!("-{status}")));
        assert_eq!(
            row.try_get::<serde_json::Value>("", "task_data").unwrap(),
            json!({"fixture": true})
        );
    }
    transaction.rollback().await.unwrap();
    fixture.close(&[]).await;
}

#[tokio::test]
async fn queue_upgrade_rejects_duplicate_ids_and_rolls_back() {
    use migration::{MigrationTrait, SchemaManager};
    let fixture = Fixture::new().await;
    let transaction = fixture.admin.begin().await.unwrap();
    let backend = transaction.get_database_backend();
    prepare_old_queue_schema(&transaction).await;
    let id = format!("duplicate-{}", uuid::Uuid::new_v4());
    for status in ["queued", "failed"] {
        let insert = Query::insert()
            .into_table("pg_loco_queue")
            .columns(["id", "name", "task_data", "status", "run_at"])
            .values_panic([
                id.as_str().into(),
                "MigrationFixture".into(),
                json!({"fixture": true}).into(),
                status.into(),
                chrono::Utc::now().into(),
            ])
            .to_owned();
        transaction
            .execute_raw(backend.build(&insert))
            .await
            .unwrap();
    }
    let savepoint = transaction.begin().await.unwrap();
    let result = migration::m20261106_000035_loco_queue::Migration
        .up(&SchemaManager::new(&savepoint))
        .await;
    assert!(
        result.is_err(),
        "duplicate job IDs must reject the migration"
    );
    savepoint.rollback().await.unwrap();
    assert!(
        !SchemaManager::new(&transaction)
            .has_column("pg_loco_queue", "attempts")
            .await
            .unwrap()
    );
    let select = Query::select()
        .column("id")
        .from("pg_loco_queue")
        .and_where(Expr::col("id").eq(id))
        .to_owned();
    assert_eq!(
        transaction
            .query_all_raw(backend.build(&select))
            .await
            .unwrap()
            .len(),
        2
    );
    transaction.rollback().await.unwrap();
    fixture.close(&[]).await;
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
