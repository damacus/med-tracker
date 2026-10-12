#[path = "../support/template.rs"]
mod template;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};

#[tokio::test]
async fn prepared_template_is_frozen_and_clones_are_isolated() {
    let first = Fixture::new().await;
    first
        .admin
        .execute_unprepared("UPDATE medications SET current_supply=123 WHERE id=80001")
        .await
        .unwrap();
    let second = Fixture::new().await;
    assert_ne!(first.effect().await.1, second.effect().await.1);
    let row = second
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT datallowconn FROM pg_database WHERE datname='care_api_template'",
        ))
        .await
        .unwrap()
        .expect("prepared template must exist");
    assert!(!row.try_get::<bool>("", "datallowconn").unwrap());
    first.close().await;
    second.close().await;
}

#[tokio::test]
async fn closing_fixture_removes_its_database_and_role() {
    let fixture = Fixture::new().await;
    let uri = url::Url::parse(&fixture.runtime_uri).unwrap();
    let name = uri.path().trim_start_matches('/');
    let role = uri.username();
    fixture.close().await;
    let control = Database::connect(std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();
    let row = control.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname=$1) AS database_exists, EXISTS(SELECT 1 FROM pg_roles WHERE rolname=$2) AS role_exists", [name.into(), role.into()])).await.unwrap().unwrap();
    assert!(!row.try_get::<bool>("", "database_exists").unwrap());
    assert!(!row.try_get::<bool>("", "role_exists").unwrap());
    control.close().await.unwrap();
}
pub struct Fixture {
    pub admin: DatabaseConnection,
    pub runtime: DatabaseConnection,
    pub runtime_uri: String,
}

impl Fixture {
    pub async fn new() -> Self {
        assert_eq!(std::env::var("MEDTRACKER_OWNED_DATABASE").unwrap(), "1");
        let uri = std::env::var("DATABASE_URL").unwrap();
        let endpoint = uri
            .strip_prefix("postgres://medtracker:medtracker_password@127.0.0.1:")
            .unwrap();
        let (port, _) = endpoint.split_once('/').unwrap();
        assert!(port.parse::<u16>().unwrap() > 0);
        template::prepare(&uri, "care_api_template", |admin| async move {
            admin
                .execute_unprepared(include_str!("../fixtures/persistence-records.sql"))
                .await
                .unwrap();
            admin
                .execute_unprepared(include_str!("../fixtures/care-doses.sql"))
                .await
                .unwrap();
            admin
                .execute_unprepared(
                    "UPDATE oauth_applications SET client_id='native' WHERE id=75001",
                )
                .await
                .unwrap();
            admin
                .execute_unprepared(include_str!("../fixtures/identity/mobile.sql"))
                .await
                .unwrap();
        })
        .await;
        let suffix = uuid::Uuid::new_v4().simple();
        let name = format!("care_api_{suffix}");
        let role = format!("care_runtime_{suffix}");
        let control = Database::connect(&uri).await.unwrap();
        control
            .execute_unprepared(&format!(
                "CREATE DATABASE {name} TEMPLATE care_api_template"
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
        let runtime_uri = format!("postgres://{role}:password@127.0.0.1:{port}/{name}");
        let runtime = Database::connect(&runtime_uri).await.unwrap();
        Self {
            admin,
            runtime,
            runtime_uri,
        }
    }

    pub async fn close(self) {
        let uri = url::Url::parse(&self.runtime_uri).unwrap();
        let name = uri.path().trim_start_matches('/');
        let role = uri.username();
        assert!(
            name.strip_prefix("care_api_")
                .is_some_and(|suffix| uuid::Uuid::parse_str(suffix).is_ok())
        );
        assert!(
            role.strip_prefix("care_runtime_")
                .is_some_and(|suffix| uuid::Uuid::parse_str(suffix).is_ok())
        );
        self.runtime.close().await.unwrap();
        self.admin.close().await.unwrap();
        let control = Database::connect(std::env::var("DATABASE_URL").unwrap())
            .await
            .unwrap();
        control
            .execute_unprepared(&format!("DROP DATABASE {name} WITH (FORCE)"))
            .await
            .unwrap();
        control
            .execute_unprepared(&format!("DROP ROLE {role}"))
            .await
            .unwrap();
        control.close().await.unwrap();
    }

    pub async fn effect(&self) -> (i64, String, i64, i64) {
        let row = self.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM medication_takes) AS takes,
                    (SELECT current_supply::text FROM medications WHERE id = 80001) AS supply,
                    (SELECT count(*) FROM versions WHERE item_type = 'MedicationTake' AND event = 'create') AS audits,
                    (SELECT count(*) FROM api_change_events WHERE record_type = 'MedicationTake' AND action = 'create') AS changes"
        )).await.unwrap().unwrap();
        (
            row.try_get("", "takes").unwrap(),
            row.try_get("", "supply").unwrap(),
            row.try_get("", "audits").unwrap(),
            row.try_get("", "changes").unwrap(),
        )
    }
}
