use loco_rs::model::TenantQueryExt;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, QueryTrait, Statement, TransactionTrait, sea_query::Expr,
};

use super::{
    authorization,
    entities::{account, account_lockout, household, medication, membership, person, schedule},
    errors::OperationError,
    ids::{AccountId, HouseholdId, PersonId},
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
    person_id: PersonId,
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

    pub fn person_id(&self) -> PersonId {
        self.person_id
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
        let (membership, person_id) = verify_membership(&transaction, scope).await?;
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
        Ok::<_, OperationError>((membership, person_id))
    }
    .await;
    match result {
        Ok((membership, person_id)) => Ok(TenantTransaction {
            transaction,
            scope: scope.clone(),
            membership,
            person_id,
            settings: settings.cloned(),
        }),
        Err(error) => {
            transaction.rollback().await?;
            Err(error)
        }
    }
}

pub async fn lock_household(transaction: &TenantTransaction) -> Result<(), OperationError> {
    recheck(transaction).await?;
    transaction
        .transaction()
        .execute_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT med_tracker.lock_current_household()",
        ))
        .await?;
    recheck(transaction).await
}

pub async fn require_participation(
    transaction: &TenantTransaction,
    person_id: i64,
) -> Result<(), OperationError> {
    if participates(transaction, person_id).await? {
        Ok(())
    } else {
        Err(OperationError::Forbidden)
    }
}

pub(crate) fn can_manage_household(transaction: &TenantTransaction) -> bool {
    authorization::household_manager(transaction.membership(), transaction.scope().household_id)
}

pub(crate) async fn participates(
    transaction: &TenantTransaction,
    person_id: i64,
) -> Result<bool, OperationError> {
    recheck(transaction).await?;
    Ok(
        person_scope(HouseholdId::from_database(transaction.scope.household_id))
            .filter(person::Column::Id.eq(person_id))
            .one(transaction.transaction())
            .await?
            .is_some(),
    )
}

pub(crate) async fn has_participants(
    transaction: &TenantTransaction,
) -> Result<bool, OperationError> {
    recheck(transaction).await?;
    Ok(
        person_scope(HouseholdId::from_database(transaction.scope.household_id))
            .one(transaction.transaction())
            .await?
            .is_some(),
    )
}

pub(crate) async fn current_people(
    transaction: &TenantTransaction,
) -> Result<Vec<person::Model>, OperationError> {
    recheck(transaction).await?;
    Ok(
        person_scope(HouseholdId::from_database(transaction.scope.household_id))
            .order_by_asc(person::Column::Id)
            .all(transaction.transaction())
            .await?,
    )
}

pub(crate) fn person_scope(household_id: HouseholdId) -> sea_orm::Select<person::Entity> {
    person::Entity::find()
        .filter(person::Column::Id.in_subquery(participating_people(household_id)))
}

fn participating_people(household_id: HouseholdId) -> sea_orm::sea_query::SelectStatement {
    membership::Entity::find()
        .select_only()
        .column(membership::Column::PersonId)
        .in_tenant(household_id.get())
        .filter(membership::Column::Status.eq("active"))
        .filter(membership::Column::RevokedAt.is_null())
        .into_query()
}

pub async fn recheck(transaction: &TenantTransaction) -> Result<(), OperationError> {
    if !authorization::ready() {
        return Err(OperationError::Forbidden);
    }
    let (membership, person_id) =
        verify_membership(transaction.transaction(), transaction.scope()).await?;
    if membership.id != transaction.membership.id
        || membership.permissions_version != transaction.membership.permissions_version
        || membership.role != transaction.membership.role
        || person_id != transaction.person_id
    {
        return Err(OperationError::Forbidden);
    }
    Ok(())
}

pub fn medication_scope(transaction: &TenantTransaction) -> sea_orm::Select<medication::Entity> {
    let query = medication::Entity::find().in_tenant(transaction.scope().household_id);
    if authorization::ready() {
        query
    } else {
        query.filter(Expr::cust("FALSE"))
    }
}

pub(crate) fn schedule_scope(transaction: &TenantTransaction) -> sea_orm::Select<schedule::Entity> {
    let household_id = transaction.scope().household_id;
    let query = schedule::Entity::find()
        .in_tenant(household_id)
        .filter(schedule::Column::RetiredAt.is_null());
    if !authorization::ready() {
        return query.filter(Expr::cust("FALSE"));
    }
    query.filter(schedule::Column::PersonId.in_subquery(participating_people(
        HouseholdId::from_database(household_id),
    )))
}

pub(crate) fn granted_people(member: &membership::Model) -> sea_orm::sea_query::SelectStatement {
    participating_people(HouseholdId::from_database(member.household_id))
}

pub(crate) async fn verify_membership(
    transaction: &DatabaseTransaction,
    scope: &HouseholdScope,
) -> Result<(membership::Model, PersonId), OperationError> {
    let person_id = verify_account(
        transaction,
        AccountId::from_database(scope.actor.account_id),
    )
    .await?;
    let household = household::Entity::find_by_id(scope.household_id)
        .one(transaction)
        .await?
        .ok_or(OperationError::Forbidden)?;
    if household.status != "active" || household.lifecycle_state != "active" {
        return Err(OperationError::Forbidden);
    }
    let membership = membership::Entity::find()
        .filter(membership::Column::PersonId.eq(person_id.get()))
        .in_tenant(scope.household_id)
        .filter(membership::Column::Status.eq("active"))
        .filter(membership::Column::RevokedAt.is_null())
        .one(transaction)
        .await?
        .ok_or(OperationError::Forbidden)?;
    Ok((membership, person_id))
}

pub(crate) async fn verify_account_actor(
    transaction: &DatabaseTransaction,
    account_id: i64,
) -> Result<PersonId, OperationError> {
    verify_account(transaction, AccountId::from_database(account_id)).await
}

async fn verify_account(
    transaction: &DatabaseTransaction,
    account_id: AccountId,
) -> Result<PersonId, OperationError> {
    let account = account::Entity::find_by_id(account_id.get())
        .one(transaction)
        .await?
        .filter(|record| record.status == 2 && record.disabled_at.is_none())
        .ok_or(OperationError::Unauthenticated)?;
    if account_lockout::Entity::find_by_id(account.id)
        .filter(account_lockout::Column::Deadline.gt(chrono::Utc::now().naive_utc()))
        .one(transaction)
        .await?
        .is_some()
    {
        return Err(OperationError::Unauthenticated);
    }
    Ok(PersonId::from_database(account.person_id))
}
