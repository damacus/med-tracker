pub(crate) mod person_queries;

use loco_rs::model::TenantQueryExt;
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend,
    EntityTrait, QueryFilter, QueryOrder, QuerySelect, QueryTrait, Statement, TransactionTrait,
    sea_query::Expr,
};

use super::{
    authorization,
    entities::{
        account, account_lockout, household, medication, membership, person, person_medication,
        schedule,
    },
    errors::OperationError,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Actor {
    pub account_id: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HouseholdScope {
    pub actor: Actor,
    pub household_id: i64,
    pub request_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PersonAccess {
    View,
    Record,
    Manage,
}

pub struct TenantTransaction {
    transaction: DatabaseTransaction,
    scope: HouseholdScope,
    membership: membership::Model,
    settings: Option<serde_json::Value>,
}

impl TenantTransaction {
    pub fn transaction(&self) -> &DatabaseTransaction {
        &self.transaction
    }

    pub fn scope(&self) -> &HouseholdScope {
        &self.scope
    }

    pub fn membership(&self) -> &membership::Model {
        &self.membership
    }

    pub fn settings(&self) -> Option<&serde_json::Value> {
        self.settings.as_ref()
    }

    pub async fn commit(self) -> Result<(), OperationError> {
        self.transaction.commit().await.map_err(Into::into)
    }

    pub async fn rollback(self) -> Result<(), OperationError> {
        self.transaction.rollback().await.map_err(Into::into)
    }
}

pub async fn begin(
    db: &DatabaseConnection,
    scope: &HouseholdScope,
    settings: Option<&serde_json::Value>,
) -> Result<TenantTransaction, OperationError> {
    if scope.actor.account_id <= 0 {
        return Err(OperationError::Unauthenticated);
    }
    let transaction = db.begin().await?;
    let result = async {
        transaction
            .execute_unprepared("SET LOCAL ROLE med_tracker_app")
            .await?;
        transaction
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT set_config('med_tracker.current_account_id', $1, true),
                    set_config('med_tracker.current_household_id', '', true),
                    set_config('med_tracker.current_membership_id', '', true),
                    set_config('med_tracker.current_invitation_token_digest', '', true)",
                [scope.actor.account_id.to_string().into()],
            ))
            .await?;
        let membership = verify_membership(&transaction, scope).await?;
        transaction
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT set_config('med_tracker.current_household_id', $1, true),
                    set_config('med_tracker.current_membership_id', $2, true)",
                [
                    scope.household_id.to_string().into(),
                    membership.id.to_string().into(),
                ],
            ))
            .await?;
        Ok::<_, OperationError>(membership)
    }
    .await;
    match result {
        Ok(membership) => Ok(TenantTransaction {
            transaction,
            scope: scope.clone(),
            membership,
            settings: settings.cloned(),
        }),
        Err(error) => {
            transaction.rollback().await?;
            Err(error)
        }
    }
}

pub async fn require_person_access(
    transaction: &TenantTransaction,
    person_id: i64,
    access: PersonAccess,
) -> Result<(), OperationError> {
    if can_access_person(transaction, person_id, access).await? {
        Ok(())
    } else {
        Err(OperationError::Forbidden)
    }
}

pub(crate) fn can_manage_household(transaction: &TenantTransaction) -> bool {
    authorization::household_manager(transaction.membership(), transaction.scope().household_id)
}

pub(crate) async fn can_access_person(
    transaction: &TenantTransaction,
    person_id: i64,
    access: PersonAccess,
) -> Result<bool, OperationError> {
    recheck(transaction).await?;
    let membership = transaction.membership();
    let Some(subject) = person_queries::household_people(transaction.scope())
        .filter(person::Column::Id.eq(person_id))
        .one(transaction.transaction())
        .await?
    else {
        return Ok(false);
    };
    if matches!(access, PersonAccess::View | PersonAccess::Record) {
        return Ok(authorization::participating_person_access(
            membership, &subject, access,
        ));
    }
    if authorization::own_person_manage(membership, &subject) {
        return Ok(true);
    }
    if super::care::guardian_declarations::manageable_subjects(transaction)
        .await?
        .contains(&person_id)
    {
        return Ok(true);
    }
    Ok(false)
}

pub(crate) async fn has_person_access(
    transaction: &TenantTransaction,
    access: PersonAccess,
) -> Result<bool, OperationError> {
    recheck(transaction).await?;
    if matches!(access, PersonAccess::View | PersonAccess::Record) {
        return Ok(person_queries::household_people(transaction.scope())
            .one(transaction.transaction())
            .await?
            .is_some());
    }
    let own = person_queries::household_people(transaction.scope())
        .filter(person::Column::Id.eq(transaction.membership().person_id))
        .one(transaction.transaction())
        .await?;
    if own
        .is_some_and(|subject| authorization::own_person_manage(transaction.membership(), &subject))
    {
        return Ok(true);
    }
    if !super::care::guardian_declarations::manageable_subjects(transaction)
        .await?
        .is_empty()
    {
        return Ok(true);
    }
    Ok(false)
}

pub(crate) async fn people_with_access(
    transaction: &TenantTransaction,
    access: PersonAccess,
) -> Result<Vec<person::Model>, OperationError> {
    recheck(transaction).await?;
    if matches!(access, PersonAccess::View | PersonAccess::Record) {
        return Ok(person_queries::household_people(transaction.scope())
            .order_by_asc(person::Column::Name)
            .all(transaction.transaction())
            .await?);
    }
    let declared = super::care::guardian_declarations::manageable_subjects(transaction).await?;
    let mut people = person_queries::household_people(transaction.scope())
        .filter(
            person::Column::Id.is_in(
                std::iter::once(transaction.membership().person_id)
                    .chain(declared.iter().copied())
                    .collect::<Vec<_>>(),
            ),
        )
        .order_by_asc(person::Column::Name)
        .all(transaction.transaction())
        .await?;
    people.retain(|subject| {
        authorization::own_person_manage(transaction.membership(), subject)
            || declared.contains(&subject.id)
    });
    Ok(people)
}

pub async fn recheck(transaction: &TenantTransaction) -> Result<(), OperationError> {
    if !authorization::ready() {
        return Err(OperationError::Forbidden);
    }
    let membership = verify_membership(transaction.transaction(), transaction.scope()).await?;
    if membership.id != transaction.membership.id
        || membership.permissions_version != transaction.membership.permissions_version
        || membership.role != transaction.membership.role
    {
        return Err(OperationError::Forbidden);
    }
    Ok(())
}

pub fn medication_scope(transaction: &TenantTransaction) -> sea_orm::Select<medication::Entity> {
    let membership = transaction.membership();
    let household_id = transaction.scope().household_id;
    let query = medication::Entity::find().in_tenant(household_id);
    if !authorization::ready() {
        return query.filter(Expr::cust("FALSE"));
    }
    if can_manage_household(transaction) {
        return query;
    }
    let granted_schedules = schedule::Entity::find()
        .select_only()
        .column(schedule::Column::MedicationId)
        .in_tenant(household_id)
        .filter(schedule::Column::PersonId.in_subquery(participating_people(membership)))
        .into_query();
    let granted_assignments = person_medication::Entity::find()
        .select_only()
        .column(person_medication::Column::MedicationId)
        .in_tenant(household_id)
        .filter(person_medication::Column::PersonId.in_subquery(participating_people(membership)))
        .into_query();
    let linked_schedules = schedule::Entity::find()
        .select_only()
        .column(schedule::Column::MedicationId)
        .in_tenant(household_id)
        .into_query();
    let linked_assignments = person_medication::Entity::find()
        .select_only()
        .column(person_medication::Column::MedicationId)
        .in_tenant(household_id)
        .into_query();
    query.filter(
        Condition::any()
            .add(medication::Column::Id.in_subquery(granted_schedules))
            .add(medication::Column::Id.in_subquery(granted_assignments))
            .add(
                Condition::all()
                    .add(medication::Column::CreatedByMembershipId.eq(membership.id))
                    .add(medication::Column::Id.not_in_subquery(linked_schedules))
                    .add(medication::Column::Id.not_in_subquery(linked_assignments)),
            ),
    )
}

pub(crate) fn schedule_scope(transaction: &TenantTransaction) -> sea_orm::Select<schedule::Entity> {
    let household_id = transaction.scope().household_id;
    let query = schedule::Entity::find()
        .in_tenant(household_id)
        .filter(schedule::Column::RetiredAt.is_null());
    let query = query.filter(schedule::Column::ExecutionEndedAt.is_null());
    if !authorization::ready() {
        return query.filter(Expr::cust("FALSE"));
    }
    query.filter(
        schedule::Column::PersonId.in_subquery(participating_people(transaction.membership())),
    )
}

pub(crate) fn participating_people(
    membership: &membership::Model,
) -> sea_orm::sea_query::SelectStatement {
    person_queries::household_participants(membership.household_id)
        .select_only()
        .column(person::Column::Id)
        .into_query()
}

pub(crate) async fn verify_membership(
    transaction: &DatabaseTransaction,
    scope: &HouseholdScope,
) -> Result<membership::Model, OperationError> {
    verify_account(transaction, scope.actor.account_id).await?;
    let household = household::Entity::find_by_id(scope.household_id)
        .one(transaction)
        .await?
        .ok_or(OperationError::NotFound)?;
    if household.status != "active" || household.lifecycle_state != "active" {
        return Err(OperationError::Forbidden);
    }
    let membership = membership::Entity::find()
        .filter(
            membership::Column::PersonId.in_subquery(
                account::Entity::find()
                    .select_only()
                    .column(account::Column::PersonId)
                    .filter(account::Column::Id.eq(scope.actor.account_id))
                    .into_query(),
            ),
        )
        .in_tenant(scope.household_id)
        .filter(membership::Column::Status.eq("active"))
        .filter(membership::Column::RevokedAt.is_null())
        .one(transaction)
        .await?
        .ok_or(OperationError::Forbidden)?;
    Ok(membership)
}

pub(crate) async fn verify_account_actor(
    transaction: &DatabaseTransaction,
    account_id: i64,
) -> Result<i64, OperationError> {
    verify_account(transaction, account_id).await?;
    let person = person_queries::account_person(Actor { account_id })
        .one(transaction)
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    Ok(person.id)
}

pub(crate) async fn verify_account_enabled(
    transaction: &impl ConnectionTrait,
    account_id: i64,
) -> Result<account::Model, OperationError> {
    account::Entity::find_by_id(account_id)
        .filter(account::Column::Active.eq(true))
        .filter(account::Column::Status.is_in([1, 2]))
        .one(transaction)
        .await?
        .ok_or(OperationError::Unauthenticated)
}

pub(crate) async fn verify_account(
    transaction: &impl ConnectionTrait,
    account_id: i64,
) -> Result<(), OperationError> {
    let account = verify_account_enabled(transaction, account_id).await?;
    if account.status != 2 {
        return Err(OperationError::Unauthenticated);
    }
    if account_lockout::Entity::find_by_id(account.id)
        .filter(account_lockout::Column::Deadline.gt(chrono::Utc::now().naive_utc()))
        .one(transaction)
        .await?
        .is_some()
    {
        return Err(OperationError::Unauthenticated);
    }
    Ok(())
}
