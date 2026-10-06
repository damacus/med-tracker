use med_tracker::models::access::{self, Actor, HouseholdScope, PersonAccess};
use med_tracker::models::errors::OperationError;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};

struct Fixture {
    admin: DatabaseConnection,
    runtime: DatabaseConnection,
    runtime_uri: String,
}

impl Fixture {
    async fn new() -> Self {
        assert_eq!(std::env::var("MEDTRACKER_OWNED_DATABASE").unwrap(), "1");
        let uri = std::env::var("DATABASE_URL").unwrap();
        let endpoint = uri
            .strip_prefix("postgres://medtracker:medtracker_password@127.0.0.1:")
            .unwrap();
        let (port, _) = endpoint.split_once('/').unwrap();
        assert!(port.parse::<u16>().unwrap() > 0);
        let suffix = uuid::Uuid::new_v4().simple();
        let name = format!("tenant_{suffix}");
        let role = format!("runtime_{suffix}");
        let control = Database::connect(&uri).await.unwrap();
        control
            .execute_unprepared(&format!(
                "CREATE DATABASE {name} TEMPLATE medtracker_reference"
            ))
            .await
            .unwrap();
        control.execute_unprepared(&format!("CREATE ROLE {role} LOGIN PASSWORD 'password' NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS IN ROLE med_tracker_app")).await.unwrap();
        control.close().await.unwrap();
        let admin = Database::connect(format!(
            "postgres://medtracker:medtracker_password@127.0.0.1:{port}/{name}"
        ))
        .await
        .unwrap();
        <migration::Migrator as migration::MigratorTrait>::up(&admin, None)
            .await
            .unwrap();
        admin
            .execute_unprepared(include_str!("fixtures/persistence-records.sql"))
            .await
            .unwrap();
        admin.execute_unprepared(
            "UPDATE public.accounts SET status = 2;
             INSERT INTO public.users(id, person_id, email_address, password_digest, created_at, updated_at)
             VALUES (77001, 73001, 'persistence@example.test', crypt('password', gen_salt('bf', 4)), now(), now());
             INSERT INTO public.person_access_grants(id, household_id, household_membership_id, person_id, access_level, relationship_type, created_at, updated_at)
             VALUES (78001, 72001, 74001, 73001, 'manage', 'self', now(), now());
             INSERT INTO public.accounts(id, email, status, password_hash, created_at, updated_at)
             VALUES (71002, 'other@example.test', 2, crypt('password', gen_salt('bf', 4)), now(), now());
             INSERT INTO public.households(id, name, slug, timezone, created_at, updated_at)
             VALUES (72002, 'Other household', 'other-fixture', 'Europe/London', now(), now());
             INSERT INTO public.people(id, account_id, household_id, name, created_at, updated_at)
             VALUES (73004, 71002, 72002, 'Other adult', now(), now());
             INSERT INTO public.household_memberships(id, account_id, household_id, person_id, joined_at, created_at, updated_at)
             VALUES (74002, 71002, 72002, 73004, now(), now(), now());
             INSERT INTO public.users(id, person_id, email_address, password_digest, created_at, updated_at)
             VALUES (77002, 73004, 'other@example.test', crypt('password', gen_salt('bf', 4)), now(), now());
             INSERT INTO public.person_access_grants(id, household_id, household_membership_id, person_id, access_level, relationship_type, created_at, updated_at)
             VALUES (78002, 72002, 74002, 73004, 'view', 'self', now(), now());"
        ).await.unwrap();
        let runtime_uri = format!("postgres://{role}:password@127.0.0.1:{port}/{name}");
        let mut options = sea_orm::ConnectOptions::new(runtime_uri.clone());
        options.max_connections(1).min_connections(1);
        let runtime = Database::connect(options).await.unwrap();
        Self {
            admin,
            runtime,
            runtime_uri,
        }
    }

    async fn close(self) {
        self.runtime.close().await.unwrap();
        self.admin.close().await.unwrap();
    }
}

fn scope(account_id: i64, household_id: i64) -> HouseholdScope {
    HouseholdScope {
        actor: Actor { account_id },
        household_id,
        request_id: "synthetic-request".into(),
    }
}

async fn visible_granted_people<C: ConnectionTrait>(db: &C) -> Vec<i64> {
    db.query_all_raw(Statement::from_string(
        DbBackend::Postgres,
        "SELECT person_id AS id FROM public.person_access_grants ORDER BY person_id",
    ))
    .await
    .unwrap()
    .into_iter()
    .map(|row| row.try_get("", "id").unwrap())
    .collect()
}

#[tokio::test]
async fn fixture_uses_canonical_administrator_role() {
    let fixture = Fixture::new().await;
    let transaction = access::begin(&fixture.runtime, &scope(71001, 72001))
        .await
        .unwrap();
    let role = transaction.membership().role.clone();
    transaction.rollback().await.unwrap();
    fixture.close().await;
    assert_eq!(role, "administrator");
}

#[tokio::test]
async fn actor_identity_is_independent_of_household_linked_subject() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared(
        "UPDATE public.household_memberships SET person_id = NULL WHERE id = 74002;
         INSERT INTO public.household_memberships(id, account_id, household_id, person_id, joined_at, created_at, updated_at)
         VALUES (74003, 71001, 72002, 73004, now(), now(), now());
         INSERT INTO public.person_access_grants(id, household_id, household_membership_id, person_id, access_level, relationship_type, created_at, updated_at)
         VALUES (78003, 72002, 74003, 73004, 'view', 'carer', now(), now());"
    ).await.unwrap();
    let transaction = access::begin(&fixture.runtime, &scope(71001, 72002))
        .await
        .unwrap();
    assert_eq!(transaction.user_id(), 77001);
    assert_eq!(transaction.membership().person_id, Some(73004));
    assert_eq!(transaction.membership().household_id, 72002);
    access::require_person_access(&transaction, 73004, PersonAccess::View)
        .await
        .unwrap();
    assert_eq!(
        access::require_person_access(&transaction, 73001, PersonAccess::View).await,
        Err(OperationError::Forbidden)
    );
    transaction.rollback().await.unwrap();
    assert!(visible_granted_people(&fixture.runtime).await.is_empty());
    fixture.close().await;
}

#[tokio::test]
async fn begin_rejects_unknown_and_locked_accounts() {
    let fixture = Fixture::new().await;
    assert!(matches!(
        access::begin(&fixture.runtime, &scope(71999, 72001)).await,
        Err(OperationError::Unauthenticated)
    ));
    fixture.admin.execute_unprepared("INSERT INTO public.account_lockouts(account_id, deadline, key) VALUES (71001, timezone('UTC', clock_timestamp()) + interval '1 hour', 'synthetic-lockout')").await.unwrap();
    assert!(matches!(
        access::begin(&fixture.runtime, &scope(71001, 72001)).await,
        Err(OperationError::Unauthenticated)
    ));
    assert!(visible_granted_people(&fixture.runtime).await.is_empty());
    fixture.close().await;
}

#[tokio::test]
async fn begin_rejects_inactive_households() {
    let fixture = Fixture::new().await;
    for change in [
        "UPDATE public.households SET status = 'archived' WHERE id = 72001",
        "UPDATE public.households SET status = 'active', lifecycle_state = 'held' WHERE id = 72001",
    ] {
        fixture.admin.execute_unprepared(change).await.unwrap();
        assert!(matches!(
            access::begin(&fixture.runtime, &scope(71001, 72001)).await,
            Err(OperationError::Forbidden)
        ));
        assert!(visible_granted_people(&fixture.runtime).await.is_empty());
    }
    fixture.close().await;
}

#[tokio::test]
async fn pool_reuse_does_not_leak_household() {
    let fixture = Fixture::new().await;
    let first = access::begin(&fixture.runtime, &scope(71001, 72001))
        .await
        .unwrap();
    assert_eq!(
        visible_granted_people(first.transaction()).await,
        vec![73001]
    );
    first.commit().await.unwrap();
    assert!(visible_granted_people(&fixture.runtime).await.is_empty());
    let second = access::begin(&fixture.runtime, &scope(71002, 72002))
        .await
        .unwrap();
    assert_eq!(
        visible_granted_people(second.transaction()).await,
        vec![73004]
    );
    second.rollback().await.unwrap();
    assert!(visible_granted_people(&fixture.runtime).await.is_empty());
    fixture.close().await;
}

#[tokio::test]
async fn actor_does_not_authorize_another_household() {
    let fixture = Fixture::new().await;
    assert!(matches!(
        access::begin(&fixture.runtime, &scope(71001, 72002)).await,
        Err(OperationError::Forbidden)
    ));
    assert!(visible_granted_people(&fixture.runtime).await.is_empty());
    fixture.close().await;
}

#[tokio::test]
async fn person_access_rechecks_live_grants() {
    let fixture = Fixture::new().await;
    let transaction = access::begin(&fixture.runtime, &scope(71001, 72001))
        .await
        .unwrap();
    access::require_person_access(&transaction, 73001, PersonAccess::Manage)
        .await
        .unwrap();
    assert_eq!(
        access::require_person_access(&transaction, 73004, PersonAccess::View).await,
        Err(OperationError::Forbidden)
    );
    fixture
        .admin
        .execute_unprepared(
            "UPDATE public.person_access_grants SET revoked_at = now() WHERE id = 78001",
        )
        .await
        .unwrap();
    assert_eq!(
        access::require_person_access(&transaction, 73001, PersonAccess::View).await,
        Err(OperationError::Forbidden)
    );
    transaction.rollback().await.unwrap();
    fixture.close().await;
}

#[tokio::test]
async fn runtime_role_cannot_alter_schema_or_migration_history() {
    let fixture = Fixture::new().await;
    for statement in [
        "CREATE TABLE public.runtime_created (id bigint)",
        "ALTER TABLE public.people ADD COLUMN runtime_added text",
        "DELETE FROM public.seaql_migrations",
        "SET ROLE med_tracker_owner",
    ] {
        assert!(fixture.runtime.execute_unprepared(statement).await.is_err());
    }
    fixture.close().await;
}

#[tokio::test]
async fn cedar_person_access_uses_current_grant_level_and_database_expiry() {
    use med_tracker::models::care::people;
    let fixture = Fixture::new().await;
    let transaction = access::begin(&fixture.runtime, &scope(71001, 72001))
        .await
        .unwrap();
    for (level, expected) in [
        ("view", [true, false, false]),
        ("record", [true, true, false]),
        ("manage", [true, true, true]),
    ] {
        fixture
            .admin
            .execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "UPDATE person_access_grants SET access_level=$1 WHERE id=78001",
                [level.into()],
            ))
            .await
            .unwrap();
        for (request, allowed) in [
            PersonAccess::View,
            PersonAccess::Record,
            PersonAccess::Manage,
        ]
        .into_iter()
        .zip(expected)
        {
            assert_eq!(
                access::require_person_access(&transaction, 73001, request).await,
                if allowed {
                    Ok(())
                } else {
                    Err(OperationError::Forbidden)
                }
            );
        }
        let visible = people::list(&transaction, people::Pagination::default(), chrono_tz::UTC)
            .await
            .unwrap();
        assert_eq!(visible["meta"]["total_count"], 1);
        assert_eq!(visible["data"][0]["id"], 73001);
    }
    fixture.admin.execute_unprepared("UPDATE person_access_grants SET expires_at=timezone('UTC',clock_timestamp())-interval '1 second' WHERE id=78001").await.unwrap();
    assert_eq!(
        access::require_person_access(&transaction, 73001, PersonAccess::View).await,
        Err(OperationError::Forbidden)
    );
    let expired = people::list(&transaction, people::Pagination::default(), chrono_tz::UTC)
        .await
        .unwrap();
    assert_eq!(expired["meta"]["total_count"], 0);
    assert_eq!(expired["data"], serde_json::json!([]));
    fixture.admin.execute_unprepared("UPDATE person_access_grants SET expires_at=timezone('UTC',clock_timestamp())+interval '1 day' WHERE id=78001").await.unwrap();
    access::require_person_access(&transaction, 73001, PersonAccess::Manage)
        .await
        .unwrap();
    let restored = people::list(&transaction, people::Pagination::default(), chrono_tz::UTC)
        .await
        .unwrap();
    assert_eq!(restored["meta"]["total_count"], 1);
    assert_eq!(restored["data"][0]["id"], 73001);
    transaction.rollback().await.unwrap();
    fixture.close().await;
}

#[tokio::test]
async fn shared_recheck_rejects_current_account_lockout() {
    let fixture = Fixture::new().await;
    let transaction = access::begin(&fixture.runtime, &scope(71001, 72001))
        .await
        .unwrap();
    fixture.admin.execute_unprepared("INSERT INTO public.account_lockouts(account_id, deadline, key) VALUES (71001, now() + interval '1 hour', 'synthetic-lockout')").await.unwrap();
    let result = access::require_person_access(&transaction, 73001, PersonAccess::Manage).await;
    transaction.rollback().await.unwrap();
    fixture.close().await;
    assert_eq!(result, Err(OperationError::Unauthenticated));
}

#[tokio::test]
async fn shared_recheck_rejects_membership_role_change() {
    let fixture = Fixture::new().await;
    let transaction = access::begin(&fixture.runtime, &scope(71001, 72001))
        .await
        .unwrap();
    fixture
        .admin
        .execute_unprepared(
            "UPDATE public.household_memberships SET role = 'member' WHERE id = 74001",
        )
        .await
        .unwrap();
    let result = access::require_person_access(&transaction, 73001, PersonAccess::Manage).await;
    transaction.rollback().await.unwrap();
    fixture.close().await;
    assert_eq!(result, Err(OperationError::Forbidden));
}

#[tokio::test]
async fn runtime_loco_boot_uses_preprovisioned_queue_without_schema_privileges() {
    use loco_rs::{
        app::Hooks,
        boot::StartMode,
        config::{Config, QueueConfig},
        environment::Environment,
    };
    let fixture = Fixture::new().await;
    let mut config = Config::new(&Environment::Test).unwrap();
    config.database.uri = fixture.runtime_uri.clone();
    assert!(!config.database.auto_migrate);
    assert!(!config.database.dangerously_recreate);
    assert!(!config.database.dangerously_truncate);
    let Some(QueueConfig::Postgres(queue)) = config.queue.as_mut() else {
        panic!("PostgreSQL queue required")
    };
    queue.uri = fixture.runtime_uri.clone();
    let result =
        med_tracker::app::App::boot(StartMode::ServerAndWorker, &Environment::Test, config).await;
    let error = match result {
        Ok(boot) => {
            assert!(boot.router.is_some());
            assert!(boot.worker.is_some());
            boot.app_context
                .queue_provider
                .as_ref()
                .unwrap()
                .shutdown()
                .unwrap();
            boot.app_context.db.clone().close().await.unwrap();
            None
        }
        Err(error) => Some(error.to_string()),
    };
    fixture.close().await;
    assert!(
        error.is_none(),
        "Restricted runtime startup failed: {error:?}"
    );
}
