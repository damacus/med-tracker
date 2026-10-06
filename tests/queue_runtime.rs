use loco_rs::{
    bgworker::{Queue, pg},
    config::{Config, PostgresQueueConfig, QueueConfig},
    environment::Environment,
};
use migration::MigratorTrait;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};
use serde_json::json;

struct Fixture {
    admin: DatabaseConnection,
    runtime: DatabaseConnection,
    config: PostgresQueueConfig,
}

impl Fixture {
    async fn new() -> Self {
        assert_eq!(std::env::var("MEDTRACKER_OWNED_DATABASE").unwrap(), "1");
        let uri = std::env::var("DATABASE_URL").unwrap();
        let endpoint = uri
            .strip_prefix("postgres://medtracker:medtracker_password@127.0.0.1:")
            .unwrap();
        let (port, _) = endpoint.split_once('/').unwrap();
        assert!(port.parse::<u16>().unwrap() > 0);
        let suffix = uuid::Uuid::new_v4().simple();
        let name = format!("queue_{suffix}");
        let role = format!("queue_runtime_{suffix}");
        let control = Database::connect(&uri).await.unwrap();
        control
            .execute_unprepared(&format!(
                "CREATE DATABASE {name} TEMPLATE medtracker_reference"
            ))
            .await
            .unwrap();
        control.execute_unprepared(&format!("CREATE ROLE {role} LOGIN PASSWORD 'password' NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS IN ROLE med_tracker_app")).await.unwrap();
        control.close().await.unwrap();
        let admin = Database::connect(format!(
            "postgres://medtracker:medtracker_password@127.0.0.1:{port}/{name}"
        ))
        .await
        .unwrap();
        migration::Migrator::up(&admin, None).await.unwrap();
        let runtime_uri = format!("postgres://{role}:password@127.0.0.1:{port}/{name}");
        let runtime = Database::connect(&runtime_uri).await.unwrap();
        let config = serde_json::from_value(json!({
            "uri": runtime_uri,
            "min_connections": 1,
            "max_connections": 2,
            "connect_timeout": 5000,
            "num_workers": 1,
            "poll_interval_sec": 1
        }))
        .unwrap();
        Self {
            admin,
            runtime,
            config,
        }
    }

    async fn queue(&self) -> Queue {
        let queue = pg::create_provider(&self.config).await.unwrap();
        queue.setup().await.unwrap();
        queue
    }

    async fn status(&self, id: &str) -> String {
        self.runtime
            .query_one_raw(sea_orm::Statement::from_sql_and_values(
                sea_orm::DbBackend::Postgres,
                "SELECT status FROM public.pg_loco_queue WHERE id = $1",
                [id.into()],
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "status")
            .unwrap()
    }

    async fn close(self) {
        self.runtime.close().await.unwrap();
        self.admin.close().await.unwrap();
    }
}

#[tokio::test]
async fn queue_setup_and_reconnection_preserve_jobs_without_runtime_ddl() {
    let fixture = Fixture::new().await;
    assert!(
        fixture
            .runtime
            .execute_unprepared("CREATE TABLE public.forbidden_queue_table(id bigint)")
            .await
            .is_err()
    );
    let queue = fixture.queue().await;
    let id = queue
        .enqueue(
            "PersistedJob".into(),
            None,
            json!({"operation_key": "synthetic-persistence"}),
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
    fixture.close().await;
}

#[tokio::test]
async fn maintained_retry_and_requeue_preserve_payload_and_recover_only_eligible_jobs() {
    let fixture = Fixture::new().await;
    let queue = fixture.queue().await;
    let mut ids = Vec::new();
    for key in ["failed", "abandoned", "active"] {
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
    fixture.runtime.execute_unprepared("UPDATE public.pg_loco_queue SET status = CASE WHEN task_data->>'operation_key' = 'failed' THEN 'failed' ELSE 'processing' END, updated_at = CASE WHEN task_data->>'operation_key' = 'abandoned' THEN now()-interval '20 minutes' ELSE now() END").await.unwrap();
    assert_eq!(queue.retry_failed(Some(&ids[0])).await.unwrap(), 1);
    assert_eq!(queue.retry_failed(Some(&ids[0])).await.unwrap(), 0);
    queue.requeue(&10).await.unwrap();
    assert_eq!(fixture.status(&ids[0]).await, "queued");
    assert_eq!(fixture.status(&ids[1]).await, "queued");
    assert_eq!(fixture.status(&ids[2]).await, "processing");
    let payloads = fixture.runtime.query_all_raw(sea_orm::Statement::from_string(sea_orm::DbBackend::Postgres, "SELECT task_data->>'operation_key' AS operation_key FROM public.pg_loco_queue ORDER BY operation_key")).await.unwrap();
    assert_eq!(
        payloads
            .iter()
            .map(|row| row.try_get::<String>("", "operation_key").unwrap())
            .collect::<Vec<_>>(),
        ["abandoned", "active", "failed"]
    );
    queue.shutdown().unwrap();
    drop(queue);
    fixture.close().await;
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
            json!({"operation_key": "synthetic-abandoned"}),
            Some(vec!["unassigned".into()]),
            None,
        )
        .await
        .unwrap()
        .unwrap();
    fixture
        .runtime
        .execute_unprepared(
            "UPDATE public.pg_loco_queue SET status='processing', updated_at=now()-interval '20 minutes'",
        )
        .await
        .unwrap();
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
    fixture.close().await;
}
