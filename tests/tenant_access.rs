#[path = "support/fixtures.rs"]
pub mod fixtures;
#[path = "tenant_access/global_person.rs"]
mod global_person;
#[path = "support/template.rs"]
mod template;
use loco_rs::model::{TenantActiveModelExt, TenantQueryExt};
use med_tracker::models::access::{self, Actor, HouseholdScope, PersonAccess};
use med_tracker::models::entities::{grant, location};
use med_tracker::models::errors::OperationError;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};
use sea_orm::{EntityTrait, Set};

#[test]
fn household_assignment_rejects_conflicting_ownership() {
    let model = location::ActiveModel::default().set_tenant(72001).unwrap();
    assert_eq!(model.household_id, Set(72001));
    assert!(matches!(
        model.set_tenant(72002),
        Err(loco_rs::model::ModelError::TenantMismatch)
    ));
}

#[tokio::test]
async fn loco_scoping_and_rls_independently_reject_other_households() {
    let fixture = Fixture::new().await;
    assert!(
        grant::Entity::find_by_id(78002)
            .one(&fixture.admin)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        grant::Entity::find_by_id(78002)
            .in_tenant(72001)
            .one(&fixture.admin)
            .await
            .unwrap()
            .is_none()
    );
    let tenant = access::begin(&fixture.runtime, &scope(71001, 72001), None)
        .await
        .unwrap();
    let db = tenant.transaction();
    assert!(
        grant::Entity::find_by_id(78002)
            .in_tenant(72001)
            .one(db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        grant::Entity::find_by_id(78002)
            .one(db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        grant::Entity::find()
            .in_tenant(72002)
            .all(db)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        grant::Entity::find()
            .in_tenant(72001)
            .all(db)
            .await
            .unwrap()
            .len(),
        1
    );
    let updated = grant::Entity::update_many()
        .col_expr(
            grant::Column::AccessLevel,
            sea_orm::sea_query::Expr::value("manage"),
        )
        .in_tenant(72002)
        .exec(db)
        .await
        .unwrap();
    assert_eq!(updated.rows_affected, 0);
    let deleted = grant::Entity::delete_many()
        .in_tenant(72002)
        .exec(db)
        .await
        .unwrap();
    assert_eq!(deleted.rows_affected, 0);
    tenant.rollback().await.unwrap();
    fixture.close().await;
}

struct Fixture {
    admin: DatabaseConnection,
    runtime: DatabaseConnection,
    runtime_uri: String,
}

impl Fixture {
    async fn new() -> Self {
        let uri = template::owned_database_url();
        let endpoint = uri
            .strip_prefix("postgres://medtracker:medtracker_password@127.0.0.1:")
            .unwrap();
        let (port, _) = endpoint.split_once('/').unwrap();
        assert!(port.parse::<u16>().unwrap() > 0);
        template::prepare(&uri, "tenant_access_template", |admin| async move {
            fixtures::seed_persistence(&admin).await;
            fixtures::seed_tenant_access(&admin).await;
        })
        .await;
        let suffix = uuid::Uuid::new_v4().simple();
        let name = format!("tenant_{suffix}");
        let role = format!("runtime_{suffix}");
        let control = Database::connect(&uri).await.unwrap();
        control
            .execute_unprepared(&format!(
                "CREATE DATABASE {name} TEMPLATE tenant_access_template"
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
    let transaction = access::begin(&fixture.runtime, &scope(71001, 72001), None)
        .await
        .unwrap();
    let role = transaction.membership().role.clone();
    transaction.rollback().await.unwrap();
    fixture.close().await;
    assert_eq!(role, "administrator");
}

#[tokio::test]
async fn actor_identity_is_independent_of_another_clinical_subject() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared(
        "INSERT INTO public.household_memberships(id, household_id, person_id, joined_at, created_at, updated_at)
         VALUES (74003, 72002, 73001, now(), now(), now());
         INSERT INTO public.person_access_grants(id, household_id, household_membership_id, person_id, access_level, relationship_type, created_at, updated_at)
         VALUES (78003, 72002, 74003, 73004, 'view', 'carer', now(), now());"
    ).await.unwrap();
    let transaction = access::begin(&fixture.runtime, &scope(71001, 72002), None)
        .await
        .unwrap();
    assert_eq!(transaction.user_id(), 77001);
    assert_eq!(transaction.membership().person_id, 73001);
    assert_eq!(transaction.membership().household_id, 72002);
    access::require_person_access(&transaction, 73004, PersonAccess::View)
        .await
        .unwrap();
    access::require_person_access(&transaction, 73001, PersonAccess::View)
        .await
        .unwrap();
    transaction.rollback().await.unwrap();
    assert!(visible_granted_people(&fixture.runtime).await.is_empty());
    fixture.close().await;
}

#[tokio::test]
async fn begin_rejects_unknown_and_locked_accounts() {
    let fixture = Fixture::new().await;
    assert!(matches!(
        access::begin(&fixture.runtime, &scope(71999, 72001), None).await,
        Err(OperationError::Unauthenticated)
    ));
    fixture.admin.execute_unprepared("INSERT INTO public.account_lockouts(account_id, deadline, key) VALUES (71001, timezone('UTC', clock_timestamp()) + interval '1 hour', 'synthetic-lockout')").await.unwrap();
    assert!(matches!(
        access::begin(&fixture.runtime, &scope(71001, 72001), None).await,
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
            access::begin(&fixture.runtime, &scope(71001, 72001), None).await,
            Err(OperationError::Forbidden)
        ));
        assert!(visible_granted_people(&fixture.runtime).await.is_empty());
    }
    fixture.close().await;
}

#[tokio::test]
async fn pool_reuse_does_not_leak_household() {
    let fixture = Fixture::new().await;
    let first = access::begin(&fixture.runtime, &scope(71001, 72001), None)
        .await
        .unwrap();
    assert_eq!(
        visible_granted_people(first.transaction()).await,
        vec![73001]
    );
    first.commit().await.unwrap();
    assert!(visible_granted_people(&fixture.runtime).await.is_empty());
    let second = access::begin(&fixture.runtime, &scope(71002, 72002), None)
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
        access::begin(&fixture.runtime, &scope(71001, 72002), None).await,
        Err(OperationError::Forbidden)
    ));
    assert!(visible_granted_people(&fixture.runtime).await.is_empty());
    fixture.close().await;
}

async fn seed_accountless_manage_grant(fixture: &Fixture) {
    fixture
        .admin
        .execute_unprepared(
            "INSERT INTO public.household_memberships(id,household_id,person_id,role,joined_at,created_at,updated_at)
             VALUES(74501,72001,73003,'member',now(),now(),now());
             INSERT INTO public.person_access_grants(id,household_id,household_membership_id,person_id,access_level,relationship_type,created_at,updated_at)
             VALUES(78003,72001,74001,73003,'manage','carer',now(),now())",
        )
        .await
        .unwrap();
    let accountless = fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT NOT EXISTS(SELECT 1 FROM public.accounts WHERE person_id=73003) AND NOT EXISTS(SELECT 1 FROM public.users WHERE person_id=73003) AS accountless",
        ))
        .await
        .unwrap()
        .unwrap();
    assert!(accountless.try_get::<bool>("", "accountless").unwrap());
}

async fn assert_person_access_controls(
    transaction: &access::TenantTransaction,
    other_manage: bool,
) {
    use med_tracker::models::care::people;
    for request in [
        PersonAccess::View,
        PersonAccess::Record,
        PersonAccess::Manage,
    ] {
        assert_eq!(
            access::require_person_access(transaction, 73001, request).await,
            Ok(()),
            "canonical self {request:?}",
        );
        assert_eq!(
            access::require_person_access(transaction, 73003, request).await,
            if request == PersonAccess::Manage && !other_manage {
                Err(OperationError::Forbidden)
            } else {
                Ok(())
            },
            "accountless participant {request:?}, other Manage allowed: {other_manage}",
        );
        assert_eq!(
            access::require_person_access(transaction, 73004, request).await,
            Err(OperationError::Forbidden),
            "foreign household subject {request:?}",
        );
    }
    let visible = people::list(transaction, people::Pagination::default(), chrono_tz::UTC)
        .await
        .unwrap();
    assert_eq!(visible["meta"]["total_count"], 2);
    let ids: Vec<_> = visible["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|person| person["id"].as_i64().unwrap())
        .collect();
    assert_eq!(ids, vec![73001, 73003]);
}

#[tokio::test]
async fn person_access_rechecks_live_grants() {
    let fixture = Fixture::new().await;
    seed_accountless_manage_grant(&fixture).await;
    let transaction = access::begin(&fixture.runtime, &scope(71001, 72001), None)
        .await
        .unwrap();
    assert_person_access_controls(&transaction, true).await;
    fixture
        .admin
        .execute_unprepared(
            "UPDATE public.person_access_grants SET revoked_at = now() WHERE id IN (78001,78003)",
        )
        .await
        .unwrap();
    assert_person_access_controls(&transaction, false).await;
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
    let fixture = Fixture::new().await;
    seed_accountless_manage_grant(&fixture).await;
    let transaction = access::begin(&fixture.runtime, &scope(71001, 72001), None)
        .await
        .unwrap();
    assert_person_access_controls(&transaction, true).await;
    for (level, other_manage) in [("view", false), ("record", false), ("manage", true)] {
        fixture
            .admin
            .execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "UPDATE person_access_grants SET access_level=$1 WHERE id IN (78001,78003)",
                [level.into()],
            ))
            .await
            .unwrap();
        assert_person_access_controls(&transaction, other_manage).await;
    }
    fixture.admin.execute_unprepared("UPDATE person_access_grants SET expires_at=timezone('UTC',clock_timestamp())-interval '1 second' WHERE id IN (78001,78003)").await.unwrap();
    assert_person_access_controls(&transaction, false).await;
    fixture.admin.execute_unprepared("UPDATE person_access_grants SET expires_at=timezone('UTC',clock_timestamp())+interval '1 day' WHERE id IN (78001,78003)").await.unwrap();
    assert_person_access_controls(&transaction, true).await;
    transaction.rollback().await.unwrap();
    fixture.close().await;
}

#[tokio::test]
async fn shared_recheck_rejects_current_account_lockout() {
    let fixture = Fixture::new().await;
    let transaction = access::begin(&fixture.runtime, &scope(71001, 72001), None)
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
    let transaction = access::begin(&fixture.runtime, &scope(71001, 72001), None)
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
