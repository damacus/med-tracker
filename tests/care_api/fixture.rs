use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};
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
        let suffix = uuid::Uuid::new_v4().simple();
        let name = format!("care_api_{suffix}");
        let role = format!("care_runtime_{suffix}");
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
        <migration::Migrator as migration::MigratorTrait>::up(&admin, None)
            .await
            .unwrap();
        admin
            .execute_unprepared(include_str!("../fixtures/persistence-records.sql"))
            .await
            .unwrap();
        admin
            .execute_unprepared(include_str!("../fixtures/care-doses.sql"))
            .await
            .unwrap();
        admin
            .execute_unprepared("UPDATE oauth_applications SET client_id='native' WHERE id=75001")
            .await
            .unwrap();
        admin
            .execute_unprepared(include_str!("../fixtures/identity/mobile.sql"))
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
        self.runtime.close().await.unwrap();
        self.admin.close().await.unwrap();
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
