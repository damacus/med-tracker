use std::sync::Arc;
use async_trait::async_trait;
use better_auth::plugins::{PasswordManagementPlugin, TwoFactorPlugin};
use better_auth_core::{AuthContext, AuthError, AuthPlugin, AuthRequest, AuthResponse, AuthResult, AuthRoute, CreateVerification, HttpMethod, UpdateAccount, utils::password::hash_password};
use sea_orm::ConnectionTrait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use super::{ClinicalAuthSchema, ClinicalStore, store::{clinical_id, database_error, statement}};

pub(super) struct Totp(pub Arc<ClinicalStore>);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Start { password: String, code: Option<String> }

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Verify { code: String, operation_id: Option<String> }

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Disable { operation_id: String, password: String, code: String }

#[derive(Serialize, Deserialize)]
struct Pending { account_id: String, session_id: String, factor: FactorBinding }

#[derive(Serialize, Deserialize, PartialEq)]
enum FactorBinding { Framework(String), Retained(String) }

fn delegated(request: &AuthRequest, path: &str, body: serde_json::Value) -> AuthResult<AuthRequest> {
    let mut delegated = AuthRequest::new(HttpMethod::Post, path);
    delegated.headers = request.headers.clone();
    delegated.body = Some(serde_json::to_vec(&body)?);
    Ok(delegated)
}

async fn plugin(request: &AuthRequest, ctx: &AuthContext<ClinicalAuthSchema>) -> AuthResult<AuthResponse> {
    TwoFactorPlugin::new().on_request(request, ctx).await?.ok_or_else(|| AuthError::internal("Authenticator operation unavailable"))
}

fn rejected_login(error: &AuthError) -> bool {
    matches!(error, AuthError::AuthenticationFailed(message) if message == "Invalid code")
        || matches!(error, AuthError::BadRequest(message) if message == "Too many attempts. Please request a new code.")
}

pub(super) async fn verify_operation_factor(request: &AuthRequest, ctx: &AuthContext<ClinicalAuthSchema>, code: &str) -> AuthResult<Option<AuthResponse>> {
    let (user, _) = ctx.require_authoritative_session(request).await?;
    let factor = ctx.database.get_two_factor_by_user_id(&user.id).await?.filter(|factor| factor.verified).ok_or(AuthError::InvalidCredentials)?;
    if factor.locked_until.is_some_and(|until| until > chrono::Utc::now()) {
        super::request::persist_totp_failure(429)?;
        return Ok(Some(AuthError::Upstream { status: 429, code: "ACCOUNT_TEMPORARILY_LOCKED", message: "Too many failed verification attempts. Your account is temporarily locked. Please try again later." }.to_auth_response()));
    }
    if factor.locked_until.is_some() { ctx.database.reset_two_factor_failures(&factor.id, Some(chrono::Utc::now())).await?; }
    match plugin(&delegated(request, "/two-factor/verify-totp", json!({"code":code,"trustDevice":false}))?, ctx).await {
        Err(error @ AuthError::AuthenticationFailed(_)) if rejected_login(&error) => {
            let policy = better_auth::plugins::two_factor::AccountLockout::default();
            ctx.database.record_two_factor_failure(&factor.id, policy.max_failed_attempts, chrono::Utc::now() + chrono::Duration::seconds(policy.duration_seconds)).await?;
            super::request::persist_totp_failure(error.status_code())?;
            Ok(Some(error.to_auth_response()))
        }
        Ok(response) if response.status < 400 => {
            ctx.database.reset_two_factor_failures(&factor.id, None).await?;
            Ok(None)
        }
        Ok(response) => Ok(Some(response)),
        Err(error) => Err(error),
    }
}

impl Totp {
    async fn binding(&self, ctx: &AuthContext<ClinicalAuthSchema>, user_id: &str) -> AuthResult<FactorBinding> {
        if let Some(factor) = ctx.database.get_two_factor_by_user_id(user_id).await?.filter(|factor| factor.verified) { return Ok(FactorBinding::Framework(factor.secret)); }
        super::legacy_totp::fingerprint(&self.0, user_id).await?.map(FactorBinding::Retained).ok_or(AuthError::InvalidCredentials)
    }

    async fn handle(&self, request: &AuthRequest, ctx: &AuthContext<ClinicalAuthSchema>) -> AuthResult<AuthResponse> {
        if request.path == "/security/totp/login" {
            let input: Verify = request.body_as_json().map_err(|_| AuthError::bad_request("Invalid authenticator operation"))?;
            if input.operation_id.is_some() { return Err(AuthError::bad_request("Invalid authenticator operation")); }
            if let Some(response) = super::legacy_totp::verify(&self.0, request, ctx, &input.code).await? { return Ok(response); }
            if ctx.require_authoritative_session(request).await.is_ok() { return Err(AuthError::forbidden("A pending password login is required")); }
            let verification = delegated(request, "/two-factor/verify-totp", json!({"code":input.code,"trustDevice":false}))?;
            return match plugin(&verification, ctx).await {
                Err(error) => {
                    if rejected_login(&error) { super::request::persist_totp_failure(error.status_code())?; }
                    let invalid_cookie = matches!(error, AuthError::AuthenticationFailed(ref message) if message == "Invalid two factor cookie");
                    let mut response = error.to_auth_response();
                    for (name, value) in verification.take_response_headers()? { response.headers.append(name, value); }
                    if invalid_cookie {
                        response.headers.append("Set-Cookie", better_auth_core::utils::cookie_utils::create_clear_cookie(&better_auth_core::utils::cookie_utils::related_cookie_name(&ctx.config, "two_factor"), &ctx.config));
                    }
                    Ok(response)
                }
                result => result,
            };
        }
        let (user, session) = ctx.require_authoritative_session(request).await?;
        if session.additional_fields.get("clinical_session_purpose").and_then(serde_json::Value::as_str) != Some("authenticated") || session.additional_fields.get("authentication_method").and_then(serde_json::Value::as_str) == Some("recovery") { return Err(AuthError::forbidden("Fresh authentication is required")); }
        let transaction = self.0.transaction().await?;
        self.0.lock_security_session(&transaction, &session).await?;
        let user = ctx.database.get_user_by_id(&user.id).await?.ok_or(AuthError::Unauthenticated)?;
        let accounts = ctx.database.get_user_accounts(&user.id).await?;
        let credential = accounts.iter().find(|account| account.provider_id == "credential" && account.password.is_some()).ok_or_else(|| AuthError::forbidden("A password is required for an authenticator app"))?;
        let response = if request.path == "/security/totp/disable/start" {
            let factor = self.binding(ctx, &user.id).await?;
            let operation_id = uuid::Uuid::new_v4().to_string();
            ctx.database.create_verification(CreateVerification { identifier: format!("totp-disable:{operation_id}"), value: serde_json::to_string(&Pending { account_id: user.id.clone(), session_id: session.id, factor })?, expires_at: chrono::Utc::now() + chrono::Duration::minutes(5) }).await?;
            AuthResponse::json(200, &json!({"operation_id":operation_id}))?
        } else if request.path == "/security/totp/disable/confirm" {
            let input: Disable = request.body_as_json().map_err(|_| AuthError::bad_request("Invalid authenticator operation"))?;
            if uuid::Uuid::parse_str(&input.operation_id).is_err() { return Err(AuthError::InvalidCredentials); }
            let identifier = format!("totp-disable:{}", input.operation_id);
            let verification = ctx.database.get_verification_by_identifier(&identifier).await?.ok_or(AuthError::InvalidCredentials)?;
            let pending: Pending = serde_json::from_str(&verification.value)?;
            let factor = self.binding(ctx, &user.id).await?;
            if pending.account_id != user.id || pending.session_id != session.id || pending.factor != factor { return Err(AuthError::InvalidCredentials); }
            let proof = delegated(request, "/verify-password", json!({"password":input.password}))?;
            let password = PasswordManagementPlugin::new().password_hasher(Arc::new(super::passwords::CompatibleHasher)).on_request(&proof, ctx).await?.ok_or_else(|| AuthError::internal("Password verification unavailable"))?;
            if password.status >= 400 { return Ok(password); }
            let retained = matches!(factor, FactorBinding::Retained(_));
            let rejected = if retained { super::legacy_totp::verify_operation(&self.0, request, ctx, &input.code).await? } else { verify_operation_factor(request, ctx, &input.code).await? };
            if let Some(rejected) = rejected {
                transaction.commit().await.map_err(database_error)?;
                return Ok(rejected);
            }
            ctx.database.consume_verification_by_identifier(&identifier).await?.ok_or(AuthError::InvalidCredentials)?;
            let response = if retained {
                transaction.execute_raw(statement("UPDATE public.identity_onboarding SET legacy_totp_disabled_at=clock_timestamp(),legacy_totp_locked_until=NULL WHERE account_id=$1", [clinical_id(&user.id)?.into()])).await.map_err(database_error)?;
                ctx.database.delete_session(&session.token).await?;
                let issued = ctx.session_manager().create_session(&user, session.ip_address, session.user_agent).await?;
                AuthResponse::json(200, &json!({"status":true}))?.with_header("Set-Cookie", better_auth_core::utils::cookie_utils::create_session_cookie(&issued.token, &ctx.config))
            } else { plugin(&delegated(request, "/two-factor/disable", json!({"password":input.password}))?, ctx).await? };
            if response.status >= 400 { return Ok(response); }
            self.0.audit(&*transaction, clinical_id(&user.id)?, "totp", "disabled").await?;
            super::mail::notice(&self.0, &user, "Authenticator app disabled", "Your authenticator app was removed from MedTracker password login. If this was not you, secure your account immediately.").await?;
            response
        } else if request.path == "/security/totp/start" {
            let input: Start = request.body_as_json().map_err(|_| AuthError::bad_request("Invalid authenticator operation"))?;
            let proof = delegated(request, "/verify-password", json!({"password":input.password}))?;
            let verified = PasswordManagementPlugin::new().password_hasher(Arc::new(super::passwords::CompatibleHasher)).on_request(&proof, ctx).await?.ok_or_else(|| AuthError::internal("Password verification unavailable"))?;
            if verified.status >= 400 { return Err(AuthError::InvalidCredentials); }
            if user.two_factor_enabled || super::legacy_totp::enabled(&self.0, &user.id).await? {
                let code = input.code.as_deref().ok_or(AuthError::InvalidCredentials)?;
                let rejected = if user.two_factor_enabled { verify_operation_factor(request, ctx, code).await? } else { super::legacy_totp::verify_operation(&self.0, request, ctx, code).await? };
                if let Some(rejected) = rejected {
                    transaction.commit().await.map_err(database_error)?;
                    return Ok(rejected);
                }
            }
            if credential.password.as_deref().is_some_and(|value| value.starts_with("$2")) {
                ctx.database.update_account(&credential.id, UpdateAccount { password: Some(hash_password(None, &input.password).await?), access_token: None, refresh_token: None, id_token: None, access_token_expires_at: None, refresh_token_expires_at: None, scope: None }).await?;
            }
            let enabled = plugin(&delegated(request, "/two-factor/enable", json!({"password":input.password}))?, ctx).await?;
            if enabled.status >= 400 { return Ok(enabled); }
            let body: serde_json::Value = serde_json::from_slice(&enabled.body)?;
            let factor = ctx.database.get_two_factor_by_user_id(&user.id).await?.ok_or_else(|| AuthError::internal("Authenticator setup unavailable"))?;
            let operation_id = uuid::Uuid::new_v4().to_string();
            ctx.database.create_verification(CreateVerification { identifier: format!("totp-enrolment:{operation_id}"), value: serde_json::to_string(&Pending { account_id: user.id.clone(), session_id: session.id, factor: FactorBinding::Framework(factor.secret) })?, expires_at: chrono::Utc::now() + chrono::Duration::minutes(5) }).await?;
            AuthResponse::json(200, &json!({"operation_id":operation_id,"totpURI":body.get("totpURI")}))?
        } else {
            let input: Verify = request.body_as_json().map_err(|_| AuthError::bad_request("Invalid authenticator operation"))?;
            let id = input.operation_id.ok_or(AuthError::InvalidCredentials)?;
            if uuid::Uuid::parse_str(&id).is_err() { return Err(AuthError::InvalidCredentials); }
            let verification = ctx.database.consume_verification_by_identifier(&format!("totp-enrolment:{id}")).await?.ok_or(AuthError::InvalidCredentials)?;
            let pending: Pending = serde_json::from_str(&verification.value)?;
            let factor = ctx.database.get_two_factor_by_user_id(&user.id).await?.ok_or(AuthError::InvalidCredentials)?;
            if pending.account_id != user.id || pending.session_id != session.id || pending.factor != FactorBinding::Framework(factor.secret) || factor.verified { return Err(AuthError::InvalidCredentials); }
            let response = plugin(&delegated(request, "/two-factor/verify-totp", json!({"code":input.code,"trustDevice":false}))?, ctx).await?;
            if response.status >= 400 { return Ok(response); }
            self.0.audit(&*transaction, clinical_id(&user.id)?, "totp", "enabled").await?;
            super::mail::notice(&self.0, &user, "Authenticator app enabled", "An authenticator app now protects password login to your MedTracker account.").await?;
            response
        };
        transaction.commit().await.map_err(database_error)?;
        Ok(response)
    }
}

#[async_trait]
impl AuthPlugin<ClinicalAuthSchema> for Totp {
    fn name(&self) -> &'static str { "clinical-totp" }
    fn routes(&self) -> Vec<AuthRoute> { vec![AuthRoute::post("/security/totp/start", "start_totp"), AuthRoute::post("/security/totp/finish", "finish_totp"), AuthRoute::post("/security/totp/login", "login_totp"), AuthRoute::post("/security/totp/disable/start", "start_disable_totp"), AuthRoute::post("/security/totp/disable/confirm", "confirm_disable_totp")] }
    async fn on_request(&self, request: &AuthRequest, ctx: &AuthContext<ClinicalAuthSchema>) -> AuthResult<Option<AuthResponse>> {
        if request.method == HttpMethod::Post && matches!(request.path.as_str(), "/security/totp/start" | "/security/totp/finish" | "/security/totp/login" | "/security/totp/disable/start" | "/security/totp/disable/confirm") { self.handle(request, ctx).await.map(Some) } else { Ok(None) }
    }
}
