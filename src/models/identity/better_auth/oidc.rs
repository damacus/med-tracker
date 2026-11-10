use super::IdentityService;
use better_auth::plugins::{OAuthPlugin, oauth::GenericOAuthConfig};
use better_auth_core::{AuthError, AuthRequest, AuthResponse, AuthResult, HttpMethod};
use serde::Deserialize;
use serde_json::json;

pub(crate) fn provider_name(settings: Option<&serde_json::Value>) -> &str {
    crate::models::settings::text(settings, &["oidc", "provider_name"]).unwrap_or("OpenID Connect")
}

#[derive(Clone)]
pub struct Provider {
    issuer: String,
    client_id: String,
    client_secret: String,
    callback: url::Url,
}

#[derive(Deserialize)]
pub struct Profile {
    pub name: String,
    pub date_of_birth: chrono::NaiveDate,
    pub invitation_token: Option<String>,
}

impl Provider {
    pub(super) fn configured(
        base_url: &str,
        settings: Option<&serde_json::Value>,
    ) -> AuthResult<Option<Self>> {
        let Some(issuer) = crate::models::settings::text(settings, &["oidc", "issuer_url"]) else {
            return Ok(None);
        };
        let parsed =
            url::Url::parse(issuer).map_err(|_| AuthError::config("Invalid OIDC issuer"))?;
        let local = parsed
            .host_str()
            .is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "::1"))
            && base_url.starts_with("http://localhost:");
        if (parsed.scheme() != "https" && !local)
            || parsed.query().is_some()
            || parsed.fragment().is_some()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
        {
            return Err(AuthError::config("Invalid OIDC issuer"));
        }
        let client_id = crate::models::settings::text(settings, &["oidc", "client_id"])
            .ok_or_else(|| AuthError::config("OIDC client ID unavailable"))?;
        let client_secret = crate::models::settings::text(settings, &["oidc", "client_secret"])
            .ok_or_else(|| AuthError::config("OIDC client secret unavailable"))?;
        let origin = url::Url::parse(base_url)
            .map_err(|_| AuthError::internal("Identity URL unavailable"))?;
        let callback = match crate::models::settings::text(settings, &["oidc", "redirect_uri"]) {
            Some(value) => url::Url::parse(value)
                .map_err(|_| AuthError::config("Invalid OIDC redirect URI"))?,
            None => origin
                .join("/auth/oidc/callback")
                .map_err(|_| AuthError::config("Invalid OIDC redirect URI"))?,
        };
        if callback.origin() != origin.origin()
            || (callback.scheme() != "https" && !local)
            || !callback.username().is_empty()
            || callback.password().is_some()
            || callback.query().is_some()
            || callback.fragment().is_some()
        {
            return Err(AuthError::config("Invalid OIDC redirect URI"));
        }
        Ok(Some(Self {
            issuer: issuer.into(),
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            callback,
        }))
    }

    pub(crate) fn callback_path(&self) -> &str {
        self.callback.path()
    }

    pub(super) fn plugin(&self) -> OAuthPlugin {
        OAuthPlugin::new().add_generic_provider(
            "oidc",
            GenericOAuthConfig {
                client_id: self.client_id.clone(),
                client_secret: Some(self.client_secret.clone()),
                discovery_url: Some(format!(
                    "{}/.well-known/openid-configuration",
                    self.issuer.trim_end_matches('/')
                )),
                redirect_uri: Some(self.callback.to_string()),
                require_id_token_verification: true,
                disable_implicit_sign_up: Some(true),
                pkce: true,
                allow_idp_initiated: false,
                disable_id_token_nonce_binding: false,
                scopes: vec!["openid".into(), "email".into(), "profile".into()],
                ..Default::default()
            },
        )
    }

    pub async fn preflight(
        &self,
        service: &IdentityService,
        request: &AuthRequest,
    ) -> AuthResult<Option<AuthResponse>> {
        let mut guarded = request.clone();
        guarded.method = HttpMethod::Post;
        guarded.path = "/security/oidc".into();
        super::limits::check(service, &guarded).await
    }

    pub async fn begin(
        &self,
        service: &IdentityService,
        request: &AuthRequest,
        operation_id: Option<String>,
        link: bool,
    ) -> AuthResult<AuthResponse> {
        if operation_id.is_some() && link {
            return Err(AuthError::InvalidCredentials);
        }
        let (path, callback) = if link {
            service
                .context()
                .require_authoritative_session(request)
                .await?;
            ("/link-social", "/account/security".into())
        } else if let Some(id) = operation_id {
            let (_, session) = service
                .context()
                .require_authoritative_session(request)
                .await?;
            super::operations::pending_password(service.context(), &session, &id).await?;
            (
                "/sign-in/social",
                format!("/account/security/provider/confirm?operation_id={id}"),
            )
        } else {
            ("/sign-in/social", "/auth/passkey/setup".into())
        };
        service.call_endpoint(HttpMethod::Post, path, better_auth::server_api::EndpointInput {
            request: Some(request.clone()), headers: Some(request.headers.clone()),
            body: Some(json!({"provider":"oidc","callbackURL":callback,"newUserCallbackURL":"/auth/passkey/setup","errorCallbackURL":"/auth/oidc/register","requestSignUp":false,"disableRedirect":true})),
            ..Default::default()
        }).await
    }

    pub async fn register(
        &self,
        service: &IdentityService,
        request: &AuthRequest,
        profile: Profile,
    ) -> AuthResult<AuthResponse> {
        let errors = crate::models::identity::signup::profile_errors(
            &profile.name,
            Some(profile.date_of_birth),
            false,
        );
        if !errors.is_empty() {
            return Err(AuthError::validation(
                "Enter your name and an adult date of birth",
            ));
        }
        service.call_endpoint(HttpMethod::Post, "/sign-in/social", better_auth::server_api::EndpointInput {
            request: Some(request.clone()), headers: Some(request.headers.clone()),
            body: Some(json!({"provider":"oidc","callbackURL":"/auth/passkey/setup","newUserCallbackURL":"/auth/passkey/setup","errorCallbackURL":"/auth/oidc/register","requestSignUp":true,"disableRedirect":true,"additionalData":{"profile":{"name":profile.name,"date_of_birth":profile.date_of_birth,"invitation_token":profile.invitation_token}}})),
            ..Default::default()
        }).await
    }

    pub async fn finish(
        &self,
        service: &IdentityService,
        request: &AuthRequest,
        query: serde_json::Value,
    ) -> AuthResult<AuthResponse> {
        service
            .call_endpoint(
                HttpMethod::Get,
                "/callback/oidc",
                better_auth::server_api::EndpointInput {
                    request: Some(request.clone()),
                    headers: Some(request.headers.clone()),
                    query: Some(query),
                    ..Default::default()
                },
            )
            .await
    }

    pub async fn complete_operation(
        &self,
        service: &IdentityService,
        request: &AuthRequest,
        operation_id: &str,
    ) -> AuthResult<AuthResponse> {
        let (user, session) = service
            .context()
            .require_authoritative_session(request)
            .await?;
        if uuid::Uuid::parse_str(operation_id).is_err()
            || session
                .additional_fields
                .get("authentication_method")
                .and_then(better_auth_core::FieldValue::as_str)
                != Some("oidc")
        {
            return Err(AuthError::InvalidCredentials);
        }
        let identifier = format!("password-change:{operation_id}");
        let record = service
            .context()
            .database
            .get_verification_by_identifier(&identifier)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;
        let pending: super::operations::PendingPassword =
            serde_json::from_str(record.value.typed()?)?;
        let created = record
            .created_at
            .typed()?
            .to_datetime()?
            .ok_or(AuthError::InvalidCredentials)?;
        let authenticated = session
            .created_at
            .typed()?
            .to_datetime()?
            .ok_or(AuthError::InvalidCredentials)?;
        if pending.account_id != user.id.display_string()?
            || pending.session_id == session.id.display_string()?
            || authenticated < created
            || record
                .expires_at
                .typed()?
                .to_datetime()?
                .is_none_or(|date| date <= chrono::Utc::now())
        {
            return Err(AuthError::InvalidCredentials);
        }
        super::operations::complete_proven_operation(
            &service.policy,
            request,
            service.context(),
            &user,
            &session,
            operation_id,
            pending,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::Provider;
    use serde_json::json;

    #[test]
    fn generic_oidc_settings_enable_provider() {
        let settings = json!({"oidc": {
            "issuer_url": "https://identity.example.com",
            "client_id": "test-client",
            "client_secret": "password",
            "provider_name": "Example Identity"
        }});
        assert!(
            Provider::configured("https://medtracker.example.com/api/auth", Some(&settings))
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn oidc_callback_and_branding_are_configurable() {
        let mut settings = json!({"oidc": {
            "issuer_url": "https://identity.example.com",
            "client_id": "test-client", "client_secret": "password"
        }});
        let provider =
            Provider::configured("https://medtracker.example.com/api/auth", Some(&settings))
                .unwrap()
                .unwrap();
        assert_eq!(
            provider.callback.as_str(),
            "https://medtracker.example.com/auth/oidc/callback"
        );
        assert_eq!(super::provider_name(Some(&settings)), "OpenID Connect");
        settings["oidc"]["redirect_uri"] = json!("https://medtracker.example.com/auth/oidc/return");
        settings["oidc"]["provider_name"] = json!("Example <Identity>");
        let provider =
            Provider::configured("https://medtracker.example.com/api/auth", Some(&settings))
                .unwrap()
                .unwrap();
        assert_eq!(provider.callback_path(), "/auth/oidc/return");
        assert_eq!(super::provider_name(Some(&settings)), "Example <Identity>");
    }

    #[test]
    fn oidc_rejects_missing_credentials_and_foreign_or_insecure_callbacks() {
        let mut settings = json!({"oidc": {"issuer_url": "https://identity.example.com"}});
        assert!(
            Provider::configured("https://medtracker.example.com/api/auth", Some(&settings))
                .is_err()
        );
        settings["oidc"]["client_id"] = json!("test-client");
        assert!(
            Provider::configured("https://medtracker.example.com/api/auth", Some(&settings))
                .is_err()
        );
        settings["oidc"]["client_secret"] = json!("password");
        for uri in [
            "https://other.example.com/auth/oidc/callback",
            "http://medtracker.example.com/auth/oidc/callback",
            "https://medtracker.example.com/auth/oidc/callback#fragment",
        ] {
            settings["oidc"]["redirect_uri"] = json!(uri);
            assert!(
                Provider::configured("https://medtracker.example.com/api/auth", Some(&settings))
                    .is_err()
            );
        }
        assert!(
            Provider::configured("https://medtracker.example.com/api/auth", None)
                .unwrap()
                .is_none()
        );
    }
}
