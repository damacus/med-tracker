use async_trait::async_trait;
use better_auth::{AuthError, AuthResult, plugins::recovery_codes::{RecoveryCodeState, RecoveryCodeStore}};
use better_auth_core::{CreateSession, wire::SessionView};
use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, FromQueryResult};

use super::{ClinicalStore, clinical_id, context, database_error, statement};
use super::super::{ClinicalAuthSchema, request};

#[derive(FromQueryResult)]
struct RecoveryRow {
    encrypted_codes: String,
    locked_until: Option<DateTime<Utc>>,
}

#[async_trait]
impl RecoveryCodeStore<ClinicalAuthSchema> for ClinicalStore {
    async fn replace_codes(&self, user_id: &str, session_token: &str, codes: &str) -> AuthResult<()> {
        let account_id = clinical_id(user_id)?;
        let actor = request::authenticated()?;
        if actor.account_id != account_id || actor.session_token != session_token { return Err(AuthError::Unauthenticated); }
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.current_account_id", user_id).await?;
        let account = transaction.query_one_raw(statement("SELECT id FROM public.accounts WHERE id=$1 AND status=2 FOR UPDATE", [account_id.into()])).await.map_err(database_error)?;
        if account.is_none() { return Err(AuthError::Unauthenticated); }
        crate::models::access::verify_account_actor(&transaction, account_id).await.map_err(super::tenant::operation_error)?;
        let session = transaction.query_one_raw(statement("SELECT id FROM public.identity_sessions WHERE account_id=$1 AND token=$2 AND active AND purpose='authenticated' AND expires_at>CURRENT_TIMESTAMP FOR SHARE", [account_id.into(), session_token.into()])).await.map_err(database_error)?;
        if session.is_none() { return Err(AuthError::Unauthenticated); }
        transaction.execute_raw(statement("INSERT INTO public.identity_recovery_codes(account_id,encrypted_codes) VALUES($1,$2) ON CONFLICT(account_id) DO UPDATE SET encrypted_codes=EXCLUDED.encrypted_codes,failed_verification_count=0,locked_until=NULL,updated_at=CURRENT_TIMESTAMP", [account_id.into(), codes.into()])).await.map_err(database_error)?;
        self.audit(&transaction, account_id, "recovery_codes", "created").await?;
        transaction.commit().await.map_err(database_error)
    }

    async fn get_codes(&self, user_id: &str) -> AuthResult<Option<RecoveryCodeState>> {
        let account_id = clinical_id(user_id)?;
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.current_account_id", user_id).await?;
        let row = RecoveryRow::find_by_statement(statement("SELECT r.encrypted_codes,r.locked_until FROM public.identity_recovery_codes r JOIN public.accounts a ON a.id=r.account_id WHERE r.account_id=$1 AND a.status=2", [account_id.into()])).one(&transaction).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        Ok(row.map(|row| RecoveryCodeState { encrypted_codes: row.encrypted_codes, locked_until: row.locked_until }))
    }

    async fn record_failure(&self, user_id: &str, max_attempts: i64, locked_until: DateTime<Utc>) -> AuthResult<()> {
        let account_id = clinical_id(user_id)?;
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.current_account_id", user_id).await?;
        let account = transaction.query_one_raw(statement("SELECT id FROM public.accounts WHERE id=$1 AND status=2 FOR UPDATE", [account_id.into()])).await.map_err(database_error)?;
        if account.is_none() { return Err(AuthError::Unauthenticated); }
        transaction.execute_raw(statement("UPDATE public.identity_recovery_codes SET failed_verification_count=CASE WHEN locked_until<=CURRENT_TIMESTAMP THEN 1 ELSE failed_verification_count+1 END,locked_until=CASE WHEN (CASE WHEN locked_until<=CURRENT_TIMESTAMP THEN 1 ELSE failed_verification_count+1 END)>=$2 THEN GREATEST(locked_until,$3) WHEN locked_until<=CURRENT_TIMESTAMP THEN NULL ELSE locked_until END,updated_at=CURRENT_TIMESTAMP WHERE account_id=$1", [account_id.into(), max_attempts.into(), locked_until.into()])).await.map_err(database_error)?;
        self.audit(&transaction, account_id, "recovery_codes", "failed").await?;
        transaction.commit().await.map_err(database_error)
    }

    async fn consume_and_create_session(&self, previous: &str, replacement: &str, input: CreateSession) -> AuthResult<Option<SessionView>> {
        let account_id = clinical_id(&input.user_id)?;
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.current_account_id", &input.user_id).await?;
        let account = transaction.query_one_raw(statement("SELECT id FROM public.accounts WHERE id=$1 AND status=2 FOR UPDATE", [account_id.into()])).await.map_err(database_error)?;
        if account.is_none() { transaction.rollback().await.map_err(database_error)?; return Ok(None); }
        crate::models::access::verify_account_actor(&transaction, account_id).await.map_err(super::tenant::operation_error)?;
        let consumed = transaction.execute_raw(statement("UPDATE public.identity_recovery_codes SET encrypted_codes=$3,failed_verification_count=0,locked_until=NULL,updated_at=CURRENT_TIMESTAMP WHERE account_id=$1 AND encrypted_codes=$2 AND (locked_until IS NULL OR locked_until<=CURRENT_TIMESTAMP)", [account_id.into(), previous.into(), replacement.into()])).await.map_err(database_error)?.rows_affected() == 1;
        if !consumed { transaction.rollback().await.map_err(database_error)?; return Ok(None); }
        transaction.execute_raw(statement("DELETE FROM public.identity_sessions WHERE account_id=$1", [account_id.into()])).await.map_err(database_error)?;
        transaction.execute_raw(statement("UPDATE public.identity_two_factors SET failed_verification_count=0,locked_until=NULL,updated_at=CURRENT_TIMESTAMP WHERE account_id=$1", [account_id.into()])).await.map_err(database_error)?;
        self.audit(&transaction, account_id, "recovery_codes", "consumed").await?;
        let session = self.create_session_in(&transaction, input).await?;
        transaction.commit().await.map_err(database_error)?;
        Ok(Some(session))
    }
}
