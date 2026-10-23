use super::*;
use crate::models::{
    entities::{account, household, membership, person, user},
    identity::signup::SignupInvitationContext,
};
use loco_rs::model::TenantQueryExt;
use sea_orm::{DbBackend, QuerySelect, Statement};

#[derive(Debug)]
pub(crate) enum SignupInvitationError {
    InvitationUnavailable,
    Unavailable,
    InvalidKey,
}

impl From<OperationError> for SignupInvitationError {
    fn from(error: OperationError) -> Self {
        match error {
            OperationError::Validation { .. } => Self::InvitationUnavailable,
            OperationError::Unavailable => Self::Unavailable,
            _ => Self::InvalidKey,
        }
    }
}

impl From<sea_orm::DbErr> for SignupInvitationError {
    fn from(error: sea_orm::DbErr) -> Self {
        Self::from(OperationError::from(error))
    }
}

pub(crate) async fn signup_invitation(
    transaction: &impl sea_orm::ConnectionTrait,
    token: &str,
) -> Result<household_invitation::Model, SignupInvitationError> {
    if token.trim().is_empty() {
        return Err(SignupInvitationError::InvitationUnavailable);
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
        .ok_or(SignupInvitationError::InvitationUnavailable)?;
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
        .ok_or(SignupInvitationError::InvitationUnavailable)?;
    if home.status != "active" || home.lifecycle_state != "active" {
        return Err(SignupInvitationError::InvitationUnavailable);
    }
    let invitation = household_invitation::Entity::find_by_id(invitation.id)
        .in_tenant(home.id)
        .lock_exclusive()
        .one(transaction)
        .await?
        .ok_or(SignupInvitationError::InvitationUnavailable)?;
    if invitation.accepted_at.is_some()
        || invitation.revoked_at.is_some()
        || invitation.expires_at <= Utc::now().naive_utc()
        || invitation.token_digest != digest
    {
        return Err(SignupInvitationError::InvitationUnavailable);
    }
    Ok(invitation)
}

pub(crate) async fn accept_signup(
    transaction: &impl sea_orm::ConnectionTrait,
    context: &SignupInvitationContext,
    token: &str,
    request_id: &str,
) -> Result<membership::Model, SignupInvitationError> {
    if token.trim().is_empty() {
        return Err(SignupInvitationError::InvitationUnavailable);
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
        return Err(SignupInvitationError::InvitationUnavailable);
    }
    let owner = account::Entity::find_by_id(context.account_id())
        .one(transaction)
        .await?
        .ok_or(SignupInvitationError::InvitationUnavailable)?;
    let person = person::Entity::find_by_id(context.person_id())
        .filter(person::Column::Id.eq(owner.person_id))
        .one(transaction)
        .await?
        .ok_or(SignupInvitationError::InvitationUnavailable)?;
    let user = user::Entity::find_by_id(context.user_id())
        .filter(user::Column::PersonId.eq(person.id))
        .filter(user::Column::Active.eq(true))
        .one(transaction)
        .await?;
    if owner.status != 1 || owner.email != context.email() || user.is_none() {
        return Err(SignupInvitationError::InvitationUnavailable);
    }
    if membership::Entity::find()
        .in_tenant(household_id)
        .filter(membership::Column::PersonId.eq(owner.person_id))
        .one(transaction)
        .await?
        .is_some()
    {
        return Err(SignupInvitationError::InvitationUnavailable);
    }
    let inviter = membership::Entity::find_by_id(invitation.invited_by_membership_id)
        .in_tenant(household_id)
        .lock_exclusive()
        .one(transaction)
        .await?
        .ok_or(SignupInvitationError::InvitationUnavailable)?;
    super::authority::authorize(transaction, &inviter, &invitation)
        .await
        .map_err(SignupInvitationError::from)?;
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
        super::acceptance_effects::PersonOrigin::New {
            user_id: context.user_id(),
        },
    )
    .await
    .map_err(SignupInvitationError::from)
}
