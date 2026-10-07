use std::sync::Arc;

use ::better_auth::{
    AuthConfig, AuthError, AuthResult, BetterAuth,
    plugins::{EmailPasswordPlugin, EmailVerificationPlugin, OrganizationConfig, OrganizationPlugin, SessionManagementPlugin, TwoFactorPlugin, organization::RolePermissions},
};
use url::Url;

mod schema;
mod store;
mod request;
mod mail;
mod onboarding;
mod passkeys;
mod passwords;
mod recovery;
mod confirmation;
mod operations;
mod credentials;
mod regeneration;
mod http;
mod limits;
pub use http::boundary as http_boundary;
mod totp;
mod legacy_totp;
mod recovery_email;
pub use confirmation::confirmation_email;
pub use passwords::validate_new_password;
pub use schema::ClinicalAuthSchema;
pub use store::ClinicalStore;
pub use request::{BrowserIdentity, browser_identity, dispatch, router};

#[derive(Clone)]
pub struct IdentityService {
    auth: Arc<BetterAuth<ClinicalAuthSchema>>,
    pub store: Arc<ClinicalStore>,
}

impl std::ops::Deref for IdentityService {
    type Target = BetterAuth<ClinicalAuthSchema>;
    fn deref(&self) -> &Self::Target { &self.auth }
}

pub async fn build(
    mut config: AuthConfig,
    store: Arc<ClinicalStore>,
) -> AuthResult<IdentityService> {
    let mut origin = Url::parse(&config.base_url)
        .map_err(|_| AuthError::internal("Invalid configured identity origin"))?;
    origin.set_path("/");
    config.session.cookie_cache = None;
    config.session.bearer = None;
    config.session.expires_in = chrono::Duration::days(7);
    config.session.additional_fields.insert("clinical_session_purpose".into(), better_auth_core::config::SessionFieldConfig { field_name: None, input: false, returned: true });
    config.session.additional_fields.insert("legacy_factor_pending".into(), better_auth_core::config::SessionFieldConfig { field_name: None, input: false, returned: true });
    config.session.additional_fields.insert("authentication_method".into(), better_auth_core::config::SessionFieldConfig { field_name: None, input: false, returned: true });
    let mut organization = OrganizationConfig {
        allow_user_to_create_organization: false,
        creator_role: "owner".into(),
        disable_organization_deletion: true,
        membership_limit: None,
        invitation_limit: None,
        ..OrganizationConfig::default()
    };
    organization.roles.insert("administrator".into(), RolePermissions {
        organization: vec!["update".into()],
        member: vec!["create".into(), "update".into(), "delete".into()],
        invitation: vec!["create".into(), "cancel".into()],
        api_key: Vec::new(),
    });
    let verification = || EmailVerificationPlugin::new()
        .auto_sign_in_after_verification(true)
        .require_verification_for_signin(true)
        .custom_send_verification_email(Arc::new(mail::VerificationMailer(store.clone())));
    let instance = BetterAuth::<ClinicalAuthSchema>::new(config)
        .store_arc(store.clone())
        .plugin(request::ClinicalRequest)
        .plugin(onboarding::Onboarding(store.clone()))
        .plugin(recovery::Recovery(store.clone()))
        .plugin(confirmation::Confirmation(store.clone()))
        .plugin(operations::Operations(store.clone()))
        .plugin(credentials::Credentials(store.clone()))
        .plugin(regeneration::Regeneration(store.clone()))
        .plugin(totp::Totp(store.clone()))
        .plugin(EmailPasswordPlugin::new().password_hasher(Arc::new(passwords::CompatibleHasher)).password_min_length(15).password_max_length(1024).auto_sign_in(false).require_email_verification(true).with_email_verification(Arc::new(verification())))
        .plugin(verification())
        .plugin(SessionManagementPlugin::new())
        .plugin(TwoFactorPlugin::new())
        .plugin(passkeys::Passkeys::new(&origin, store.clone())?)
        .plugin(OrganizationPlugin::with_config(organization))
        .build()
        .await?;
    Ok(IdentityService { auth: Arc::new(instance), store })
}
