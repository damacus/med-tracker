use super::*;
use crate::models::{
    entities::{account, household, membership, person, user},
    identity::signup::SignupInvitationContext,
};
use sea_orm::{ConnectionTrait, DatabaseTransaction, DbBackend, QuerySelect, Statement};

pub(crate) async fn signup_invitation(
    transaction: &DatabaseTransaction,
    token: &str,
) -> Result<household_invitation::Model, OperationError> {
    if token.trim().is_empty() {
        return Err(super::acceptance::unavailable());
    }
    let digest = tokens::digest(token);
    transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT set_config('med_tracker.current_invitation_token_digest',$1,true)",
            [digest.clone().into()],
        ))
        .await?;
    let invitation = household_invitation::Entity::find()
        .filter(household_invitation::Column::TokenDigest.eq(&digest))
        .filter(household_invitation::Column::AcceptedAt.is_null())
        .filter(household_invitation::Column::RevokedAt.is_null())
        .filter(household_invitation::Column::ExpiresAt.gt(Utc::now().naive_utc()))
        .one(transaction)
        .await?
        .ok_or_else(super::acceptance::unavailable)?;
    transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT set_config('med_tracker.current_household_id',$1,true)",
            [invitation.household_id.to_string().into()],
        ))
        .await?;
    let home = household::Entity::find_by_id(invitation.household_id)
        .lock_exclusive()
        .one(transaction)
        .await?
        .ok_or_else(super::acceptance::unavailable)?;
    if home.status != "active" || home.lifecycle_state != "active" {
        return Err(super::acceptance::unavailable());
    }
    let invitation = household_invitation::Entity::find_by_id(invitation.id)
        .filter(household_invitation::Column::HouseholdId.eq(home.id))
        .lock_exclusive()
        .one(transaction)
        .await?
        .ok_or_else(super::acceptance::unavailable)?;
    if invitation.accepted_at.is_some()
        || invitation.revoked_at.is_some()
        || invitation.expires_at <= Utc::now().naive_utc()
        || invitation.token_digest != digest
    {
        return Err(super::acceptance::unavailable());
    }
    Ok(invitation)
}

pub async fn accept_signup(
    transaction: &DatabaseTransaction,
    context: &SignupInvitationContext,
    token: &str,
    request_id: &str,
) -> Result<membership::Model, OperationError> {
    if token.trim().is_empty() {
        return Err(super::acceptance::unavailable());
    }
    transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT set_config('med_tracker.current_account_id',$1,true)",
            [context.account_id().to_string().into()],
        ))
        .await?;
    let invitation = signup_invitation(transaction, token).await?;
    let household_id = invitation.household_id;
    if invitation.email != context.email().trim().to_lowercase() {
        return Err(super::acceptance::unavailable());
    }
    let owner = account::Entity::find_by_id(context.account_id())
        .one(transaction)
        .await?
        .ok_or_else(super::acceptance::unavailable)?;
    let person = person::Entity::find_by_id(context.person_id())
        .filter(person::Column::HouseholdId.eq(household_id))
        .filter(person::Column::AccountId.eq(owner.id))
        .one(transaction)
        .await?
        .ok_or_else(super::acceptance::unavailable)?;
    let user = user::Entity::find_by_id(context.user_id())
        .filter(user::Column::PersonId.eq(person.id))
        .filter(user::Column::Active.eq(true))
        .one(transaction)
        .await?;
    if owner.status != 1 || owner.email != context.email() || user.is_none() {
        return Err(super::acceptance::unavailable());
    }
    if membership::Entity::find()
        .filter(membership::Column::HouseholdId.eq(household_id))
        .filter(membership::Column::AccountId.eq(owner.id))
        .one(transaction)
        .await?
        .is_some()
    {
        return Err(super::acceptance::unavailable());
    }
    let inviter = membership::Entity::find_by_id(invitation.invited_by_membership_id)
        .filter(membership::Column::HouseholdId.eq(household_id))
        .lock_exclusive()
        .one(transaction)
        .await?
        .ok_or_else(super::acceptance::unavailable)?;
    if !crate::models::authorization::household_manager(&inviter, household_id) {
        return Err(super::acceptance::unavailable());
    }
    let actor = super::acceptance::AcceptanceActor {
        account_id: owner.id,
        person_id: person.id,
        email: owner.email,
        provenance: None,
    };
    super::acceptance_effects::apply_to_person(
        transaction,
        &actor,
        person,
        &inviter,
        invitation,
        request_id,
    )
    .await
}
