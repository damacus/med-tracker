use migration::{MigratorTrait, sea_orm::ConnectionTrait};

#[tokio::test]
async fn fresh_migration_rejects_inherited_privileges_without_changing_the_database() {
    assert_eq!(std::env::var("MEDTRACKER_OWNED_DATABASE").unwrap(), "1");
    let uri = std::env::var("DATABASE_URL").unwrap();
    let admin = migration::sea_orm::Database::connect(&uri).await.unwrap();
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let setup_name = format!("role_setup_{suffix}");
    let target_name = format!("role_target_{suffix}");
    let bridge = format!("role_bridge_{suffix}");
    let (base, _) = uri.rsplit_once('/').unwrap();
    for name in [&setup_name, &target_name] {
        admin
            .execute_unprepared(&format!("CREATE DATABASE {name} TEMPLATE template0"))
            .await
            .unwrap();
    }
    let setup = migration::sea_orm::Database::connect(format!("{base}/{setup_name}"))
        .await
        .unwrap();
    migration::Migrator::up(&setup, None).await.unwrap();
    setup.close().await.unwrap();
    admin
        .execute_unprepared(&format!(
            "CREATE ROLE {bridge} NOLOGIN; GRANT med_tracker_owner TO {bridge}; GRANT {bridge} TO med_tracker_app"
        ))
        .await
        .unwrap();
    let target = migration::sea_orm::Database::connect(format!("{base}/{target_name}"))
        .await
        .unwrap();
    let result = migration::Migrator::up(&target, None).await;
    let row = target
        .query_one_raw(migration::sea_orm::Statement::from_string(
            migration::sea_orm::DbBackend::Postgres,
            "SELECT count(*) AS objects FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace WHERE n.nspname IN ('public', 'med_tracker')",
        ))
        .await
        .unwrap()
        .unwrap();
    let objects: i64 = row.try_get("", "objects").unwrap();
    target.close().await.unwrap();
    admin
        .execute_unprepared(&format!(
            "REVOKE {bridge} FROM med_tracker_app; REVOKE med_tracker_owner FROM {bridge}; DROP ROLE {bridge}"
        ))
        .await
        .unwrap();
    for name in [&setup_name, &target_name] {
        admin
            .execute_unprepared(&format!("DROP DATABASE {name}"))
            .await
            .unwrap();
    }
    admin.close().await.unwrap();
    assert!(
        result.is_err(),
        "Inherited owner privileges must be rejected"
    );
    assert_eq!(objects, 0, "Rejected migrations must roll back all objects");
}
