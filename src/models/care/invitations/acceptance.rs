use super::*;
use crate::models::care::person_schedules::sharing::ConsentAuthority;
use crate::models::{
    entities::{household, membership, person},
    identity::api_session::ApiSessionPrincipal,
    identity::browser::BrowserPrincipal,
};
use loco_rs::model::TenantQueryExt;
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, QuerySelect, Statement,
    TransactionTrait,
};

pub async fn accept(
    db: &DatabaseConnection,
    principal: &ApiSessionPrincipal,
    token: &str,
    _consent: SharingConsent,
    request_id: &str,
    settings: Option<&serde_json::Value>,
) -> Result<Value, OperationError> {
    accept_validated(
        db,
        Principal::Api(principal),
        token,
        None,
        request_id,
        settings,
    )
    .await
}

pub async fn accept_browser(
    db: &DatabaseConnection,
    principal: &BrowserPrincipal,
    token: &str,
    _consent: SharingConsent,
    request_id: &str,
    settings: Option<&serde_json::Value>,
) -> Result<Value, OperationError> {
    accept_validated(
        db,
        Principal::Browser(principal),
        token,
        None,
        request_id,
        settings,
    )
    .await
}

pub(crate) async fn accept_dependent(
    db: &DatabaseConnection,
    principal: &ApiSessionPrincipal,
    token: &str,
    _consent: SharingConsent,
    subject: crate::models::ids::PersonId,
    request_id: &str,
    settings: Option<&serde_json::Value>,
) -> Result<Value, OperationError> {
    accept_validated(
        db,
        Principal::Api(principal),
        token,
        Some(subject),
        request_id,
        settings,
    )
    .await
}

pub(crate) async fn accept_dependent_browser(
    db: &DatabaseConnection,
    principal: &BrowserPrincipal,
    token: &str,
    _consent: SharingConsent,
    subject: crate::models::ids::PersonId,
    request_id: &str,
    settings: Option<&serde_json::Value>,
) -> Result<Value, OperationError> {
    accept_validated(
        db,
        Principal::Browser(principal),
        token,
        Some(subject),
        request_id,
        settings,
    )
    .await
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
        settings: Option<&serde_json::Value>,
    ) -> Result<AcceptanceActor, OperationError> {
        use crate::models::care::doses::{CredentialMethod, CredentialProvenance};
        match self {
            Self::Api(principal) => {
                let actor = principal
                    .revalidate(transaction, settings)
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
    dependent: Option<crate::models::ids::PersonId>,
    request_id: &str,
    settings: Option<&serde_json::Value>,
) -> Result<Value, OperationError> {
    if token.trim().is_empty() {
        return Err(unavailable());
    }
    let transaction = db.begin().await?;
    transaction
        .execute_unprepared("SET LOCAL ROLE med_tracker_app")
        .await?;
    let actor = principal.revalidate(&transaction, settings).await?;
    let subject = dependent.map_or(actor.person_id, crate::models::ids::PersonId::get);
    let authority = if let Some(dependent) = dependent {
        if dependent.get() == actor.person_id {
            return Err(OperationError::Forbidden);
        }
        let id = crate::models::care::person_schedules::sharing::guardian(&transaction, dependent)
            .await?
            .ok_or(OperationError::Forbidden)?;
        ConsentAuthority::Guardian(
            crate::models::care::guardian_declarations::DeclarationId::from_database(id),
        )
    } else {
        ConsentAuthority::SelfPerson
    };
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
        let current = principal.revalidate(&transaction, settings).await?;
        if current.account_id != actor.account_id
            || current.provenance.as_ref().map(|value| &value.reference)
                != actor.provenance.as_ref().map(|value| &value.reference)
        {
            return Err(unavailable());
        }
        let invitation = household_invitation::Entity::find_by_id(invitation.id)
            .in_tenant(invitation.household_id)
            .lock_exclusive()
            .one(&transaction)
            .await?
            .ok_or_else(unavailable)?;
        revalidate_current(
            &transaction,
            &principal,
            &actor,
            subject,
            authority,
            settings,
        )
        .await?;
        if invitation.email != actor.email.trim().to_lowercase()
            || invitation.dependent_sharing != dependent.is_some()
            || invitation.token_digest != digest
            || invitation.revoked_at.is_some()
            || invitation.expires_at <= Utc::now().naive_utc()
        {
            return Err(unavailable());
        }
        if invitation.accepted_at.is_some() {
            if !crate::models::care::person_schedules::sharing::current(
                &transaction,
                invitation.household_id,
                subject,
                invitation.id,
            )
            .await?
            {
                return Err(unavailable());
            }
            membership::Entity::find()
                .in_tenant(invitation.household_id)
                .filter(membership::Column::PersonId.eq(subject))
                .filter(membership::Column::Status.eq("active"))
                .filter(membership::Column::RevokedAt.is_null())
                .one(&transaction)
                .await?
                .ok_or_else(unavailable)?
        } else {
            let inviter = membership::Entity::find_by_id(invitation.invited_by_membership_id)
                .in_tenant(invitation.household_id)
                .lock_exclusive()
                .one(&transaction)
                .await?
                .ok_or_else(unavailable)?;
            super::authority::authorize(&transaction, &inviter, &invitation).await?;
            let existing = membership::Entity::find()
                .in_tenant(invitation.household_id)
                .filter(membership::Column::PersonId.eq(subject))
                .one(&transaction)
                .await?;
            use crate::models::entities::sharing_approval;
            let prior = sharing_approval::Entity::find()
                .filter(sharing_approval::Column::HouseholdId.eq(invitation.household_id))
                .filter(sharing_approval::Column::PersonId.eq(subject))
                .all(&transaction)
                .await?;
            if let Some(existing) = existing {
                if !["active", "revoked"].contains(&existing.status.as_str()) {
                    return Err(unavailable());
                }
            }
            for approval in prior
                .into_iter()
                .filter(|approval| approval.ended_at.is_none())
            {
                crate::models::care::person_schedules::sharing::withdraw(
                    &transaction,
                    crate::models::ids::AccountId::from_database(actor.account_id),
                    approval.id,
                    request_id,
                    actor
                        .provenance
                        .as_ref()
                        .ok_or(OperationError::Unauthenticated)?,
                )
                .await?;
            }
            let source = person::Entity::find_by_id(subject)
                .one(&transaction)
                .await?
                .ok_or_else(unavailable)?;
            let invitation_id = invitation.id;
            let member = super::acceptance_effects::apply(
                &transaction,
                &actor,
                &source,
                &inviter,
                invitation,
                request_id,
                authority,
            )
            .await?;
            let attachment = transaction.begin().await?;
            match crate::models::care::person_schedules::sharing::attach_origins(
                &attachment,
                invitation_id,
                member.household_id,
                crate::models::ids::PersonId::from_database(subject),
            )
            .await
            {
                Ok(()) => attachment.commit().await?,
                Err(error) => {
                    attachment.rollback().await?;
                    // A failed validation wait may coincide with real authority
                    // loss. Recheck that authority without inspecting database
                    // strings; otherwise preserve the original database error.
                    revalidate_current(
                        &transaction,
                        &principal,
                        &actor,
                        subject,
                        authority,
                        settings,
                    )
                    .await?;
                    return Err(error);
                }
            }
            member
        }
    } else {
        retry(
            &transaction,
            &principal,
            &actor,
            &digest,
            subject,
            authority,
            settings,
        )
        .await?
        .ok_or_else(unavailable)?
    };
    revalidate_current(
        &transaction,
        &principal,
        &actor,
        subject,
        authority,
        settings,
    )
    .await?;
    transaction.commit().await?;
    Ok(
        json!({"data":{"household_id":member.household_id.to_string(),"membership_id":member.id.to_string(),"person_id":member.person_id.to_string(),"role":member.role}}),
    )
}

async fn revalidate_current(
    transaction: &DatabaseTransaction,
    principal: &Principal<'_>,
    actor: &AcceptanceActor,
    subject: i64,
    authority: ConsentAuthority,
    settings: Option<&serde_json::Value>,
) -> Result<(), OperationError> {
    let current = principal.revalidate(transaction, settings).await?;
    if current.account_id != actor.account_id
        || current.person_id != actor.person_id
        || current.email != actor.email
        || current.provenance.as_ref().map(|value| &value.reference)
            != actor.provenance.as_ref().map(|value| &value.reference)
    {
        return Err(unavailable());
    }
    if !authority
        .current(
            transaction,
            crate::models::ids::PersonId::from_database(subject),
        )
        .await?
    {
        return Err(OperationError::Forbidden);
    }
    Ok(())
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
    subject: i64,
    authority: ConsentAuthority,
    settings: Option<&serde_json::Value>,
) -> Result<Option<membership::Model>, OperationError> {
    let accepted = household_invitation::Entity::find()
        .filter(household_invitation::Column::TokenDigest.eq(digest))
        .filter(household_invitation::Column::Email.eq(&actor.email))
        .filter(household_invitation::Column::DependentSharing.eq(authority.dependent()))
        .filter(household_invitation::Column::AcceptedAt.is_not_null())
        .filter(household_invitation::Column::RevokedAt.is_null())
        .one(transaction)
        .await?;
    let Some(accepted) = accepted else {
        return Ok(None);
    };
    context(transaction, accepted.household_id).await?;
    let members = membership::Entity::find()
        .filter(membership::Column::PersonId.eq(subject))
        .filter(membership::Column::HouseholdId.eq(accepted.household_id))
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
        let current = principal.revalidate(transaction, settings).await?;
        if !authority
            .current(
                transaction,
                crate::models::ids::PersonId::from_database(subject),
            )
            .await?
        {
            return Err(OperationError::Forbidden);
        }
        if current.account_id != actor.account_id
            || current.provenance.as_ref().map(|value| &value.reference)
                != actor.provenance.as_ref().map(|value| &value.reference)
        {
            return Err(unavailable());
        }
        let current = membership::Entity::find_by_id(member.id)
            .filter(membership::Column::PersonId.eq(subject))
            .filter(membership::Column::Status.eq("active"))
            .filter(membership::Column::RevokedAt.is_null())
            .lock_exclusive()
            .one(transaction)
            .await?;
        if let Some(current) = current {
            let accepted = household_invitation::Entity::find()
                .in_tenant(member.household_id)
                .filter(household_invitation::Column::TokenDigest.eq(digest))
                .filter(household_invitation::Column::Email.eq(&actor.email))
                .filter(household_invitation::Column::AcceptedAt.is_not_null())
                .filter(household_invitation::Column::RevokedAt.is_null())
                .one(transaction)
                .await?;
            if let Some(accepted) = accepted
                && crate::models::care::person_schedules::sharing::current(
                    transaction,
                    member.household_id,
                    subject,
                    accepted.id,
                )
                .await?
            {
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
        AuthenticationError::Forbidden
        | AuthenticationError::NotFound
        | AuthenticationError::InsufficientScope { .. } => OperationError::Forbidden,
        AuthenticationError::Unavailable => OperationError::Unavailable,
    }
}
