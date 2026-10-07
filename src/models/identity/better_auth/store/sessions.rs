use async_trait::async_trait;
use better_auth_core::{AuthError, AuthResult, CreateSession, store::SessionStore, wire::SessionView};
use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, FromQueryResult};
use serde_json::{Map, Value};

use super::{ClinicalStore, clinical_id, context, database_error, statement};
use super::super::ClinicalAuthSchema;

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
        let mut fields = row.additional_fields.as_object().cloned().ok_or_else(|| AuthError::internal("Invalid stored session fields"))?;
        fields.insert("clinical_session_purpose".into(), row.purpose.into());
        Ok(Self { id: row.id, user_id: row.account_id.to_string(), token: row.token, expires_at: row.expires_at, created_at: row.created_at, updated_at: row.updated_at, ip_address: row.ip_address, user_agent: row.user_agent, impersonated_by: row.impersonated_by.map(|id| id.to_string()), active_organization_id: row.active_household_id.map(|id| id.to_string()), active: row.active, additional_fields: fields })
    }
}

impl ClinicalStore {
    pub(crate) async fn lock_security_session(&self, transaction: &sea_orm::DatabaseTransaction, session: &SessionView) -> AuthResult<()> {
        context(transaction, "med_tracker.current_account_id", &session.user_id).await?;
        let account_id = clinical_id(&session.user_id)?;
        transaction.query_one_raw(statement("SELECT id FROM public.accounts WHERE id=$1 AND status=2 FOR UPDATE", [account_id.into()])).await.map_err(database_error)?.ok_or(AuthError::Unauthenticated)?;
        transaction.query_one_raw(statement("SELECT id FROM public.identity_sessions WHERE id=$1 AND token=$2 AND account_id=$3 AND active AND purpose='authenticated' AND expires_at>clock_timestamp() AND created_at>clock_timestamp()-interval '30 days'", [session.id.clone().into(), session.token.clone().into(), account_id.into()])).await.map_err(database_error)?.ok_or(AuthError::Unauthenticated)?;
        Ok(())
    }

    async fn session_transaction(&self, token: &str) -> AuthResult<(super::Transaction, Option<SessionRow>)> {
        let transaction = self.transaction().await?;
        context(&*transaction, "med_tracker.identity_session_token", token).await?;
        let row = SessionRow::find_by_statement(statement("SELECT s.* FROM public.identity_sessions s JOIN public.accounts a ON a.id=s.account_id WHERE s.token=$1 AND s.active AND s.expires_at>CURRENT_TIMESTAMP AND s.created_at>CURRENT_TIMESTAMP-interval '30 days' AND a.status=2", [token.into()])).one(&*transaction).await.map_err(database_error)?;
        if let Some(row) = &row {
            context(&*transaction, "med_tracker.current_account_id", &row.account_id.to_string()).await?;
        }
        Ok((transaction, row))
    }
}

impl ClinicalStore {
    pub(super) async fn create_session_in(&self, transaction: &sea_orm::DatabaseTransaction, input: CreateSession) -> AuthResult<SessionView> {
        let account_id = clinical_id(&input.user_id)?;
        context(transaction, "med_tracker.current_account_id", &input.user_id).await?;
        let account = transaction.query_one_raw(statement("SELECT id FROM public.accounts WHERE id=$1 AND status=2 FOR UPDATE", [account_id.into()])).await.map_err(database_error)?;
        if account.is_none() { return Err(AuthError::Unauthenticated); }
        crate::models::access::verify_account_actor(transaction, account_id).await.map_err(super::tenant::operation_error)?;
        transaction.execute_raw(statement("INSERT INTO public.identity_onboarding(account_id) VALUES($1) ON CONFLICT(account_id) DO NOTHING", [account_id.into()])).await.map_err(database_error)?;
        let household = input.active_organization_id.as_deref().map(clinical_id).transpose()?;
        if let Some(household) = household {
            let membership = transaction.query_one_raw(statement("SELECT id FROM public.household_memberships WHERE account_id=$1 AND household_id=$2 AND status='active' AND revoked_at IS NULL", [account_id.into(), household.into()])).await.map_err(database_error)?;
            if membership.is_none() { return Err(AuthError::forbidden("Household membership unavailable")); }
        }
        if input.impersonated_by.is_some() { return Err(AuthError::forbidden("Impersonation is unavailable")); }
        let row = SessionRow::find_by_statement(statement("INSERT INTO public.identity_sessions (id,account_id,token,expires_at,ip_address,user_agent,active_household_id,purpose) VALUES($1,$2,$3,LEAST($4,CURRENT_TIMESTAMP+interval '7 days'),$5,$6,$7,CASE WHEN EXISTS(SELECT 1 FROM public.identity_onboarding WHERE account_id=$2 AND recovery_saved_at IS NOT NULL) THEN 'authenticated' ELSE 'enrolment' END) RETURNING *", [uuid::Uuid::new_v4().to_string().into(), account_id.into(), format!("session_{}",uuid::Uuid::new_v4()).into(), input.expires_at.into(), input.ip_address.into(), input.user_agent.into(), household.into()])).one(transaction).await.map_err(database_error)?.ok_or_else(|| AuthError::internal("Session was not stored"))?;
        let row = if super::super::request::password_login() && transaction.query_one_raw(statement("SELECT k.id FROM public.account_otp_keys k JOIN public.identity_onboarding o ON o.account_id=k.id WHERE k.id=$1 AND o.legacy_totp_disabled_at IS NULL AND NOT EXISTS(SELECT 1 FROM public.identity_two_factors WHERE account_id=k.id AND verified)", [account_id.into()])).await.map_err(database_error)?.is_some() {
            SessionRow::find_by_statement(statement("UPDATE public.identity_sessions SET purpose='enrolment',expires_at=clock_timestamp()+interval '5 minutes',additional_fields=jsonb_build_object('legacy_factor_pending',true) WHERE id=$1 RETURNING *", [row.id.into()])).one(transaction).await.map_err(database_error)?.ok_or(AuthError::Unauthenticated)?
        } else { row };
        let session = row.try_into()?;
        self.audit(transaction, account_id, "session", "created").await?;
        Ok(session)
    }
}

#[async_trait]
impl SessionStore<ClinicalAuthSchema> for ClinicalStore {
    async fn create_session(&self, input: CreateSession) -> AuthResult<SessionView> {
        let transaction = self.transaction().await?;
        let session = self.create_session_in(&*transaction, input).await?;
        transaction.commit().await.map_err(database_error)?;
        Ok(session)
    }

    async fn get_session(&self, token: &str) -> AuthResult<Option<SessionView>> {
        let (transaction, row) = self.session_transaction(token).await?;
        let session = row.map(TryInto::try_into).transpose()?;
        transaction.commit().await.map_err(database_error)?;
        Ok(session)
    }

    async fn update_session_fields(&self, token: &str, fields: Map<String, Value>) -> AuthResult<Option<SessionView>> {
        if fields.contains_key("clinical_session_purpose") { return Err(AuthError::forbidden("Session assurance cannot be changed through profile fields")); }
        let (transaction, row) = self.session_transaction(token).await?;
        if row.is_none() { transaction.rollback().await.map_err(database_error)?; return Ok(None); }
        let row = SessionRow::find_by_statement(statement("UPDATE public.identity_sessions SET additional_fields=additional_fields || $2::jsonb,updated_at=CURRENT_TIMESTAMP WHERE token=$1 RETURNING *", [token.into(), Value::Object(fields).into()])).one(&*transaction).await.map_err(database_error)?;
        let session = row.map(TryInto::try_into).transpose()?;
        transaction.commit().await.map_err(database_error)?;
        Ok(session)
    }

    async fn get_user_sessions(&self, user_id: &str) -> AuthResult<Vec<SessionView>> {
        let id = clinical_id(user_id)?;
        let transaction = self.transaction().await?;
        context(&*transaction,"med_tracker.current_account_id",user_id).await?;
        let rows = SessionRow::find_by_statement(statement("SELECT * FROM public.identity_sessions WHERE account_id=$1 AND active AND expires_at>CURRENT_TIMESTAMP ORDER BY created_at",[id.into()])).all(&*transaction).await.map_err(database_error)?;
        let sessions = rows.into_iter().map(TryInto::try_into).collect::<AuthResult<Vec<_>>>()?;
        transaction.commit().await.map_err(database_error)?;
        Ok(sessions)
    }

    async fn update_session_expiry(&self, token: &str, expires_at: DateTime<Utc>) -> AuthResult<()> {
        let (transaction,row) = self.session_transaction(token).await?;
        if row.is_some() { transaction.execute_raw(statement("UPDATE public.identity_sessions SET expires_at=LEAST($2,created_at+interval '30 days'),updated_at=CURRENT_TIMESTAMP WHERE token=$1",[token.into(),expires_at.into()])).await.map_err(database_error)?; }
        transaction.commit().await.map_err(database_error)
    }

    async fn delete_session(&self, token: &str) -> AuthResult<()> {
        let (transaction,row) = self.session_transaction(token).await?;
        if row.is_some() { transaction.execute_raw(statement("DELETE FROM public.identity_sessions WHERE token=$1",[token.into()])).await.map_err(database_error)?; }
        transaction.commit().await.map_err(database_error)
    }

    async fn delete_user_sessions(&self, user_id: &str) -> AuthResult<()> {
        let id = clinical_id(user_id)?;
        let transaction = self.transaction().await?;
        context(&*transaction,"med_tracker.current_account_id",user_id).await?;
        transaction.execute_raw(statement("DELETE FROM public.identity_sessions WHERE account_id=$1",[id.into()])).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)
    }

    async fn delete_expired_sessions(&self) -> AuthResult<usize> {
        let transaction = self.transaction().await?;
        let result = transaction.query_one_raw(statement("SELECT public.identity_delete_expired_sessions() AS deleted",[])).await.map_err(database_error)?.ok_or_else(|| AuthError::internal("Session cleanup unavailable"))?;
        let count: i64 = result.try_get("", "deleted").map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        usize::try_from(count).map_err(|_| AuthError::internal("Session cleanup count overflow"))
    }

    async fn update_session_active_organization(&self, token: &str, organization_id: Option<&str>) -> AuthResult<SessionView> {
        let (transaction,row) = self.session_transaction(token).await?;
        let session = row.ok_or(AuthError::Unauthenticated)?;
        if session.purpose != "authenticated" { return Err(AuthError::Unauthenticated); }
        let household = organization_id.map(clinical_id).transpose()?;
        if let Some(id) = household {
            let membership = transaction.query_one_raw(statement("SELECT id FROM public.household_memberships WHERE account_id=$1 AND household_id=$2 AND status='active' AND revoked_at IS NULL",[session.account_id.into(),id.into()])).await.map_err(database_error)?;
            if membership.is_none() { return Err(AuthError::forbidden("Household membership unavailable")); }
        }
        let row = SessionRow::find_by_statement(statement("UPDATE public.identity_sessions SET active_household_id=$2,updated_at=CURRENT_TIMESTAMP WHERE token=$1 RETURNING *",[token.into(),household.into()])).one(&*transaction).await.map_err(database_error)?.ok_or(AuthError::Unauthenticated)?;
        let session = row.try_into()?;
        transaction.commit().await.map_err(database_error)?;
        Ok(session)
    }
}
