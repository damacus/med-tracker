use super::{
    ClinicalAuthSchema, ClinicalStore,
    store::{clinical_id, database_error, statement},
};
use crate::models::identity::store::digest as token_digest;
use async_trait::async_trait;
use better_auth::plugins::{PasswordManagementPlugin, password_management::SendResetPassword};
use better_auth_core::{
    AuthContext, AuthError, AuthPlugin, AuthRequest, AuthResponse, AuthResult, CreateVerification,
    HttpMethod,
    store::VerificationStore,
    wire::{SessionView, UserView},
};
use sea_orm::ConnectionTrait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
struct Binding {
    account_id: String,
    session_id: String,
    token_hash: String,
}

struct Sender {
    store: Arc<ClinicalStore>,
    account_id: String,
    session_id: String,
    operation_id: String,
    label: &'static str,
}

fn label(action: &super::credentials::Action) -> AuthResult<&'static str> {
    match action {
        super::credentials::Action::Password => Ok("Confirm password change"),
        super::credentials::Action::AddPasskey => Ok("Confirm passkey replacement"),
        _ => Err(AuthError::InvalidCredentials),
    }
}

pub async fn confirmation_label(
    service: &super::IdentityService,
    request: &AuthRequest,
    operation_id: &str,
) -> AuthResult<&'static str> {
    let (_, session) = service
        .context()
        .require_authoritative_session(request)
        .await?;
    if session
        .additional_fields
        .get("authentication_method")
        .and_then(serde_json::Value::as_str)
        != Some("recovery")
    {
        return Err(AuthError::Unauthenticated);
    }
    label(
        &super::operations::pending_password(service.context(), &session, operation_id)
            .await?
            .action,
    )
}

fn token_hash(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

#[async_trait]
impl SendResetPassword for Sender {
    async fn send(&self, user: &serde_json::Value, url: &str, token: &str) -> AuthResult<()> {
        let user: UserView = serde_json::from_value(user.clone())?;
        if user.id != self.account_id {
            return Err(AuthError::InvalidCredentials);
        }
        let mut link = url::Url::parse(url)
            .map_err(|_| AuthError::internal("Email confirmation unavailable"))?;
        link.set_path("/account/security/password/email");
        link.set_query(None);
        link.query_pairs_mut()
            .append_pair("operation_id", &self.operation_id)
            .append_pair("token", token);
        self.store
            .create_verification(CreateVerification {
                identifier: format!("password-email:{}", self.operation_id),
                value: serde_json::to_string(&Binding {
                    account_id: self.account_id.clone(),
                    session_id: self.session_id.clone(),
                    token_hash: token_hash(token),
                })?,
                expires_at: chrono::Utc::now() + chrono::Duration::minutes(30),
            })
            .await?;
        super::mail::notice(&self.store, &user, self.label, &format!("{} for your MedTracker account: {link}\nThis link expires in 30 minutes. If you did not request this change, do not confirm it.", self.label)).await
    }
}

pub(super) async fn send(
    store: &Arc<ClinicalStore>,
    ctx: &AuthContext<ClinicalAuthSchema>,
    user: &UserView,
    session: &SessionView,
    operation_id: &str,
) -> AuthResult<()> {
    let label = label(
        &super::operations::pending_password(ctx, session, operation_id)
            .await?
            .action,
    )?;
    let sender = Arc::new(Sender {
        store: store.clone(),
        account_id: user.id.clone(),
        session_id: session.id.clone(),
        operation_id: operation_id.into(),
        label,
    });
    let mut request = AuthRequest::new(HttpMethod::Post, "/request-password-reset");
    request.body = Some(serde_json::to_vec(
        &json!({"email":user.email,"redirectTo":"/account/security/password/email"}),
    )?);
    let response = PasswordManagementPlugin::new()
        .send_reset_password(sender)
        .reset_token_expiry_hours(1)
        .on_request(&request, ctx)
        .await?
        .ok_or_else(|| AuthError::internal("Email confirmation unavailable"))?;
    if response.status >= 400
        || store
            .get_verification_by_identifier(&format!("password-email:{operation_id}"))
            .await?
            .is_none()
    {
        return Err(AuthError::internal("Email confirmation unavailable"));
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Confirmation {
    pub operation_id: String,
    pub token: String,
    pub confirmed: bool,
}

pub(super) async fn confirm(
    store: &ClinicalStore,
    request: &AuthRequest,
    ctx: &AuthContext<ClinicalAuthSchema>,
) -> AuthResult<AuthResponse> {
    let input: Confirmation = request
        .body_as_json()
        .map_err(|_| AuthError::bad_request("Invalid email confirmation"))?;
    if !input.confirmed || input.token.len() > 128 {
        return Err(AuthError::InvalidCredentials);
    }
    let (user, session) = ctx.require_authoritative_session(request).await?;
    if session
        .additional_fields
        .get("authentication_method")
        .and_then(serde_json::Value::as_str)
        != Some("recovery")
    {
        return Err(AuthError::forbidden("A recovery session is required"));
    }
    let transaction = store.transaction().await?;
    store.lock_security_session(&transaction, &session).await?;
    let pending = super::operations::pending_password(ctx, &session, &input.operation_id).await?;
    let identifier = format!("password-email:{}", input.operation_id);
    let binding = ctx
        .database
        .get_verification_by_identifier(&identifier)
        .await?
        .ok_or(AuthError::InvalidCredentials)?;
    if binding.expires_at <= chrono::Utc::now() {
        return Err(AuthError::InvalidCredentials);
    }
    let binding: Binding = serde_json::from_str(&binding.value)?;
    if binding.account_id != user.id
        || binding.session_id != session.id
        || binding.token_hash != token_hash(&input.token)
    {
        return Err(AuthError::InvalidCredentials);
    }
    let mut validation =
        AuthRequest::new(HttpMethod::Get, format!("/reset-password/{}", input.token));
    validation.query.insert(
        "callbackURL".into(),
        "/account/security/password/email".into(),
    );
    let response = PasswordManagementPlugin::new()
        .on_request(&validation, ctx)
        .await?
        .ok_or(AuthError::InvalidCredentials)?;
    let location = response
        .headers
        .get("location")
        .and_then(|value| url::Url::parse(value).ok())
        .ok_or(AuthError::InvalidCredentials)?;
    if !location
        .query_pairs()
        .any(|(name, value)| name == "token" && value == input.token)
    {
        return Err(AuthError::InvalidCredentials);
    }
    let token = ctx
        .database
        .consume_verification_by_identifier(&format!("reset-password:{}", input.token))
        .await?
        .ok_or(AuthError::InvalidCredentials)?;
    if token.value != user.id {
        return Err(AuthError::InvalidCredentials);
    }
    ctx.database
        .consume_verification_by_identifier(&identifier)
        .await?
        .ok_or(AuthError::InvalidCredentials)?;
    let response = super::operations::complete_password(
        store,
        request,
        ctx,
        &user,
        &session,
        &input.operation_id,
    )
    .await?;
    let response = if pending.action == super::credentials::Action::AddPasskey {
        response
    } else {
        complete_session(store, ctx, &user, &session, response).await?
    };
    transaction.commit().await.map_err(database_error)?;
    Ok(response)
}

pub(super) async fn complete_session(
    store: &ClinicalStore,
    ctx: &AuthContext<ClinicalAuthSchema>,
    user: &UserView,
    session: &SessionView,
    response: AuthResponse,
) -> AuthResult<AuthResponse> {
    let transaction = store.transaction().await?;
    store.lock_security_session(&transaction, session).await?;
    ctx.database.delete_session(&session.token).await?;
    let issued = ctx
        .session_manager()
        .create_session(user, session.ip_address.clone(), session.user_agent.clone())
        .await?;
    transaction.execute_raw(statement("UPDATE public.identity_sessions SET additional_fields=additional_fields || jsonb_build_object('authentication_method','credential-replaced') WHERE token=$1 AND account_id=$2", [token_digest(&issued.token).into(), clinical_id(&user.id)?.into()])).await.map_err(database_error)?;
    transaction.commit().await.map_err(database_error)?;
    Ok(response.with_header(
        "Set-Cookie",
        better_auth_core::utils::cookie_utils::create_session_cookie(&issued.token, &ctx.config),
    ))
}
