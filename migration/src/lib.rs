use sea_orm::TransactionTrait;
use sea_orm_migration::prelude::*;
pub use sea_orm_migration::{MigrationTrait, MigratorTrait, sea_orm};
use serde_json::{Map, Value};

pub mod m20261005_000002_provision_runtime;
pub mod m20261006_000003_canonical_take_identity;
pub mod m20261006_000004_browser_sessions;
pub mod m20261006_000005_access_token_scopes;
pub mod m20261006_000006_registration_policy;
pub mod m20261006_000007_better_auth_identity;
pub use m20261006_000003_canonical_take_identity::CANONICAL_CLIENT_UUID_SQL;

pub struct Migrator;
struct StandardMigrator;
struct AdoptMedtracker;

const VERSION: &str = "m20261005_000001_adopt_medtracker";
const RUNTIME_VERSION: &str = "m20261005_000002_provision_runtime";
const TAKE_IDENTITY_VERSION: &str = "m20261006_000003_canonical_take_identity";
const BROWSER_SESSION_VERSION: &str = "m20261006_000004_browser_sessions";
const ACCESS_SCOPE_VERSION: &str = "m20261006_000005_access_token_scopes";
const REGISTRATION_POLICY_VERSION: &str = "m20261006_000006_registration_policy";
const IDENTITY_VERSION: &str = "m20261006_000007_better_auth_identity";

impl MigrationName for AdoptMedtracker {
    fn name(&self) -> &str {
        VERSION
    }
}

#[async_trait::async_trait]
impl MigrationTrait for AdoptMedtracker {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "REVOKE ALL ON public.seaql_migrations FROM PUBLIC, med_tracker_app, med_tracker_audit_exporter, med_tracker_audit_verifier",
        ).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl MigratorTrait for StandardMigrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(AdoptMedtracker),
            Box::new(m20261005_000002_provision_runtime::Migration),
            Box::new(m20261006_000003_canonical_take_identity::Migration),
            Box::new(m20261006_000004_browser_sessions::Migration),
            Box::new(m20261006_000005_access_token_scopes::Migration),
            Box::new(m20261006_000006_registration_policy::Migration),
            Box::new(m20261006_000007_better_auth_identity::Migration),
        ]
    }
}

fn migration_error(message: impl Into<String>) -> DbErr {
    DbErr::Migration(message.into())
}

fn recorded_catalog(source: &str) -> Result<Map<String, Value>, DbErr> {
    serde_json::from_str(source)
        .map_err(|error| migration_error(format!("Invalid recorded catalog: {error}")))
}

async fn catalog<C: ConnectionTrait>(db: &C) -> Result<Map<String, Value>, DbErr> {
    let row = db
        .query_one_raw(sea_orm::Statement::from_string(
            sea_orm::DbBackend::Postgres,
            include_str!("../catalog.sql"),
        ))
        .await?
        .ok_or_else(|| migration_error("Catalog query returned no result"))?;
    recorded_catalog(&row.try_get::<String>("", "catalog")?)
}

async fn verify_catalog<C: ConnectionTrait>(db: &C, require_ledger: bool) -> Result<(), DbErr> {
    let actual = catalog(db).await?;
    let mut expected = recorded_catalog(include_str!("../catalog.json"))?;
    let has_ledger = actual.contains_key("relation:public.seaql_migrations");
    if has_ledger {
        let ledger = recorded_catalog(include_str!("../ledger-catalog.json"))?;
        for (identity, definition) in &ledger {
            if actual.get(identity) != Some(definition) {
                return Err(migration_error(format!("Catalog drift: {identity}")));
            }
        }
        expected.extend(ledger);
        let rows = db
            .query_all_raw(sea_orm::Statement::from_string(
                sea_orm::DbBackend::Postgres,
                "SELECT version, applied_at FROM public.seaql_migrations ORDER BY version",
            ))
            .await?;
        let versions = [
            VERSION,
            RUNTIME_VERSION,
            TAKE_IDENTITY_VERSION,
            BROWSER_SESSION_VERSION,
            ACCESS_SCOPE_VERSION,
            REGISTRATION_POLICY_VERSION,
            IDENTITY_VERSION,
        ];
        if rows.is_empty() || rows.len() > versions.len() {
            return Err(migration_error(
                "Migration ledger must contain a supported ordered migration prefix",
            ));
        }
        for (row, version) in rows.iter().zip(versions) {
            if row.try_get::<String>("", "version")? != version
                || row.try_get::<i64>("", "applied_at")? <= 0
            {
                return Err(migration_error(
                    "Migration ledger must contain a supported ordered migration prefix",
                ));
            }
        }
        if rows.len() >= 2 {
            expected.extend(recorded_catalog(include_str!("../queue-catalog.json"))?);
        }
        if rows.len() >= 3 {
            expected.extend(recorded_catalog(include_str!(
                "../take-identity-catalog.json"
            ))?);
        }
        if rows.len() >= 4 {
            expected.extend(recorded_catalog(include_str!(
                "../browser-session-catalog.json"
            ))?);
        }
        if rows.len() >= 5 {
            expected.extend(recorded_catalog(include_str!(
                "../access-token-scopes-catalog.json"
            ))?);
        }
        if rows.len() >= 6 {
            expected.extend(recorded_catalog(include_str!(
                "../registration-policy-catalog.json"
            ))?);
        }
        if rows.len() >= 7 {
            expected.extend(recorded_catalog(include_str!("../identity-catalog.json"))?);
        }
    } else if require_ledger {
        return Err(migration_error(
            "Adoption did not create its migration ledger",
        ));
    }
    let mut changed: Vec<&str> = expected
        .keys()
        .chain(actual.keys())
        .filter(|key| expected.get(*key) != actual.get(*key))
        .map(String::as_str)
        .collect();
    changed.sort_unstable();
    changed.dedup();
    if !changed.is_empty() {
        return Err(migration_error(format!(
            "Catalog drift: {}",
            changed.join(", ")
        )));
    }
    Ok(())
}

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        StandardMigrator::migrations()
    }

    async fn install<C: ConnectionTrait>(_db: &C) -> Result<(), DbErr> {
        Err(migration_error(
            "Use guarded adoption; standalone ledger installation and mutating status are disabled",
        ))
    }

    async fn fresh<'c, C: IntoSchemaManagerConnection<'c>>(_db: C) -> Result<(), DbErr> {
        Err(migration_error(
            "Destructive migration commands are disabled while Rails rollback is retained",
        ))
    }

    async fn refresh<'c, C: IntoSchemaManagerConnection<'c>>(_db: C) -> Result<(), DbErr> {
        Err(migration_error(
            "Destructive migration commands are disabled while Rails rollback is retained",
        ))
    }

    async fn reset<'c, C: IntoSchemaManagerConnection<'c>>(_db: C) -> Result<(), DbErr> {
        Err(migration_error(
            "Destructive migration commands are disabled while Rails rollback is retained",
        ))
    }

    async fn uninstall<'c, C: IntoSchemaManagerConnection<'c>>(_db: C) -> Result<(), DbErr> {
        Err(migration_error(
            "Removing the adoption ledger is disabled while Rails rollback is retained",
        ))
    }

    async fn down<'c, C: IntoSchemaManagerConnection<'c>>(
        _db: C,
        _steps: Option<u32>,
    ) -> Result<(), DbErr> {
        Err(migration_error(
            "Destructive migration commands are disabled while Rails rollback is retained",
        ))
    }

    async fn up<'c, C>(db: C, steps: Option<u32>) -> Result<(), DbErr>
    where
        C: IntoSchemaManagerConnection<'c>,
    {
        let sea_orm::DatabaseExecutor::Connection(db) = db.into_database_executor() else {
            return Err(migration_error(
                "Guarded adoption requires a database connection and owns its transaction",
            ));
        };
        if db.get_database_backend() != sea_orm::DbBackend::Postgres {
            return Err(migration_error(
                "MedTracker adoption requires PostgreSQL 18",
            ));
        }
        let transaction = db.begin().await?;
        let result = async {
            transaction
                .execute_unprepared(
                    "SET LOCAL search_path = pg_catalog, public, pg_temp;
                 SET LOCAL lock_timeout = '5s';
                 SET LOCAL statement_timeout = '30s';
                 DO $$ BEGIN
                   IF current_setting('server_version_num')::int NOT BETWEEN 180000 AND 189999
                   THEN RAISE EXCEPTION 'MedTracker adoption requires PostgreSQL 18'; END IF;
                 END $$;
                 SELECT pg_advisory_xact_lock(5567948073909809714)",
                )
                .await?;
            verify_catalog(&transaction, false).await?;
            transaction
                .execute_unprepared(
                    "SET LOCAL ROLE med_tracker_owner; SET LOCAL search_path = public, pg_temp",
                )
                .await?;
            StandardMigrator::up(&transaction, steps).await?;
            transaction
                .execute_unprepared("SET LOCAL search_path = pg_catalog, public, pg_temp")
                .await?;
            verify_catalog(&transaction, true).await
        }
        .await;
        match result {
            Ok(()) => transaction.commit().await,
            Err(error) => {
                transaction.rollback().await?;
                Err(error)
            }
        }
    }
}
