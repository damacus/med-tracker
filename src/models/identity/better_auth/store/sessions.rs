use crate::models::identity::store::digest as token_digest;
use async_trait::async_trait;
use better_auth_core::{
    AuthError, AuthResult, CreateSession, FieldMap, FieldValue, store::SessionStore,
    wire::SessionView,
};
use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, FromQueryResult};
use serde_json::Value;

use super::super::ClinicalAuthSchema;
use super::{ClinicalStore, clinical_id, context, database_error, statement};

const RESERVED_SESSION_FIELDS: &[&str] = &[
    "id",
    "userId",
    "token",
    "expiresAt",
    "createdAt",
    "updatedAt",
    "ipAddress",
    "userAgent",
    "impersonatedBy",
    "activeOrganizationId",
    "activeTeamId",
    "active",
    "clinical_session_purpose",
    "authentication_method",
    "legacy_factor_pending",
];

#[derive(FromQueryResult)]
struct SessionRow {
    id: String,
    account_id: i64,
    token: String,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    ip_address: Option<String>,
    user_agent: Option<String>,
    impersonated_by: Option<i64>,
    active_household_id: Option<i64>,
    active: bool,
    purpose: String,
    additional_fields: Value,
}

impl TryFrom<SessionRow> for SessionView {
    type Error = AuthError;
    fn try_from(row: SessionRow) -> AuthResult<Self> {
        let mut fields = FieldMap::from_json(
            row.additional_fields
                .as_object()
                .cloned()
                .ok_or_else(|| AuthError::internal("Invalid stored session fields"))?,
        )?;
        fields.insert("clinical_session_purpose".into(), row.purpose.into());
        Ok(Self {
            field_order: Vec::new(),
            visible_fields: None,
            id: row.id.into(),
            user_id: row.account_id.to_string().into(),
            token: row.token.into(),
            expires_at: row.expires_at.into(),
            created_at: row.created_at.into(),
            updated_at: row.updated_at.into(),
            ip_address: row.ip_address.into(),
            user_agent: row.user_agent.into(),
            impersonated_by: row.impersonated_by.map(|id| id.to_string()).into(),
            active_organization_id: row.active_household_id.map(|id| id.to_string()).into(),
            active_team_id: None.into(),
            active: row.active,
            additional_fields: fields,
        })
    }
}

impl ClinicalStore {
    pub(crate) async fn lock_security_session(
        &self,
        transaction: &sea_orm::DatabaseTransaction,
        session: &SessionView,
    ) -> AuthResult<()> {
        context(
            transaction,
            "med_tracker.current_account_id",
            session.user_id.typed()?,
        )
        .await?;
        let account_id = clinical_id(session.user_id.typed()?)?;
        transaction
            .query_one_raw(statement(
                "SELECT id FROM public.accounts WHERE id=$1 AND status=2 FOR UPDATE",
                [account_id.into()],
            ))
            .await
            .map_err(database_error)?
            .ok_or(AuthError::Unauthenticated)?;
        transaction.query_one_raw(statement("SELECT id FROM public.identity_sessions WHERE id=$1 AND token=$2 AND account_id=$3 AND active AND purpose='authenticated' AND expires_at>clock_timestamp() AND created_at>clock_timestamp()-interval '30 days'", [session.id.typed()?.clone().into(), token_digest(session.token.typed()?).into(), account_id.into()])).await.map_err(database_error)?.ok_or(AuthError::Unauthenticated)?;
        Ok(())
    }

    pub(crate) async fn delete_user_session_in(
        &self,
        transaction: &sea_orm::DatabaseTransaction,
        user_id: &str,
        session_id: &str,
    ) -> AuthResult<()> {
        context(transaction, "med_tracker.current_account_id", user_id).await?;
        transaction
            .execute_raw(statement(
                "DELETE FROM public.identity_sessions WHERE id=$1 AND account_id=$2",
                [session_id.into(), clinical_id(user_id)?.into()],
            ))
            .await
            .map_err(database_error)?;
        Ok(())
    }

    async fn session_transaction(
        &self,
        token: &str,
    ) -> AuthResult<(super::Transaction, Option<SessionRow>)> {
        let transaction = self.transaction().await?;
        context(
            &transaction,
            "med_tracker.identity_session_token",
            &token_digest(token),
        )
        .await?;
        let mut row = SessionRow::find_by_statement(statement("SELECT s.* FROM public.identity_sessions s JOIN public.accounts a ON a.id=s.account_id WHERE s.token=$1 AND s.active AND s.expires_at>clock_timestamp() AND s.created_at>clock_timestamp()-interval '30 days' AND (s.additional_fields->>'legacy_factor_pending' IS DISTINCT FROM 'true' OR s.created_at>clock_timestamp()-interval '5 minutes') AND a.status=2", [token_digest(token).into()])).one(&*transaction).await.map_err(database_error)?;
        if let Some(row) = &mut row {
            row.token = token.to_owned();
            context(
                &transaction,
                "med_tracker.current_account_id",
                &row.account_id.to_string(),
            )
            .await?;
            crate::models::access::verify_account(&transaction, row.account_id)
                .await
                .map_err(super::tenant::operation_error)?;
        }
        Ok((transaction, row))
    }
}

impl ClinicalStore {
    pub(super) async fn create_session_in(
        &self,
        transaction: &sea_orm::DatabaseTransaction,
        input: CreateSession,
    ) -> AuthResult<SessionView> {
        let mut additional_fields = input.inherited_fields.clone();
        for field in RESERVED_SESSION_FIELDS {
            additional_fields.remove(field);
        }
        additional_fields.extend(self.session_defaults()?);
        additional_fields.extend(input.additional_fields.clone());
        if RESERVED_SESSION_FIELDS
            .iter()
            .any(|field| additional_fields.contains_key(*field))
        {
            return Err(AuthError::forbidden(
                "Session assurance and membership require verified operations",
            ));
        }
        let additional_fields = FieldValue::from(additional_fields)
            .json()?
            .ok_or_else(|| AuthError::validation("Invalid session fields"))?;
        let user_id = input.user_id.typed()?;
        let account_id = clinical_id(user_id)?;
        context(transaction, "med_tracker.current_account_id", user_id).await?;
        let account = transaction
            .query_one_raw(statement(
                "SELECT id FROM public.accounts WHERE id=$1 AND status=2 FOR UPDATE",
                [account_id.into()],
            ))
            .await
            .map_err(database_error)?;
        if account.is_none() {
            return Err(AuthError::Unauthenticated);
        }
        crate::models::access::verify_account(transaction, account_id)
            .await
            .map_err(super::tenant::operation_error)?;
        transaction.execute_raw(statement("INSERT INTO public.identity_onboarding(account_id) VALUES($1) ON CONFLICT(account_id) DO NOTHING", [account_id.into()])).await.map_err(database_error)?;
        let household = input
            .active_organization_id
            .as_deref()
            .map(clinical_id)
            .transpose()?;
        if let Some(household) = household {
            let membership = transaction.query_one_raw(statement("SELECT id FROM public.household_memberships WHERE person_id=(SELECT person_id FROM public.accounts WHERE id=$1) AND household_id=$2 AND status='active' AND revoked_at IS NULL", [account_id.into(), household.into()])).await.map_err(database_error)?;
            if membership.is_none() {
                return Err(AuthError::forbidden("Household membership unavailable"));
            }
        }
        if input.impersonated_by.is_some() {
            return Err(AuthError::forbidden("Impersonation is unavailable"));
        }
        let token = format!("session_{}", uuid::Uuid::new_v4());
        let expires_at = input
            .expires_at
            .to_datetime()?
            .ok_or_else(|| AuthError::validation("Invalid session expiry"))?;
        let row = SessionRow::find_by_statement(statement("INSERT INTO public.identity_sessions (id,account_id,token,expires_at,ip_address,user_agent,active_household_id,purpose,additional_fields) VALUES($1,$2,$3,LEAST($4,CURRENT_TIMESTAMP+interval '7 days'),$5,$6,$7,CASE WHEN EXISTS(SELECT 1 FROM public.identity_onboarding WHERE account_id=$2 AND recovery_saved_at IS NOT NULL) THEN 'authenticated' ELSE 'enrolment' END,$8::jsonb) RETURNING *", [uuid::Uuid::new_v4().to_string().into(), account_id.into(), token_digest(&token).into(), expires_at.into(), input.ip_address.into(), input.user_agent.into(), household.into(), additional_fields.into()])).one(transaction).await.map_err(database_error)?.ok_or_else(|| AuthError::internal("Session was not stored"))?;
        let mut row = if super::super::request::password_login() && transaction.query_one_raw(statement("SELECT k.id FROM public.account_otp_keys k JOIN public.identity_onboarding o ON o.account_id=k.id WHERE k.id=$1 AND o.legacy_totp_disabled_at IS NULL AND NOT EXISTS(SELECT 1 FROM public.identity_two_factors WHERE account_id=k.id AND verified)", [account_id.into()])).await.map_err(database_error)?.is_some() {
            SessionRow::find_by_statement(statement("UPDATE public.identity_sessions SET purpose='enrolment',expires_at=clock_timestamp()+interval '5 minutes',additional_fields=additional_fields || jsonb_build_object('legacy_factor_pending',true) WHERE id=$1 RETURNING *", [row.id.into()])).one(transaction).await.map_err(database_error)?.ok_or(AuthError::Unauthenticated)?
        } else { row };
        row.token = token;
        let session = row.try_into()?;
        self.audit(transaction, account_id, "session", "created")
            .await?;
        Ok(session)
    }
}

#[async_trait]
impl SessionStore<ClinicalAuthSchema> for ClinicalStore {
    async fn create_session_optional(
        &self,
        input: CreateSession,
    ) -> AuthResult<Option<SessionView>> {
        self.create_session(input).await.map(Some)
    }

    async fn accept_invitation_with_teams(
        &self,
        _invitation_id: &str,
        _user_id: &str,
        _session_token: Option<&str>,
        _teams_enabled: bool,
        _maximum: better_auth_core::store::TeamMemberLimits<'_>,
    ) -> AuthResult<(
        better_auth_core::Member,
        better_auth_core::Invitation,
        Option<SessionView>,
    )> {
        Err(AuthError::forbidden(
            "Use the verified household invitation acceptance operation",
        ))
    }

    async fn get_session_snapshots(
        &self,
        _tokens: &[String],
        _only_active: bool,
    ) -> AuthResult<
        Vec<(
            SessionView,
            Option<
                better_auth_core::session::SessionData<
                    better_auth_core::store::JoinValue<better_auth_core::wire::UserView>,
                >,
            >,
        )>,
    > {
        Err(AuthError::config(
            "Token-batch session lookup is not configured",
        ))
    }

    async fn create_session(&self, input: CreateSession) -> AuthResult<SessionView> {
        let transaction = self.transaction().await?;
        let session = self.create_session_in(&transaction, input).await?;
        transaction.commit().await.map_err(database_error)?;
        Ok(session)
    }

    async fn get_session(&self, token: &str) -> AuthResult<Option<SessionView>> {
        let (transaction, row) = self.session_transaction(token).await?;
        let session = row.map(TryInto::try_into).transpose()?;
        transaction.commit().await.map_err(database_error)?;
        Ok(session)
    }

    async fn update_session_fields(
        &self,
        token: &str,
        mut fields: FieldMap,
    ) -> AuthResult<Option<SessionView>> {
        if RESERVED_SESSION_FIELDS
            .iter()
            .filter(|field| !matches!(**field, "expiresAt" | "updatedAt"))
            .any(|field| fields.contains_key(*field))
        {
            return Err(AuthError::forbidden(
                "Session assurance cannot be changed through profile fields",
            ));
        }
        let expires_at = fields
            .remove("expiresAt")
            .map(|value| {
                value
                    .as_date()
                    .ok_or_else(|| AuthError::validation("Invalid session expiry"))?
                    .to_datetime()?
                    .ok_or_else(|| AuthError::validation("Invalid session expiry"))
            })
            .transpose()?;
        if let Some(updated_at) = fields.remove("updatedAt") {
            updated_at
                .as_date()
                .ok_or_else(|| AuthError::validation("Invalid session update time"))?
                .to_datetime()?
                .ok_or_else(|| AuthError::validation("Invalid session update time"))?;
        }
        let (transaction, row) = self.session_transaction(token).await?;
        if row.is_none() {
            transaction.rollback().await.map_err(database_error)?;
            return Ok(None);
        }
        let fields = FieldValue::from(fields)
            .json()?
            .ok_or_else(|| AuthError::validation("Invalid session fields"))?;
        let mut row = if let Some(expires_at) = expires_at {
            SessionRow::find_by_statement(statement("UPDATE public.identity_sessions SET expires_at=LEAST($2,clock_timestamp()+interval '7 days',created_at+CASE WHEN additional_fields->>'legacy_factor_pending'='true' THEN interval '5 minutes' ELSE interval '30 days' END),additional_fields=(additional_fields - 'expiresAt' - 'updatedAt') || $3::jsonb,updated_at=clock_timestamp() WHERE token=$1 RETURNING *", [token_digest(token).into(), expires_at.into(), fields.into()])).one(&*transaction).await.map_err(database_error)?
        } else {
            SessionRow::find_by_statement(statement("UPDATE public.identity_sessions SET additional_fields=(additional_fields - 'expiresAt' - 'updatedAt') || $2::jsonb,updated_at=clock_timestamp() WHERE token=$1 RETURNING *", [token_digest(token).into(), fields.into()])).one(&*transaction).await.map_err(database_error)?
        };
        if let Some(row) = &mut row {
            row.token = token.to_owned();
        }
        let session = row.map(TryInto::try_into).transpose()?;
        transaction.commit().await.map_err(database_error)?;
        Ok(session)
    }

    async fn get_user_sessions(&self, user_id: &str) -> AuthResult<Vec<SessionView>> {
        let id = clinical_id(user_id)?;
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.current_account_id", user_id).await?;
        let rows = SessionRow::find_by_statement(statement("SELECT * FROM public.identity_sessions WHERE account_id=$1 AND active AND expires_at>CURRENT_TIMESTAMP ORDER BY created_at",[id.into()])).all(&*transaction).await.map_err(database_error)?;
        let sessions = rows
            .into_iter()
            .map(TryInto::try_into)
            .collect::<AuthResult<Vec<_>>>()?;
        transaction.commit().await.map_err(database_error)?;
        Ok(sessions)
    }

    async fn update_session_expiry(
        &self,
        token: &str,
        expires_at: DateTime<Utc>,
    ) -> AuthResult<SessionView> {
        let (transaction, row) = self.session_transaction(token).await?;
        row.ok_or(AuthError::SessionNotFound)?;
        let mut row = SessionRow::find_by_statement(statement("UPDATE public.identity_sessions SET expires_at=LEAST($2,clock_timestamp()+interval '7 days',created_at+CASE WHEN additional_fields->>'legacy_factor_pending'='true' THEN interval '5 minutes' ELSE interval '30 days' END),updated_at=clock_timestamp() WHERE token=$1 RETURNING *",[token_digest(token).into(),expires_at.into()])).one(&*transaction).await.map_err(database_error)?.ok_or(AuthError::SessionNotFound)?;
        row.token = token.to_owned();
        let session = row.try_into()?;
        transaction.commit().await.map_err(database_error)?;
        Ok(session)
    }

    async fn delete_session(&self, token: &str) -> AuthResult<()> {
        let (transaction, row) = self.session_transaction(token).await?;
        if row.is_some() {
            transaction
                .execute_raw(statement(
                    "DELETE FROM public.identity_sessions WHERE token=$1",
                    [token_digest(token).into()],
                ))
                .await
                .map_err(database_error)?;
        }
        transaction.commit().await.map_err(database_error)
    }

    async fn delete_user_sessions(&self, user_id: &str) -> AuthResult<()> {
        let id = clinical_id(user_id)?;
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.current_account_id", user_id).await?;
        transaction
            .execute_raw(statement(
                "DELETE FROM public.identity_sessions WHERE account_id=$1",
                [id.into()],
            ))
            .await
            .map_err(database_error)?;
        transaction.commit().await.map_err(database_error)
    }

    async fn delete_expired_sessions(&self) -> AuthResult<usize> {
        let transaction = self.transaction().await?;
        let result = transaction
            .query_one_raw(statement(
                "SELECT public.identity_delete_expired_sessions() AS deleted",
                [],
            ))
            .await
            .map_err(database_error)?
            .ok_or_else(|| AuthError::internal("Session cleanup unavailable"))?;
        let count: i64 = result.try_get("", "deleted").map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        usize::try_from(count).map_err(|_| AuthError::internal("Session cleanup count overflow"))
    }

    async fn update_session_active_organization(
        &self,
        token: &str,
        organization_id: Option<&str>,
    ) -> AuthResult<SessionView> {
        let (transaction, row) = self.session_transaction(token).await?;
        let session = row.ok_or(AuthError::Unauthenticated)?;
        if session.purpose != "authenticated" {
            return Err(AuthError::Unauthenticated);
        }
        transaction
            .query_one_raw(statement(
                "SELECT id FROM public.accounts WHERE id=$1 AND status=2 FOR SHARE",
                [session.account_id.into()],
            ))
            .await
            .map_err(database_error)?
            .ok_or(AuthError::Unauthenticated)?;
        crate::models::access::verify_account(&transaction, session.account_id)
            .await
            .map_err(super::tenant::operation_error)?;
        let household = organization_id.map(clinical_id).transpose()?;
        if let Some(id) = household {
            transaction.query_one_raw(statement("SELECT id FROM public.households WHERE id=$1 AND status='active' AND lifecycle_state='active' FOR SHARE",[id.into()])).await.map_err(database_error)?.ok_or_else(|| AuthError::forbidden("Household unavailable"))?;
            let membership = transaction.query_one_raw(statement("SELECT id FROM public.household_memberships WHERE person_id=(SELECT person_id FROM public.accounts WHERE id=$1) AND household_id=$2 AND status='active' AND revoked_at IS NULL FOR SHARE",[session.account_id.into(),id.into()])).await.map_err(database_error)?;
            if membership.is_none() {
                return Err(AuthError::forbidden("Household membership unavailable"));
            }
        }
        let mut row = SessionRow::find_by_statement(statement("UPDATE public.identity_sessions SET active_household_id=$2,updated_at=CURRENT_TIMESTAMP WHERE token=$1 AND account_id=$3 AND active AND purpose='authenticated' AND expires_at>clock_timestamp() AND created_at>clock_timestamp()-interval '30 days' RETURNING *",[token_digest(token).into(),household.into(),session.account_id.into()])).one(&*transaction).await.map_err(database_error)?.ok_or(AuthError::Unauthenticated)?;
        row.token = token.to_owned();
        let session = row.try_into()?;
        transaction.commit().await.map_err(database_error)?;
        Ok(session)
    }

    async fn update_session_active_team(
        &self,
        _token: &str,
        _team_id: Option<&str>,
    ) -> AuthResult<SessionView> {
        Err(AuthError::forbidden("Teams are unavailable"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn plugin_invitation_acceptance_rejects_before_any_database_access() {
        let store = ClinicalStore::new(sea_orm::DatabaseConnection::default());
        let result = SessionStore::<ClinicalAuthSchema>::accept_invitation_with_teams(
            &store,
            "123",
            "456",
            Some("session-token"),
            true,
            better_auth_core::store::TeamMemberLimits::Fixed(Some(1)),
        )
        .await;
        assert!(matches!(result, Err(AuthError::Forbidden(_))));
    }
}
