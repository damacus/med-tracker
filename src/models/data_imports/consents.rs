use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    IntoActiveModel, QueryFilter, QuerySelect,
};
use serde::{Deserialize, Serialize};

use super::contracts::{PersonId, RecordKind, SourceIdentity};
use crate::models::{
    _entities::data_import_consents as consent,
    access::{self, HouseholdScope, TenantTransaction},
    entities::{account, membership, person},
    errors::OperationError,
    ids::AccountId,
};

pub async fn begin_for_actor(
    db: &DatabaseConnection,
    scope: &HouseholdScope,
    settings: Option<&serde_json::Value>,
) -> Result<TenantTransaction, OperationError> {
    access::begin_with_exclusive_accounts(db, scope, settings, &[]).await
}

pub async fn begin_for_target(
    db: &DatabaseConnection,
    scope: &HouseholdScope,
    settings: Option<&serde_json::Value>,
    target: PersonId,
) -> Result<TenantTransaction, OperationError> {
    let lookup = access::begin(db, scope, settings).await?;
    if !access::can_manage_household(&lookup) || target.0 == lookup.membership().person_id {
        return Err(OperationError::Forbidden);
    }
    membership::Entity::find()
        .filter(membership::Column::HouseholdId.eq(scope.household_id))
        .filter(membership::Column::PersonId.eq(target.0))
        .filter(membership::Column::Status.eq("active"))
        .filter(membership::Column::RevokedAt.is_null())
        .one(lookup.transaction())
        .await?
        .ok_or(OperationError::Forbidden)?;
    let recipient = account::Entity::find()
        .filter(account::Column::PersonId.eq(target.0))
        .one(lookup.transaction())
        .await?
        .ok_or(OperationError::Forbidden)?
        .id;
    lookup.rollback().await?;
    let tenant = access::begin_with_exclusive_accounts(
        db,
        scope,
        settings,
        &[AccountId::from_database(recipient)],
    )
    .await?;
    let current = account::Entity::find()
        .filter(account::Column::PersonId.eq(target.0))
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Forbidden)?;
    if current.id != recipient || !access::can_manage_household(&tenant) {
        return Err(OperationError::Forbidden);
    }
    access::recheck(&tenant).await?;
    Ok(tenant)
}

pub async fn begin_for_requests(
    db: &DatabaseConnection,
    scope: &HouseholdScope,
    settings: Option<&serde_json::Value>,
    ids: &[String],
) -> Result<TenantTransaction, OperationError> {
    let lookup = access::begin(db, scope, settings).await?;
    if !access::can_manage_household(&lookup) {
        return Err(OperationError::Forbidden);
    }
    let mut bindings = Vec::with_capacity(ids.len());
    for id in ids {
        let row = consent::Entity::find_by_id(id.clone())
            .filter(consent::Column::RequesterAccountId.eq(scope.actor.account_id))
            .filter(consent::Column::RequesterMembershipId.eq(lookup.membership().id))
            .one(lookup.transaction())
            .await?
            .ok_or(OperationError::NotFound)?;
        bindings.push((id.clone(), row.recipient_account_id));
    }
    lookup.rollback().await?;
    let accounts: Vec<_> = bindings
        .iter()
        .map(|(_, account)| AccountId::from_database(*account))
        .collect();
    let tenant = access::begin_with_exclusive_accounts(db, scope, settings, &accounts).await?;
    for (id, recipient) in bindings {
        let row = consent::Entity::find_by_id(id)
            .filter(consent::Column::RequesterAccountId.eq(scope.actor.account_id))
            .filter(consent::Column::RequesterMembershipId.eq(tenant.membership().id))
            .one(tenant.transaction())
            .await?
            .ok_or(OperationError::NotFound)?;
        if row.recipient_account_id != recipient {
            return Err(OperationError::Forbidden);
        }
    }
    access::recheck(&tenant).await?;
    Ok(tenant)
}

fn require_locked(tenant: &TenantTransaction, account: i64) -> Result<(), OperationError> {
    if tenant.holds_exclusive_account(AccountId::from_database(account)) {
        Ok(())
    } else {
        Err(OperationError::Forbidden)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConsentBinding {
    pub source_sha256: String,
    pub source_person: SourceIdentity,
    pub target_person: PersonId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Pending,
    Approved,
    Rejected,
}

#[derive(Clone, Debug, Serialize)]
pub struct PersonalRequest {
    pub id: String,
    pub household_id: i64,
    pub requester_account_id: i64,
    pub binding: ConsentBinding,
    pub decision: Decision,
}

fn valid(binding: &ConsentBinding) -> bool {
    binding.source_sha256.len() == 64
        && binding
            .source_sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        && binding.source_person.kind == RecordKind::Person
        && !binding.source_person.instance.trim().is_empty()
        && !binding.source_person.id.trim().is_empty()
        && binding.target_person.0 > 0
}

fn binding(row: &consent::Model) -> Result<ConsentBinding, OperationError> {
    Ok(ConsentBinding {
        source_sha256: row.source_sha256.clone(),
        source_person: serde_json::from_value(row.source_person.clone())
            .map_err(|_| OperationError::unavailable("import_consent_decode"))?,
        target_person: PersonId(row.target_person_id),
    })
}

fn personal(row: &consent::Model) -> Result<PersonalRequest, OperationError> {
    let decision = match row.decision.as_str() {
        "pending" => Decision::Pending,
        "approved" => Decision::Approved,
        "rejected" => Decision::Rejected,
        _ => return Err(OperationError::unavailable("import_consent_decision")),
    };
    Ok(PersonalRequest {
        id: row.id.clone(),
        household_id: row.household_id,
        requester_account_id: row.requester_account_id,
        binding: binding(row)?,
        decision,
    })
}

async fn lock_account_and_membership(
    tenant: &TenantTransaction,
    account_id: i64,
    membership_id: i64,
    person_id: i64,
) -> Result<(), OperationError> {
    let account = account::Entity::find_by_id(account_id)
        .lock_shared()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Forbidden)?;
    let member = membership::Entity::find_by_id(membership_id)
        .lock_shared()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Forbidden)?;
    access::verify_account(tenant.transaction(), account_id).await?;
    if account.person_id != person_id
        || member.person_id != person_id
        || member.household_id != tenant.scope().household_id
        || member.status != "active"
        || member.revoked_at.is_some()
    {
        return Err(OperationError::Forbidden);
    }
    Ok(())
}

async fn eligible_recipient(
    tenant: &TenantTransaction,
    row: &consent::Model,
) -> Result<(), OperationError> {
    lock_account_and_membership(
        tenant,
        row.recipient_account_id,
        row.recipient_membership_id,
        row.target_person_id,
    )
    .await?;
    let subject = person::Entity::find_by_id(row.target_person_id)
        .lock_shared()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Forbidden)?;
    access::verify_account(tenant.transaction(), row.recipient_account_id).await?;
    if subject.person_type != 0
        || !subject.has_capacity
        || subject.date_of_birth.is_some_and(|birth| {
            crate::models::care::people::age(birth, Utc::now().date_naive()) < 18
        })
    {
        return Err(OperationError::Forbidden);
    }
    Ok(())
}

pub async fn request(
    tenant: &TenantTransaction,
    input: ConsentBinding,
) -> Result<String, OperationError> {
    require_locked(tenant, tenant.scope().actor.account_id)?;
    access::recheck(tenant).await?;
    if !access::can_manage_household(tenant)
        || !valid(&input)
        || input.target_person.0 == tenant.membership().person_id
    {
        return Err(OperationError::Forbidden);
    }
    lock_account_and_membership(
        tenant,
        tenant.scope().actor.account_id,
        tenant.membership().id,
        tenant.membership().person_id,
    )
    .await?;
    let member = membership::Entity::find()
        .filter(membership::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(membership::Column::PersonId.eq(input.target_person.0))
        .filter(membership::Column::Status.eq("active"))
        .filter(membership::Column::RevokedAt.is_null())
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Forbidden)?;
    let recipient = account::Entity::find()
        .filter(account::Column::PersonId.eq(input.target_person.0))
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Forbidden)?;
    require_locked(tenant, recipient.id)?;
    let row = consent::Model {
        id: uuid::Uuid::new_v4().to_string(),
        household_id: tenant.scope().household_id,
        requester_account_id: tenant.scope().actor.account_id,
        requester_membership_id: tenant.membership().id,
        recipient_account_id: recipient.id,
        recipient_membership_id: member.id,
        target_person_id: input.target_person.0,
        source_sha256: input.source_sha256,
        source_person: serde_json::to_value(input.source_person)
            .map_err(|_| OperationError::unavailable("import_consent_encode"))?,
        decision: "pending".into(),
        created_at: Utc::now().naive_utc(),
        decided_at: None,
    };
    eligible_recipient(tenant, &row).await?;
    access::recheck(tenant).await?;
    access::verify_account(tenant.transaction(), row.recipient_account_id).await?;
    let id = row.id.clone();
    row.into_active_model().insert(tenant.transaction()).await?;
    Ok(id)
}

pub async fn read(tenant: &TenantTransaction, id: &str) -> Result<PersonalRequest, OperationError> {
    require_locked(tenant, tenant.scope().actor.account_id)?;
    access::recheck(tenant).await?;
    let row = consent::Entity::find_by_id(id.to_owned())
        .filter(consent::Column::RecipientAccountId.eq(tenant.scope().actor.account_id))
        .filter(consent::Column::RecipientMembershipId.eq(tenant.membership().id))
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    eligible_recipient(tenant, &row).await?;
    access::recheck(tenant).await?;
    personal(&row)
}

pub async fn decide(
    tenant: &TenantTransaction,
    id: &str,
    expected: &ConsentBinding,
    decision: Decision,
) -> Result<PersonalRequest, OperationError> {
    require_locked(tenant, tenant.scope().actor.account_id)?;
    if decision == Decision::Pending || !valid(expected) {
        return Err(OperationError::Forbidden);
    }
    let row = consent::Entity::find_by_id(id.to_owned())
        .filter(consent::Column::RecipientAccountId.eq(tenant.scope().actor.account_id))
        .filter(consent::Column::RecipientMembershipId.eq(tenant.membership().id))
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    eligible_recipient(tenant, &row).await?;
    access::recheck(tenant).await?;
    if binding(&row)? != *expected
        || !(row.decision == "pending"
            || (row.decision == "approved" && decision == Decision::Rejected))
    {
        return Err(OperationError::Conflict {
            code: "import_consent_changed".into(),
            details: serde_json::json!({}),
        });
    }
    let mut update = row.into_active_model();
    update.decision = Set(match decision {
        Decision::Approved => "approved",
        Decision::Rejected => "rejected",
        Decision::Pending => return Err(OperationError::Forbidden),
    }
    .into());
    update.decided_at = Set(Some(Utc::now().naive_utc()));
    personal(&update.update(tenant.transaction()).await?)
}

pub async fn current(
    tenant: &TenantTransaction,
    id: &str,
    expected: &ConsentBinding,
) -> Result<bool, OperationError> {
    require_locked(tenant, tenant.scope().actor.account_id)?;
    access::recheck(tenant).await?;
    if !access::can_manage_household(tenant) || !valid(expected) {
        return Err(OperationError::Forbidden);
    }
    let row = consent::Entity::find_by_id(id.to_owned())
        .filter(consent::Column::RequesterAccountId.eq(tenant.scope().actor.account_id))
        .filter(consent::Column::RequesterMembershipId.eq(tenant.membership().id))
        .lock_shared()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    require_locked(tenant, row.recipient_account_id)?;
    lock_account_and_membership(
        tenant,
        row.requester_account_id,
        row.requester_membership_id,
        tenant.membership().person_id,
    )
    .await?;
    eligible_recipient(tenant, &row).await?;
    access::recheck(tenant).await?;
    access::verify_account(tenant.transaction(), row.recipient_account_id).await?;
    Ok(row.decision == "approved" && binding(&row)? == *expected)
}
