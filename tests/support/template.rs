use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};
use std::future::Future;

pub async fn prepare<F, Fut>(uri: &str, name: &str, seed: F)
where
    F: FnOnce(DatabaseConnection) -> Fut,
    Fut: Future<Output = ()>,
{
    static READY: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();
    assert_eq!(std::env::var("MEDTRACKER_OWNED_DATABASE").unwrap(), "1");
    assert!(
        name.len() <= 63
            && name
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
    );
    READY
        .get_or_init(|| async {
            let control = Database::connect(uri).await.unwrap();
            let existing = control
                .query_one_raw(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    "SELECT datallowconn FROM pg_database WHERE datname=$1",
                    [name.into()],
                ))
                .await
                .unwrap();
            if let Some(row) = existing {
                assert!(!row.try_get::<bool>("", "datallowconn").unwrap());
                control.close().await.unwrap();
                return;
            }
            control
                .execute_unprepared(&format!(
                    "CREATE DATABASE {name} TEMPLATE medtracker_reference"
                ))
                .await
                .unwrap();
            let mut template_uri = url::Url::parse(uri).unwrap();
            template_uri.set_path(name);
            let template = Database::connect(template_uri.to_string()).await.unwrap();
            <migration::Migrator as migration::MigratorTrait>::up(&template, None)
                .await
                .unwrap();
            seed(template.clone()).await;
            template.close().await.unwrap();
            control
                .execute_unprepared(&format!("ALTER DATABASE {name} ALLOW_CONNECTIONS false"))
                .await
                .unwrap();
            control.close().await.unwrap();
        })
        .await;
}
