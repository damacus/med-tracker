use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use better_auth_core::{AuthContext, AuthError, AuthPlugin, AuthRequest, AuthResponse, AuthResult, AuthRoute, CreatePasskey, CreateVerification, HttpMethod, utils::cookie_utils::{create_session_like_cookie, get_cookie, sign_cookie_value, verify_cookie_value}};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use webauthn_rs::prelude::{DiscoverableAuthentication, DiscoverableKey, Passkey, PasskeyAuthentication, PasskeyRegistration, PublicKeyCredential, RegisterPublicKeyCredential, Webauthn, WebauthnBuilder};

use super::ClinicalAuthSchema;

const CHALLENGE_COOKIE: &str = "medtracker_passkey_challenge";

pub(super) struct Passkeys(Webauthn, std::sync::Arc<super::ClinicalStore>);

#[derive(Serialize, Deserialize)]
struct Registration {
    account_id: String,
    session_token: String,
    user_handle: uuid::Uuid,
    state: PasskeyRegistration,
    operation_id: Option<String>,
}

#[derive(Deserialize)]
struct RegistrationResponse {
    response: RegisterPublicKeyCredential,
    name: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct OperationProof { account_id: String, session_id: String, operation_id: String, state: PasskeyAuthentication }

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProofStart { operation_id: String }

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProofFinish { challenge_id: String, operation_id: String, response: PublicKeyCredential }

impl Passkeys {
    async fn proof_start(&self, request: &AuthRequest, ctx: &AuthContext<ClinicalAuthSchema>) -> AuthResult<AuthResponse> {
        let input: ProofStart = request.body_as_json().map_err(|_| AuthError::bad_request("Invalid passkey proof"))?;
        let (user, session) = ctx.require_authoritative_session(request).await?;
        if session.additional_fields.get("authentication_method").and_then(serde_json::Value::as_str) == Some("recovery") { return Err(AuthError::forbidden("Fresh email confirmation is required")); }
        let transaction = self.1.transaction().await?;
        self.1.lock_security_session(&transaction, &session).await?;
        super::operations::pending_password(ctx, &session, &input.operation_id).await?;
        let stored = ctx.database.list_passkeys_by_user(&user.id).await?;
        let credentials = stored.iter().map(|key| serde_json::from_str::<Passkey>(&key.credential)).collect::<Result<Vec<_>, _>>()?;
        if credentials.is_empty() { return Err(AuthError::InvalidCredentials); }
        let (options, state) = self.0.start_passkey_authentication(&credentials).map_err(|_| AuthError::InvalidCredentials)?;
        let challenge_id = uuid::Uuid::new_v4().to_string();
        ctx.database.create_verification(CreateVerification { identifier: format!("security-passkey:{challenge_id}"), value: serde_json::to_string(&OperationProof { account_id: user.id, session_id: session.id, operation_id: input.operation_id, state })?, expires_at: Utc::now() + Duration::minutes(5) }).await?;
        transaction.commit().await.map_err(super::store::database_error)?;
        Ok(AuthResponse::json(200, &json!({"challenge_id":challenge_id,"publicKey":serde_json::to_value(options)?["publicKey"]}))?)
    }

    async fn proof_finish(&self, request: &AuthRequest, ctx: &AuthContext<ClinicalAuthSchema>) -> AuthResult<AuthResponse> {
        let input: ProofFinish = request.body_as_json().map_err(|_| AuthError::bad_request("Invalid passkey proof"))?;
        let (user, session) = ctx.require_authoritative_session(request).await?;
        if session.additional_fields.get("authentication_method").and_then(serde_json::Value::as_str) == Some("recovery") { return Err(AuthError::forbidden("Fresh email confirmation is required")); }
        let transaction = self.1.transaction().await?;
        self.1.lock_security_session(&transaction, &session).await?;
        super::operations::pending_password(ctx, &session, &input.operation_id).await?;
        let proof = ctx.database.consume_verification_by_identifier(&format!("security-passkey:{}", input.challenge_id)).await?.ok_or(AuthError::InvalidCredentials)?;
        let proof: OperationProof = serde_json::from_str(&proof.value)?;
        if proof.account_id != user.id || proof.session_id != session.id || proof.operation_id != input.operation_id { return Err(AuthError::InvalidCredentials); }
        let result = self.0.finish_passkey_authentication(&input.response, &proof.state).map_err(|_| AuthError::InvalidCredentials)?;
        if !result.user_verified() { return Err(AuthError::InvalidCredentials); }
        let stored = ctx.database.get_passkey_by_credential_id(&URL_SAFE_NO_PAD.encode(result.cred_id())).await?.filter(|key| key.user_id == user.id).ok_or(AuthError::InvalidCredentials)?;
        let mut key: Passkey = serde_json::from_str(&stored.credential)?;
        key.update_credential(&result).ok_or(AuthError::InvalidCredentials)?;
        ctx.database.update_passkey_authentication(&stored.id, better_auth_core::types::UpdatePasskeyAuthentication { credential: serde_json::to_string(&key)?, counter: u64::from(result.counter()), backed_up: result.backup_state(), device_type: stored.device_type }).await?;
        let response = super::operations::complete_password(&self.1, ctx, &user, &session, &input.operation_id).await?;
        transaction.commit().await.map_err(super::store::database_error)?;
        Ok(response)
    }
    async fn login_options(&self, ctx: &AuthContext<ClinicalAuthSchema>) -> AuthResult<AuthResponse> {
        let (options, state) = self.0.start_discoverable_authentication().map_err(|_| AuthError::internal("Passkey authentication unavailable"))?;
        let challenge = uuid::Uuid::new_v4().to_string();
        ctx.database.create_verification(CreateVerification { identifier: format!("authentication:{challenge}"), value: serde_json::to_string(&state)?, expires_at: Utc::now() + Duration::minutes(5) }).await?;
        let cookie = create_session_like_cookie(CHALLENGE_COOKIE, &sign_cookie_value(&challenge, &ctx.config.secret), Some(300), &ctx.config);
        Ok(AuthResponse::json(200, &options)?.with_header("Set-Cookie", cookie))
    }

    async fn login(&self, request: &AuthRequest, ctx: &AuthContext<ClinicalAuthSchema>) -> AuthResult<AuthResponse> {
        let challenge = get_cookie(request, CHALLENGE_COOKIE).and_then(|value| verify_cookie_value(&value, &ctx.config.secret)).ok_or(AuthError::Unauthenticated)?;
        let verification = ctx.database.consume_verification_by_identifier(&format!("authentication:{challenge}")).await?.ok_or(AuthError::Unauthenticated)?;
        let state: DiscoverableAuthentication = serde_json::from_str(&verification.value)?;
        let response: PublicKeyCredential = request.body_as_json()?;
        let (handle, credential_id) = self.0.identify_discoverable_authentication(&response).map_err(|_| AuthError::InvalidCredentials)?;
        let stored = ctx.database.get_passkey_by_credential_id(&URL_SAFE_NO_PAD.encode(credential_id)).await?.ok_or(AuthError::InvalidCredentials)?;
        let user = ctx.database.get_user_by_id(&stored.user_id).await?.ok_or(AuthError::InvalidCredentials)?;
        if !user.email_verified || user.metadata.get("passkey_user_handle").and_then(serde_json::Value::as_str) != Some(handle.to_string().as_str()) { return Err(AuthError::InvalidCredentials); }
        let mut passkey: Passkey = serde_json::from_str(&stored.credential)?;
        let result = self.0.finish_discoverable_authentication(&response, state, &[DiscoverableKey::from(&passkey)]).map_err(|_| AuthError::InvalidCredentials)?;
        passkey.update_credential(&result).ok_or(AuthError::InvalidCredentials)?;
        ctx.database.update_passkey_authentication(&stored.id, better_auth_core::types::UpdatePasskeyAuthentication { credential: serde_json::to_string(&passkey)?, counter: u64::from(result.counter()), backed_up: result.backup_state(), device_type: stored.device_type }).await?;
        let session = ctx.session_manager().create_session(&user, None, request.headers.get("user-agent").cloned()).await?;
        Ok(AuthResponse::json(200, &json!({"status":true}))?.with_header("Set-Cookie", better_auth_core::utils::cookie_utils::create_session_cookie(&session.token, &ctx.config)))
    }

    pub(super) fn new(origin: &url::Url, store: std::sync::Arc<super::ClinicalStore>) -> AuthResult<Self> {
        let rp_id = origin.host_str().ok_or_else(|| AuthError::internal("Invalid passkey host"))?;
        let webauthn = WebauthnBuilder::new(rp_id, origin)
            .map_err(|_| AuthError::internal("Invalid passkey origin"))?
            .rp_name("MedTracker")
            .build().map_err(|_| AuthError::internal("Invalid passkey configuration"))?;
        Ok(Self(webauthn, store))
    }

    async fn begin(&self, request: &AuthRequest, ctx: &AuthContext<ClinicalAuthSchema>) -> AuthResult<AuthResponse> {
        let (user, session) = ctx.require_authoritative_session(request).await?;
        if super::legacy_totp::pending(&session) { return Err(AuthError::Unauthenticated); }
        let operation_id = request.query.get("operation_id").cloned();
        let transaction = self.1.transaction().await?;
        if let Some(operation_id) = &operation_id {
            self.1.lock_security_session(&transaction, &session).await?;
            super::credentials::registration_grant(ctx, &session, operation_id).await?;
        } else if session.additional_fields.get("clinical_session_purpose").and_then(serde_json::Value::as_str) != Some("enrolment") { return Err(AuthError::forbidden("Fresh authentication is required")); }
        let passkeys = ctx.database.list_passkeys_by_user(&user.id).await?;
        if operation_id.is_none() && !passkeys.is_empty() { return Err(AuthError::forbidden("A passkey is already enrolled")); }
        let excluded = passkeys.iter().map(|key| serde_json::from_str::<Passkey>(&key.credential).map(|key| key.cred_id().clone())).collect::<Result<Vec<_>, _>>()?;
        let handle = user.metadata.get("passkey_user_handle").and_then(serde_json::Value::as_str).ok_or_else(|| AuthError::internal("Passkey account handle unavailable"))?;
        let user_handle = uuid::Uuid::parse_str(handle).map_err(|_| AuthError::internal("Invalid passkey account handle"))?;
        let (options, state) = self.0.start_passkey_registration(
            user_handle, user.email.as_deref().unwrap_or_default(),
            user.name.as_deref().unwrap_or("MedTracker account"), Some(excluded),
        ).map_err(|_| AuthError::internal("Passkey registration unavailable"))?;
        let challenge = uuid::Uuid::new_v4().to_string();
        ctx.database.create_verification(CreateVerification {
            identifier: format!("registration:{challenge}"),
            value: serde_json::to_string(&Registration { account_id: user.id, session_token: session.token, user_handle, state, operation_id })?,
            expires_at: Utc::now() + Duration::minutes(5),
        }).await?;
        let cookie = create_session_like_cookie(CHALLENGE_COOKIE, &sign_cookie_value(&challenge, &ctx.config.secret), Some(300), &ctx.config);
        let mut options = serde_json::to_value(options)?;
        options["publicKey"]["authenticatorSelection"]["residentKey"] = json!("required");
        options["publicKey"]["authenticatorSelection"]["requireResidentKey"] = json!(true);
        transaction.commit().await.map_err(super::store::database_error)?;
        Ok(AuthResponse::json(200, &options["publicKey"])?.with_header("Set-Cookie", cookie))
    }

    async fn finish(&self, request: &AuthRequest, ctx: &AuthContext<ClinicalAuthSchema>) -> AuthResult<AuthResponse> {
        let (user, session) = ctx.require_authoritative_session(request).await?;
        use sea_orm::ConnectionTrait;
        let transaction = self.1.transaction().await?;
        super::store::context(&*transaction, "med_tracker.current_account_id", &user.id).await?;
        transaction.query_one_raw(super::store::statement("SELECT id FROM public.accounts WHERE id=$1 AND status=2 FOR UPDATE", [super::store::clinical_id(&user.id)?.into()])).await.map_err(super::store::database_error)?.ok_or(AuthError::Unauthenticated)?;
        let challenge = get_cookie(request, CHALLENGE_COOKIE)
            .and_then(|value| verify_cookie_value(&value, &ctx.config.secret)).ok_or(AuthError::Unauthenticated)?;
        let verification = ctx.database.consume_verification_by_identifier(&format!("registration:{challenge}")).await?.ok_or(AuthError::Unauthenticated)?;
        let stored: Registration = serde_json::from_str(&verification.value)?;
        if stored.account_id != user.id || stored.session_token != session.token { return Err(AuthError::Unauthenticated); }
        if let Some(operation_id) = &stored.operation_id {
            self.1.lock_security_session(&transaction, &session).await?;
            super::credentials::registration_grant(ctx, &session, operation_id).await?;
            ctx.database.consume_verification_by_identifier(&format!("passkey-add:{operation_id}")).await?.ok_or(AuthError::InvalidCredentials)?;
        } else {
            let allowed = transaction.query_one_raw(super::store::statement("SELECT id FROM public.identity_sessions WHERE token=$1 AND account_id=$2 AND purpose='enrolment' AND active AND expires_at>clock_timestamp() AND NOT EXISTS(SELECT 1 FROM public.identity_passkeys WHERE account_id=$2)", [session.token.clone().into(), super::store::clinical_id(&user.id)?.into()])).await.map_err(super::store::database_error)?;
            if allowed.is_none() || super::legacy_totp::pending(&session) { return Err(AuthError::forbidden("Passkey enrolment is unavailable")); }
        }
        let response: RegistrationResponse = serde_json::from_slice(request.body.as_deref().unwrap_or_default())?;
        let passkey = self.0.finish_passkey_registration(&response.response, &stored.state)
            .map_err(|_| AuthError::InvalidCredentials)?;
        let credential_id = URL_SAFE_NO_PAD.encode(passkey.cred_id());
        let stored = ctx.database.create_passkey(CreatePasskey {
            user_id: user.id.clone(), name: response.name, credential_id,
            public_key: String::new(), counter: 0, device_type: "passkey".into(),
            backed_up: false, transports: None, credential: serde_json::to_string(&passkey)?, aaguid: None,
        }).await?;
        self.1.audit(&transaction, super::store::clinical_id(&user.id)?, "passkey", "created").await?;
        super::mail::notice(&self.1, &user, "Passkey added", "A passkey was added to your MedTracker account.").await?;
        transaction.commit().await.map_err(super::store::database_error)?;
        Ok(AuthResponse::json(200, &json!({"id":stored.id}))?)
    }
}

#[async_trait]
impl AuthPlugin<ClinicalAuthSchema> for Passkeys {
    fn name(&self) -> &'static str { "clinical-passkeys" }

    fn routes(&self) -> Vec<AuthRoute> {
        vec![AuthRoute::get("/passkey/generate-register-options", "passkey_registration_options"), AuthRoute::post("/passkey/verify-registration", "passkey_registration"), AuthRoute::get("/passkey/generate-authenticate-options", "passkey_authentication_options"), AuthRoute::post("/passkey/verify-authentication", "passkey_authentication"), AuthRoute::post("/security/passkey/start", "start_passkey_proof"), AuthRoute::post("/security/passkey/confirm", "confirm_passkey_proof")]
    }

    async fn on_request(&self, request: &AuthRequest, ctx: &AuthContext<ClinicalAuthSchema>) -> AuthResult<Option<AuthResponse>> {
        match (request.method(), request.path()) {
            (HttpMethod::Post, "/security/passkey/start") => self.proof_start(request, ctx).await.map(Some),
            (HttpMethod::Post, "/security/passkey/confirm") => self.proof_finish(request, ctx).await.map(Some),
            (HttpMethod::Get, "/passkey/generate-authenticate-options") => self.login_options(ctx).await.map(Some),
            (HttpMethod::Post, "/passkey/verify-authentication") => self.login(request, ctx).await.map(Some),
            (HttpMethod::Get, "/passkey/generate-register-options") => self.begin(request, ctx).await.map(Some),
            (HttpMethod::Post, "/passkey/verify-registration") => self.finish(request, ctx).await.map(Some),
            _ => Ok(None),
        }
    }
}
