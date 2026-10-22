use better_auth::{AuthConfig, AuthError, AuthResult, BetterAuth};
use std::sync::Arc;

mod credentials;
mod http;
mod limits;
mod mail;
mod native_hooks;
mod native_instance;
mod native_mail;
pub mod oidc;
mod onboarding;
mod operations;
mod passwordless_registration;
mod passwords;
mod policy;
pub use policy::IdentityPolicy;
mod passkey_policy;
mod passkeys;
pub mod personal_keys;
mod request;
pub(crate) mod schema;
pub mod security;
mod sessions;
mod totp;
pub use http::boundary as http_boundary;
pub(crate) use http::normalize_cookie_headers;
pub use passwords::validate_new_password;
pub(crate) use policy::clinical_id;
pub(crate) use request::trusted;
pub use request::{BrowserIdentity, browser_identity, dispatch, router};
pub use schema::AccountSchema as ClinicalAuthSchema;

pub(crate) fn session_digest(token: &str) -> String {
    crate::models::identity::store::digest(token)
}

#[derive(Clone)]
pub struct IdentityService {
    auth: Arc<BetterAuth<ClinicalAuthSchema>>,
    pub policy: Arc<IdentityPolicy>,
    pub provider: Option<oidc::Provider>,
}

impl axum::extract::FromRef<IdentityService> for Arc<BetterAuth<ClinicalAuthSchema>> {
    fn from_ref(service: &IdentityService) -> Self {
        service.auth.clone()
    }
}

impl std::ops::Deref for IdentityService {
    type Target = BetterAuth<ClinicalAuthSchema>;
    fn deref(&self) -> &Self::Target {
        &self.auth
    }
}

pub async fn build(config: AuthConfig, policy: Arc<IdentityPolicy>) -> AuthResult<IdentityService> {
    let base_url = config
        .base_url
        .as_static()
        .ok_or_else(|| AuthError::internal("Identity URL unavailable"))?;
    let provider = oidc::Provider::configured(base_url, policy.settings())?;
    let native = native_instance::build(
        config,
        policy.clone(),
        provider.as_ref().map(oidc::Provider::plugin),
    )
    .await?;
    Ok(IdentityService {
        auth: Arc::new(native),
        policy,
        provider,
    })
}
