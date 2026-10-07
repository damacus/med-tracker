use async_trait::async_trait;
use better_auth_core::{
    AuthError, AuthResult, CreateInvitation, Invitation, InvitationStatus, store::InvitationStore,
};
use chrono::{DateTime, Utc};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use serde_json::json;

use super::super::request;
use super::{
    ClinicalStore, clinical_id, context, database_error, statement, tenant::operation_error,
};
use crate::models::{
    care::{administration, invitations},
    entities::{household_invitation, membership},
};

fn value(row: &household_invitation::Model, inviter: i64) -> Invitation {
    Invitation {
        id: row.id.to_string(),
        organization_id: row.household_id.to_string(),
        email: row.email.clone(),
        role: row.membership_role.clone(),
        inviter_id: inviter.to_string(),
        expires_at: row.expires_at.and_utc(),
        created_at: row.created_at.and_utc(),
        status: if row.accepted_at.is_some() {
            InvitationStatus::Accepted
        } else if row.revoked_at.is_some() {
            InvitationStatus::Canceled
        } else {
            InvitationStatus::Pending
        },
    }
}

impl ClinicalStore {
    async fn invitation_value(
        &self,
        transaction: &sea_orm::DatabaseTransaction,
        row: &household_invitation::Model,
    ) -> AuthResult<Invitation> {
        let found = transaction.query_one_raw(statement("SELECT COALESCE((SELECT account_id FROM public.household_memberships WHERE id=$1 AND household_id=$2),public.identity_invitation_inviter($3)) AS inviter", [row.invited_by_membership_id.into(),row.household_id.into(),row.id.into()])).await.map_err(database_error)?.ok_or_else(||AuthError::internal("Invitation owner unavailable"))?;
        let inviter: i64 = found.try_get("", "inviter").map_err(database_error)?;
        Ok(value(row, inviter))
    }

    async fn invitation_rows(
        &self,
        org: Option<i64>,
        email: Option<&str>,
        pending: bool,
    ) -> AuthResult<Vec<Invitation>> {
        let actor = request::authenticated()?;
        let transaction = self.account_transaction().await?;
        if let Some(id) = org {
            context(
                &transaction,
                "med_tracker.current_household_id",
                &id.to_string(),
            )
            .await?;
            let member = membership::Entity::find()
                .filter(membership::Column::HouseholdId.eq(id))
                .filter(membership::Column::AccountId.eq(actor.account_id))
                .one(&*transaction)
                .await
                .map_err(database_error)?
                .ok_or_else(|| AuthError::forbidden("Household access denied"))?;
            if !crate::models::authorization::household_manager(&member, id) {
                return Err(AuthError::forbidden("Household access denied"));
            }
        }
        if let Some(email) = email {
            let current = self
                .canonical_user_in(&transaction, &actor.account_id.to_string())
                .await?
                .ok_or(AuthError::Unauthenticated)?;
            if current.email.as_deref() != Some(email) {
                return Err(AuthError::forbidden("Invitation access denied"));
            }
        }
        let rows=transaction.query_all_raw(statement("SELECT i.id,i.household_id,i.email,i.membership_role,i.expires_at,i.created_at,i.accepted_at,i.revoked_at,COALESCE(m.account_id,public.identity_invitation_inviter(i.id)) AS inviter FROM public.household_invitations i LEFT JOIN public.household_memberships m ON m.id=i.invited_by_membership_id AND m.household_id=i.household_id WHERE ($1::bigint IS NULL OR i.household_id=$1) AND ($2::text IS NULL OR i.email=$2) AND (NOT $3 OR (i.accepted_at IS NULL AND i.revoked_at IS NULL AND i.expires_at>CURRENT_TIMESTAMP)) ORDER BY i.id",[org.into(),email.map(str::to_owned).into(),pending.into()])).await.map_err(database_error)?;
        let result = rows
            .into_iter()
            .map(|row| {
                let accepted: Option<chrono::NaiveDateTime> =
                    row.try_get("", "accepted_at").map_err(database_error)?;
                let revoked: Option<chrono::NaiveDateTime> =
                    row.try_get("", "revoked_at").map_err(database_error)?;
                Ok(Invitation {
                    id: row
                        .try_get::<i64>("", "id")
                        .map_err(database_error)?
                        .to_string(),
                    organization_id: row
                        .try_get::<i64>("", "household_id")
                        .map_err(database_error)?
                        .to_string(),
                    email: row.try_get("", "email").map_err(database_error)?,
                    role: row.try_get("", "membership_role").map_err(database_error)?,
                    inviter_id: row
                        .try_get::<i64>("", "inviter")
                        .map_err(database_error)?
                        .to_string(),
                    expires_at: row
                        .try_get::<chrono::NaiveDateTime>("", "expires_at")
                        .map_err(database_error)?
                        .and_utc(),
                    created_at: row
                        .try_get::<chrono::NaiveDateTime>("", "created_at")
                        .map_err(database_error)?
                        .and_utc(),
                    status: if accepted.is_some() {
                        InvitationStatus::Accepted
                    } else if revoked.is_some() {
                        InvitationStatus::Canceled
                    } else {
                        InvitationStatus::Pending
                    },
                })
            })
            .collect::<AuthResult<Vec<_>>>()?;
        transaction.commit().await.map_err(database_error)?;
        Ok(result)
    }
}

#[async_trait]
impl InvitationStore for ClinicalStore {
    async fn create_invitation(&self, input: CreateInvitation) -> AuthResult<Invitation> {
        let actor = request::authenticated()?;
        if clinical_id(&input.inviter_id)? != actor.account_id {
            return Err(AuthError::forbidden("Inviter does not match session"));
        }
        let tenant = self.tenant(clinical_id(&input.organization_id)?).await?;
        let result = invitations::create(
            &tenant,
            json!({"email":input.email,"membership_role":input.role}),
            None,
            None,
        )
        .await
        .map_err(operation_error)?;
        let id = result["data"]["id"]
            .as_i64()
            .ok_or_else(|| AuthError::internal("Created invitation unavailable"))?;
        let row = household_invitation::Entity::find_by_id(id)
            .one(tenant.transaction())
            .await
            .map_err(database_error)?
            .ok_or_else(|| AuthError::internal("Created invitation unavailable"))?;
        let result = value(&row, actor.account_id);
        tenant.commit().await.map_err(operation_error)?;
        Ok(result)
    }

    async fn get_invitation_by_id(&self, id: &str) -> AuthResult<Option<Invitation>> {
        let transaction = self.account_transaction().await?;
        if let Some(household) = request::authenticated()?.active_household_id {
            context(
                &transaction,
                "med_tracker.current_household_id",
                &household.to_string(),
            )
            .await?;
        }
        let row = household_invitation::Entity::find_by_id(clinical_id(id)?)
            .one(&*transaction)
            .await
            .map_err(database_error)?;
        let result = match row {
            Some(row) => Some(self.invitation_value(&transaction, &row).await?),
            None => None,
        };
        transaction.commit().await.map_err(database_error)?;
        Ok(result)
    }

    async fn get_pending_invitation(
        &self,
        org: &str,
        email: &str,
    ) -> AuthResult<Option<Invitation>> {
        Ok(self
            .invitation_rows(Some(clinical_id(org)?), None, true)
            .await?
            .into_iter()
            .find(|row| row.email == email))
    }

    async fn update_invitation_status(
        &self,
        id: &str,
        status: InvitationStatus,
    ) -> AuthResult<Invitation> {
        let current = self
            .get_invitation_by_id(id)
            .await?
            .ok_or_else(|| AuthError::not_found("Invitation not found"))?;
        if status == InvitationStatus::Accepted {
            return Err(AuthError::forbidden(
                "Invitation acceptance requires the atomic operation",
            ));
        }
        if status == InvitationStatus::Pending {
            return Err(AuthError::forbidden("Invitation status cannot be reset"));
        }
        let tenant = self.tenant(clinical_id(&current.organization_id)?).await?;
        invitations::revoke(&tenant, clinical_id(id)?, None)
            .await
            .map_err(operation_error)?;
        tenant.commit().await.map_err(operation_error)?;
        Ok(Invitation { status, ..current })
    }

    async fn update_invitation_expiry(
        &self,
        id: &str,
        expires_at: DateTime<Utc>,
    ) -> AuthResult<Invitation> {
        let current = self
            .get_invitation_by_id(id)
            .await?
            .ok_or_else(|| AuthError::not_found("Invitation not found"))?;
        let tenant = self.tenant(clinical_id(&current.organization_id)?).await?;
        administration::authorize(&tenant)
            .await
            .map_err(operation_error)?;
        let changed=tenant.transaction().execute_raw(statement("UPDATE public.household_invitations SET expires_at=$2,updated_at=CURRENT_TIMESTAMP WHERE id=$1 AND accepted_at IS NULL AND revoked_at IS NULL",[clinical_id(id)?.into(),expires_at.naive_utc().into()])).await.map_err(database_error)?.rows_affected();
        if changed != 1 {
            return Err(AuthError::bad_request("Invitation cannot be renewed"));
        }
        self.audit(
            tenant.transaction(),
            request::authenticated()?.account_id,
            "household_invitation",
            "renewed",
        )
        .await?;
        tenant.commit().await.map_err(operation_error)?;
        Ok(Invitation {
            expires_at,
            ..current
        })
    }

    async fn list_organization_invitations(&self, org: &str) -> AuthResult<Vec<Invitation>> {
        self.invitation_rows(Some(clinical_id(org)?), None, false)
            .await
    }
    async fn count_pending_organization_invitations(&self, org: &str) -> AuthResult<i64> {
        Ok(self
            .invitation_rows(Some(clinical_id(org)?), None, true)
            .await?
            .len() as i64)
    }
    async fn list_user_invitations(&self, email: &str) -> AuthResult<Vec<Invitation>> {
        self.invitation_rows(None, Some(email), false).await
    }
}
