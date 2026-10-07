use std::sync::Arc;

use async_trait::async_trait;
use better_auth::plugins::PasswordManagementPlugin;
use better_auth_core::{
    AuthContext, AuthError, AuthPlugin, AuthRequest, AuthResponse, AuthResult, AuthRoute,
    CreateAccount, CreateVerification, HttpMethod, UpdateAccount,
    utils::password::hash_password,
    wire::{SessionView, UserView},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{
    ClinicalAuthSchema, ClinicalStore,
    store::{clinical_id, database_error},
};

pub(super) struct Operations(pub Arc<ClinicalStore>);

#[derive(Serialize, Deserialize)]
pub(super) struct PendingPassword {
    pub account_id: String,
    pub session_id: String,
    pub credential_id: Option<String>,
    pub password_hash: String,
    pub action: super::credentials::Action,
    #[serde(default)]
    pub factor: Option<super::totp::FactorBinding>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StartPassword {
    new_password: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfirmPassword {
    operation_id: String,
    password: String,
    totp_code: Option<String>,
}

pub(super) async fn pending_password(
    ctx: &AuthContext<ClinicalAuthSchema>,
    session: &SessionView,
    operation_id: &str,
) -> AuthResult<PendingPassword> {
    if uuid::Uuid::parse_str(operation_id).is_err() {
        return Err(AuthError::InvalidCredentials);
    }
    let verification = ctx
        .database
        .get_verification_by_identifier(&format!("password-change:{operation_id}"))
        .await?
        .ok_or(AuthError::InvalidCredentials)?;
    if verification.expires_at <= chrono::Utc::now() {
        return Err(AuthError::InvalidCredentials);
    }
    let pending: PendingPassword = serde_json::from_str(&verification.value)?;
    let accounts = ctx.database.get_user_accounts(&session.user_id).await?;
    let credential = accounts
        .iter()
        .find(|account| account.provider_id == "credential");
    if pending.account_id != session.user_id
        || pending.session_id != session.id
        || pending.credential_id.as_deref() != credential.map(|account| account.id.as_str())
    {
        return Err(AuthError::InvalidCredentials);
    }
    Ok(pending)
}

pub(super) async fn complete_password(
    store: &ClinicalStore,
    request: &AuthRequest,
    ctx: &AuthContext<ClinicalAuthSchema>,
    user: &UserView,
    session: &SessionView,
    operation_id: &str,
) -> AuthResult<AuthResponse> {
    let transaction = store.transaction().await?;
    store.lock_security_session(&transaction, session).await?;
    let pending = pending_password(ctx, session, operation_id).await?;
    ctx.database
        .consume_verification_by_identifier(&format!("password-change:{operation_id}"))
        .await?
        .ok_or(AuthError::InvalidCredentials)?;
    if pending.action != super::credentials::Action::Password {
        let response =
            super::credentials::complete(store, request, ctx, user, session, operation_id, pending)
                .await?;
        transaction.commit().await.map_err(database_error)?;
        return Ok(response);
    }
    if let Some(credential_id) = pending.credential_id {
        ctx.database
            .update_account(
                &credential_id,
                UpdateAccount {
                    password: Some(pending.password_hash),
                    access_token: None,
                    refresh_token: None,
                    id_token: None,
                    access_token_expires_at: None,
                    refresh_token_expires_at: None,
                    scope: None,
                },
            )
            .await?;
    } else {
        ctx.database
            .create_account(CreateAccount {
                user_id: user.id.clone(),
                account_id: user.id.clone(),
                provider_id: "credential".into(),
                password: Some(pending.password_hash),
                access_token: None,
                refresh_token: None,
                id_token: None,
                access_token_expires_at: None,
                refresh_token_expires_at: None,
                scope: None,
            })
            .await?;
    }
    store
        .audit(&transaction, clinical_id(&user.id)?, "password", "changed")
        .await?;
    super::mail::notice(store, user, "Password changed", "Your MedTracker password was changed. If this was not you, secure your account immediately.").await?;
    transaction.commit().await.map_err(database_error)?;
    Ok(AuthResponse::json(200, &json!({"status":true}))?)
}

impl Operations {
    async fn password(
        &self,
        request: &AuthRequest,
        ctx: &AuthContext<ClinicalAuthSchema>,
    ) -> AuthResult<AuthResponse> {
        let (user, session) = ctx.require_authoritative_session(request).await?;
        if session
            .additional_fields
            .get("clinical_session_purpose")
            .and_then(serde_json::Value::as_str)
            != Some("authenticated")
        {
            return Err(AuthError::Unauthenticated);
        }
        let recovery = session
            .additional_fields
            .get("authentication_method")
            .and_then(serde_json::Value::as_str)
            == Some("recovery");
        if recovery && request.path != "/security/password/start" {
            return Err(AuthError::forbidden("Fresh email confirmation is required"));
        }
        let transaction = self.0.transaction().await?;
        self.0.lock_security_session(&transaction, &session).await?;
        let user = ctx
            .database
            .get_user_by_id(&user.id)
            .await?
            .ok_or(AuthError::Unauthenticated)?;
        let accounts = ctx.database.get_user_accounts(&user.id).await?;
        let credential = accounts
            .iter()
            .find(|account| account.provider_id == "credential");
        let response = if request.path == "/security/password/start" {
            let input: StartPassword = request
                .body_as_json()
                .map_err(|_| AuthError::bad_request("Invalid password operation"))?;
            super::validate_new_password(&input.new_password).map_err(AuthError::validation)?;
            let operation_id = uuid::Uuid::new_v4().to_string();
            let pending = PendingPassword {
                account_id: user.id.clone(),
                session_id: session.id.clone(),
                credential_id: credential.map(|account| account.id.clone()),
                password_hash: hash_password(None, &input.new_password).await?,
                action: super::credentials::Action::Password,
                factor: None,
            };
            ctx.database
                .create_verification(CreateVerification {
                    identifier: format!("password-change:{operation_id}"),
                    value: serde_json::to_string(&pending)?,
                    expires_at: chrono::Utc::now()
                        + chrono::Duration::minutes(if recovery { 30 } else { 5 }),
                })
                .await?;
            if recovery {
                super::recovery_email::send(&self.0, ctx, &user, &session, &operation_id).await?;
            }
            AuthResponse::json(
                200,
                &json!({"operation_id":operation_id,"email_proof":recovery,"password_proof":!recovery && credential.is_some_and(|account| account.password.is_some()),"passkey_proof":!recovery && !ctx.database.list_passkeys_by_user(&user.id).await?.is_empty(),"oidc_proof":!recovery && accounts.iter().any(|account| account.provider_id=="zitadel")}),
            )?
        } else {
            let input: ConfirmPassword = request
                .body_as_json()
                .map_err(|_| AuthError::bad_request("Invalid password operation"))?;
            if credential.is_none_or(|account| account.password.is_none()) {
                return Err(AuthError::InvalidCredentials);
            }
            pending_password(ctx, &session, &input.operation_id).await?;
            let mut proof = AuthRequest::new(HttpMethod::Post, "/verify-password");
            proof.headers = request.headers.clone();
            proof.body = Some(serde_json::to_vec(&json!({"password":input.password}))?);
            let result = PasswordManagementPlugin::new()
                .password_hasher(Arc::new(super::passwords::CompatibleHasher))
                .on_request(&proof, ctx)
                .await?
                .ok_or_else(|| AuthError::internal("Password verification unavailable"))?;
            if result.status >= 400 {
                return Err(AuthError::InvalidCredentials);
            }
            if user.two_factor_enabled || super::legacy_totp::enabled(&self.0, &user.id).await? {
                let code = input
                    .totp_code
                    .as_deref()
                    .ok_or(AuthError::InvalidCredentials)?;
                let rejected = if user.two_factor_enabled {
                    super::totp::verify_operation_factor(request, ctx, code).await?
                } else {
                    super::legacy_totp::verify_operation(&self.0, request, ctx, code).await?
                };
                if let Some(rejected) = rejected {
                    transaction.commit().await.map_err(database_error)?;
                    return Ok(rejected);
                }
            }
            complete_password(&self.0, request, ctx, &user, &session, &input.operation_id).await?
        };
        transaction.commit().await.map_err(database_error)?;
        Ok(response)
    }
}

#[async_trait]
impl AuthPlugin<ClinicalAuthSchema> for Operations {
    fn name(&self) -> &'static str {
        "clinical-security-operations"
    }
    fn routes(&self) -> Vec<AuthRoute> {
        vec![
            AuthRoute::post("/security/password/start", "start_password_change"),
            AuthRoute::post("/security/password/confirm", "confirm_password_change"),
            AuthRoute::post(
                "/security/password/email/confirm",
                "confirm_recovery_password_change",
            ),
        ]
    }
    async fn on_request(
        &self,
        request: &AuthRequest,
        ctx: &AuthContext<ClinicalAuthSchema>,
    ) -> AuthResult<Option<AuthResponse>> {
        if request.method == HttpMethod::Post && request.path == "/security/password/email/confirm"
        {
            return super::recovery_email::confirm(&self.0, request, ctx)
                .await
                .map(Some);
        }
        if request.method == HttpMethod::Post
            && matches!(
                request.path.as_str(),
                "/security/password/start" | "/security/password/confirm"
            )
        {
            self.password(request, ctx).await.map(Some)
        } else {
            Ok(None)
        }
    }
}
