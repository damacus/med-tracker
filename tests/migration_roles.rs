#[allow(dead_code)]
#[path = "support/template.rs"]
mod template;
use migration::{MigratorTrait, sea_orm::ConnectionTrait};

#[tokio::test]
async fn fresh_migration_rejects_populated_unrecognised_schema_without_adoption() {
    let uri = template::owned_database_url();
    let admin = migration::sea_orm::Database::connect(&uri).await.unwrap();
    let name = format!("old_schema_{}", uuid::Uuid::new_v4().simple());
    let (base, _) = uri.rsplit_once('/').unwrap();
    admin
        .execute_unprepared(&format!("CREATE DATABASE {name} TEMPLATE template0"))
        .await
        .unwrap();
    let target = migration::sea_orm::Database::connect(format!("{base}/{name}"))
        .await
        .unwrap();
    target
        .execute_unprepared(
            "CREATE TABLE public.legacy_people (id bigint PRIMARY KEY, name text NOT NULL);
             INSERT INTO public.legacy_people VALUES (1, 'Retained person')",
        )
        .await
        .unwrap();
    let result = migration::Migrator::up(&target, None).await;
    let row = target
        .query_one_raw(migration::sea_orm::Statement::from_string(
            migration::sea_orm::DbBackend::Postgres,
            "SELECT name, to_regclass('public.seaql_migrations') IS NULL AS no_ledger,
                    to_regclass('public.accounts') IS NULL AS no_accounts
             FROM public.legacy_people WHERE id = 1",
        ))
        .await
        .unwrap()
        .unwrap();
    let retained: String = row.try_get("", "name").unwrap();
    let no_ledger: bool = row.try_get("", "no_ledger").unwrap();
    let no_accounts: bool = row.try_get("", "no_accounts").unwrap();
    target.close().await.unwrap();
    admin
        .execute_unprepared(&format!("DROP DATABASE {name}"))
        .await
        .unwrap();
    admin.close().await.unwrap();
    assert!(
        result.is_err(),
        "Fresh migrations must not adopt populated schemas"
    );
    assert_eq!(retained, "Retained person");
    assert!(no_ledger, "Rejection must not create a migration ledger");
    assert!(no_accounts, "Rejection must not create application tables");
}

#[tokio::test]
async fn fresh_migration_rejects_inherited_privileges_without_changing_the_database() {
    let uri = template::owned_database_url();
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
