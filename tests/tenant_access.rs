#[path = "support/scenario.rs"]
pub mod scenario;
use loco_rs::model::{TenantActiveModelExt, TenantQueryExt};
use med_tracker::models::_entities::{account_lockouts, accounts, household_memberships, people};
use med_tracker::models::access::{self, Actor, HouseholdScope, PersonAccess};
use med_tracker::models::entities::{grant, location};
use med_tracker::models::errors::OperationError;
use scenario::Scenario;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, Database, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};

#[test]
fn household_assignment_rejects_conflicting_ownership() {
    let model = <location::ActiveModel as Default>::default()
        .set_tenant(1)
        .unwrap();
    assert_eq!(model.household_id, Set(1));
    assert!(matches!(
        model.set_tenant(2),
        Err(loco_rs::model::ModelError::TenantMismatch)
    ));
}

#[tokio::test]
async fn loco_scoping_and_rls_independently_reject_other_households() {
    let fixture = Fixture::new().await;
    assert!(
        grant::Entity::find_by_id(fixture.other_grant.id)
            .one(&fixture.admin)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        grant::Entity::find_by_id(fixture.other_grant.id)
            .in_tenant(fixture.household.id)
            .one(&fixture.admin)
            .await
            .unwrap()
            .is_none()
    );
    let tenant = access::begin(&fixture.runtime, &fixture.scope(), None)
        .await
        .unwrap();
    let db = tenant.transaction();
    assert!(
        grant::Entity::find_by_id(fixture.other_grant.id)
            .in_tenant(fixture.household.id)
            .one(db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        grant::Entity::find_by_id(fixture.other_grant.id)
            .one(db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        grant::Entity::find()
            .in_tenant(fixture.other.household.id)
            .all(db)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        grant::Entity::find()
            .in_tenant(fixture.household.id)
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
        .in_tenant(fixture.other.household.id)
        .exec(db)
        .await
        .unwrap();
    assert_eq!(updated.rows_affected, 0);
    let deleted = grant::Entity::delete_many()
        .in_tenant(fixture.other.household.id)
        .exec(db)
        .await
        .unwrap();
    assert_eq!(deleted.rows_affected, 0);
    tenant.rollback().await.unwrap();
    fixture.close().await;
}

struct Fixture {
    scenario: Scenario,
    other: Scenario,
    own_grant: grant::Model,
    other_grant: grant::Model,
    accountless: people::Model,
    runtime_uri: String,
}

impl std::ops::Deref for Fixture {
    type Target = Scenario;
    fn deref(&self) -> &Self::Target {
        &self.scenario
    }
}

fn now() -> chrono::NaiveDateTime {
    chrono::Utc::now().naive_utc()
}

async fn membership(
    db: &impl ConnectionTrait,
    household_id: i64,
    person_id: i64,
    role: &str,
) -> household_memberships::Model {
    household_memberships::ActiveModel {
        household_id: Set(household_id),
        person_id: Set(person_id),
        role: Set(role.into()),
        status: Set("active".into()),
        joined_at: Set(now()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

async fn person(
    db: &impl ConnectionTrait,
    name: &str,
    person_type: i32,
    has_capacity: bool,
) -> people::Model {
    people::ActiveModel {
        name: Set(name.into()),
        person_type: Set(person_type),
        has_capacity: Set(has_capacity),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

async fn legacy_grant(
    db: &impl ConnectionTrait,
    member: &household_memberships::Model,
    person_id: i64,
    level: &str,
    relationship: &str,
) -> grant::Model {
    grant::ActiveModel {
        household_id: Set(member.household_id),
        household_membership_id: Set(member.id),
        person_id: Set(person_id),
        access_level: Set(level.into()),
        relationship_type: Set(relationship.into()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

async fn set_role(scenario: &Scenario, role: &str) {
    let mut row = scenario.membership.clone().into_active_model();
    row.role = Set(role.into());
    row.update(&scenario.admin).await.unwrap();
}

impl Fixture {
    async fn new() -> Self {
        let mut scenario = Scenario::new().await;
        set_role(&scenario, "administrator").await;
        let other = Scenario::new().await;
        set_role(&other, "member").await;
        let own_grant = legacy_grant(
            &scenario.admin,
            &scenario.membership,
            scenario.account.person_id,
            "manage",
            "self",
        )
        .await;
        let other_grant = legacy_grant(
            &scenario.admin,
            &other.membership,
            other.account.person_id,
            "view",
            "self",
        )
        .await;
        let accountless = person(&scenario.admin, "Synthetic dependent", 2, false).await;
        let runtime_uri = std::env::var("DATABASE_URL").unwrap();
        let mut options = sea_orm::ConnectOptions::new(runtime_uri.clone());
        options.max_connections(1).min_connections(1);
        let runtime = Database::connect(options).await.unwrap();
        let previous = std::mem::replace(&mut scenario.runtime, runtime);
        previous.close().await.unwrap();
        Self {
            scenario,
            other,
            own_grant,
            other_grant,
            accountless,
            runtime_uri,
        }
    }

    async fn lock(&self) {
        account_lockouts::ActiveModel {
            account_id: Set(self.account.id),
            deadline: Set(now() + chrono::Duration::hours(1)),
            key: Set(uuid::Uuid::new_v4().to_string()),
            ..Default::default()
        }
        .insert(&self.admin)
        .await
        .unwrap();
    }

    async fn close(self) {
        self.scenario.close().await;
        self.other.close().await;
    }
}

fn scope(account_id: i64, household_id: i64) -> HouseholdScope {
    HouseholdScope {
        actor: Actor { account_id },
        household_id,
        request_id: uuid::Uuid::new_v4().to_string(),
    }
}

async fn visible_granted_people<C: ConnectionTrait>(db: &C) -> Vec<i64> {
    grant::Entity::find()
        .select_only()
        .column(grant::Column::PersonId)
        .order_by_asc(grant::Column::PersonId)
        .into_tuple()
        .all(db)
        .await
        .unwrap()
}

#[tokio::test]
async fn fixture_uses_canonical_administrator_role() {
    let fixture = Fixture::new().await;
    let transaction = access::begin(&fixture.runtime, &fixture.scope(), None)
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
    let actor_membership = membership(
        &fixture.admin,
        fixture.other.household.id,
        fixture.account.person_id,
        "member",
    )
    .await;
    legacy_grant(
        &fixture.admin,
        &actor_membership,
        fixture.other.account.person_id,
        "view",
        "carer",
    )
    .await;
    let transaction = access::begin(
        &fixture.runtime,
        &scope(fixture.account.id, fixture.other.household.id),
        None,
    )
    .await
    .unwrap();
    assert_eq!(transaction.scope().actor.account_id, fixture.account.id);
    assert_eq!(
        transaction.membership().person_id,
        fixture.account.person_id
    );
    assert_eq!(
        transaction.membership().household_id,
        fixture.other.household.id
    );
    access::require_person_access(
        &transaction,
        fixture.other.account.person_id,
        PersonAccess::View,
    )
    .await
    .unwrap();
    access::require_person_access(&transaction, fixture.account.person_id, PersonAccess::View)
        .await
        .unwrap();
    transaction.rollback().await.unwrap();
    assert!(visible_granted_people(&fixture.runtime).await.is_empty());
    fixture.close().await;
}

#[tokio::test]
async fn begin_rejects_unknown_and_locked_accounts() {
    let fixture = Fixture::new().await;
    let unknown_account = accounts::ActiveModel {
        person_id: Set(fixture.accountless.id),
        email: Set(format!("unknown-{}@example.test", uuid::Uuid::new_v4())),
        status: Set(2),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
    accounts::Entity::delete_by_id(unknown_account.id)
        .exec(&fixture.admin)
        .await
        .unwrap();
    assert!(matches!(
        access::begin(
            &fixture.runtime,
            &scope(unknown_account.id, fixture.household.id),
            None
        )
        .await,
        Err(OperationError::Unauthenticated)
    ));
    fixture.lock().await;
    assert!(matches!(
        access::begin(&fixture.runtime, &fixture.scope(), None).await,
        Err(OperationError::Unauthenticated)
    ));
    assert!(visible_granted_people(&fixture.runtime).await.is_empty());
    fixture.close().await;
}

#[tokio::test]
async fn begin_rejects_inactive_households() {
    let fixture = Fixture::new().await;
    for (status, lifecycle) in [("archived", "active"), ("active", "held")] {
        let mut household = fixture.household.clone().into_active_model();
        household.status = Set(status.into());
        household.lifecycle_state = Set(lifecycle.into());
        household.update(&fixture.admin).await.unwrap();
        assert!(matches!(
            access::begin(&fixture.runtime, &fixture.scope(), None).await,
            Err(OperationError::Forbidden)
        ));
        assert!(visible_granted_people(&fixture.runtime).await.is_empty());
    }
    fixture.close().await;
}

#[tokio::test]
async fn pool_reuse_does_not_leak_household() {
    let fixture = Fixture::new().await;
    let first = access::begin(&fixture.runtime, &fixture.scope(), None)
        .await
        .unwrap();
    assert_eq!(
        visible_granted_people(first.transaction()).await,
        vec![fixture.own_grant.person_id]
    );
    first.commit().await.unwrap();
    assert!(visible_granted_people(&fixture.runtime).await.is_empty());
    let second = access::begin(&fixture.runtime, &fixture.other.scope(), None)
        .await
        .unwrap();
    assert_eq!(
        visible_granted_people(second.transaction()).await,
        vec![fixture.other.account.person_id]
    );
    second.rollback().await.unwrap();
    assert!(visible_granted_people(&fixture.runtime).await.is_empty());
    fixture.close().await;
}

#[tokio::test]
async fn actor_does_not_authorize_another_household() {
    let fixture = Fixture::new().await;
    assert!(matches!(
        access::begin(
            &fixture.runtime,
            &scope(fixture.account.id, fixture.other.household.id),
            None
        )
        .await,
        Err(OperationError::Forbidden)
    ));
    assert!(visible_granted_people(&fixture.runtime).await.is_empty());
    fixture.close().await;
}

async fn seed_accountless_manage_grant(fixture: &Fixture) {
    membership(
        &fixture.admin,
        fixture.household.id,
        fixture.accountless.id,
        "member",
    )
    .await;
    legacy_grant(
        &fixture.admin,
        &fixture.membership,
        fixture.accountless.id,
        "manage",
        "carer",
    )
    .await;
    assert_eq!(
        accounts::Entity::find()
            .filter(accounts::Column::PersonId.eq(fixture.accountless.id))
            .count(&fixture.admin)
            .await
            .unwrap(),
        0
    );
}

async fn assert_person_access_controls(
    transaction: &access::TenantTransaction,
    fixture: &Fixture,
    other_manage: bool,
) {
    use med_tracker::models::care::people;
    for request in [
        PersonAccess::View,
        PersonAccess::Record,
        PersonAccess::Manage,
    ] {
        assert_eq!(
            access::require_person_access(transaction, fixture.account.person_id, request).await,
            Ok(()),
            "canonical self {request:?}",
        );
        assert_eq!(
            access::require_person_access(transaction, fixture.accountless.id, request).await,
            if request == PersonAccess::Manage && !other_manage {
                Err(OperationError::Forbidden)
            } else {
                Ok(())
            },
            "accountless participant {request:?}, other Manage allowed: {other_manage}",
        );
        assert_eq!(
            access::require_person_access(transaction, fixture.other.account.person_id, request)
                .await,
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
    let mut ids = ids;
    ids.sort_unstable();
    let mut expected = vec![fixture.account.person_id, fixture.accountless.id];
    expected.sort_unstable();
    assert_eq!(ids, expected);
}

#[tokio::test]
async fn person_access_ignores_legacy_grants_before_and_after_revocation() {
    let fixture = Fixture::new().await;
    seed_accountless_manage_grant(&fixture).await;
    let transaction = access::begin(&fixture.runtime, &fixture.scope(), None)
        .await
        .unwrap();
    assert_person_access_controls(&transaction, &fixture, false).await;
    grant::Entity::update_many()
        .col_expr(
            grant::Column::RevokedAt,
            sea_orm::sea_query::Expr::val(now()),
        )
        .filter(grant::Column::HouseholdId.eq(fixture.household.id))
        .exec(&fixture.admin)
        .await
        .unwrap();
    assert_person_access_controls(&transaction, &fixture, false).await;
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
async fn cedar_person_access_never_uses_legacy_grant_level_or_expiry() {
    let fixture = Fixture::new().await;
    seed_accountless_manage_grant(&fixture).await;
    let transaction = access::begin(&fixture.runtime, &fixture.scope(), None)
        .await
        .unwrap();
    assert_person_access_controls(&transaction, &fixture, false).await;
    for level in ["view", "record", "manage"] {
        grant::Entity::update_many()
            .col_expr(
                grant::Column::AccessLevel,
                sea_orm::sea_query::Expr::val(level),
            )
            .filter(grant::Column::HouseholdId.eq(fixture.household.id))
            .exec(&fixture.admin)
            .await
            .unwrap();
        assert_person_access_controls(&transaction, &fixture, false).await;
    }
    grant::Entity::update_many()
        .col_expr(
            grant::Column::ExpiresAt,
            sea_orm::sea_query::Expr::val(now() - chrono::Duration::seconds(1)),
        )
        .filter(grant::Column::HouseholdId.eq(fixture.household.id))
        .exec(&fixture.admin)
        .await
        .unwrap();
    assert_person_access_controls(&transaction, &fixture, false).await;
    grant::Entity::update_many()
        .col_expr(
            grant::Column::ExpiresAt,
            sea_orm::sea_query::Expr::val(now() + chrono::Duration::days(1)),
        )
        .filter(grant::Column::HouseholdId.eq(fixture.household.id))
        .exec(&fixture.admin)
        .await
        .unwrap();
    assert_person_access_controls(&transaction, &fixture, false).await;
    transaction.rollback().await.unwrap();
    fixture.close().await;
}

#[tokio::test]
async fn shared_recheck_rejects_current_account_lockout() {
    let fixture = Fixture::new().await;
    let transaction = access::begin(&fixture.runtime, &fixture.scope(), None)
        .await
        .unwrap();
    fixture.lock().await;
    let result = access::require_person_access(
        &transaction,
        fixture.account.person_id,
        PersonAccess::Manage,
    )
    .await;
    transaction.rollback().await.unwrap();
    fixture.close().await;
    assert_eq!(result, Err(OperationError::Unauthenticated));
}

#[tokio::test]
async fn shared_recheck_rejects_membership_role_change() {
    let fixture = Fixture::new().await;
    let transaction = access::begin(&fixture.runtime, &fixture.scope(), None)
        .await
        .unwrap();
    set_role(&fixture, "member").await;
    let result = access::require_person_access(
        &transaction,
        fixture.account.person_id,
        PersonAccess::Manage,
    )
    .await;
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

#[path = "tenant_access/global_person.rs"]
mod global_person;
