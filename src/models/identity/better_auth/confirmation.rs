use std::sync::Arc;
use async_trait::async_trait;
use better_auth::{plugins::EmailVerificationPlugin};
use better_auth_core::{AuthContext, AuthError, AuthPlugin, AuthRequest, AuthResponse, AuthResult, AuthRoute, HttpMethod, store::{UserStore, VerificationStore}};
use sha2::{Digest, Sha256};
use serde::Deserialize;

use super::{ClinicalAuthSchema, ClinicalStore};

pub(super) fn identifier(token: &str) -> String { format!("email-confirmation:{}", hex::encode(Sha256::digest(token.as_bytes()))) }

pub async fn confirmation_email(service: &super::IdentityService, token: &str) -> AuthResult<String> {
    if token.len() > 4096 { return Err(AuthError::InvalidCredentials); }
    let verification = service.store.get_verification_by_identifier(&identifier(token)).await?.ok_or(AuthError::InvalidCredentials)?;
    let user = service.store.get_user_by_id(&verification.value).await?.ok_or(AuthError::InvalidCredentials)?;
    user.email.ok_or(AuthError::InvalidCredentials)
}

pub(super) struct Confirmation(pub Arc<ClinicalStore>);

#[derive(Deserialize)]
struct Input { token: String, confirmed: bool }

#[async_trait]
impl AuthPlugin<ClinicalAuthSchema> for Confirmation {
    fn name(&self) -> &'static str { "clinical-email-confirmation" }
    fn routes(&self) -> Vec<AuthRoute> { vec![AuthRoute::post("/onboarding/confirm-email", "confirm_email")] }
    async fn on_request(&self, request: &AuthRequest, ctx: &AuthContext<ClinicalAuthSchema>) -> AuthResult<Option<AuthResponse>> {
        if request.method != HttpMethod::Post || request.path != "/onboarding/confirm-email" { return Ok(None); }
        let input: Input = request.body_as_json()?;
        if !input.confirmed || input.token.len() > 4096 { return Err(AuthError::InvalidCredentials); }
        self.0.consume_verification_by_identifier(&identifier(&input.token)).await?.ok_or(AuthError::InvalidCredentials)?;
        let mut verification = AuthRequest::new(HttpMethod::Get, "/verify-email");
        verification.headers = request.headers.clone();
        verification.query.insert("token".into(), input.token);
        verification.query.insert("callbackURL".into(), "/auth/passkey/setup".into());
        EmailVerificationPlugin::new().auto_sign_in_after_verification(true).require_verification_for_signin(true).on_request(&verification, ctx).await
    }
}
