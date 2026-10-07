use super::{
    ClinicalAuthSchema, ClinicalStore,
    store::{clinical_id, database_error},
};
use async_trait::async_trait;
use better_auth::plugins::{
    EmailVerificationPlugin, UserManagementPlugin,
    email_verification::SendVerificationEmail,
    user_management::{SendChangeEmailConfirmation, UserInfo},
};
use better_auth_core::{
    AuthContext, AuthError, AuthPlugin, AuthRequest, AuthResponse, AuthResult, AuthRoute,
    CreateVerification, HttpMethod,
    store::VerificationStore,
    wire::{SessionView, UserView},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

pub(super) struct EmailChange(pub Arc<ClinicalStore>);
#[derive(Serialize, Deserialize)]
struct Binding {
    account_id: String,
    session_id: String,
    old_email: String,
    new_email: String,
    token_hash: String,
}
struct Capture(Mutex<Option<String>>);
#[async_trait]
impl SendChangeEmailConfirmation for Capture {
    async fn send(
        &self,
        _user: &UserInfo,
        _email: &str,
        _url: &str,
        token: &str,
    ) -> AuthResult<()> {
        *self
            .0
            .lock()
            .map_err(|_| AuthError::internal("Email confirmation unavailable"))? =
            Some(token.into());
        Ok(())
    }
}
struct Sender {
    store: ClinicalStore,
    account_id: String,
    session_id: String,
    old_email: String,
    new_email: String,
    operation_id: String,
}
fn hash(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}
#[async_trait]
impl SendVerificationEmail for Sender {
    async fn send(&self, user: &UserView, url: &str, token: &str) -> AuthResult<()> {
        if user.id != self.account_id || user.email.as_deref() != Some(&self.new_email) {
            return Err(AuthError::InvalidCredentials);
        }
        let mut link = url::Url::parse(url)
            .map_err(|_| AuthError::internal("Email confirmation unavailable"))?;
        link.set_path("/account/security/email/confirm");
        link.set_query(None);
        link.query_pairs_mut()
            .append_pair("operation_id", &self.operation_id)
            .append_pair("token", token);
        self.store
            .create_verification(CreateVerification {
                identifier: format!("email-change:{}", self.operation_id),
                value: serde_json::to_string(&Binding {
                    account_id: self.account_id.clone(),
                    session_id: self.session_id.clone(),
                    old_email: self.old_email.clone(),
                    new_email: self.new_email.clone(),
                    token_hash: hash(token),
                })?,
                expires_at: chrono::Utc::now() + chrono::Duration::minutes(30),
            })
            .await?;
        super::mail::notice(&self.store, user, "Verify your new email address", &format!("Verify your new email address: {link}\nUse this link in the browser where you requested the change within 30 minutes.")).await
    }
}
pub(super) async fn send(
    store: &ClinicalStore,
    request: &AuthRequest,
    ctx: &AuthContext<ClinicalAuthSchema>,
    user: &UserView,
    session: &SessionView,
    operation_id: &str,
    new_email: &str,
) -> AuthResult<AuthResponse> {
    let capture = Arc::new(Capture(Mutex::new(None)));
    let mut change = AuthRequest::new(HttpMethod::Post, "/change-email");
    change.headers = request.headers.clone();
    change.body = Some(serde_json::to_vec(&json!({"newEmail":new_email}))?);
    let response = UserManagementPlugin::new()
        .change_email_enabled(true)
        .send_change_email_confirmation(capture.clone())
        .on_request(&change, ctx)
        .await?
        .ok_or(AuthError::InvalidCredentials)?;
    if response.status >= 400 {
        return Err(AuthError::bad_request("Email change unavailable"));
    }
    let token = capture
        .0
        .lock()
        .map_err(|_| AuthError::internal("Email confirmation unavailable"))?
        .take()
        .ok_or_else(|| AuthError::internal("Email confirmation unavailable"))?;
    let sender = Arc::new(Sender {
        store: store.clone(),
        account_id: user.id.clone(),
        session_id: session.id.clone(),
        old_email: user.email.clone().ok_or(AuthError::InvalidCredentials)?,
        new_email: new_email.to_lowercase(),
        operation_id: operation_id.into(),
    });
    let mut verify = AuthRequest::new(HttpMethod::Get, "/verify-email");
    verify.headers = request.headers.clone();
    verify.query.insert("token".into(), token);
    let result = EmailVerificationPlugin::new()
        .custom_send_verification_email(sender)
        .on_request(&verify, ctx)
        .await?
        .ok_or(AuthError::InvalidCredentials)?;
    if result.status >= 400 {
        return Err(AuthError::InvalidCredentials);
    }
    Ok(AuthResponse::json(
        200,
        &json!({"status":true,"redirect":"/account/security/email/pending"}),
    )?)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Confirm {
    operation_id: String,
    token: String,
    confirmed: bool,
}
impl EmailChange {
    async fn confirm(
        &self,
        request: &AuthRequest,
        ctx: &AuthContext<ClinicalAuthSchema>,
    ) -> AuthResult<AuthResponse> {
        let input: Confirm = request
            .body_as_json()
            .map_err(|_| AuthError::bad_request("Invalid email confirmation"))?;
        if !input.confirmed
            || input.token.len() > 4096
            || uuid::Uuid::parse_str(&input.operation_id).is_err()
        {
            return Err(AuthError::InvalidCredentials);
        }
        let (user, session) = ctx.require_authoritative_session(request).await?;
        let transaction = self.0.transaction().await?;
        self.0.lock_security_session(&transaction, &session).await?;
        let user = ctx
            .database
            .get_user_by_id(&user.id)
            .await?
            .ok_or(AuthError::Unauthenticated)?;
        let identifier = format!("email-change:{}", input.operation_id);
        let record = ctx
            .database
            .get_verification_by_identifier(&identifier)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;
        if record.expires_at <= chrono::Utc::now() {
            return Err(AuthError::InvalidCredentials);
        }
        let binding: Binding = serde_json::from_str(&record.value)?;
        if binding.account_id != user.id
            || binding.session_id != session.id
            || user.email.as_deref() != Some(&binding.old_email)
            || hash(&input.token) != binding.token_hash
        {
            return Err(AuthError::InvalidCredentials);
        }
        if ctx
            .database
            .get_user_by_email(&binding.new_email)
            .await?
            .is_some()
        {
            return Err(AuthError::bad_request("Email change unavailable"));
        }
        super::request::allow_email_change(&user.id, &binding.new_email)?;
        let mut verify = AuthRequest::new(HttpMethod::Get, "/verify-email");
        verify.headers = request.headers.clone();
        verify.query.insert("token".into(), input.token);
        let response = EmailVerificationPlugin::new()
            .on_request(&verify, ctx)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;
        if response.status >= 400 {
            return Err(AuthError::InvalidCredentials);
        }
        ctx.database
            .consume_verification_by_identifier(&identifier)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;
        self.0
            .audit(&transaction, clinical_id(&user.id)?, "email", "changed")
            .await?;
        super::mail::notice(&self.0, &user, "Email address changed", "Your MedTracker email address was changed. If this was not you, secure your account immediately.").await?;
        transaction.commit().await.map_err(database_error)?;
        Ok(AuthResponse::json(200, &json!({"status":true}))?)
    }
}
#[async_trait]
impl AuthPlugin<ClinicalAuthSchema> for EmailChange {
    fn name(&self) -> &'static str {
        "clinical-email-change"
    }
    fn routes(&self) -> Vec<AuthRoute> {
        vec![AuthRoute::post(
            "/security/email/confirm",
            "confirm_email_change",
        )]
    }
    async fn on_request(
        &self,
        request: &AuthRequest,
        ctx: &AuthContext<ClinicalAuthSchema>,
    ) -> AuthResult<Option<AuthResponse>> {
        if request.method == HttpMethod::Post && request.path == "/security/email/confirm" {
            self.confirm(request, ctx).await.map(Some)
        } else {
            Ok(None)
        }
    }
}
