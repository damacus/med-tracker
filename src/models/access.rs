use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend,
    EntityTrait, QueryFilter, QueryOrder, QuerySelect, QueryTrait, Statement, TransactionTrait,
    sea_query::{Expr, ExprTrait},
};

use super::{
    entities::{
        account, account_lockout, grant, household, medication, membership, person,
        person_medication, schedule, user,
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
    user_id: i64,
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

    pub fn user_id(&self) -> i64 {
        self.user_id
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
        let (membership, user_id) = verify_membership(&transaction, scope).await?;
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
        Ok::<_, OperationError>((membership, user_id))
    }
    .await;
    match result {
        Ok((membership, user_id)) => Ok(TenantTransaction {
            transaction,
            scope: scope.clone(),
            membership,
            user_id,
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
    recheck(transaction).await?;
    let membership = transaction.membership();
    if person::Entity::find_by_id(person_id)
        .filter(person::Column::HouseholdId.eq(transaction.scope.household_id))
        .one(transaction.transaction())
        .await?
        .is_none()
    {
        return Err(OperationError::Forbidden);
    }
    let levels = match access {
        PersonAccess::View => vec!["view", "record", "manage"],
        PersonAccess::Record => vec!["record", "manage"],
        PersonAccess::Manage => vec!["manage"],
    };
    let allowed = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(transaction.scope.household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(membership.id))
        .filter(grant::Column::PersonId.eq(person_id))
        .filter(grant::Column::AccessLevel.is_in(levels))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(
            Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(
                    Expr::col(grant::Column::ExpiresAt)
                        .gt(Expr::cust("timezone('UTC', clock_timestamp())")),
                ),
        )
        .one(transaction.transaction())
        .await?;
    allowed.map(|_| ()).ok_or(OperationError::Forbidden)
}

pub async fn recheck(transaction: &TenantTransaction) -> Result<(), OperationError> {
    let (membership, user_id) =
        verify_membership(transaction.transaction(), transaction.scope()).await?;
    if membership.id != transaction.membership.id
        || membership.permissions_version != transaction.membership.permissions_version
        || membership.role != transaction.membership.role
        || user_id != transaction.user_id
    {
        return Err(OperationError::Forbidden);
    }
    Ok(())
}

pub fn medication_scope(transaction: &TenantTransaction) -> sea_orm::Select<medication::Entity> {
    let membership = transaction.membership();
    let household_id = transaction.scope().household_id;
    let query = medication::Entity::find().filter(medication::Column::HouseholdId.eq(household_id));
    if membership.role == "owner" || membership.role == "administrator" {
        return query;
    }
    let granted_schedules = schedule::Entity::find()
        .select_only()
        .column(schedule::Column::MedicationId)
        .filter(schedule::Column::HouseholdId.eq(household_id))
        .filter(schedule::Column::PersonId.in_subquery(granted_people(membership)))
        .into_query();
    let granted_assignments = person_medication::Entity::find()
        .select_only()
        .column(person_medication::Column::MedicationId)
        .filter(person_medication::Column::HouseholdId.eq(household_id))
        .filter(person_medication::Column::PersonId.in_subquery(granted_people(membership)))
        .into_query();
    let linked_schedules = schedule::Entity::find()
        .select_only()
        .column(schedule::Column::MedicationId)
        .filter(schedule::Column::HouseholdId.eq(household_id))
        .into_query();
    let linked_assignments = person_medication::Entity::find()
        .select_only()
        .column(person_medication::Column::MedicationId)
        .filter(person_medication::Column::HouseholdId.eq(household_id))
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

fn granted_people(membership: &membership::Model) -> sea_orm::sea_query::SelectStatement {
    grant::Entity::find()
        .select_only()
        .column(grant::Column::PersonId)
        .filter(grant::Column::HouseholdId.eq(membership.household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(membership.id))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(grant::Column::AccessLevel.is_in(["view", "record", "manage"]))
        .filter(
            Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(
                    Expr::col(grant::Column::ExpiresAt)
                        .gt(Expr::cust("timezone('UTC', clock_timestamp())")),
                ),
        )
        .into_query()
}

pub(crate) async fn verify_membership(
    transaction: &DatabaseTransaction,
    scope: &HouseholdScope,
) -> Result<(membership::Model, i64), OperationError> {
    verify_account(transaction, scope.actor.account_id).await?;
    let household = household::Entity::find_by_id(scope.household_id)
        .one(transaction)
        .await?
        .ok_or(OperationError::NotFound)?;
    if household.status != "active" || household.lifecycle_state != "active" {
        return Err(OperationError::Forbidden);
    }
    let membership = membership::Entity::find()
        .filter(membership::Column::AccountId.eq(scope.actor.account_id))
        .filter(membership::Column::HouseholdId.eq(scope.household_id))
        .filter(membership::Column::Status.eq("active"))
        .filter(membership::Column::RevokedAt.is_null())
        .one(transaction)
        .await?
        .ok_or(OperationError::Forbidden)?;
    let user_id = account_user(transaction, scope.actor.account_id).await?;
    Ok((membership, user_id))
}

pub(crate) async fn verify_account_actor(
    transaction: &DatabaseTransaction,
    account_id: i64,
) -> Result<i64, OperationError> {
    verify_account(transaction, account_id).await?;
    account_user(transaction, account_id).await
}

async fn verify_account(
    transaction: &DatabaseTransaction,
    account_id: i64,
) -> Result<(), OperationError> {
    let account = account::Entity::find_by_id(account_id)
        .one(transaction)
        .await?
        .filter(|record| record.status == 2)
        .ok_or(OperationError::Unauthenticated)?;
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

async fn account_user(
    transaction: &DatabaseTransaction,
    account_id: i64,
) -> Result<i64, OperationError> {
    let person = person::Entity::find()
        .filter(person::Column::AccountId.eq(account_id))
        .order_by_asc(person::Column::Id)
        .one(transaction)
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    let user = user::Entity::find()
        .filter(user::Column::PersonId.eq(person.id))
        .filter(user::Column::Active.eq(true))
        .one(transaction)
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    Ok(user.id)
}
