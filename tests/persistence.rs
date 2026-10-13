#[allow(dead_code)]
#[path = "support/template.rs"]
mod template;
use loco_rs::{config::Database, prelude::ConnectionTrait};
use migration::MigratorTrait;

async fn catalog(db: &loco_rs::prelude::DatabaseConnection) -> serde_json::Value {
    db.execute_unprepared("SET search_path = pg_catalog, public, pg_temp")
        .await
        .unwrap();
    let row = db
        .query_one_raw(migration::sea_orm::Statement::from_string(
            migration::sea_orm::DbBackend::Postgres,
            include_str!("../migration/catalog.sql"),
        ))
        .await
        .unwrap()
        .unwrap();
    serde_json::from_str(&row.try_get::<String>("", "catalog").unwrap()).unwrap()
}

fn expected_catalog_without_review() -> serde_json::Map<String, serde_json::Value> {
    let mut expected = serde_json::Map::new();
    for source in [
        include_str!("../migration/catalog.json"),
        include_str!("../migration/ledger-catalog.json"),
        include_str!("../migration/queue-catalog.json"),
        include_str!("../migration/take-identity-catalog.json"),
        include_str!("../migration/browser-session-catalog.json"),
        include_str!("../migration/access-token-scopes-catalog.json"),
        include_str!("../migration/registration-policy-catalog.json"),
        include_str!("../migration/identity-catalog.json"),
        include_str!("../migration/support-access-catalog.json"),
        include_str!("../migration/notification-deliveries-catalog.json"),
    ] {
        expected.extend(
            serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(source).unwrap(),
        );
    }
    expected
}

#[tokio::test]
async fn capture_review_edit_catalog_when_requested() {
    let Some(output) = std::env::var_os("REVIEW_CATALOG_CAPTURE") else {
        return;
    };
    let output = std::path::PathBuf::from(output);
    assert!(output.is_absolute() && output.is_dir());
    let db = restored_database().await;
    migration::Migrator::up(&db, None).await.unwrap();
    let actual = catalog(&db).await;
    let baseline = expected_catalog_without_review();
    let delta = actual
        .as_object()
        .unwrap()
        .iter()
        .filter(|(key, value)| baseline.get(*key) != Some(*value))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<serde_json::Map<_, _>>();
    assert!(delta.contains_key("relation:public.medication_review_edit_leases"));
    assert!(delta.contains_key("relation:public.medication_review_histories"));
    assert!(
        delta.contains_key("column:public.medication_review_prompts.evidence_source_fingerprint")
    );
    std::fs::write(
        output.join("medtracker-review-edit-catalog.json"),
        serde_json::to_string_pretty(&delta).unwrap(),
    )
    .unwrap();
    db.close().await.unwrap();
}

async fn restored_database() -> loco_rs::prelude::DatabaseConnection {
    let uri = template::owned_database_url();
    let admin = migration::sea_orm::Database::connect(&uri).await.unwrap();
    let name = format!("persistence_{}", uuid::Uuid::new_v4().simple());
    admin
        .execute_unprepared(&format!("CREATE DATABASE {name} TEMPLATE template0"))
        .await
        .unwrap();
    admin.close().await.unwrap();
    let (base, _) = uri.rsplit_once('/').unwrap();
    let mut options = migration::sea_orm::ConnectOptions::new(format!("{base}/{name}"));
    options.max_connections(1);
    migration::sea_orm::Database::connect(options)
        .await
        .unwrap()
}

async fn ledger_rows(db: &loco_rs::prelude::DatabaseConnection) -> Vec<(String, i64)> {
    db.query_all_raw(migration::sea_orm::Statement::from_string(
        migration::sea_orm::DbBackend::Postgres,
        "SELECT version, applied_at FROM public.seaql_migrations ORDER BY version",
    ))
    .await
    .unwrap()
    .into_iter()
    .map(|row| {
        (
            row.try_get("", "version").unwrap(),
            row.try_get("", "applied_at").unwrap(),
        )
    })
    .collect()
}

#[tokio::test]
async fn fresh_database_is_created_by_loco_migrations() {
    let uri = template::owned_database_url();
    let admin = migration::sea_orm::Database::connect(&uri).await.unwrap();
    let name = format!("fresh_{}", uuid::Uuid::new_v4().simple());
    admin
        .execute_unprepared(&format!("CREATE DATABASE {name} TEMPLATE template0"))
        .await
        .unwrap();
    let (base, _) = uri.rsplit_once('/').unwrap();
    let db = migration::sea_orm::Database::connect(format!("{base}/{name}"))
        .await
        .unwrap();
    migration::Migrator::up(&db, None)
        .await
        .expect("Loco must build an empty database without a schema import");
    let before = catalog(&db).await;
    assert!(before.get("relation:public.accounts").is_some());
    assert!(before.get("relation:public.audit_ledger_entries").is_some());
    let mut expected = expected_catalog_without_review();
    expected.extend(
        serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(include_str!(
            "../migration/review-edit-catalog.json"
        ))
        .unwrap(),
    );
    let actual = before.as_object().unwrap();
    let differences: Vec<_> = expected
        .keys()
        .chain(actual.keys())
        .filter(|key| expected.get(*key) != actual.get(*key))
        .map(|key| {
            format!(
                "{key}: expected {:?}, got {:?}",
                expected.get(key),
                actual.get(key)
            )
        })
        .collect();
    assert!(
        differences.is_empty(),
        "Schema parity differences:\n{}",
        differences.join("\n")
    );
    let ledger = ledger_rows(&db).await;
    assert_eq!(ledger.len(), 10);
    assert_eq!(ledger[9].0, "m20261013_000010_review_edit");
    migration::Migrator::up(&db, None).await.unwrap();
    assert_eq!(catalog(&db).await, before);
    assert_eq!(ledger_rows(&db).await, ledger);
    db.close().await.unwrap();
    admin
        .execute_unprepared(&format!("DROP DATABASE {name}"))
        .await
        .unwrap();
    admin.close().await.unwrap();
}

#[tokio::test]
async fn migrations_provision_the_standard_durable_browser_session_store() {
    let db = restored_database().await;
    migration::Migrator::up(&db, None).await.unwrap();
    let exists = db
        .query_one_raw(migration::sea_orm::Statement::from_string(
            migration::sea_orm::DbBackend::Postgres,
            "SELECT to_regclass('public.browser_sessions') IS NOT NULL AS present",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<bool>("", "present")
        .unwrap();
    db.close().await.unwrap();
    assert!(exists, "Standard durable browser session store is absent");
}

#[tokio::test]
async fn medication_review_history_is_append_only_for_the_application_role() {
    let db = restored_database().await;
    migration::Migrator::up(&db, None).await.unwrap();
    let row = db
        .query_one_raw(migration::sea_orm::Statement::from_string(
            migration::sea_orm::DbBackend::Postgres,
            "SELECT has_table_privilege('med_tracker_app','public.medication_review_histories','SELECT') AS readable,has_table_privilege('med_tracker_app','public.medication_review_histories','INSERT') AS appendable,has_table_privilege('med_tracker_app','public.medication_review_histories','UPDATE') AS mutable,has_table_privilege('med_tracker_app','public.medication_review_histories','DELETE') AS deletable",
        ))
        .await
        .unwrap()
        .unwrap();
    assert!(row.try_get::<bool>("", "readable").unwrap());
    assert!(row.try_get::<bool>("", "appendable").unwrap());
    assert!(!row.try_get::<bool>("", "mutable").unwrap());
    assert!(!row.try_get::<bool>("", "deletable").unwrap());
    db.close().await.unwrap();
}

#[tokio::test]
async fn registration_policy_exposes_only_active_owner_existence() {
    let db = restored_database().await;
    migration::Migrator::up(&db, None).await.unwrap();
    db.execute_unprepared(include_str!("fixtures/persistence-records.sql"))
        .await
        .unwrap();
    for (change, expected) in [
        ("", false),
        (
            "UPDATE public.household_memberships SET role='owner' WHERE id=74001",
            true,
        ),
        (
            "UPDATE public.household_memberships SET status='revoked' WHERE id=74001",
            false,
        ),
    ] {
        db.execute_unprepared("RESET ROLE").await.unwrap();
        if !change.is_empty() {
            db.execute_unprepared(change).await.unwrap();
        }
        db.execute_unprepared("SET ROLE med_tracker_app")
            .await
            .unwrap();
        let result = db.query_one_raw(migration::sea_orm::Statement::from_string(
            migration::sea_orm::DbBackend::Postgres,
            "SELECT public.registration_has_active_owner() AS has_owner, (SELECT count(*) FROM public.household_memberships) AS visible_memberships, pg_has_role(current_user, 'med_tracker_owner', 'MEMBER') AS can_assume_owner_role",
        )).await.unwrap().unwrap();
        assert_eq!(result.try_get::<bool>("", "has_owner").unwrap(), expected);
        assert_eq!(result.try_get::<i64>("", "visible_memberships").unwrap(), 0);
        assert!(!result.try_get::<bool>("", "can_assume_owner_role").unwrap());
    }
    db.close().await.unwrap();
}

#[tokio::test]
async fn development_seed_creates_a_usable_account_and_household() {
    use loco_rs::{
        app::{AppContext, Hooks},
        config::Config,
        environment::Environment,
    };
    use med_tracker::models::{
        access::{self, Actor, HouseholdScope},
        entities::{account, medication},
    };
    use sea_orm::EntityTrait;
    let db = restored_database().await;
    let context = AppContext::builder(
        Environment::Development,
        db.clone(),
        Config::new(&Environment::Development).unwrap(),
    )
    .build();
    let database = db
        .query_one_raw(migration::sea_orm::Statement::from_string(
            migration::sea_orm::DbBackend::Postgres,
            "SELECT current_database() AS name",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<String>("", "name")
        .unwrap();
    let owned_url = template::owned_database_url();
    let (base, _) = owned_url.rsplit_once('/').unwrap();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        tokio::process::Command::new("task")
            .args([
                "db:prepare",
                &format!("SEED_DATABASE_URL={base}/{database}"),
                &format!(
                    "SEED_EXECUTABLE={}",
                    std::env::var("LOCO_TEST_APPLICATION")
                        .unwrap_or_else(|_| env!("CARGO_BIN_EXE_med-tracker").to_owned())
                ),
                "LOCO_ENV=test",
            ])
            .env(
                "DATABASE_URL",
                "postgres://invalid:invalid@127.0.0.1:1/do_not_use",
            )
            .kill_on_drop(true)
            .output(),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(
        result.status.success(),
        "Standard development seeding must be usable: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let account = account::Entity::find_by_id(71001)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(account.email, "persistence@example.test");
    assert!(bcrypt::verify("password", account.password_hash.as_deref().unwrap()).unwrap());
    let tenant = access::begin(
        &db,
        &HouseholdScope {
            actor: Actor {
                account_id: account.id,
            },
            household_id: 72001,
            request_id: "seed-proof".into(),
        },
    )
    .await
    .unwrap();
    let medicines = access::medication_scope(&tenant)
        .all(tenant.transaction())
        .await
        .unwrap();
    assert_eq!(medicines.len(), 1);
    assert_eq!(medicines[0].id, 80001);
    assert_eq!(medicines[0].current_supply.unwrap().to_string(), "10.00");
    tenant.rollback().await.unwrap();
    let production =
        AppContext::builder(Environment::Production, db.clone(), context.config.clone()).build();
    assert!(
        med_tracker::app::App::seed(&production, std::path::Path::new("src/fixtures"))
            .await
            .is_err()
    );
    assert_eq!(medication::Entity::find().all(&db).await.unwrap().len(), 1);
    assert!(
        med_tracker::app::App::seed(&context, std::path::Path::new("src/fixtures"))
            .await
            .is_err()
    );
    db.close().await.unwrap();
}

#[tokio::test]
async fn failed_fresh_migration_rolls_back_schema_and_ledger() {
    let db = restored_database().await;
    db.execute_unprepared(
        "CREATE TABLE accounts (sentinel integer); INSERT INTO accounts VALUES (42)",
    )
    .await
    .unwrap();
    let before = catalog(&db).await;
    assert!(migration::Migrator::up(&db, None).await.is_err());
    assert_eq!(catalog(&db).await, before);
    db.close().await.unwrap();
}

#[tokio::test]
async fn pending_migrations_keep_the_schema_owner() {
    let db = restored_database().await;
    migration::Migrator::up(&db, Some(1)).await.unwrap();
    migration::Migrator::up(&db, None).await.unwrap();
    let schema = catalog(&db).await;
    assert_eq!(
        schema["relation:public.pg_loco_queue"]["owner"],
        "med_tracker_owner"
    );
    db.close().await.unwrap();
}

#[tokio::test]
async fn loco_browser_seed_builds_an_empty_database() {
    use loco_rs::{
        app::{AppContext, Hooks},
        config::Config,
        environment::Environment,
    };
    let db = restored_database().await;
    let ctx = AppContext::builder(
        Environment::Test,
        db.clone(),
        Config::new(&Environment::Test).unwrap(),
    )
    .build();
    med_tracker::app::App::seed(&ctx, std::path::Path::new("tests/fixtures/browser"))
        .await
        .unwrap();
    let count: i64 = db
        .query_one_raw(migration::sea_orm::Statement::from_string(
            migration::sea_orm::DbBackend::Postgres,
            "SELECT count(*)::bigint AS count FROM accounts WHERE status=2",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "count")
        .unwrap();
    assert_eq!(count, 2);
    db.close().await.unwrap();
}

#[tokio::test]
async fn fresh_audit_triggers_preserve_concurrent_chain_and_runtime_permissions() {
    use migration::sea_orm::{DbBackend, Statement};
    let db = restored_database().await;
    migration::Migrator::up(&db, None).await.unwrap();
    db.execute_unprepared(include_str!("fixtures/persistence-records.sql"))
        .await
        .unwrap();
    let database: String = db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT current_database() AS name",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "name")
        .unwrap();
    let uri = template::owned_database_url();
    let (base, _) = uri.rsplit_once('/').unwrap();
    let mut writers = tokio::task::JoinSet::new();
    for _ in 0..6 {
        let uri = format!("{base}/{database}");
        writers.spawn(async move {
            let mut options = migration::sea_orm::ConnectOptions::new(uri);
            options.max_connections(1);
            let writer = migration::sea_orm::Database::connect(options).await.unwrap();
            writer.execute_unprepared("SET ROLE med_tracker_app; SET med_tracker.current_household_id = '72001'; INSERT INTO security_audit_events(household_id,event_type,created_at,updated_at) VALUES (72001,'migration-proof',now(),now()); INSERT INTO versions(household_id,item_type,item_id,event,created_at) VALUES (72001,'Medication',80001,'update',now())").await.unwrap();
            for statement in [
                "INSERT INTO audit_ledger_entries DEFAULT VALUES",
                "UPDATE audit_ledger_entries SET sequence=0",
                "DELETE FROM audit_ledger_entries",
                "UPDATE security_audit_events SET event_type='tampered'",
                "DELETE FROM versions",
                "UPDATE seaql_migrations SET applied_at=0",
            ] {
                assert!(writer.execute_unprepared(statement).await.is_err(), "Runtime accepted {statement}");
            }
            writer.close().await.unwrap();
        });
    }
    while let Some(result) = writers.join_next().await {
        result.unwrap();
    }
    let row = db.query_one_raw(Statement::from_string(DbBackend::Postgres, r#"
        SELECT count(*)::bigint AS entries,
            bool_and(sequence = ordinal AND previous_hash IS NOT DISTINCT FROM previous_entry_hash) AS linked,
            bool_and(entry_hash = digest(convert_to('medtracker.audit.ledger.v1' || chr(31) || chain_key || chr(31) || chain_epoch::text || chr(31) || sequence::text || chr(31), 'UTF8') || COALESCE(previous_hash, '\x'::bytea) || canonical_payload, 'sha256')) AS hashes,
            bool_and(retain_until >= occurred_at + interval '9 years') AS retained
        FROM (SELECT *, row_number() OVER (ORDER BY sequence) AS ordinal, lag(entry_hash) OVER (ORDER BY sequence) AS previous_entry_hash FROM audit_ledger_entries) entries
    "#)).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "entries").unwrap(), 12);
    for column in ["linked", "hashes", "retained"] {
        assert!(
            row.try_get::<bool>("", column).unwrap(),
            "Audit check failed: {column}"
        );
    }
    let pending: i64 = db.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT count(*)::bigint AS pending FROM audit_export_deliveries WHERE status='pending' AND audit_ledger_entry_id IS NOT NULL")).await.unwrap().unwrap().try_get("", "pending").unwrap();
    assert_eq!(pending, 12);
    db.execute_unprepared("DELETE FROM security_audit_events; DELETE FROM versions")
        .await
        .unwrap();
    let retained: i64 = db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*)::bigint AS retained FROM audit_ledger_entries",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "retained")
        .unwrap();
    assert_eq!(retained, 12);
    db.close().await.unwrap();
}

#[test]
fn development_has_a_stable_session_key() {
    use base64::Engine;
    use loco_rs::{config::Config, environment::Environment};
    let first = Config::new(&Environment::Development).unwrap();
    let second = Config::new(&Environment::Development).unwrap();
    let key = first
        .settings
        .as_ref()
        .and_then(|settings| settings["browser_session"]["key"].as_str())
        .expect("Development must provide a stable session key");
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(key)
            .unwrap()
            .len(),
        64
    );
    assert_eq!(
        second.settings.as_ref().unwrap()["browser_session"]["key"],
        key
    );
}

#[tokio::test]
async fn owned_postgres_fixture() {
    let uri = template::owned_database_url();
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
