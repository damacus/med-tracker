use better_auth_core::{AuthContext, AuthError, AuthRequest, AuthResponse, AuthResult, wire::SessionView};
use sea_orm::{ConnectionTrait, FromQueryResult};
use sha2::{Digest, Sha256};
use super::{ClinicalAuthSchema, ClinicalStore, store::{clinical_id, context, database_error, statement}};

pub(super) fn pending(session: &SessionView) -> bool {
    session.additional_fields.get("legacy_factor_pending").and_then(serde_json::Value::as_bool) == Some(true)
}

#[derive(FromQueryResult)]
struct LegacyFactor {
    key: String,
    last_use: chrono::NaiveDateTime,
    num_failures: i32,
    legacy_totp_locked_until: Option<chrono::DateTime<chrono::Utc>>,
}

pub(super) async fn enabled(store: &ClinicalStore, user_id: &str) -> AuthResult<bool> {
    Ok(fingerprint(store, user_id).await?.is_some())
}

impl ClinicalStore {
    pub(crate) async fn retained_factor_enabled(&self, user_id: &str) -> AuthResult<bool> { enabled(self, user_id).await }
}

pub(super) async fn fingerprint(store: &ClinicalStore, user_id: &str) -> AuthResult<Option<String>> {
    let transaction = store.transaction().await?;
    context(&transaction, "med_tracker.current_account_id", user_id).await?;
    let row = transaction.query_one_raw(statement("SELECT k.key FROM public.account_otp_keys k LEFT JOIN public.identity_onboarding o ON o.account_id=k.id WHERE k.id=$1 AND o.legacy_totp_disabled_at IS NULL AND NOT EXISTS(SELECT 1 FROM public.identity_two_factors WHERE account_id=k.id AND verified)", [clinical_id(user_id)?.into()])).await.map_err(database_error)?;
    let fingerprint = row.map(|row| row.try_get::<String>("", "key").map(|key| hex::encode(Sha256::digest(key.as_bytes())))).transpose().map_err(database_error)?;
    transaction.commit().await.map_err(database_error)?;
    Ok(fingerprint)
}

pub(super) async fn verify_operation(store: &ClinicalStore, request: &AuthRequest, ctx: &AuthContext<ClinicalAuthSchema>, code: &str) -> AuthResult<Option<AuthResponse>> {
    let (_, session) = ctx.require_authoritative_session(request).await?;
    let transaction = store.transaction().await?;
    store.lock_security_session(&transaction, &session).await?;
    let response = check_code(&transaction, clinical_id(&session.user_id)?, code).await?;
    transaction.commit().await.map_err(database_error)?;
    Ok(response)
}

pub(super) async fn verify(store: &ClinicalStore, request: &AuthRequest, ctx: &AuthContext<ClinicalAuthSchema>, code: &str) -> AuthResult<Option<AuthResponse>> {
    let Ok((user, session)) = ctx.require_authoritative_session(request).await else { return Ok(None); };
    if !pending(&session) { return Ok(None); }
    let transaction = store.transaction().await?;
    let account_id = clinical_id(&user.id)?;
    context(&transaction, "med_tracker.current_account_id", &user.id).await?;
    transaction.query_one_raw(statement("SELECT id FROM public.accounts WHERE id=$1 AND status=2 FOR UPDATE", [account_id.into()])).await.map_err(database_error)?.ok_or(AuthError::Unauthenticated)?;
    transaction.query_one_raw(statement("SELECT id FROM public.identity_sessions WHERE token=$1 AND account_id=$2 AND active AND purpose='enrolment' AND expires_at>clock_timestamp() AND additional_fields->>'legacy_factor_pending'='true'", [session.token.clone().into(), account_id.into()])).await.map_err(database_error)?.ok_or(AuthError::Unauthenticated)?;
    if let Some(rejected) = check_code(&transaction, account_id, code).await? {
        transaction.commit().await.map_err(database_error)?;
        return Ok(Some(rejected));
    }
    ctx.database.delete_session(&session.token).await?;
    let issued = ctx.session_manager().create_session(&user, session.ip_address, session.user_agent).await?;
    store.audit(&transaction, account_id, "totp", "verified").await?;
    transaction.commit().await.map_err(database_error)?;
    Ok(Some(AuthResponse::json(200, &serde_json::json!({"status":true}))?.with_header("Set-Cookie", better_auth_core::utils::cookie_utils::create_session_cookie(&issued.token, &ctx.config))))
}

async fn check_code(transaction: &sea_orm::DatabaseTransaction, account_id: i64, code: &str) -> AuthResult<Option<AuthResponse>> {
    let factor = LegacyFactor::find_by_statement(statement("SELECT k.key,k.last_use,k.num_failures,o.legacy_totp_locked_until FROM public.account_otp_keys k JOIN public.identity_onboarding o ON o.account_id=k.id WHERE k.id=$1 AND o.legacy_totp_disabled_at IS NULL FOR UPDATE OF k", [account_id.into()])).one(&*transaction).await.map_err(database_error)?.ok_or(AuthError::InvalidCredentials)?;
    let now = chrono::Utc::now();
    if factor.legacy_totp_locked_until.is_some_and(|until| until > now) { return Err(AuthError::Upstream { status: 429, code: "ACCOUNT_TEMPORARILY_LOCKED", message: "Too many failed verification attempts. Try again later." }); }
    let expired_lock = factor.legacy_totp_locked_until.is_some();
    if expired_lock {
        transaction.execute_raw(statement("UPDATE public.account_otp_keys SET num_failures=0 WHERE id=$1", [account_id.into()])).await.map_err(database_error)?;
        transaction.execute_raw(statement("UPDATE public.identity_onboarding SET legacy_totp_locked_until=NULL WHERE account_id=$1", [account_id.into()])).await.map_err(database_error)?;
    }
    let current = std::env::var("RAILS_SECRET_KEY_BASE").map_err(|_| AuthError::internal("Retained authenticator configuration unavailable"))?;
    let old = std::env::var("RAILS_OLD_SECRET_KEY_BASE").ok();
    let verified = if code.len() <= 128 { crate::models::identity::totp::verify_rodauth_otp(&factor.key, Some(current.as_bytes()), old.as_deref().map(str::as_bytes), code, now.timestamp(), factor.last_use.and_utc().timestamp()).map_err(|_| AuthError::internal("Retained authenticator unavailable"))? } else { None };
    if verified.is_none() {
        transaction.execute_raw(statement("UPDATE public.account_otp_keys SET num_failures=num_failures+1 WHERE id=$1", [account_id.into()])).await.map_err(database_error)?;
        if (if expired_lock { 0 } else { factor.num_failures }) + 1 >= 5 {
            transaction.execute_raw(statement("UPDATE public.identity_onboarding SET legacy_totp_locked_until=clock_timestamp()+interval '15 minutes' WHERE account_id=$1", [account_id.into()])).await.map_err(database_error)?;
        }
        super::request::persist_totp_failure(401)?;
        return Ok(Some(AuthError::InvalidCredentials.to_auth_response()));
    }
    transaction.execute_raw(statement("UPDATE public.account_otp_keys SET num_failures=0,last_use=timezone('UTC',clock_timestamp()) WHERE id=$1", [account_id.into()])).await.map_err(database_error)?;
    Ok(None)
}
