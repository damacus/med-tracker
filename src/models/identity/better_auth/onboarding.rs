use std::sync::Arc;

use async_trait::async_trait;
use better_auth::{AuthError, AuthResult, plugins::{ApiKeyPlugin, EmailVerificationPlugin, api_key::CreateKeyRequest}};
use better_auth_core::{AuthRequest, AuthResponse, CreateAccount, HttpMethod, plugin::{AuthContext, AuthPlugin, AuthRoute}, store::{AccountStore, UserStore}, utils::password::hash_password};
use sea_orm::ConnectionTrait;
use serde::Deserialize;
use serde_json::json;

use super::{ClinicalAuthSchema, ClinicalStore, mail::VerificationMailer, store::{clinical_id, context, database_error, statement}};

pub(super) const RECOVERY_CONFIG: &str = "medtracker-recovery";
pub(super) struct Onboarding(pub Arc<ClinicalStore>);

#[derive(Deserialize)]
struct Signup {
    name: String,
    email: String,
    date_of_birth: chrono::NaiveDate,
    invitation_token: Option<String>,
    credential: String,
    password: Option<String>,
}

#[async_trait]
impl AuthPlugin<ClinicalAuthSchema> for Onboarding {
    fn name(&self) -> &'static str { "clinical-onboarding" }

    fn routes(&self) -> Vec<AuthRoute> {
        vec![AuthRoute::post("/onboarding/signup", "clinical_signup"), AuthRoute::post("/onboarding/recovery-codes", "onboarding_recovery_codes"), AuthRoute::post("/onboarding/complete", "complete_onboarding")]
    }

    async fn on_request(&self, request: &AuthRequest, ctx: &AuthContext<ClinicalAuthSchema>) -> AuthResult<Option<AuthResponse>> {
        if request.method != HttpMethod::Post { return Ok(None); }
        match request.path.as_str() {
            "/onboarding/signup" => self.signup(request, ctx).await.map(Some),
            "/onboarding/recovery-codes" => self.codes(request, ctx).await.map(Some),
            "/onboarding/complete" => self.complete(request, ctx).await.map(Some),
            _ => Ok(None),
        }
    }
}

impl Onboarding {
    async fn signup(&self, request: &AuthRequest, ctx: &AuthContext<ClinicalAuthSchema>) -> AuthResult<AuthResponse> {
        let input: Signup = serde_json::from_slice(request.body.as_deref().unwrap_or_default())?;
        let password = match input.credential.as_str() {
            "passkey" if input.password.as_deref().is_none_or(str::is_empty) => None,
            "password" => {
                let password = input.password.as_deref().ok_or_else(|| AuthError::validation("Password is required"))?;
                super::validate_new_password(password).map_err(AuthError::validation)?;
                Some(hash_password(None, password).await?)
            }
            _ => return Err(AuthError::validation("Choose password or passkey")),
        };
        let user = self.0.create_user(better_auth_core::CreateUser {
            name: Some(input.name), email: Some(input.email),
            metadata: Some(json!({"date_of_birth": input.date_of_birth, "invitation_token": input.invitation_token})),
            ..Default::default()
        }).await?;
        if let Some(password) = password {
            self.0.create_account(CreateAccount { user_id: user.id.clone(), account_id: user.id.clone(), provider_id: "credential".into(), password: Some(password), access_token: None, refresh_token: None, id_token: None, access_token_expires_at: None, refresh_token_expires_at: None, scope: None }).await?;
        }
        let mut send = AuthRequest::new(HttpMethod::Post, "/send-verification-email");
        send.body = Some(serde_json::to_vec(&json!({"email":user.email, "callbackURL":"/auth/passkey/setup"}))?);
        EmailVerificationPlugin::new().custom_send_verification_email(Arc::new(VerificationMailer(self.0.clone())))
            .on_request(&send, ctx).await?.ok_or_else(|| AuthError::internal("Verification email unavailable"))?;
        Ok(AuthResponse::json(200, &json!({"status":true}))?)
    }

    async fn codes(&self, request: &AuthRequest, ctx: &AuthContext<ClinicalAuthSchema>) -> AuthResult<AuthResponse> {
        let (user, session) = ctx.require_authoritative_session(request).await?;
        if super::legacy_totp::pending(&session) { return Err(AuthError::Unauthenticated); }
        if session.additional_fields.get("clinical_session_purpose").and_then(serde_json::Value::as_str) != Some("enrolment") { return Err(AuthError::Unauthenticated); }
        let transaction = self.0.transaction().await?;
        let account_id = clinical_id(&user.id)?;
        context(&*transaction, "med_tracker.current_account_id", &user.id).await?;
        transaction.query_one_raw(statement("SELECT id FROM public.accounts WHERE id=$1 AND status=2 FOR UPDATE", [account_id.into()])).await.map_err(database_error)?.ok_or(AuthError::Unauthenticated)?;
        let row = transaction.query_one_raw(statement("SELECT a.id FROM public.accounts a JOIN public.identity_onboarding o ON o.account_id=a.id WHERE a.id=$1 AND o.recovery_saved_at IS NULL AND EXISTS(SELECT 1 FROM public.identity_sessions WHERE token=$2 AND account_id=a.id AND purpose='enrolment') AND (EXISTS(SELECT 1 FROM public.identity_passkeys WHERE account_id=a.id) OR EXISTS(SELECT 1 FROM public.identity_provider_accounts WHERE account_id=a.id AND provider_id='credential' AND password IS NOT NULL))", [account_id.into(), session.token.clone().into()])).await.map_err(database_error)?;
        if row.is_none() { return Err(AuthError::forbidden("Recovery codes are unavailable")); }
        let result = issue_codes(&self.0, ctx, &user.id, true).await?;
        transaction.commit().await.map_err(database_error)?;
        Ok(AuthResponse::json(200, &result)?)
    }

    async fn complete(&self, request: &AuthRequest, ctx: &AuthContext<ClinicalAuthSchema>) -> AuthResult<AuthResponse> {
        let (user, session) = ctx.require_authoritative_session(request).await?;
        if super::legacy_totp::pending(&session) { return Err(AuthError::Unauthenticated); }
        let body: serde_json::Value = serde_json::from_slice(request.body.as_deref().unwrap_or_default())?;
        if body.get("saved").and_then(serde_json::Value::as_bool) != Some(true) || session.additional_fields.get("clinical_session_purpose").and_then(serde_json::Value::as_str) != Some("enrolment") { return Err(AuthError::forbidden("Save your recovery codes before continuing")); }
        let transaction = self.0.transaction().await?;
        context(&*transaction, "med_tracker.current_account_id", &user.id).await?;
        transaction.query_one_raw(statement("SELECT id FROM public.accounts WHERE id=$1 FOR UPDATE", [clinical_id(&user.id)?.into()])).await.map_err(database_error)?.ok_or(AuthError::Unauthenticated)?;
        let generation = body.get("generation").and_then(serde_json::Value::as_str).ok_or_else(|| AuthError::forbidden("Save the current recovery codes before continuing"))?;
        let changed = transaction.execute_raw(statement("UPDATE public.identity_onboarding SET recovery_saved_at=CURRENT_TIMESTAMP WHERE account_id=$1 AND recovery_saved_at IS NULL AND (SELECT count(*) FROM public.identity_api_keys WHERE account_id=$1 AND payload->>'configId'=$2 AND (payload->>'metadata')::jsonb->>'generation'=$3)=10", [clinical_id(&user.id)?.into(), RECOVERY_CONFIG.into(), generation.into()])).await.map_err(database_error)?.rows_affected();
        if changed != 1 { return Err(AuthError::forbidden("Onboarding is unavailable")); }
        transaction.execute_raw(statement("UPDATE public.identity_sessions SET purpose='authenticated' WHERE token=$1 AND account_id=$2 AND purpose='enrolment'", [session.token.into(), clinical_id(&user.id)?.into()])).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        Ok(AuthResponse::json(200, &json!({"status":true}))?)
    }
}

pub(super) async fn issue_codes(store: &ClinicalStore, ctx: &AuthContext<ClinicalAuthSchema>, user_id: &str, acknowledged: bool) -> AuthResult<serde_json::Value> {
    let transaction = store.transaction().await?;
    let account_id = clinical_id(user_id)?;
    context(&transaction, "med_tracker.current_account_id", user_id).await?;
    transaction.query_one_raw(statement("SELECT id FROM public.accounts WHERE id=$1 AND status=2 FOR UPDATE", [account_id.into()])).await.map_err(database_error)?.ok_or(AuthError::Unauthenticated)?;
    transaction.execute_raw(statement("DELETE FROM public.identity_api_keys WHERE account_id=$1 AND payload->>'configId'=$2", [account_id.into(), RECOVERY_CONFIG.into()])).await.map_err(database_error)?;
    let generation = uuid::Uuid::new_v4().to_string();
    super::request::allow_recovery_issuance(account_id)?;
    let plugin = ApiKeyPlugin::builder().config_id(RECOVERY_CONFIG.to_owned()).key_length(64).prefix("recovery_".to_owned()).store_starting_characters(false).enable_metadata(true).build();
    let mut codes = Vec::with_capacity(10);
    for _ in 0..10 {
        let key = plugin.create_key(ctx, &CreateKeyRequest { config_id: Some(RECOVERY_CONFIG.into()), user_id: Some(user_id.into()), remaining: Some(1.0), rate_limit_enabled: Some(false), permissions: Some(Default::default()), metadata: Some(json!({"generation":generation,"acknowledged":acknowledged})), ..Default::default() }).await?;
        codes.push(key.key);
    }
    transaction.commit().await.map_err(database_error)?;
    Ok(json!({"recoveryCodes":codes,"generation":generation}))
}
