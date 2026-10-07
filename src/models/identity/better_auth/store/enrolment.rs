use better_auth_core::{AuthError, AuthResult, Passkey, store::{CompletePasskeyEnrolment, VerifiedEmailEnrolment}, wire::SessionView};
use sea_orm::{ConnectionTrait, DatabaseTransaction};

use super::{ClinicalStore, clinical_id, context, database_error, statement};

async fn credentialless(transaction: &DatabaseTransaction, account_id: i64) -> AuthResult<bool> {
    let row = transaction.query_one_raw(statement(
        "SELECT NOT EXISTS(SELECT 1 FROM public.identity_passkeys WHERE account_id=$1) AND NOT EXISTS(SELECT 1 FROM public.identity_provider_accounts WHERE account_id=$1) AND NOT EXISTS(SELECT 1 FROM public.identity_recovery_codes WHERE account_id=$1) AND NOT EXISTS(SELECT 1 FROM public.identity_two_factors WHERE account_id=$1 AND verified) AND NOT EXISTS(SELECT 1 FROM public.account_webauthn_keys WHERE account_id=$1) AND NOT EXISTS(SELECT 1 FROM public.account_otp_keys WHERE id=$1) AND NOT EXISTS(SELECT 1 FROM public.account_recovery_codes WHERE id=$1) AND NOT EXISTS(SELECT 1 FROM public.users u JOIN public.people p ON p.id=u.person_id WHERE p.account_id=$1 AND u.password_digest IS NOT NULL) AS eligible",
        [account_id.into()],
    )).await.map_err(database_error)?.ok_or_else(|| AuthError::internal("Account credential state unavailable"))?;
    row.try_get("", "eligible").map_err(database_error)
}

impl ClinicalStore {
    pub(super) async fn begin_passkey_enrolment_in(&self, transaction: &DatabaseTransaction, input: VerifiedEmailEnrolment) -> AuthResult<Option<SessionView>> {
        if input.session.user_id != input.user_id || input.session.impersonated_by.is_some() || input.session.active_organization_id.is_some() {
            return Ok(None);
        }
        let account_id = clinical_id(&input.user_id)?;
        context(transaction, "med_tracker.current_account_id", &input.user_id).await?;
        let account = transaction.query_one_raw(statement("SELECT status FROM public.accounts WHERE id=$1 AND email=$2 AND status IN(1,2) AND password_hash IS NULL FOR UPDATE", [account_id.into(), input.email.into()])).await.map_err(database_error)?;
        let Some(account) = account else { return Ok(None); };
        let lifetime = transaction.query_one_raw(statement("SELECT $1::timestamptz>clock_timestamp() AND $1::timestamptz<=clock_timestamp()+interval '5 minutes' AS valid", [input.session.expires_at.into()])).await.map_err(database_error)?.ok_or_else(|| AuthError::internal("Enrolment clock unavailable"))?;
        if !lifetime.try_get::<bool>("", "valid").map_err(database_error)? || !credentialless(transaction, account_id).await? { return Ok(None); }
        context(transaction, "med_tracker.identity_verification_identifier", &input.verification_identifier).await?;
        let consumed = transaction.execute_raw(statement("DELETE FROM public.identity_verifications WHERE identifier=$1 AND value=$2 AND expires_at>clock_timestamp()", [input.verification_identifier.into(), input.verification_value.into()])).await.map_err(database_error)?.rows_affected();
        if consumed == 0 { return Ok(None); }
        if account.try_get::<i32>("", "status").map_err(database_error)? == 1 {
            transaction.execute_raw(statement("UPDATE public.accounts SET status=2,updated_at=CURRENT_TIMESTAMP WHERE id=$1", [account_id.into()])).await.map_err(database_error)?;
            self.audit(transaction, account_id, "email_verification", "verified").await?;
        }
        crate::models::access::verify_account_actor(transaction, account_id).await.map_err(super::tenant::operation_error)?;
        transaction.execute_raw(statement("DELETE FROM public.identity_sessions WHERE account_id=$1 AND purpose='enrolment'", [account_id.into()])).await.map_err(database_error)?;
        let mut session = self.create_session_in(transaction, input.session).await?;
        transaction.execute_raw(statement("UPDATE public.identity_sessions SET purpose='enrolment' WHERE id=$1 AND account_id=$2", [session.id.clone().into(), account_id.into()])).await.map_err(database_error)?;
        session.additional_fields.insert("clinical_session_purpose".into(), "enrolment".into());
        Ok(Some(session))
    }

    pub(super) async fn complete_passkey_enrolment_in(&self, transaction: &DatabaseTransaction, input: CompletePasskeyEnrolment) -> AuthResult<Option<(Passkey, SessionView)>> {
        if input.passkey.user_id != input.session.user_id || input.session.impersonated_by.is_some() || input.session.active_organization_id.is_some() || input.encrypted_recovery_codes.is_empty() {
            return Ok(None);
        }
        let account_id = clinical_id(&input.session.user_id)?;
        context(transaction, "med_tracker.current_account_id", &input.session.user_id).await?;
        let account = transaction.query_one_raw(statement("SELECT id FROM public.accounts WHERE id=$1 AND status=2 AND password_hash IS NULL FOR UPDATE", [account_id.into()])).await.map_err(database_error)?;
        if account.is_none() || !credentialless(transaction, account_id).await? { return Ok(None); }
        crate::models::access::verify_account_actor(transaction, account_id).await.map_err(super::tenant::operation_error)?;
        let session = transaction.query_one_raw(statement("SELECT id FROM public.identity_sessions WHERE account_id=$1 AND token=$2 AND purpose='enrolment' AND active AND expires_at>clock_timestamp() FOR UPDATE", [account_id.into(), input.enrolment_session_token.into()])).await.map_err(database_error)?;
        if session.is_none() { return Ok(None); }
        let passkey = self.create_passkey_in(transaction, input.passkey).await?;
        transaction.execute_raw(statement("INSERT INTO public.identity_recovery_codes(account_id,encrypted_codes) VALUES($1,$2)", [account_id.into(), input.encrypted_recovery_codes.into()])).await.map_err(database_error)?;
        self.audit(transaction, account_id, "recovery_codes", "created").await?;
        transaction.execute_raw(statement("DELETE FROM public.identity_sessions WHERE account_id=$1", [account_id.into()])).await.map_err(database_error)?;
        let session = self.create_session_in(transaction, input.session).await?;
        Ok(Some((passkey, session)))
    }
}
