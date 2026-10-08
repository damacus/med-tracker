use super::{
    IdentityService,
    store::{clinical_id, context, database_error, statement},
};
use better_auth_core::{
    AuthError, AuthRequest, AuthResponse, AuthResult, CreateAccount, CreateVerification,
    UpdateUser, store::VerificationStore,
};
use chrono::Utc;
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
    core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata},
};
use sea_orm::ConnectionTrait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

#[derive(Clone)]
pub struct Provider {
    issuer: String,
    client_id: String,
    client_secret: String,
    callback: String,
    http: openidconnect::reqwest::Client,
}
#[derive(Serialize, Deserialize)]
struct Flow {
    csrf: String,
    nonce: String,
    verifier: String,
    operation_id: Option<String>,
    #[serde(default)]
    link_id: Option<String>,
    account_id: Option<String>,
    session_id: Option<String>,
}
#[derive(Serialize, Deserialize)]
pub(super) struct LinkGrant {
    pub account_id: String,
    pub session_id: String,
}
#[derive(Serialize, Deserialize)]
struct Claims {
    issuer: String,
    subject: String,
    email: String,
    name: String,
}
#[derive(Serialize, Deserialize)]
struct Registration {
    csrf: String,
    claims: Claims,
}

#[derive(Deserialize)]
pub struct Profile {
    pub name: String,
    pub date_of_birth: chrono::NaiveDate,
    pub credential: String,
    pub password: Option<String>,
    pub invitation_token: Option<String>,
}

fn denied() -> AuthError {
    AuthError::InvalidCredentials
}
fn identifier(state: &str) -> String {
    format!(
        "oidc-state:{}",
        hex::encode(Sha256::digest(state.as_bytes()))
    )
}

impl Provider {
    pub async fn link_available(
        &self,
        service: &IdentityService,
        request: &AuthRequest,
        id: &str,
    ) -> AuthResult<()> {
        if uuid::Uuid::parse_str(id).is_err() {
            return Err(denied());
        }
        let (user, session) = service
            .context()
            .require_authoritative_session(request)
            .await?;
        let record = service
            .store
            .get_verification_by_identifier(&format!("provider-link:{id}"))
            .await?
            .ok_or_else(denied)?;
        if record.expires_at <= Utc::now() {
            return Err(denied());
        }
        let grant: LinkGrant = serde_json::from_str(&record.value)?;
        if grant.account_id != user.id
            || grant.session_id != session.id
            || session
                .additional_fields
                .get("authentication_method")
                .and_then(serde_json::Value::as_str)
                == Some("recovery")
        {
            return Err(denied());
        }
        Ok(())
    }

    pub async fn preflight(
        &self,
        service: &IdentityService,
        request: &AuthRequest,
    ) -> AuthResult<Option<AuthResponse>> {
        let mut guarded = AuthRequest::new(better_auth_core::HttpMethod::Post, "/security/oidc");
        guarded.headers = request.headers.clone();
        super::limits::check(service, &guarded).await
    }

    pub async fn registration(
        &self,
        service: &IdentityService,
        id: &str,
        verify_browser: impl FnOnce(&str) -> bool,
    ) -> AuthResult<serde_json::Value> {
        if uuid::Uuid::parse_str(id).is_err() {
            return Err(denied());
        }
        let record = service
            .store
            .get_verification_by_identifier(&format!("oidc-registration:{id}"))
            .await?
            .ok_or_else(denied)?;
        if record.expires_at <= Utc::now() {
            return Err(denied());
        }
        let pending: Registration = serde_json::from_str(&record.value)?;
        if !verify_browser(&pending.csrf) {
            return Err(denied());
        }
        Ok(json!({"email":pending.claims.email,"name":pending.claims.name}))
    }

    pub async fn register(
        &self,
        service: &IdentityService,
        request: &AuthRequest,
        request_id: String,
        id: &str,
        profile: Profile,
        verify_browser: impl FnOnce(&str) -> bool,
    ) -> AuthResult<AuthResponse> {
        let registration = self.registration(service, id, verify_browser).await?;
        let mut guarded =
            AuthRequest::new(better_auth_core::HttpMethod::Post, "/onboarding/signup");
        guarded.headers = request.headers.clone();
        guarded.body = Some(serde_json::to_vec(&registration)?);
        if let Some(response) = super::limits::check(service, &guarded).await? {
            return Ok(response);
        }
        let password = match profile.credential.as_str() {
            "passkey" if profile.password.as_deref().is_none_or(str::is_empty) => None,
            "password" => {
                let password = profile
                    .password
                    .as_deref()
                    .ok_or_else(|| AuthError::validation("Password is required"))?;
                super::validate_new_password(password).map_err(AuthError::validation)?;
                Some(better_auth_core::utils::password::hash_password(None, password).await?)
            }
            _ => return Err(AuthError::validation("Choose password or passkey")),
        };
        super::request::trusted(service, request, request_id, async {
            let record = service.store.consume_verification_by_identifier(&format!("oidc-registration:{id}")).await?.ok_or_else(denied)?;
            let pending: Registration = serde_json::from_str(&record.value)?;
            let ctx = service.context();
            let identity = serde_json::to_string(&(&pending.claims.issuer, &pending.claims.subject))?;
            if service.store.provider_identity("zitadel", &identity).await?.is_some() { return Err(denied()); }
            let user = ctx.database.create_user(better_auth_core::CreateUser { name: Some(profile.name), email: Some(pending.claims.email.clone()), metadata: Some(json!({"date_of_birth":profile.date_of_birth,"invitation_token":profile.invitation_token})), ..Default::default() }).await?;
            let user = ctx.database.update_user(&user.id, UpdateUser { email_verified: Some(true), ..Default::default() }).await?;
            if let Some(password) = password {
                ctx.database.create_account(CreateAccount { user_id: user.id.clone(), account_id: user.id.clone(), provider_id: "credential".into(), password: Some(password), access_token: None, refresh_token: None, id_token: None, access_token_expires_at: None, refresh_token_expires_at: None, scope: None }).await?;
            }
            self.apply(service, request, &Flow { csrf: pending.csrf, nonce: String::new(), verifier: String::new(), operation_id: None, link_id: None, account_id: None, session_id: None }, pending.claims, true).await
        }).await
    }

    pub(super) fn configured(base_url: &str) -> AuthResult<Option<Self>> {
        let Ok(issuer) = std::env::var("MEDTRACKER_ZITADEL_ISSUER") else {
            return Ok(None);
        };
        let parsed =
            url::Url::parse(&issuer).map_err(|_| AuthError::internal("Invalid ZITADEL issuer"))?;
        let local = parsed
            .host_str()
            .is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "::1"))
            && base_url.starts_with("http://localhost:");
        if parsed.scheme() != "https" && !local {
            return Err(AuthError::internal("ZITADEL requires HTTPS"));
        }
        if parsed.query().is_some()
            || parsed.fragment().is_some()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
        {
            return Err(AuthError::internal("Invalid ZITADEL issuer"));
        }
        let client_id = std::env::var("MEDTRACKER_ZITADEL_CLIENT_ID")
            .map_err(|_| AuthError::internal("ZITADEL client unavailable"))?;
        let client_secret = std::env::var("MEDTRACKER_ZITADEL_CLIENT_SECRET")
            .map_err(|_| AuthError::internal("ZITADEL client unavailable"))?;
        let mut callback = url::Url::parse(base_url)
            .map_err(|_| AuthError::internal("Identity URL unavailable"))?;
        callback.set_path("/auth/zitadel/callback");
        let http = openidconnect::reqwest::Client::builder()
            .redirect(openidconnect::reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|_| AuthError::internal("ZITADEL transport unavailable"))?;
        Ok(Some(Self {
            issuer,
            client_id,
            client_secret,
            callback: callback.to_string(),
            http,
        }))
    }

    async fn metadata(&self) -> AuthResult<CoreProviderMetadata> {
        CoreProviderMetadata::discover_async(
            IssuerUrl::new(self.issuer.clone()).map_err(|_| denied())?,
            &self.http,
        )
        .await
        .map_err(|_| AuthError::internal("ZITADEL is unavailable"))
    }

    pub async fn begin(
        &self,
        service: &IdentityService,
        request: &AuthRequest,
        csrf: String,
        operation_id: Option<String>,
        link_id: Option<String>,
    ) -> AuthResult<String> {
        if operation_id.is_some() && link_id.is_some() {
            return Err(denied());
        }
        let (account_id, session_id) = if operation_id.is_some() || link_id.is_some() {
            let (user, session) = service
                .context()
                .require_authoritative_session(request)
                .await?;
            if session
                .additional_fields
                .get("authentication_method")
                .and_then(serde_json::Value::as_str)
                == Some("recovery")
            {
                return Err(denied());
            }
            if let Some(operation_id) = operation_id.as_deref() {
                super::operations::pending_password(service.context(), &session, operation_id)
                    .await?;
            }
            if let Some(link_id) = link_id.as_deref() {
                self.link_available(service, request, link_id).await?;
            }
            (Some(user.id), Some(session.id))
        } else {
            (None, None)
        };
        let client = CoreClient::from_provider_metadata(
            self.metadata().await?,
            ClientId::new(self.client_id.clone()),
            Some(ClientSecret::new(self.client_secret.clone())),
        )
        .set_redirect_uri(RedirectUrl::new(self.callback.clone()).map_err(|_| denied())?);
        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        let mut authorization = client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scope(Scope::new("email".into()))
            .add_scope(Scope::new("profile".into()))
            .set_pkce_challenge(challenge);
        if operation_id.is_some() || link_id.is_some() {
            authorization = authorization.set_max_age(std::time::Duration::ZERO);
        }
        let (url, state, nonce) = authorization.url();
        service
            .store
            .create_verification(CreateVerification {
                identifier: identifier(state.secret()),
                value: serde_json::to_string(&Flow {
                    csrf,
                    nonce: nonce.secret().clone(),
                    verifier: verifier.secret().clone(),
                    operation_id,
                    link_id,
                    account_id,
                    session_id,
                })?,
                expires_at: Utc::now() + chrono::Duration::minutes(5),
            })
            .await?;
        Ok(url.to_string())
    }

    pub async fn finish(
        &self,
        service: &IdentityService,
        request: &AuthRequest,
        request_id: String,
        code: &str,
        state: &str,
        verify_browser: impl FnOnce(&str) -> bool,
    ) -> AuthResult<AuthResponse> {
        if state.len() > 512 || code.len() > 8192 {
            return Err(denied());
        }
        let record = service
            .store
            .get_verification_by_identifier(&identifier(state))
            .await?
            .ok_or_else(denied)?;
        let flow: Flow = serde_json::from_str(&record.value)?;
        if !verify_browser(&flow.csrf) {
            return Err(denied());
        }
        service
            .store
            .consume_verification(&identifier(state), &record.value)
            .await?
            .ok_or_else(denied)?;
        let client = CoreClient::from_provider_metadata(
            self.metadata().await?,
            ClientId::new(self.client_id.clone()),
            Some(ClientSecret::new(self.client_secret.clone())),
        )
        .set_redirect_uri(RedirectUrl::new(self.callback.clone()).map_err(|_| denied())?);
        let response = client
            .exchange_code(AuthorizationCode::new(code.into()))
            .map_err(|_| denied())?
            .set_pkce_verifier(PkceCodeVerifier::new(flow.verifier.clone()))
            .request_async(&self.http)
            .await
            .map_err(|_| denied())?;
        let token = response.id_token().ok_or_else(denied)?;
        let fresh = flow.operation_id.is_some() || flow.link_id.is_some();
        let verifier = client
            .id_token_verifier()
            .set_auth_time_verifier_fn(move |time| {
                if !fresh
                    || time.is_some_and(|time| {
                        time <= Utc::now() && time > Utc::now() - chrono::Duration::minutes(5)
                    })
                {
                    Ok(())
                } else {
                    Err("Fresh provider authentication is required".into())
                }
            });
        let nonce = Nonce::new(flow.nonce.clone());
        let verified = token.claims(&verifier, &nonce).map_err(|_| denied())?;
        if verified.email_verified() != Some(true) {
            return Err(denied());
        }
        let email = verified
            .email()
            .ok_or_else(denied)?
            .as_str()
            .trim()
            .to_lowercase();
        crate::models::care::invitations::validated_email(&email).map_err(|_| denied())?;
        let claims = Claims {
            issuer: self.issuer.clone(),
            subject: verified.subject().as_str().into(),
            email,
            name: verified
                .name()
                .and_then(|name| name.get(None))
                .map(|name| name.as_str().to_owned())
                .unwrap_or_default(),
        };
        super::request::trusted(
            service,
            request,
            request_id,
            self.apply(service, request, &flow, claims, false),
        )
        .await
    }

    async fn apply(
        &self,
        service: &IdentityService,
        request: &AuthRequest,
        flow: &Flow,
        claims: Claims,
        require_enrolment: bool,
    ) -> AuthResult<AuthResponse> {
        let ctx = service.context();
        let identity = serde_json::to_string(&(&claims.issuer, &claims.subject))?;
        let linked = service
            .store
            .provider_identity("zitadel", &identity)
            .await?;
        if let Some(link_id) = flow.link_id.as_deref() {
            let (user, session) = ctx.require_authoritative_session(request).await?;
            let transaction = service.store.transaction().await?;
            service
                .store
                .lock_security_session(&transaction, &session)
                .await?;
            self.link_available(service, request, link_id).await?;
            let user = ctx
                .database
                .get_user_by_id(&user.id)
                .await?
                .ok_or_else(denied)?;
            let linked = service
                .store
                .provider_identity("zitadel", &identity)
                .await?;
            if flow.account_id.as_deref() != Some(&user.id)
                || flow.session_id.as_deref() != Some(&session.id)
                || user.email.as_deref() != Some(&claims.email)
                || !user.email_verified
                || linked
                    .as_ref()
                    .is_some_and(|account| account.user_id != user.id)
                || ctx
                    .database
                    .get_user_accounts(&user.id)
                    .await?
                    .iter()
                    .any(|account| account.provider_id == "zitadel")
            {
                return Err(denied());
            }
            ctx.database
                .consume_verification_by_identifier(&format!("provider-link:{link_id}"))
                .await?
                .ok_or_else(denied)?;
            if let Some(linked) = linked {
                service.store.set_provider_enabled(&linked.id, true).await?;
            } else {
                ctx.database
                    .create_account(CreateAccount {
                        user_id: user.id.clone(),
                        account_id: identity,
                        provider_id: "zitadel".into(),
                        password: None,
                        access_token: None,
                        refresh_token: None,
                        id_token: None,
                        access_token_expires_at: None,
                        refresh_token_expires_at: None,
                        scope: None,
                    })
                    .await?;
            }
            service
                .store
                .audit(&transaction, clinical_id(&user.id)?, "provider", "linked")
                .await?;
            super::mail::notice(&service.store, &user, "ZITADEL linked", "ZITADEL was linked to your MedTracker account. Your local sign-in methods remain available.").await?;
            transaction.commit().await.map_err(database_error)?;
            return Ok(AuthResponse::json(
                200,
                &json!({"status":true,"redirect":"/account/security"}),
            )?);
        }
        if let Some(linked) = linked.as_ref()
            && !service.store.provider_enabled(&linked.id).await?
        {
            return Err(denied());
        }
        if let Some(operation_id) = flow.operation_id.as_deref() {
            let (user, session) = ctx.require_authoritative_session(request).await?;
            let transaction = service.store.transaction().await?;
            service
                .store
                .lock_security_session(&transaction, &session)
                .await?;
            let linked = service
                .store
                .provider_identity("zitadel", &identity)
                .await?;
            if flow.account_id.as_deref() != Some(&user.id)
                || flow.session_id.as_deref() != Some(&session.id)
                || linked
                    .as_ref()
                    .is_none_or(|account| account.user_id != user.id)
            {
                return Err(denied());
            }
            if !service
                .store
                .provider_enabled(&linked.as_ref().ok_or_else(denied)?.id)
                .await?
            {
                return Err(denied());
            }
            let response = super::operations::complete_password(
                &service.store,
                request,
                ctx,
                &user,
                &session,
                operation_id,
            )
            .await?;
            transaction.commit().await.map_err(database_error)?;
            return Ok(response);
        }
        let user = if let Some(account) = linked.as_ref() {
            Some(
                ctx.database
                    .get_user_by_id(&account.user_id)
                    .await?
                    .ok_or_else(denied)?,
            )
        } else {
            ctx.database.get_user_by_email(&claims.email).await?
        };
        let Some(mut user) = user else {
            if require_enrolment {
                return Err(denied());
            }
            let id = uuid::Uuid::new_v4().to_string();
            service
                .store
                .create_verification(CreateVerification {
                    identifier: format!("oidc-registration:{id}"),
                    value: serde_json::to_string(&Registration {
                        csrf: flow.csrf.clone(),
                        claims,
                    })?,
                    expires_at: Utc::now() + chrono::Duration::minutes(5),
                })
                .await?;
            return Ok(AuthResponse::json(
                200,
                &json!({"status":true,"redirect":format!("/auth/zitadel/register?registration_id={id}")}),
            )?);
        };
        let transaction = service.store.transaction().await?;
        context(&transaction, "med_tracker.current_account_id", &user.id).await?;
        transaction
            .query_one_raw(statement(
                "SELECT id FROM public.accounts WHERE id=$1 AND status=2 FOR UPDATE",
                [clinical_id(&user.id)?.into()],
            ))
            .await
            .map_err(database_error)?
            .ok_or_else(denied)?;
        user = ctx
            .database
            .get_user_by_id(&user.id)
            .await?
            .ok_or_else(denied)?;
        if !user.email_verified || user.banned {
            return Err(denied());
        }
        let linked = service
            .store
            .provider_identity("zitadel", &identity)
            .await?;
        if linked
            .as_ref()
            .is_some_and(|account| account.user_id != user.id)
            || linked.is_none() && user.email.as_deref() != Some(&claims.email)
        {
            return Err(denied());
        }
        if let Some(linked) = linked.as_ref()
            && !service.store.provider_enabled(&linked.id).await?
        {
            return Err(denied());
        }
        if user.email.as_deref() != Some(&claims.email) {
            if ctx
                .database
                .get_user_by_email(&claims.email)
                .await?
                .is_some()
            {
                return Err(denied());
            }
            super::request::allow_email_change(&user.id, &claims.email)?;
            super::mail::notice(
                &service.store,
                &user,
                "Email address changed",
                "Your verified ZITADEL email address changed and was synchronised with MedTracker.",
            )
            .await?;
            user = ctx
                .database
                .update_user(
                    &user.id,
                    UpdateUser {
                        email: Some(claims.email.clone()),
                        email_verified: Some(true),
                        ..Default::default()
                    },
                )
                .await?;
            service
                .store
                .audit(
                    &transaction,
                    clinical_id(&user.id)?,
                    "email",
                    "provider_updated",
                )
                .await?;
        }
        if linked.is_none() {
            ctx.database
                .create_account(CreateAccount {
                    user_id: user.id.clone(),
                    account_id: identity,
                    provider_id: "zitadel".into(),
                    password: None,
                    access_token: None,
                    refresh_token: None,
                    id_token: None,
                    access_token_expires_at: None,
                    refresh_token_expires_at: None,
                    scope: None,
                })
                .await?;
            service
                .store
                .audit(&transaction, clinical_id(&user.id)?, "provider", "linked")
                .await?;
            super::mail::notice(&service.store, &user, "ZITADEL linked", "ZITADEL was linked to your MedTracker account. Your local sign-in methods remain available.").await?;
        }
        let session = ctx
            .session_manager()
            .create_session(
                &user,
                request.headers.get("x-forwarded-for").cloned(),
                request.headers.get("user-agent").cloned(),
            )
            .await?;
        if require_enrolment
            && session
                .additional_fields
                .get("clinical_session_purpose")
                .and_then(serde_json::Value::as_str)
                != Some("enrolment")
        {
            return Err(AuthError::forbidden(
                "Local credential and recovery-code acknowledgement are required",
            ));
        }
        transaction.execute_raw(statement("UPDATE public.identity_sessions SET additional_fields=additional_fields || jsonb_build_object('authentication_method','zitadel') WHERE id=$1", [session.id.clone().into()])).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        Ok(AuthResponse::json(200, &json!({"status":true,"redirect":if session.additional_fields.get("clinical_session_purpose").and_then(serde_json::Value::as_str)==Some("authenticated") {"/"}else{"/auth/passkey/setup"}}))?.with_header("Set-Cookie", better_auth_core::utils::cookie_utils::create_session_cookie(&session.token, &ctx.config)))
    }
}
