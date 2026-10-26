use super::fixture::Fixture;
use super::template;
use sea_orm::{ConnectionTrait, Database, DbBackend, Statement};

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
    let control = Database::connect(template::owned_database_url())
        .await
        .unwrap();
    let row = control.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname=$1) AS database_exists, EXISTS(SELECT 1 FROM pg_roles WHERE rolname=$2) AS role_exists", [name.into(), role.into()])).await.unwrap().unwrap();
    assert!(!row.try_get::<bool>("", "database_exists").unwrap());
    assert!(!row.try_get::<bool>("", "role_exists").unwrap());
    control.close().await.unwrap();
}

#[tokio::test]
async fn cloning_an_aged_template_renews_short_lived_credentials() {
    Fixture::new().await.close().await;
    let uri = template::owned_database_url();
    let parsed = url::Url::parse(&uri).unwrap();
    let name = format!("aged_template_{}", uuid::Uuid::new_v4().simple());
    let control = Database::connect(&uri).await.unwrap();
    control
        .execute_unprepared(&format!(
            "CREATE DATABASE {name} TEMPLATE care_api_template"
        ))
        .await
        .unwrap();
    let mut source_uri = parsed;
    source_uri.set_path(&name);
    let source = Database::connect(source_uri.to_string()).await.unwrap();
    source
        .execute_unprepared("UPDATE oauth_grants SET expires_in=timezone('UTC',clock_timestamp())-interval '1 hour'")
        .await
        .unwrap();
    source.close().await.unwrap();
    let cloned = Fixture::from_template(&uri, &name).await;
    let row = cloned.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT expires_in > timezone('UTC',clock_timestamp()) + interval '4 minutes' AS fresh FROM oauth_grants WHERE id=76001"
    )).await.unwrap().unwrap();
    let fresh: bool = row.try_get("", "fresh").unwrap();
    cloned.close().await;
    control
        .execute_unprepared(&format!("DROP DATABASE {name}"))
        .await
        .unwrap();
    control.close().await.unwrap();
    assert!(
        fresh,
        "Each clone must receive a fresh short-lived OAuth code"
    );
}

#[tokio::test]
async fn cloned_fixture_issues_a_fresh_authorization_code() {
    let first = Fixture::new().await;
    let cloned_at = chrono::Utc::now().naive_utc();
    let second = Fixture::new().await;
    let row = second.admin.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT created_at >= $1 AS fresh, expires_in > $1 + interval '4 minutes' AS usable FROM oauth_grants WHERE id=76001",
        [cloned_at.into()],
    )).await.unwrap().unwrap();
    assert!(row.try_get::<bool>("", "fresh").unwrap());
    assert!(row.try_get::<bool>("", "usable").unwrap());
    first.close().await;
    second.close().await;
}
