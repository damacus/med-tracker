use std::sync::Arc;
use async_trait::async_trait;
use better_auth_core::{AuthContext, AuthError, AuthPlugin, AuthRequest, AuthResponse, AuthResult, AuthRoute, CreateVerification, HttpMethod, wire::{SessionView, UserView}};
use sea_orm::ConnectionTrait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use super::{ClinicalAuthSchema, ClinicalStore, operations::PendingPassword, store::{clinical_id, database_error, statement}};

#[derive(Serialize, Deserialize, PartialEq)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Action { Password, RemovePassword, RemovePasskey { target: String }, AddPasskey, RegenerateRecovery }

#[derive(Serialize, Deserialize)]
pub(super) struct RegistrationGrant { pub account_id: String, pub session_id: String }

pub(super) struct Credentials(pub Arc<ClinicalStore>);

async fn validate(store: &ClinicalStore, ctx: &AuthContext<ClinicalAuthSchema>, user: &UserView, action: &Action) -> AuthResult<()> {
    let factor_enabled = ctx.database.get_two_factor_by_user_id(&user.id).await?.is_some_and(|factor| factor.verified);
    let accounts = ctx.database.get_user_accounts(&user.id).await?;
    let password = accounts.iter().any(|account| account.provider_id == "credential" && account.password.is_some());
    let keys = ctx.database.list_passkeys_by_user(&user.id).await?;
    match action {
        Action::RemovePassword if !password || keys.is_empty() => Err(AuthError::bad_request("Keep a local password or passkey")),
        Action::RemovePassword if factor_enabled || super::legacy_totp::enabled(store, &user.id).await? => Err(AuthError::bad_request("Disable your authenticator app before removing your password")),
        Action::RemovePasskey { target } if !keys.iter().any(|key| key.id == *target) => Err(AuthError::InvalidCredentials),
        Action::RemovePasskey { .. } if !password && keys.len() < 2 => Err(AuthError::bad_request("Keep a local password or passkey")),
        Action::Password => Err(AuthError::bad_request("Use the password change operation")),
        _ => Ok(()),
    }
}

impl Credentials {
    async fn start(&self, request: &AuthRequest, ctx: &AuthContext<ClinicalAuthSchema>) -> AuthResult<AuthResponse> {
        let action: Action = request.body_as_json().map_err(|_| AuthError::bad_request("Invalid account operation"))?;
        let (user, session) = ctx.require_authoritative_session(request).await?;
        if session.additional_fields.get("authentication_method").and_then(serde_json::Value::as_str) == Some("recovery") { return Err(AuthError::forbidden("Fresh email confirmation is required")); }
        let transaction = self.0.transaction().await?;
        self.0.lock_security_session(&transaction, &session).await?;
        validate(&self.0, ctx, &user, &action).await?;
        let accounts = ctx.database.get_user_accounts(&user.id).await?;
        let credential = accounts.iter().find(|account| account.provider_id == "credential");
        let operation_id = uuid::Uuid::new_v4().to_string();
        let pending = PendingPassword { account_id: user.id.clone(), session_id: session.id, credential_id: credential.map(|account| account.id.clone()), password_hash: String::new(), action };
        ctx.database.create_verification(CreateVerification { identifier: format!("password-change:{operation_id}"), value: serde_json::to_string(&pending)?, expires_at: chrono::Utc::now() + chrono::Duration::minutes(5) }).await?;
        let response = AuthResponse::json(200, &json!({"operation_id":operation_id,"password_proof":credential.is_some_and(|account| account.password.is_some()),"passkey_proof":!ctx.database.list_passkeys_by_user(&user.id).await?.is_empty()}))?;
        transaction.commit().await.map_err(database_error)?;
        Ok(response)
    }
}

pub(super) async fn complete(store: &ClinicalStore, ctx: &AuthContext<ClinicalAuthSchema>, user: &UserView, session: &SessionView, operation_id: &str, pending: PendingPassword) -> AuthResult<AuthResponse> {
    validate(store, ctx, user, &pending.action).await?;
    if pending.action == Action::RegenerateRecovery { return super::regeneration::issue(store, ctx, user, session).await; }
    if pending.action == Action::AddPasskey {
        ctx.database.create_verification(CreateVerification { identifier: format!("passkey-add:{operation_id}"), value: serde_json::to_string(&RegistrationGrant { account_id: user.id.clone(), session_id: session.id.clone() })?, expires_at: chrono::Utc::now() + chrono::Duration::minutes(5) }).await?;
        return Ok(AuthResponse::json(200, &json!({"status":true,"redirect":format!("/account/security/passkey?operation_id={operation_id}")}))?);
    }
    let transaction = store.transaction().await?;
    let (kind, message) = match pending.action {
        Action::RemovePassword => {
            transaction.execute_raw(statement("UPDATE public.identity_provider_accounts SET password=NULL,updated_at=clock_timestamp() WHERE id=$1 AND account_id=$2 AND provider_id='credential'", [pending.credential_id.ok_or(AuthError::InvalidCredentials)?.into(), clinical_id(&user.id)?.into()])).await.map_err(database_error)?;
            ("password", "Your MedTracker password was removed.")
        }
        Action::RemovePasskey { target } => { ctx.database.delete_passkey(&target).await?; ("passkey", "A passkey was removed from your MedTracker account.") }
        _ => return Err(AuthError::InvalidCredentials),
    };
    store.audit(&transaction, clinical_id(&user.id)?, kind, "removed").await?;
    super::mail::notice(store, user, "Sign-in method removed", message).await?;
    transaction.commit().await.map_err(database_error)?;
    Ok(AuthResponse::json(200, &json!({"status":true}))?)
}

pub(super) async fn registration_grant(ctx: &AuthContext<ClinicalAuthSchema>, session: &SessionView, operation_id: &str) -> AuthResult<()> {
    if uuid::Uuid::parse_str(operation_id).is_err() { return Err(AuthError::InvalidCredentials); }
    let record = ctx.database.get_verification_by_identifier(&format!("passkey-add:{operation_id}")).await?.ok_or(AuthError::InvalidCredentials)?;
    if record.expires_at <= chrono::Utc::now() { return Err(AuthError::InvalidCredentials); }
    let grant: RegistrationGrant = serde_json::from_str(&record.value)?;
    if grant.account_id != session.user_id || grant.session_id != session.id { return Err(AuthError::InvalidCredentials); }
    Ok(())
}

#[async_trait]
impl AuthPlugin<ClinicalAuthSchema> for Credentials {
    fn name(&self) -> &'static str { "clinical-credential-lifecycle" }
    fn routes(&self) -> Vec<AuthRoute> { vec![AuthRoute::post("/security/operation/start", "start_account_operation")] }
    async fn on_request(&self, request: &AuthRequest, ctx: &AuthContext<ClinicalAuthSchema>) -> AuthResult<Option<AuthResponse>> {
        if request.method == HttpMethod::Post && request.path == "/security/operation/start" { self.start(request, ctx).await.map(Some) } else { Ok(None) }
    }
}
