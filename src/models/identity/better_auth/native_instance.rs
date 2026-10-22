use std::sync::Arc;

use better_auth::{
    AuthConfig, AuthError, AuthResult, BetterAuth,
    plugins::{
        AccountManagementPlugin, ApiKeyPlugin, EmailPasswordPlugin, EmailVerificationPlugin,
        OAuthPlugin, PasskeyPlugin, PasswordManagementPlugin, SessionManagementPlugin,
        TwoFactorPlugin, UserManagementPlugin, api_key::ApiKeyConfig,
    },
};
use better_auth_core::user_fields::{UserFieldConfig, UserFieldType};

use super::{
    IdentityPolicy,
    schema::{AccountSchema, PluginSchema},
};

pub(super) async fn build(
    mut config: AuthConfig,
    policy: Arc<IdentityPolicy>,
    oauth: Option<OAuthPlugin>,
) -> AuthResult<BetterAuth<AccountSchema>> {
    let origin = url::Url::parse(
        config
            .base_url
            .as_static()
            .ok_or_else(|| AuthError::internal("Identity URL unavailable"))?,
    )
    .map_err(|_| AuthError::internal("Invalid configured identity origin"))?;
    config.app_name = "MedTracker".into();
    config.advanced.database.generate_id = Some(better_auth::config::IdGeneration::Database);
    config.advanced.background_tasks = None;
    config.session.cookie_cache = None;
    config.session.bearer = None;
    let lifetime = super::super::store::Lifetime::from_settings(policy.settings())
        .map_err(|_| AuthError::config("Session policy unavailable"))?;
    config.session.expires_in = Some(chrono::Duration::days(lifetime.inactivity_days()));
    for name in ["clinical_session_purpose", "authentication_method"] {
        config.session.fields_mut().insert(
            name.into(),
            better_auth_core::config::SessionFieldConfig {
                input: Some(false),
                returned: Some(matches!(
                    name,
                    "clinical_session_purpose" | "authentication_method"
                )),
                ..Default::default()
            },
        );
    }
    config.session.fields_mut().insert(
        "active_household_id".into(),
        better_auth_core::config::SessionFieldConfig {
            field_type: UserFieldType::Number,
            input: Some(false),
            returned: Some(true),
            bigint: Some(true),
            ..Default::default()
        },
    );
    for (name, kind, input) in [
        ("person_id", UserFieldType::Number, false),
        ("status", UserFieldType::Number, false),
        ("preferences", UserFieldType::Json, false),
        ("metadata", UserFieldType::Json, true),
    ] {
        config.user.fields_mut().insert(
            name.into(),
            UserFieldConfig {
                field_type: kind,
                required: Some(false),
                input: Some(input),
                returned: Some(false),
                bigint: Some(name == "person_id"),
                ..Default::default()
            },
        );
    }
    config.account.account_linking.enabled = Some(true);
    config.account.account_linking.disable_implicit_linking = true;
    config.account.account_linking.allow_different_emails = false;
    config.account.account_linking.allow_unlinking_all = Some(false);
    let store = better_auth_seaorm::SeaOrmStore::<AccountSchema>::new(
        config.clone(),
        policy.database().clone(),
    )
    .with_plugin_schema::<PluginSchema>()
    .hook(super::native_hooks::DomainHooks {
        settings: policy.settings().cloned(),
    });
    let verification = || {
        EmailVerificationPlugin::new()
            .auto_sign_in_after_verification(true)
            .require_verification_for_signin(true)
            .send_on_sign_up(true)
    };
    let keys = ApiKeyPlugin::with_config(ApiKeyConfig {
        config_id: super::personal_keys::CONFIG.into(),
        key_length: 64,
        prefix: Some("medtracker_".into()),
        enable_metadata: true,
        store_starting_characters: false,
        ..Default::default()
    });
    let mut auth = BetterAuth::<AccountSchema>::new(config)
        .store(store)
        .plugin(super::request::ClinicalRequest(policy.clone()))
        .plugin(super::onboarding::Onboarding(policy.clone()))
        .plugin(super::credentials::Credentials(policy.clone()))
        .plugin(super::personal_keys::PersonalKeys(policy.clone()))
        .plugin(
            EmailPasswordPlugin::new()
                .enable_signup(true)
                .password_min_length(15)
                .password_max_length(1024)
                .auto_sign_in(false)
                .require_email_verification(true)
                .with_email_verification(Arc::new(verification())),
        )
        .plugin(
            verification().callbacks(super::native_mail::verification(policy.database().clone())),
        )
        .plugin(
            PasswordManagementPlugin::new()
                .callbacks(super::native_mail::password(policy.database().clone())),
        )
        .plugin(SessionManagementPlugin::new())
        .plugin(TwoFactorPlugin::new())
        .plugin(
            PasskeyPlugin::new()
                .rp_id(
                    origin
                        .host_str()
                        .ok_or_else(|| AuthError::internal("Identity hostname unavailable"))?,
                )
                .rp_name("MedTracker")
                .origin(origin.origin().ascii_serialization())
                .authenticator_selection(better_auth::plugins::passkey::AuthenticatorSelection {
                    resident_key: Some(better_auth::plugins::passkey::PasskeyRequirement::Required),
                    user_verification: Some(
                        better_auth::plugins::passkey::PasskeyRequirement::Required,
                    ),
                    ..Default::default()
                })
                .registration(better_auth::plugins::passkey::PasskeyRegistrationOptions {
                    after_verification: Some(Arc::new(
                        super::passkey_policy::RequiredUserVerification,
                    )),
                    ..Default::default()
                })
                .authentication(
                    better_auth::plugins::passkey::PasskeyAuthenticationOptions {
                        after_verification: Some(Arc::new(
                            super::passkey_policy::RequiredUserVerification,
                        )),
                        ..Default::default()
                    },
                ),
        )
        .plugin(AccountManagementPlugin::new())
        .plugin(UserManagementPlugin::new().change_email_enabled(true))
        .plugin(keys);
    if let Some(oauth) = oauth {
        if !oauth.has_provider("zitadel").await? {
            return Err(AuthError::config(
                "Configured ZITADEL provider is unavailable",
            ));
        }
        auth = auth.plugin(oauth);
    }
    auth.build().await
}
