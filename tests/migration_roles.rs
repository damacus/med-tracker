#[path = "support/scenario.rs"]
pub mod scenario;
use migration::{MigratorTrait, sea_orm::ConnectionTrait};
use sea_orm::sea_query::{Expr, ExprTrait, Query};

#[tokio::test]
async fn fresh_migration_rejects_inherited_privileges_without_changing_the_database() {
    let admin = scenario::setup_connection().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let bridge = format!("role_bridge_{suffix}");
    // SeaQuery has no PostgreSQL schema/role administration statements. This
    // exclusive migration test resets only the UUID/marker-verified run database;
    // production migration/reset guards remain unchanged.
    admin
        .execute_unprepared(
            "DROP SCHEMA public CASCADE; DROP SCHEMA med_tracker CASCADE; CREATE SCHEMA public AUTHORIZATION pg_database_owner; GRANT USAGE ON SCHEMA public TO PUBLIC",
        )
        .await
        .unwrap();
    admin
        .execute_unprepared(&format!(
            "CREATE ROLE {bridge} NOLOGIN; GRANT med_tracker_owner TO {bridge}; GRANT {bridge} TO med_tracker_app"
        ))
        .await
        .unwrap();
    let result = migration::Migrator::up(&admin, None).await;
    let query = Query::select()
        .column(("c", "oid"))
        .from_as(("pg_catalog", "pg_class"), "c")
        .join_as(
            sea_orm::sea_query::JoinType::InnerJoin,
            ("pg_catalog", "pg_namespace"),
            "n",
            Expr::col(("n", "oid")).equals(("c", "relnamespace")),
        )
        .and_where(Expr::col(("n", "nspname")).is_in(["public", "med_tracker"]))
        .to_owned();
    let objects = admin
        .query_all_raw(admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .len();
    admin
        .execute_unprepared(&format!(
            "REVOKE {bridge} FROM med_tracker_app; REVOKE med_tracker_owner FROM {bridge}; DROP ROLE {bridge}"
        ))
        .await
        .unwrap();
    migration::Migrator::up(&admin, None).await.unwrap();
    admin.close().await.unwrap();
    assert!(
        result.is_err(),
        "Inherited owner privileges must be rejected"
    );
    assert_eq!(objects, 0, "Rejected migrations must roll back all objects");
}
