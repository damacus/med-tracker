use sea_orm::sqlx::{self, AssertSqlSafe, Connection, PgConnection, Row};
use sea_orm::{Database, DatabaseConnection};
use std::future::Future;

/// The empty `medtracker_reference` database is the base template for every
/// prepared template. Create it lazily so no external psql step is required.
/// The JS browser harness reuses this name for a seeded, frozen template; the
/// Rust convention keeps it empty — concurrent creators serialise on a
/// dedicated advisory lock so the check-then-create cannot race.
async fn ensure_reference(control: &mut PgConnection) {
    sqlx::query("SELECT pg_advisory_lock(hashtextextended('medtracker_reference', 0))")
        .execute(&mut *control)
        .await
        .unwrap();
    let exists =
        sqlx::query("SELECT 1 AS present FROM pg_database WHERE datname='medtracker_reference'")
            .fetch_optional(&mut *control)
            .await
            .unwrap()
            .is_some();
    if !exists {
        sqlx::query("CREATE DATABASE medtracker_reference")
            .execute(&mut *control)
            .await
            .unwrap();
    }
    sqlx::query("SELECT pg_advisory_unlock(hashtextextended('medtracker_reference', 0))")
        .execute(&mut *control)
        .await
        .unwrap();
}

pub async fn prepare<F, Fut>(uri: &str, name: &str, seed: F)
where
    F: FnOnce(DatabaseConnection) -> Fut,
    Fut: Future<Output = ()>,
{
    require_owned_database();
    assert!(is_owned_database_endpoint(&url::Url::parse(uri).unwrap()));
    assert!(
        name.len() <= 63
            && name
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
    );
    let mut control = PgConnection::connect(uri).await.unwrap();
    sqlx::query("SET lock_timeout = '60s'")
        .execute(&mut control)
        .await
        .unwrap();
    ensure_reference(&mut control).await;
    sqlx::query("SELECT pg_advisory_lock(hashtextextended($1, 0))")
        .bind(name)
        .execute(&mut control)
        .await
        .unwrap();
    let existing = sqlx::query("SELECT datallowconn FROM pg_database WHERE datname=$1")
        .bind(name)
        .fetch_optional(&mut control)
        .await
        .unwrap();
    if let Some(row) = existing {
        assert!(!row.try_get::<bool, _>("datallowconn").unwrap());
        control.close().await.unwrap();
        return;
    }
    sqlx::query(AssertSqlSafe(format!(
        "CREATE DATABASE {name} TEMPLATE medtracker_reference"
    )))
    .execute(&mut control)
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
    sqlx::query(AssertSqlSafe(format!(
        "ALTER DATABASE {name} ALLOW_CONNECTIONS false"
    )))
    .execute(&mut control)
    .await
    .unwrap();
    control.close().await.unwrap();
}

pub fn require_owned_database() {
    assert_eq!(std::env::var("MEDTRACKER_OWNED_DATABASE").unwrap(), "1");
}

pub fn owned_database_url() -> String {
    require_owned_database();
    let uri = std::env::var("DATABASE_URL").expect("the suite must provide its owned database");
    assert!(is_owned_database_endpoint(&url::Url::parse(&uri).unwrap()));
    uri
}

pub fn is_owned_database_endpoint(uri: &url::Url) -> bool {
    let host = match uri.host_str() {
        Some("127.0.0.1") => uri.port().is_some_and(|port| port != 0),
        Some("postgres") => uri.port().is_none_or(|port| port == 5432),
        _ => false,
    };
    host && uri.scheme() == "postgres"
        && uri.username() == "medtracker"
        && uri.password() == Some("medtracker_password")
        && uri.path() == "/medtracker_loco"
        && uri.query().is_none()
        && uri.fragment().is_none()
}

#[test]
fn database_fixture_endpoints_require_exact_synthetic_control_identity() {
    for uri in [
        "postgres://medtracker:medtracker_password@127.0.0.1:5432/medtracker_loco",
        "postgres://medtracker:medtracker_password@127.0.0.1:8123/medtracker_loco",
        "postgres://medtracker:medtracker_password@postgres/medtracker_loco",
        "postgres://medtracker:medtracker_password@postgres:5432/medtracker_loco",
    ] {
        assert!(is_owned_database_endpoint(&url::Url::parse(uri).unwrap()));
    }
    for uri in [
        "postgres://medtracker:medtracker_password@example.com/medtracker_loco",
        "postgres://medtracker:medtracker_password@postgres.example.com/medtracker_loco",
        "postgres://medtracker:medtracker_password@postgres:5433/medtracker_loco",
        "postgres://medtracker:wrong@postgres/medtracker_loco",
        "postgres://other:medtracker_password@postgres/medtracker_loco",
        "postgres://medtracker:medtracker_password@postgres/production",
        "postgres://medtracker:medtracker_password@postgres/medtracker_loco/extra",
        "postgres://medtracker:medtracker_password@postgres/medtracker_loco?host=remote",
        "postgres://medtracker:medtracker_password@postgres/medtracker_loco#remote",
        "http://medtracker:medtracker_password@postgres/medtracker_loco",
        "postgres://medtracker:medtracker_password@127.0.0.1/medtracker_loco",
        "postgres://medtracker:medtracker_password@127.0.0.1:0/medtracker_loco",
    ] {
        assert!(
            !is_owned_database_endpoint(&url::Url::parse(uri).unwrap()),
            "{uri}"
        );
    }
}

#[allow(dead_code)]
pub fn test_settings() -> Option<serde_json::Value> {
    loco_rs::config::Config::new(&loco_rs::environment::Environment::Test)
        .ok()
        .and_then(|config| config.settings)
}
