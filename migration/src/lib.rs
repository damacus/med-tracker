use sea_orm::TransactionTrait;
use sea_orm_migration::prelude::*;
pub use sea_orm_migration::{MigrationTrait, MigratorTrait, sea_orm};

mod fresh;
pub mod m20261005_000002_provision_runtime;
pub mod m20261006_000003_canonical_take_identity;
pub mod m20261006_000004_browser_sessions;
pub mod m20261006_000005_access_token_scopes;
pub mod m20261006_000006_registration_policy;
pub mod m20261006_000007_better_auth_identity;
pub mod m20261008_000008_support_access;
pub mod m20261012_000009_notification_deliveries;
pub mod m20261013_000010_review_edit;
pub mod m20261016_000011_account_avatars;
pub mod m20261016_000012_support_expiry;
pub mod m20261017_000013_guardian_declarations;
pub mod m20261021_000014_member_directory_identity;
pub use m20261006_000003_canonical_take_identity::CANONICAL_CLIENT_UUID_SQL;

pub struct Migrator;
struct StandardMigrator;

#[async_trait::async_trait]
impl MigratorTrait for StandardMigrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(fresh::Migration),
            Box::new(m20261005_000002_provision_runtime::Migration),
            Box::new(m20261006_000003_canonical_take_identity::Migration),
            Box::new(m20261006_000004_browser_sessions::Migration),
            Box::new(m20261006_000005_access_token_scopes::Migration),
            Box::new(m20261006_000006_registration_policy::Migration),
            Box::new(m20261006_000007_better_auth_identity::Migration),
            Box::new(m20261008_000008_support_access::Migration),
            Box::new(m20261012_000009_notification_deliveries::Migration),
            Box::new(m20261013_000010_review_edit::Migration),
            Box::new(m20261016_000011_account_avatars::Migration),
            Box::new(m20261016_000012_support_expiry::Migration),
            Box::new(m20261017_000013_guardian_declarations::Migration),
            Box::new(m20261021_000014_member_directory_identity::Migration),
        ]
    }
}

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        StandardMigrator::migrations()
    }

    async fn fresh<'c, C: IntoSchemaManagerConnection<'c>>(_db: C) -> Result<(), DbErr> {
        Err(DbErr::Migration(
            "Destructive migration commands are disabled; create a new empty database".into(),
        ))
    }

    async fn refresh<'c, C: IntoSchemaManagerConnection<'c>>(_db: C) -> Result<(), DbErr> {
        Err(DbErr::Migration(
            "Destructive migration commands are disabled; create a new empty database".into(),
        ))
    }

    async fn reset<'c, C: IntoSchemaManagerConnection<'c>>(_db: C) -> Result<(), DbErr> {
        Err(DbErr::Migration(
            "Destructive migration commands are disabled; create a new empty database".into(),
        ))
    }

    async fn uninstall<'c, C: IntoSchemaManagerConnection<'c>>(_db: C) -> Result<(), DbErr> {
        Err(DbErr::Migration(
            "Destructive migration commands are disabled; create a new empty database".into(),
        ))
    }

    async fn down<'c, C: IntoSchemaManagerConnection<'c>>(
        _db: C,
        _steps: Option<u32>,
    ) -> Result<(), DbErr> {
        Err(DbErr::Migration(
            "Destructive migration commands are disabled; create a new empty database".into(),
        ))
    }

    async fn up<'c, C: IntoSchemaManagerConnection<'c>>(
        db: C,
        steps: Option<u32>,
    ) -> Result<(), DbErr> {
        let sea_orm::DatabaseExecutor::Connection(db) = db.into_database_executor() else {
            return Err(DbErr::Migration(
                "Migration requires its own database connection and transaction".into(),
            ));
        };
        if db.get_database_backend() != sea_orm::DbBackend::Postgres {
            return Err(DbErr::Migration("MedTracker requires PostgreSQL 18".into()));
        }
        let transaction = db.begin().await?;
        let result = async {
            transaction
                .execute_unprepared(
                    "SET LOCAL search_path = public, pg_temp;
                 SET LOCAL lock_timeout = '5s';
                 SET LOCAL statement_timeout = '60s';
                 DO $$ BEGIN
                   IF current_setting('server_version_num')::int NOT BETWEEN 180000 AND 189999
                   THEN RAISE EXCEPTION 'MedTracker requires PostgreSQL 18'; END IF;
                 END $$;
                 SELECT pg_advisory_xact_lock(5567948073909809714)",
                )
                .await?;
            StandardMigrator::install(&transaction).await?;
            let installed = transaction.query_one_raw(sea_orm::Statement::from_sql_and_values(
                sea_orm::DbBackend::Postgres,
                "SELECT EXISTS (SELECT 1 FROM public.seaql_migrations WHERE version = $1) AS installed",
                [fresh::Migration.name().into()],
            )).await?.ok_or_else(|| DbErr::Migration("Cannot inspect migration ledger".into()))?;
            if installed.try_get::<bool>("", "installed")? {
                transaction.execute_unprepared("SET LOCAL ROLE med_tracker_owner").await?;
            }
            StandardMigrator::up(&transaction, steps).await?;
            Ok(())
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
