use crate::{db::OwnedDatabase, fixtures, template};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};

pub struct Fixture {
    pub admin: DatabaseConnection,
    pub runtime: DatabaseConnection,
    pub runtime_uri: String,
    inner: OwnedDatabase,
}

impl Fixture {
    pub async fn new() -> Self {
        let uri = template::owned_database_url();
        template::prepare(&uri, "care_api_template", |admin| async move {
            fixtures::seed_core(&admin).await;
            fixtures::set_native_client_id(&admin).await;
        })
        .await;
        Self::from_template(&uri, "care_api_template").await
    }

    pub(super) async fn from_template(uri: &str, template_name: &str) -> Self {
        let owned = OwnedDatabase::clone(uri, template_name, "care_api_", "care_runtime_").await;
        fixtures::seed_mobile_grant(&owned.admin).await;
        Self {
            admin: owned.admin.clone(),
            runtime: owned.runtime.clone(),
            runtime_uri: owned.runtime_uri.clone(),
            inner: owned,
        }
    }

    pub async fn close(self) {
        self.inner.close().await;
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
