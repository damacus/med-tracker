#[path = "support/db.rs"]
mod db;
#[path = "support/fixtures.rs"]
pub mod fixtures;
#[allow(dead_code)]
#[path = "support/template.rs"]
mod template;
use loco_rs::{config::Database, prelude::ConnectionTrait};
use migration::MigratorTrait;

#[tokio::test]
async fn fixture_slices_preserve_baseline_and_care_memberships() {
    use med_tracker::models::_entities::{accounts, household_memberships, medications, users};
    use sea_orm::{EntityTrait, PaginatorTrait};

    let db = restored_database().await;
    migration::Migrator::up(&db, None).await.unwrap();
    fixtures::seed_persistence(&db).await;
    let account = accounts::Entity::find_by_id(71001)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(account.status, 1);
    assert_eq!(
        household_memberships::Entity::find()
            .count(&db)
            .await
            .unwrap(),
        1
    );
    assert_eq!(users::Entity::find().count(&db).await.unwrap(), 0);
    fixtures::seed_care_doses(&db).await;
    assert_eq!(
        accounts::Entity::find_by_id(71001)
            .one(&db)
            .await
            .unwrap()
            .unwrap()
            .status,
        2
    );
    assert_eq!(
        household_memberships::Entity::find()
            .count(&db)
            .await
            .unwrap(),
        3
    );
    assert_eq!(users::Entity::find().count(&db).await.unwrap(), 1);
    assert_eq!(medications::Entity::find().count(&db).await.unwrap(), 1);
    db.close().await.unwrap();
}

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

#[derive(serde::Deserialize)]
struct CanonicalIdentityCatalogue {
    removed: Vec<String>,
    changed: serde_json::Map<String, serde_json::Value>,
}

fn apply_guardian_recovery_catalogue(
    expected: &mut serde_json::Map<String, serde_json::Value>,
) -> Result<(), serde_json::Error> {
    let recovery: CanonicalIdentityCatalogue =
        serde_json::from_str(include_str!("../migration/guardian-recovery-catalog.json"))?;
    for key in recovery.removed {
        assert!(
            expected.remove(&key).is_some(),
            "Expected removed guardian recovery key: {key}"
        );
    }
    expected.extend(recovery.changed);
    Ok(())
}

#[tokio::test]
async fn capture_guardian_recovery_catalog_when_requested() {
    let Some(output) = std::env::var_os("GUARDIAN_RECOVERY_CATALOG_CAPTURE") else {
        return;
    };
    let output = std::path::PathBuf::from(output);
    assert!(output.is_absolute() && output.is_dir());
    let db = restored_database().await;
    migration::Migrator::up(&db, None).await.unwrap();
    let actual = catalog(&db).await;
    let baseline: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(include_str!("../migration/catalog.json")).unwrap();
    let actual = actual.as_object().unwrap();
    let removed = baseline
        .keys()
        .filter(|key| !actual.contains_key(*key))
        .cloned()
        .collect::<Vec<_>>();
    let changed = actual
        .iter()
        .filter(|(key, value)| baseline.get(*key) != Some(*value))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<serde_json::Map<_, _>>();
    assert!(
        changed
            .keys()
            .any(|key| key.starts_with("function:med_tracker.guardian_recovery_command("))
    );
    let delta = serde_json::json!({"removed":removed,"changed":changed});
    std::fs::write(
        output.join("guardian-recovery-catalog.json"),
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
async fn fresh_database_is_created_by_loco_migrations() -> Result<(), serde_json::Error> {
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
    let mut expected: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(include_str!("../migration/catalog.json"))?;
    apply_guardian_recovery_catalogue(&mut expected)?;
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
    assert_eq!(ledger.len(), 16);
    assert_eq!(ledger[10].0, "m20261016_000011_account_avatars");
    assert_eq!(ledger[11].0, "m20261016_000012_support_expiry");
    assert_eq!(ledger[12].0, "m20261017_000013_guardian_declarations");
    assert_eq!(ledger[13].0, "m20261021_000014_member_directory_identity");
    assert_eq!(
        ledger.last().map(|row| row.0.as_str()),
        Some("m20261022_000017_guardian_recovery")
    );
    migration::Migrator::up(&db, None).await.unwrap();
    assert_eq!(catalog(&db).await, before);
    assert_eq!(ledger_rows(&db).await, ledger);
    db.close().await.unwrap();
    admin
        .execute_unprepared(&format!("DROP DATABASE {name}"))
        .await
        .unwrap();
    admin.close().await.unwrap();
    Ok(())
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
    fixtures::seed_persistence(&db).await;
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
        tokio::process::Command::new(env!("CARGO_BIN_EXE_med-tracker"))
            .args(["db", "seed", "--environment", "test"])
            .env("DATABASE_URL", format!("{base}/{database}"))
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
    let tenant = access::begin(
        &db,
        &HouseholdScope {
            actor: Actor {
                account_id: account.id,
            },
            household_id: 72001,
            request_id: "seed-proof".into(),
        },
        None,
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
    let seeded =
        db::OwnedDatabase::clone(&owned_url, &database, "seed_login_", "seed_runtime_").await;
    let mut config = Config::new(&Environment::Test).unwrap();
    config.database.uri = seeded.runtime_uri.clone();
    let Some(loco_rs::config::QueueConfig::Postgres(queue)) = config.queue.as_mut() else {
        panic!("PostgreSQL queue required")
    };
    queue.uri = seeded.runtime_uri.clone();
    let boot = med_tracker::app::App::boot(
        loco_rs::boot::StartMode::ServerOnly,
        &Environment::Test,
        config,
    )
    .await
    .unwrap();
    let mut server = loco_rs::TestServer::new(
        boot.router
            .unwrap()
            .into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .unwrap();
    server.save_cookies();
    let login = server
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&serde_json::json!({"email":"persistence@example.test","password":"password"}))
        .await;
    assert_eq!(login.status_code(), 200, "{}", login.text());
    assert_eq!(
        login.json::<serde_json::Value>()["user"]["id"]
            .to_string()
            .trim_matches('"'),
        "71001"
    );
    let session = server.get("/api/auth/get-session").await;
    assert_eq!(session.status_code(), 200, "{}", session.text());
    assert_eq!(
        session.json::<serde_json::Value>()["user"]["id"]
            .to_string()
            .trim_matches('"'),
        "71001"
    );
    let medicines = server
        .get("/households/persistence-fixture/medications")
        .await;
    assert_eq!(medicines.status_code(), 200, "{}", medicines.text());
    assert!(medicines.text().contains("Synthetic"));
    drop(server);
    if let Some(queue) = boot.app_context.queue_provider.as_ref() {
        queue.shutdown().unwrap();
    }
    boot.app_context.db.close().await.unwrap();
    seeded.close().await;
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
async fn fresh_audit_triggers_preserve_concurrent_chain_and_runtime_permissions() {
    use migration::sea_orm::{DbBackend, Statement};
    let db = restored_database().await;
    migration::Migrator::up(&db, None).await.unwrap();
    fixtures::seed_persistence(&db).await;
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
    let baseline: i64 = db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS entries FROM audit_ledger_entries",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "entries")
        .unwrap();
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
    let clinical = db.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM security_audit_events WHERE event_type='migration-proof') AS security,(SELECT count(*) FROM versions WHERE item_type='Medication' AND item_id=80001 AND event='update') AS medications")).await.unwrap().unwrap();
    assert_eq!(clinical.try_get::<i64>("", "security").unwrap(), 6);
    assert_eq!(clinical.try_get::<i64>("", "medications").unwrap(), 6);
    let row = db.query_one_raw(Statement::from_string(DbBackend::Postgres, r#"
        SELECT count(*)::bigint AS entries,
            bool_and(sequence = ordinal AND previous_hash IS NOT DISTINCT FROM previous_entry_hash) AS linked,
            bool_and(entry_hash = digest(convert_to('medtracker.audit.ledger.v1' || chr(31) || chain_key || chr(31) || chain_epoch::text || chr(31) || sequence::text || chr(31), 'UTF8') || COALESCE(previous_hash, '\x'::bytea) || canonical_payload, 'sha256')) AS hashes,
            bool_and(retain_until >= occurred_at + interval '9 years') AS retained
        FROM (SELECT *, row_number() OVER (PARTITION BY chain_key,chain_epoch ORDER BY sequence) AS ordinal, lag(entry_hash) OVER (PARTITION BY chain_key,chain_epoch ORDER BY sequence) AS previous_entry_hash FROM audit_ledger_entries) entries
    "#)).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "entries").unwrap(), baseline + 12);
    for column in ["linked", "hashes", "retained"] {
        assert!(
            row.try_get::<bool>("", column).unwrap(),
            "Audit check failed: {column}"
        );
    }
    let pending: i64 = db.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT count(*)::bigint AS pending FROM audit_export_deliveries WHERE status='pending' AND audit_ledger_entry_id IS NOT NULL")).await.unwrap().unwrap().try_get("", "pending").unwrap();
    assert_eq!(pending, baseline + 12);
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
    assert_eq!(retained, baseline + 12);
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

#[tokio::test]
async fn fresh_setup_refuses_an_unrecognised_schema_without_changing_data() {
    use med_tracker::models::_entities::accounts;
    use sea_orm::{
        EntityTrait,
        sea_query::{Alias, Expr, ExprTrait, Query},
    };

    let db = restored_database().await;
    migration::Migrator::up(&db, None).await.unwrap();
    fixtures::seed_persistence(&db).await;
    db.execute(
        Query::update()
            .table(Alias::new("seaql_migrations"))
            .value(Alias::new("version"), "retired_authentication_baseline")
            .and_where(
                Expr::col(Alias::new("version")).eq("m20261013_000001_create_medtracker_native"),
            ),
    )
    .await
    .unwrap();
    let before = catalog(&db).await;
    let ledger = ledger_rows(&db).await;
    let account = accounts::Entity::find_by_id(71001).one(&db).await.unwrap();
    assert!(account.is_some());
    assert!(migration::Migrator::up(&db, None).await.is_err());
    assert_eq!(catalog(&db).await, before);
    assert_eq!(ledger_rows(&db).await, ledger);
    assert_eq!(
        accounts::Entity::find_by_id(71001).one(&db).await.unwrap(),
        account
    );
    db.close().await.unwrap();
}

#[tokio::test]
async fn fresh_authentication_schema_never_creates_legacy_credentials() {
    let db = restored_database().await;
    migration::Migrator::up(&db, Some(1)).await.unwrap();
    let initial = catalog(&db).await;
    for table in [
        "account_identities",
        "account_otp_keys",
        "account_recovery_codes",
        "account_webauthn_keys",
        "account_webauthn_user_ids",
    ] {
        assert!(
            initial.get(format!("relation:public.{table}")).is_none(),
            "Obsolete credential table: {table}"
        );
    }
    assert!(
        initial
            .get("column:public.accounts.password_hash")
            .is_none()
    );
    assert!(initial.get("column:public.users.password_digest").is_none());
    assert!(
        initial
            .get("column:public.accounts.email_verified")
            .is_some()
    );
    migration::Migrator::up(&db, Some(6)).await.unwrap();
    let native = catalog(&db).await;
    for (table, column) in [
        ("auth_credentials", "password"),
        ("identity_sessions", "authentication_method"),
        ("identity_onboarding", "completed_at"),
        ("identity_api_keys", "reference_id"),
    ] {
        assert!(
            native
                .get(format!("column:public.{table}.{column}"))
                .is_some(),
            "Missing native authentication field: {table}.{column}"
        );
    }
    assert!(
        native
            .get("relation:public.identity_provider_accounts")
            .is_none()
    );
    assert!(
        native
            .get("relation:public.identity_device_codes")
            .is_none()
    );
    db.close().await.unwrap();
}
