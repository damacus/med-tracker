use super::*;
use crate::models::{
    entities::{household, membership, person},
    identity::api_session::ApiSessionPrincipal,
    identity::browser::BrowserPrincipal,
};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, QuerySelect, Statement,
    TransactionTrait,
};

pub async fn accept(
    db: &DatabaseConnection,
    principal: &ApiSessionPrincipal,
    token: &str,
    request_id: &str,
) -> Result<Value, OperationError> {
    accept_validated(db, Principal::Api(principal), token, request_id).await
}

pub async fn accept_browser(
    db: &DatabaseConnection,
    principal: &BrowserPrincipal,
    token: &str,
    request_id: &str,
) -> Result<Value, OperationError> {
    accept_validated(db, Principal::Browser(principal), token, request_id).await
}

pub(crate) struct AcceptanceActor {
    pub account_id: i64,
    pub person_id: i64,
    pub email: String,
    pub provenance: Option<crate::models::care::doses::CredentialProvenance>,
}

enum Principal<'a> {
    Api(&'a ApiSessionPrincipal),
    Browser(&'a BrowserPrincipal),
}

impl Principal<'_> {
    async fn revalidate(
        &self,
        transaction: &DatabaseTransaction,
    ) -> Result<AcceptanceActor, OperationError> {
        use crate::models::care::doses::{CredentialMethod, CredentialProvenance};
        match self {
            Self::Api(principal) => {
                let actor = principal
                    .revalidate(transaction)
                    .await
                    .map_err(authentication)?;
                Ok(AcceptanceActor {
                    account_id: actor.account_id,
                    person_id: actor.person_id,
                    email: actor.email,
                    provenance: Some(CredentialProvenance {
                        method: CredentialMethod::ApiSession,
                        reference: actor.session_id.to_string(),
                    }),
                })
            }
            Self::Browser(principal) => {
                let actor = principal
                    .revalidate_invitation(transaction)
                    .await
                    .map_err(authentication)?;
                Ok(AcceptanceActor {
                    account_id: actor.account_id,
                    person_id: actor.person_id,
                    email: actor.email,
                    provenance: Some(actor.provenance),
                })
            }
        }
    }
}

async fn accept_validated(
    db: &DatabaseConnection,
    principal: Principal<'_>,
    token: &str,
    request_id: &str,
) -> Result<Value, OperationError> {
    if token.trim().is_empty() {
        return Err(unavailable());
    }
    let transaction = db.begin().await?;
    transaction
        .execute_unprepared("SET LOCAL ROLE med_tracker_app")
        .await?;
    let actor = principal.revalidate(&transaction).await?;
    let digest = tokens::digest(token);
    transaction.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,"SELECT set_config('med_tracker.current_household_id','',true),set_config('med_tracker.current_membership_id','',true),set_config('med_tracker.current_invitation_token_digest',$1,true)",[digest.clone().into()])).await?;
    let invitation = household_invitation::Entity::find()
        .filter(household_invitation::Column::TokenDigest.eq(&digest))
        .filter(household_invitation::Column::AcceptedAt.is_null())
        .filter(household_invitation::Column::RevokedAt.is_null())
        .filter(household_invitation::Column::ExpiresAt.gt(Utc::now().naive_utc()))
        .one(&transaction)
        .await?;
    let member = if let Some(invitation) = invitation {
        context(&transaction, invitation.household_id).await?;
        if !operational(&transaction, invitation.household_id).await? {
            return Err(unavailable());
        }
        let current = principal.revalidate(&transaction).await?;
        if current.account_id != actor.account_id
            || current.provenance.as_ref().map(|value| &value.reference)
                != actor.provenance.as_ref().map(|value| &value.reference)
        {
            return Err(unavailable());
        }
        let invitation = household_invitation::Entity::find_by_id(invitation.id)
            .filter(household_invitation::Column::HouseholdId.eq(invitation.household_id))
            .lock_exclusive()
            .one(&transaction)
            .await?
            .ok_or_else(unavailable)?;
        if invitation.email != actor.email.trim().to_lowercase()
            || invitation.token_digest != digest
            || invitation.revoked_at.is_some()
            || invitation.expires_at <= Utc::now().naive_utc()
        {
            return Err(unavailable());
        }
        if invitation.accepted_at.is_some() {
            membership::Entity::find()
                .filter(membership::Column::HouseholdId.eq(invitation.household_id))
                .filter(membership::Column::AccountId.eq(actor.account_id))
                .filter(membership::Column::Status.eq("active"))
                .filter(membership::Column::RevokedAt.is_null())
                .one(&transaction)
                .await?
                .ok_or_else(unavailable)?
        } else {
            let inviter = membership::Entity::find_by_id(invitation.invited_by_membership_id)
                .filter(membership::Column::HouseholdId.eq(invitation.household_id))
                .lock_exclusive()
                .one(&transaction)
                .await?
                .ok_or_else(unavailable)?;
            if !crate::models::authorization::household_manager(&inviter, invitation.household_id) {
                return Err(unavailable());
            }
            if membership::Entity::find()
                .filter(membership::Column::HouseholdId.eq(invitation.household_id))
                .filter(membership::Column::AccountId.eq(actor.account_id))
                .one(&transaction)
                .await?
                .is_some()
            {
                return Err(unavailable());
            }
            let source = person::Entity::find_by_id(actor.person_id)
                .filter(person::Column::AccountId.eq(actor.account_id))
                .one(&transaction)
                .await?
                .ok_or_else(unavailable)?;
            super::acceptance_effects::apply(
                &transaction,
                &actor,
                &source,
                &inviter,
                invitation,
                request_id,
            )
            .await?
        }
    } else {
        retry(&transaction, &principal, &actor, &digest)
            .await?
            .ok_or_else(unavailable)?
    };
    transaction.commit().await?;
    Ok(
        json!({"data":{"household_id":member.household_id.to_string(),"membership_id":member.id.to_string(),"person_id":member.person_id.map(|id|id.to_string()),"role":member.role}}),
    )
}

pub(super) fn unavailable() -> OperationError {
    invalid("base", "Invitation unavailable")
}

async fn context(
    transaction: &DatabaseTransaction,
    household_id: i64,
) -> Result<(), OperationError> {
    transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT set_config('med_tracker.current_household_id',$1,true)",
            [household_id.to_string().into()],
        ))
        .await?;
    Ok(())
}
async fn operational(
    transaction: &DatabaseTransaction,
    household_id: i64,
) -> Result<bool, OperationError> {
    let home = household::Entity::find_by_id(household_id)
        .lock_exclusive()
        .one(transaction)
        .await?;
    Ok(home.is_some_and(|home| home.status == "active" && home.lifecycle_state == "active"))
}

async fn retry(
    transaction: &DatabaseTransaction,
    principal: &Principal<'_>,
    actor: &AcceptanceActor,
    digest: &str,
) -> Result<Option<membership::Model>, OperationError> {
    let members = membership::Entity::find()
        .filter(membership::Column::AccountId.eq(actor.account_id))
        .filter(membership::Column::Status.eq("active"))
        .filter(membership::Column::RevokedAt.is_null())
        .order_by_asc(membership::Column::HouseholdId)
        .order_by_asc(membership::Column::Id)
        .all(transaction)
        .await?;
    for member in members {
        context(transaction, member.household_id).await?;
        if !operational(transaction, member.household_id).await? {
            continue;
        }
        let current = principal.revalidate(transaction).await?;
        if current.account_id != actor.account_id
            || current.provenance.as_ref().map(|value| &value.reference)
                != actor.provenance.as_ref().map(|value| &value.reference)
        {
            return Err(unavailable());
        }
        let current = membership::Entity::find_by_id(member.id)
            .filter(membership::Column::AccountId.eq(actor.account_id))
            .filter(membership::Column::Status.eq("active"))
            .filter(membership::Column::RevokedAt.is_null())
            .lock_exclusive()
            .one(transaction)
            .await?;
        if let Some(current) = current {
            let accepted = household_invitation::Entity::find()
                .filter(household_invitation::Column::HouseholdId.eq(member.household_id))
                .filter(household_invitation::Column::TokenDigest.eq(digest))
                .filter(household_invitation::Column::Email.eq(&actor.email))
                .filter(household_invitation::Column::AcceptedAt.is_not_null())
                .filter(household_invitation::Column::RevokedAt.is_null())
                .one(transaction)
                .await?;
            if accepted.is_some() {
                return Ok(Some(current));
            }
        }
    }
    Ok(None)
}

fn authentication(error: crate::models::identity::resource::AuthenticationError) -> OperationError {
    use crate::models::identity::resource::AuthenticationError;
    match error {
        AuthenticationError::Unauthenticated => OperationError::Unauthenticated,
        AuthenticationError::Forbidden | AuthenticationError::InsufficientScope { .. } => {
            OperationError::Forbidden
        }
        AuthenticationError::Unavailable => OperationError::Unavailable,
    }
}
