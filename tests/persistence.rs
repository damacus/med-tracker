use loco_rs::{config::Database, prelude::ConnectionTrait};
use migration::MigratorTrait;
use migration::sea_orm::TransactionTrait;

struct StandardLedger;

#[async_trait::async_trait]
impl MigratorTrait for StandardLedger {
    fn migrations() -> Vec<Box<dyn migration::MigrationTrait>> {
        vec![
            Box::new(migration::m20261005_000002_provision_runtime::Migration),
            Box::new(migration::m20261006_000003_canonical_take_identity::Migration),
            Box::new(migration::m20261006_000004_browser_sessions::Migration),
            Box::new(migration::m20261006_000005_access_token_scopes::Migration),
            Box::new(migration::m20261006_000006_registration_policy::Migration),
        ]
    }
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

fn sorted_catalog_keys(catalog: &serde_json::Map<String, serde_json::Value>) -> Vec<&str> {
    let mut keys = catalog.keys().map(String::as_str).collect::<Vec<_>>();
    keys.sort_unstable();
    keys
}

#[tokio::test]
#[ignore = "Explicit reference capture through slice:catalog only"]
async fn capture_persistence_catalog() {
    let db = restored_database().await;
    let before = catalog(&db).await;
    db.execute_unprepared("SET ROLE med_tracker_owner; SET search_path = public, pg_temp")
        .await
        .unwrap();
    StandardLedger::install(&db).await.unwrap();
    db.execute_unprepared("REVOKE ALL ON public.seaql_migrations FROM PUBLIC, med_tracker_app, med_tracker_audit_exporter, med_tracker_audit_verifier").await.unwrap();
    let after = catalog(&db).await;
    let delta: serde_json::Map<String, serde_json::Value> = after
        .as_object()
        .unwrap()
        .iter()
        .filter(|(key, value)| before.get(*key) != Some(*value))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    assert_eq!(
        sorted_catalog_keys(&delta),
        vec![
            "column:public.seaql_migrations.applied_at",
            "column:public.seaql_migrations.version",
            "constraint:public.seaql_migrations.seaql_migrations_applied_at_not_null",
            "constraint:public.seaql_migrations.seaql_migrations_pkey",
            "constraint:public.seaql_migrations.seaql_migrations_version_not_null",
            "index:public.seaql_migrations_pkey",
            "relation:public.seaql_migrations",
        ]
    );
    let output = std::env::var_os("PERSISTENCE_CATALOG_OUTPUT_DIR")
        .filter(|path| !path.is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    assert!(output.is_absolute() && output.is_dir());
    std::fs::write(
        output.join("medtracker-reference-catalog.json"),
        serde_json::to_string_pretty(&before).unwrap(),
    )
    .unwrap();
    std::fs::write(
        output.join("medtracker-ledger-catalog.json"),
        serde_json::to_string_pretty(&delta).unwrap(),
    )
    .unwrap();
    db.execute_unprepared("SET search_path = public, pg_temp")
        .await
        .unwrap();
    StandardLedger::up(&db, Some(1)).await.unwrap();
    let queued = catalog(&db).await;
    let queue: serde_json::Map<String, serde_json::Value> = queued
        .as_object()
        .unwrap()
        .iter()
        .filter(|(key, value)| after.get(*key) != Some(*value))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    let expected = [
        "column:public.pg_loco_queue.created_at",
        "column:public.pg_loco_queue.id",
        "column:public.pg_loco_queue.interval",
        "column:public.pg_loco_queue.name",
        "column:public.pg_loco_queue.priority",
        "column:public.pg_loco_queue.run_at",
        "column:public.pg_loco_queue.status",
        "column:public.pg_loco_queue.tags",
        "column:public.pg_loco_queue.task_data",
        "column:public.pg_loco_queue.updated_at",
        "constraint:public.pg_loco_queue.pg_loco_queue_created_at_not_null",
        "constraint:public.pg_loco_queue.pg_loco_queue_id_not_null",
        "constraint:public.pg_loco_queue.pg_loco_queue_name_not_null",
        "constraint:public.pg_loco_queue.pg_loco_queue_priority_not_null",
        "constraint:public.pg_loco_queue.pg_loco_queue_run_at_not_null",
        "constraint:public.pg_loco_queue.pg_loco_queue_status_not_null",
        "constraint:public.pg_loco_queue.pg_loco_queue_task_data_not_null",
        "constraint:public.pg_loco_queue.pg_loco_queue_updated_at_not_null",
        "relation:public.pg_loco_queue",
    ];
    assert_eq!(sorted_catalog_keys(&queue), expected);
    std::fs::write(
        output.join("medtracker-queue-catalog.json"),
        serde_json::to_string_pretty(&queue).unwrap(),
    )
    .unwrap();
    let mut previous = queued;
    for (name, expected_keys) in [
        (
            "take-identity",
            vec!["index:public.index_medication_takes_on_client_uuid_canonical"],
        ),
        (
            "browser-session",
            vec![
                "column:public.browser_sessions.expires",
                "column:public.browser_sessions.id",
                "column:public.browser_sessions.session",
                "constraint:public.browser_sessions.browser_sessions_id_not_null",
                "constraint:public.browser_sessions.browser_sessions_pkey",
                "constraint:public.browser_sessions.browser_sessions_session_not_null",
                "index:public.browser_sessions_pkey",
                "relation:public.browser_sessions",
            ],
        ),
        (
            "access-token-scopes",
            vec![
                "column:public.oauth_grants.access_token_scope_hash",
                "column:public.oauth_grants.access_token_scopes",
            ],
        ),
        (
            "registration-policy",
            vec![
                "function:public.registration_has_active_owner()",
                "policy:public.household_memberships.registration_active_owner_lookup",
            ],
        ),
    ] {
        db.execute_unprepared("SET search_path = public, pg_temp")
            .await
            .unwrap();
        StandardLedger::up(&db, Some(1)).await.unwrap();
        let current = catalog(&db).await;
        let delta: serde_json::Map<String, serde_json::Value> = current
            .as_object()
            .unwrap()
            .iter()
            .filter(|(key, value)| previous.get(*key) != Some(*value))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        assert_eq!(sorted_catalog_keys(&delta), expected_keys);
        std::fs::write(
            output.join(format!("medtracker-{name}-catalog.json")),
            serde_json::to_string_pretty(&delta).unwrap(),
        )
        .unwrap();
        previous = current;
    }
    db.close().await.unwrap();
}

async fn restored_database() -> loco_rs::prelude::DatabaseConnection {
    assert_eq!(std::env::var("MEDTRACKER_OWNED_DATABASE").unwrap(), "1");
    let uri = std::env::var("DATABASE_URL").unwrap();
    let admin = migration::sea_orm::Database::connect(&uri).await.unwrap();
    let name = format!("persistence_{}", uuid::Uuid::new_v4().simple());
    admin
        .execute_unprepared(&format!(
            "CREATE DATABASE {name} TEMPLATE medtracker_reference"
        ))
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

#[tokio::test]
async fn historical_global_uuid_duplicates_reject_adoption_without_mutation() {
    let db = restored_database().await;
    migration::Migrator::up(&db, Some(2)).await.unwrap();
    db.execute_unprepared(include_str!("fixtures/persistence-records.sql"))
        .await
        .unwrap();
    db.execute_unprepared(include_str!("fixtures/care-doses.sql"))
        .await
        .unwrap();
    db.execute_unprepared("INSERT INTO medication_takes(household_id,client_uuid,person_medication_id,dose_amount,dose_unit,taken_at,created_at,updated_at) VALUES(72001,'006b49d9-1da2-42f1-800b-8c80867aee1c',81001,2,'tablet',now(),now(),now()); INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(92001,71001,'Other synthetic household','uuid-migration-household','UTC',now(),now()); INSERT INTO people(id,household_id,name,person_type,has_capacity,created_at,updated_at) VALUES(92002,92001,'Other synthetic adult',0,true,now(),now()); INSERT INTO locations(id,household_id,name,created_at,updated_at) VALUES(92003,92001,'Other synthetic cabinet',now(),now()); INSERT INTO medications(id,household_id,location_id,name,current_supply,dose_amount,dose_unit,created_at,updated_at) VALUES(92004,92001,92003,'Other synthetic tablets',10,2,'tablet',now(),now()); INSERT INTO person_medications(id,household_id,person_id,medication_id,dose_amount,dose_unit,position,created_at,updated_at) VALUES(92005,92001,92002,92004,2,'tablet',0,now(),now()); INSERT INTO medication_takes(household_id,client_uuid,person_medication_id,dose_amount,dose_unit,taken_at,created_at,updated_at) VALUES(92001,'006B49D9-1DA2-42F1-800B-8C80867AEE1C',92005,2,'tablet',now(),now(),now())").await.unwrap();
    let before = catalog(&db).await;
    let ledger = ledger_rows(&db).await;
    let records = db
        .query_one_raw(migration::sea_orm::Statement::from_string(
            migration::sea_orm::DbBackend::Postgres,
            "SELECT jsonb_agg(to_jsonb(t) ORDER BY id)::text AS data FROM medication_takes t",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<String>("", "data")
        .unwrap();
    let result = migration::Migrator::up(&db, None).await;
    let after = db
        .query_one_raw(migration::sea_orm::Statement::from_string(
            migration::sea_orm::DbBackend::Postgres,
            "SELECT jsonb_agg(to_jsonb(t) ORDER BY id)::text AS data FROM medication_takes t",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<String>("", "data")
        .unwrap();
    assert_eq!(records, after);
    assert_eq!(catalog(&db).await, before);
    assert_eq!(ledger_rows(&db).await, ledger);
    db.close().await.unwrap();
    assert!(
        result.is_err(),
        "Global historical UUID duplicates were accepted"
    );
}

#[tokio::test]
async fn adoption_provisions_the_standard_durable_browser_session_store() {
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
async fn unknown_catalog_drift_is_rejected_before_ledger_installation() {
    let db = restored_database().await;
    db.execute_unprepared("CREATE INDEX unexpected_accounts_email ON public.accounts (email)")
        .await
        .unwrap();
    let result = migration::Migrator::up(&db, None).await;
    let ledger = db
        .query_one_raw(migration::sea_orm::Statement::from_string(
            migration::sea_orm::DbBackend::Postgres,
            "SELECT to_regclass('public.seaql_migrations') IS NULL AS absent",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<bool>("", "absent")
        .unwrap();
    db.close().await.unwrap();
    assert!(result.is_err(), "Unknown catalog drift was accepted");
    assert!(ledger, "Rejected adoption installed a migration ledger");
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("unexpected_accounts_email")
    );
}

#[tokio::test]
async fn adoption_records_the_supported_standard_migration() {
    let db = restored_database().await;
    migration::Migrator::up(&db, Some(1)).await.unwrap();
    let rows = db
        .query_all_raw(migration::sea_orm::Statement::from_string(
            migration::sea_orm::DbBackend::Postgres,
            "SELECT version, applied_at FROM public.seaql_migrations",
        ))
        .await
        .unwrap();
    db.close().await.unwrap();
    assert_eq!(
        rows.len(),
        1,
        "Adoption must record its supported migration"
    );
    assert_eq!(
        rows[0].try_get::<String>("", "version").unwrap(),
        "m20261005_000001_adopt_medtracker"
    );
    assert!(rows[0].try_get::<i64>("", "applied_at").unwrap() > 0);
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

async fn ledger_absent(db: &loco_rs::prelude::DatabaseConnection) -> bool {
    db.query_one_raw(migration::sea_orm::Statement::from_string(
        migration::sea_orm::DbBackend::Postgres,
        "SELECT to_regclass('public.seaql_migrations') IS NULL AS absent",
    ))
    .await
    .unwrap()
    .unwrap()
    .try_get("", "absent")
    .unwrap()
}

#[tokio::test]
async fn populated_adoption_preserves_records_and_rails_metadata() {
    let db = restored_database().await;
    db.execute_unprepared(include_str!("fixtures/persistence-records.sql"))
        .await
        .unwrap();
    let query = "SELECT jsonb_build_object(
        'accounts', (SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM public.accounts t),
        'households', (SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM public.households t),
        'people', (SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM public.people t),
        'memberships', (SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM public.household_memberships t),
        'clients', (SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM public.oauth_applications t),
        'grants', (SELECT jsonb_agg(to_jsonb(t) - 'access_token_scopes' - 'access_token_scope_hash' ORDER BY id) FROM public.oauth_grants t),
        'rails_migrations', (SELECT jsonb_agg(to_jsonb(t) ORDER BY version) FROM public.schema_migrations t),
        'rails_metadata', (SELECT jsonb_agg(to_jsonb(t) ORDER BY key) FROM public.ar_internal_metadata t)
    )::text AS records";
    let before: String = db
        .query_one_raw(migration::sea_orm::Statement::from_string(
            migration::sea_orm::DbBackend::Postgres,
            query,
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "records")
        .unwrap();
    let records: serde_json::Value = serde_json::from_str(&before).unwrap();
    assert_eq!(records["rails_migrations"].as_array().unwrap().len(), 177);
    assert!(!records["rails_metadata"].as_array().unwrap().is_empty());
    migration::Migrator::up(&db, None).await.unwrap();
    let adopted = ledger_rows(&db).await;
    assert_eq!(adopted.len(), 6);
    assert_eq!(adopted[5].0, "m20261006_000006_registration_policy");
    let after: String = db
        .query_one_raw(migration::sea_orm::Statement::from_string(
            migration::sea_orm::DbBackend::Postgres,
            query,
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "records")
        .unwrap();
    assert!(
        before == after,
        "Adoption changed synthetic records or Rails metadata"
    );
    let added_fields_are_null: bool = db.query_one_raw(migration::sea_orm::Statement::from_string(migration::sea_orm::DbBackend::Postgres,
        "SELECT NOT EXISTS (SELECT 1 FROM public.oauth_grants WHERE access_token_scopes IS NOT NULL OR access_token_scope_hash IS NOT NULL) AS unchanged"
    )).await.unwrap().unwrap().try_get("", "unchanged").unwrap();
    assert!(
        added_fields_are_null,
        "Adoption populated legacy grant access-scope metadata"
    );
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
    let owned_url = std::env::var("DATABASE_URL").unwrap();
    let (base, _) = owned_url.rsplit_once('/').unwrap();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        tokio::process::Command::new("task")
            .args([
                "db:seed",
                &format!("SEED_DATABASE_URL={base}/{database}"),
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
async fn disabled_foreign_key_triggers_are_rejected() {
    let db = restored_database().await;
    db.execute_unprepared("ALTER TABLE public.people DISABLE TRIGGER ALL")
        .await
        .unwrap();
    let result = migration::Migrator::up(&db, None).await;
    let absent = ledger_absent(&db).await;
    db.close().await.unwrap();
    assert!(
        result.is_err(),
        "Disabled foreign-key enforcement was accepted"
    );
    assert!(absent);
}

#[tokio::test]
async fn repeat_adoption_preserves_catalog_and_timestamp() {
    let db = restored_database().await;
    migration::Migrator::up(&db, None).await.unwrap();
    let before = catalog(&db).await;
    let ledger = ledger_rows(&db).await;
    migration::Migrator::up(&db, None).await.unwrap();
    assert_eq!(catalog(&db).await, before);
    assert_eq!(ledger_rows(&db).await, ledger);
    db.close().await.unwrap();
}

#[tokio::test]
async fn catalog_drift_is_rejected_before_and_after_adoption() {
    let cases = [
        (
            "CREATE INDEX unexpected_accounts_email ON public.accounts(email)",
            "index:public.unexpected_accounts_email",
        ),
        (
            "CREATE OR REPLACE FUNCTION med_tracker.current_account_id() RETURNS bigint LANGUAGE sql AS 'SELECT 999::bigint'",
            "function:med_tracker.current_account_id()",
        ),
        (
            "ALTER TABLE public.people DISABLE ROW LEVEL SECURITY",
            "relation:public.people",
        ),
        (
            "GRANT UPDATE ON public.accounts TO med_tracker_audit_exporter",
            "relation:public.accounts",
        ),
        (
            "ALTER TABLE public.accounts ALTER COLUMN status SET DEFAULT 9",
            "column:public.accounts.status",
        ),
        (
            "CREATE SCHEMA unexpected_schema",
            "schema:unexpected_schema",
        ),
    ];
    for adopted in [false, true] {
        for (change, identity) in cases {
            let db = restored_database().await;
            if adopted {
                migration::Migrator::up(&db, None).await.unwrap();
            }
            let ledger = if adopted {
                ledger_rows(&db).await
            } else {
                Vec::new()
            };
            db.execute_unprepared(change).await.unwrap();
            let before = catalog(&db).await;
            let error = migration::Migrator::up(&db, None).await.unwrap_err();
            assert!(error.to_string().contains(identity), "{error}");
            assert_eq!(catalog(&db).await, before);
            if adopted {
                assert_eq!(ledger_rows(&db).await, ledger);
            } else {
                assert!(ledger_absent(&db).await);
            }
            db.close().await.unwrap();
        }
    }
}

#[tokio::test]
async fn postflight_failure_rolls_back_standard_ledger_installation() {
    let db = restored_database().await;
    let before = catalog(&db).await;
    assert!(migration::Migrator::up(&db, Some(0)).await.is_err());
    assert!(ledger_absent(&db).await);
    assert_eq!(catalog(&db).await, before);
    db.close().await.unwrap();
}

#[tokio::test]
async fn unsupported_ledgers_are_rejected_without_mutation() {
    for mutation in [
        "DELETE FROM public.seaql_migrations",
        "UPDATE public.seaql_migrations SET version = 'unknown_migration' WHERE version = 'm20261005_000001_adopt_medtracker'",
        "DELETE FROM public.seaql_migrations WHERE version = 'm20261005_000001_adopt_medtracker'",
        "INSERT INTO public.seaql_migrations(version, applied_at) VALUES ('unknown_migration', 1)",
        "ALTER TABLE public.seaql_migrations ADD COLUMN unexpected text",
        "GRANT INSERT ON public.seaql_migrations TO med_tracker_app",
    ] {
        let db = restored_database().await;
        migration::Migrator::up(&db, None).await.unwrap();
        db.execute_unprepared(mutation).await.unwrap();
        let before = catalog(&db).await;
        let rows = ledger_rows(&db).await;
        assert!(migration::Migrator::up(&db, None).await.is_err());
        assert_eq!(catalog(&db).await, before);
        assert_eq!(ledger_rows(&db).await, rows);
        db.close().await.unwrap();
    }
}

#[tokio::test]
async fn cooperating_adoptions_serialize() {
    let db = restored_database().await;
    let database: String = db
        .query_one_raw(migration::sea_orm::Statement::from_string(
            migration::sea_orm::DbBackend::Postgres,
            "SELECT current_database() AS name",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "name")
        .unwrap();
    let uri = std::env::var("DATABASE_URL").unwrap();
    let (base, _) = uri.rsplit_once('/').unwrap();
    let other = migration::sea_orm::Database::connect(format!("{base}/{database}"))
        .await
        .unwrap();
    let (first, second) = tokio::join!(
        migration::Migrator::up(&db, None),
        migration::Migrator::up(&other, None),
    );
    first.unwrap();
    second.unwrap();
    let adopted = ledger_rows(&db).await;
    assert_eq!(adopted.len(), 6);
    assert_eq!(adopted[5].0, "m20261006_000006_registration_policy");
    other.close().await.unwrap();
    db.close().await.unwrap();
}

#[tokio::test]
async fn unsupported_migration_entrypoints_cannot_change_the_database() {
    let mut violations = Vec::new();
    for operation in [
        "install",
        "status",
        "fresh",
        "refresh",
        "reset",
        "down",
        "uninstall",
    ] {
        let db = restored_database().await;
        let before = catalog(&db).await;
        let result = match operation {
            "install" => migration::Migrator::install(&db).await,
            "status" => migration::Migrator::status(&db).await,
            "fresh" => migration::Migrator::fresh(&db).await,
            "refresh" => migration::Migrator::refresh(&db).await,
            "reset" => migration::Migrator::reset(&db).await,
            "down" => migration::Migrator::down(&db, None).await,
            "uninstall" => migration::Migrator::uninstall(&db).await,
            _ => unreachable!(),
        };
        if result.is_ok() || catalog(&db).await != before {
            violations.push(operation);
        }
        db.close().await.unwrap();
    }
    assert!(
        violations.is_empty(),
        "Unsupported entrypoints changed or accepted the database: {violations:?}"
    );
}

#[tokio::test]
async fn adoption_lock_wait_is_bounded_and_leaves_no_ledger() {
    let db = restored_database().await;
    let database: String = db
        .query_one_raw(migration::sea_orm::Statement::from_string(
            migration::sea_orm::DbBackend::Postgres,
            "SELECT current_database() AS name",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "name")
        .unwrap();
    let uri = std::env::var("DATABASE_URL").unwrap();
    let (base, _) = uri.rsplit_once('/').unwrap();
    let other = migration::sea_orm::Database::connect(format!("{base}/{database}"))
        .await
        .unwrap();
    let holder = db.begin().await.unwrap();
    holder
        .execute_unprepared("SELECT pg_advisory_xact_lock(5567948073909809714)")
        .await
        .unwrap();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        migration::Migrator::up(&other, None),
    )
    .await;
    holder.rollback().await.unwrap();
    let absent = ledger_absent(&db).await;
    other.close().await.unwrap();
    db.close().await.unwrap();
    let error = result
        .expect("Adoption waited beyond its configured lock timeout")
        .unwrap_err();
    assert!(error.to_string().contains("lock timeout"), "{error}");
    assert!(absent);
}

#[tokio::test]
async fn adoption_rejects_caller_transaction_without_changing_context() {
    let db = restored_database().await;
    let transaction = db.begin().await.unwrap();
    transaction.execute_unprepared("SET LOCAL search_path = pg_catalog; SET LOCAL lock_timeout = '17s'; SET LOCAL statement_timeout = '19s'").await.unwrap();
    let state = "SELECT jsonb_build_object('role', current_user, 'path', current_setting('search_path'), 'lock_timeout', current_setting('lock_timeout'), 'statement_timeout', current_setting('statement_timeout'), 'advisory_locks', (SELECT count(*) FROM pg_catalog.pg_locks WHERE pid = pg_backend_pid() AND locktype = 'advisory'), 'ledger', to_regclass('public.seaql_migrations'))::text AS state";
    let before: String = transaction
        .query_one_raw(migration::sea_orm::Statement::from_string(
            migration::sea_orm::DbBackend::Postgres,
            state,
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "state")
        .unwrap();
    let result = migration::Migrator::up(&transaction, None).await;
    let after: String = transaction
        .query_one_raw(migration::sea_orm::Statement::from_string(
            migration::sea_orm::DbBackend::Postgres,
            state,
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "state")
        .unwrap();
    transaction.rollback().await.unwrap();
    assert!(ledger_absent(&db).await);
    db.close().await.unwrap();
    assert!(result.is_err(), "Caller-owned transaction was accepted");
    assert_eq!(before, after, "Rejected adoption changed caller context");
}

#[tokio::test]
async fn baseline_only_adoption_upgrades_to_the_supported_runtime_state() {
    let db = restored_database().await;
    migration::Migrator::up(&db, Some(1)).await.unwrap();
    let baseline = ledger_rows(&db).await;
    assert_eq!(baseline.len(), 1);
    migration::Migrator::up(&db, None).await.unwrap();
    let runtime = ledger_rows(&db).await;
    assert_eq!(runtime.len(), 6);
    assert_eq!(runtime[0], baseline[0]);
    assert_eq!(runtime[1].0, "m20261005_000002_provision_runtime");
    assert_eq!(runtime[2].0, "m20261006_000003_canonical_take_identity");
    assert_eq!(runtime[3].0, "m20261006_000004_browser_sessions");
    assert_eq!(runtime[4].0, "m20261006_000005_access_token_scopes");
    assert_eq!(runtime[5].0, "m20261006_000006_registration_policy");
    let before = catalog(&db).await;
    migration::Migrator::up(&db, None).await.unwrap();
    assert_eq!(ledger_rows(&db).await, runtime);
    assert_eq!(catalog(&db).await, before);
    db.close().await.unwrap();
}

#[tokio::test]
async fn runtime_queue_drift_is_rejected() {
    let db = restored_database().await;
    migration::Migrator::up(&db, None).await.unwrap();
    db.execute_unprepared("ALTER TABLE public.pg_loco_queue ALTER COLUMN priority SET DEFAULT 1")
        .await
        .unwrap();
    let rows = ledger_rows(&db).await;
    let error = migration::Migrator::up(&db, None).await.unwrap_err();
    assert!(
        error
            .to_string()
            .contains("column:public.pg_loco_queue.priority")
    );
    assert_eq!(ledger_rows(&db).await, rows);
    db.close().await.unwrap();
}

#[tokio::test]
async fn adoption_rejects_an_owned_transaction_executor() {
    let db = restored_database().await;
    let transaction = db.begin().await.unwrap();
    let result = migration::Migrator::up(
        migration::sea_orm::DatabaseExecutor::OwnedTransaction(transaction),
        None,
    )
    .await;
    assert!(result.is_err());
    assert!(ledger_absent(&db).await);
    db.close().await.unwrap();
}

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
