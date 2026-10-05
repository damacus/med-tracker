use loco_rs::{config::Database, prelude::ConnectionTrait};

#[tokio::test]
async fn owned_postgres_fixture() {
    assert_eq!(std::env::var("MEDTRACKER_OWNED_DATABASE").unwrap(), "1");
    let uri =
        std::env::var("DATABASE_URL").expect("The slice runner must provide its owned database");
    let endpoint = uri
        .strip_prefix("postgres://medtracker:medtracker_password@127.0.0.1:")
        .expect("The database must use the owned loopback endpoint");
    let (port, database) = endpoint.split_once('/').unwrap();
    assert!(port.parse::<u16>().unwrap() > 0);
    assert_eq!(database, "medtracker_loco");
    let config = Database {
        uri,
        enable_logging: false,
        min_connections: 1,
        max_connections: 1,
        connect_timeout: 5000,
        idle_timeout: 5000,
        acquire_timeout: Some(5000),
        auto_migrate: false,
        dangerously_truncate: false,
        dangerously_recreate: false,
        run_on_start: None,
    };
    let db = loco_rs::db::connect(&config).await.unwrap();
    db.execute_unprepared(
        "DO $$ BEGIN
           IF current_setting('server_version_num')::int NOT BETWEEN 180000 AND 189999
              OR current_database() <> 'medtracker_loco' OR current_user <> 'medtracker'
           THEN RAISE EXCEPTION 'Unexpected persistence fixture'; END IF;
         END $$",
    )
    .await
    .unwrap();
    db.close().await.unwrap();
    assert_ne!(
        std::env::var("MEDTRACKER_PERSISTENCE_FORCE_FAILURE").as_deref(),
        Ok("1"),
        "Forced persistence fixture failure"
    );
}
