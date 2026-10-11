use super::{
    ClinicalAuthSchema, ClinicalStore,
    operations::PendingPassword,
    store::{clinical_id, database_error, statement},
};
use async_trait::async_trait;
use better_auth_core::{
    AuthContext, AuthError, AuthPlugin, AuthRequest, AuthResponse, AuthResult, AuthRoute,
    CreateVerification, HttpMethod,
    wire::{SessionView, UserView},
};
use sea_orm::ConnectionTrait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::models::errors::OperationError;

#[derive(Serialize, Deserialize, PartialEq)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Action {
    Password,
    RemovePassword,
    RemovePasskey {
        target: String,
    },
    AddPasskey,
    RegenerateRecovery,
    ChangeEmail {
        new_email: String,
    },
    CloseAccount,
    LinkProvider,
    UnlinkProvider,
    DisableTotp,
    CreateApiKey {
        key: super::personal_keys::Creation,
    },
    PlatformAdministrator {
        account_id: i64,
        grant: bool,
    },
    PlatformUser {
        account_id: i64,
        active: bool,
    },
    PlatformSettings {
        invite_only: bool,
        medicine_lookup_base_url: String,
        medicine_lookup_token_url: String,
        medicine_lookup_source_priority: Vec<String>,
    },
    PlatformOwnerRecovery {
        household_id: i64,
        membership_id: i64,
        reason: String,
    },
    SupportRequest {
        household_id: i64,
        reason: String,
    },
    SupportApprove {
        support_id: i64,
    },
    SupportActivate {
        support_id: i64,
    },
}

#[derive(Serialize, Deserialize)]
pub(super) struct RegistrationGrant {
    pub account_id: String,
    pub session_id: String,
    pub recovery: bool,
}

pub(super) struct Credentials(pub Arc<ClinicalStore>);

async fn validate(
    store: &ClinicalStore,
    ctx: &AuthContext<ClinicalAuthSchema>,
    user: &UserView,
    action: &Action,
) -> AuthResult<()> {
    let factor_enabled = ctx
        .database
        .get_two_factor_by_user_id(&user.id)
        .await?
        .is_some_and(|factor| factor.verified);
    let accounts = ctx.database.get_user_accounts(&user.id).await?;
    let password = accounts
        .iter()
        .any(|account| account.provider_id == "credential" && account.password.is_some());
    let keys = ctx.database.list_passkeys_by_user(&user.id).await?;
    match action {
        Action::DisableTotp
            if !factor_enabled && !super::legacy_totp::enabled(store, &user.id).await? =>
        {
            Err(AuthError::InvalidCredentials)
        }
        Action::LinkProvider
            if accounts
                .iter()
                .any(|account| account.provider_id == "zitadel") =>
        {
            Err(AuthError::bad_request("ZITADEL is already linked"))
        }
        Action::UnlinkProvider
            if !accounts
                .iter()
                .any(|account| account.provider_id == "zitadel") =>
        {
            Err(AuthError::bad_request("ZITADEL is not linked"))
        }
        Action::UnlinkProvider if !password && keys.is_empty() => {
            Err(AuthError::bad_request("Keep a local password or passkey"))
        }
        Action::CreateApiKey { key } => super::personal_keys::validate(store, user, key).await,
        Action::ChangeEmail { new_email }
            if new_email.len() > 254
                || !new_email.contains('@')
                || new_email.trim() != new_email
                || user.email.as_deref() == Some(new_email) =>
        {
            Err(AuthError::bad_request(
                "Enter a different valid email address",
            ))
        }
        Action::RemovePassword if !password || keys.is_empty() => {
            Err(AuthError::bad_request("Keep a local password or passkey"))
        }
        Action::RemovePassword
            if factor_enabled || super::legacy_totp::enabled(store, &user.id).await? =>
        {
            Err(AuthError::bad_request(
                "Disable your authenticator app before removing your password",
            ))
        }
        Action::RemovePasskey { target } if !keys.iter().any(|key| key.id == *target) => {
            Err(AuthError::InvalidCredentials)
        }
        Action::RemovePasskey { .. } if !password && keys.len() < 2 => {
            Err(AuthError::bad_request("Keep a local password or passkey"))
        }
        Action::Password => Err(AuthError::bad_request("Use the password change operation")),
        Action::PlatformAdministrator { account_id, grant } => {
            let transaction = store.transaction().await?;
            crate::models::platform::check_rights(
                &transaction,
                clinical_id(&user.id)?,
                *account_id,
                *grant,
            )
            .await
            .map_err(platform_error)?;
            transaction.commit().await.map_err(database_error)
        }
        Action::PlatformUser { account_id, active } => {
            let transaction = store.transaction().await?;
            crate::models::platform::check_user(
                &transaction,
                clinical_id(&user.id)?,
                *account_id,
                *active,
            )
            .await
            .map_err(platform_error)?;
            transaction.commit().await.map_err(database_error)
        }
        Action::PlatformSettings {
            invite_only,
            medicine_lookup_base_url,
            medicine_lookup_token_url,
            medicine_lookup_source_priority,
        } => {
            let transaction = store.transaction().await?;
            crate::models::platform::check_settings(
                &transaction,
                clinical_id(&user.id)?,
                *invite_only,
                medicine_lookup_base_url,
                medicine_lookup_token_url,
                medicine_lookup_source_priority,
            )
            .await
            .map_err(platform_error)?;
            transaction.commit().await.map_err(database_error)
        }
        Action::PlatformOwnerRecovery {
            household_id,
            membership_id,
            reason,
        } => {
            let transaction = store.transaction().await?;
            crate::models::platform::recovery::check(
                &transaction,
                clinical_id(&user.id)?,
                *household_id,
                *membership_id,
                reason,
            )
            .await
            .map_err(platform_error)?;
            transaction.commit().await.map_err(database_error)
        }
        Action::SupportRequest {
            household_id,
            reason,
        } => {
            let transaction = store.transaction().await?;
            crate::models::platform::support::check_request(
                &transaction,
                clinical_id(&user.id)?,
                *household_id,
                reason,
            )
            .await
            .map_err(platform_error)?;
            transaction.commit().await.map_err(database_error)
        }
        Action::SupportApprove { support_id } => {
            let transaction = store.transaction().await?;
            crate::models::platform::support::check_approve(
                &transaction,
                clinical_id(&user.id)?,
                *support_id,
            )
            .await
            .map_err(platform_error)?;
            transaction.commit().await.map_err(database_error)
        }
        Action::SupportActivate { support_id } => {
            let transaction = store.transaction().await?;
            crate::models::platform::support::check_activate(
                &transaction,
                clinical_id(&user.id)?,
                *support_id,
            )
            .await
            .map_err(platform_error)?;
            transaction.commit().await.map_err(database_error)
        }
        _ => Ok(()),
    }
}

fn request_meta(
    session: &SessionView,
    request: &AuthRequest,
) -> crate::models::platform::RequestMeta {
    let (session_reference, request_id, ip) = platform_audit(session, request);
    crate::models::platform::RequestMeta {
        session_reference,
        request_id,
        ip,
    }
}

fn platform_error(error: OperationError) -> AuthError {
    match error {
        OperationError::Unauthenticated => AuthError::Unauthenticated,
        OperationError::Forbidden => {
            AuthError::forbidden("Platform administrator rights are required")
        }
        OperationError::NotFound => AuthError::bad_request("Platform target is unavailable"),
        OperationError::Validation { .. } => {
            AuthError::bad_request("The platform target is not eligible for this change")
        }
        OperationError::Conflict { .. } => {
            AuthError::forbidden("At least one usable platform administrator must remain")
        }
        OperationError::Unavailable => AuthError::internal("Platform persistence unavailable"),
    }
}

fn platform_audit(
    session: &SessionView,
    request: &AuthRequest,
) -> (Option<String>, Option<String>, Option<String>) {
    (
        Some(crate::models::identity::store::digest(&session.token)),
        super::request::request_id(),
        request
            .headers
            .get("x-forwarded-for")
            .map(std::string::ToString::to_string),
    )
}

impl Credentials {
    async fn start(
        &self,
        request: &AuthRequest,
        ctx: &AuthContext<ClinicalAuthSchema>,
    ) -> AuthResult<AuthResponse> {
        let action: Action = request
            .body_as_json()
            .map_err(|_| AuthError::bad_request("Invalid account operation"))?;
        let (user, session) = ctx.require_authoritative_session(request).await?;
        let recovery = session
            .additional_fields
            .get("authentication_method")
            .and_then(serde_json::Value::as_str)
            == Some("recovery");
        if recovery && action != Action::AddPasskey {
            return Err(AuthError::forbidden("Fresh email confirmation is required"));
        }
        let transaction = self.0.transaction().await?;
        self.0.lock_security_session(&transaction, &session).await?;
        validate(&self.0, ctx, &user, &action).await?;
        let accounts = ctx.database.get_user_accounts(&user.id).await?;
        let credential = accounts
            .iter()
            .find(|account| account.provider_id == "credential");
        let operation_id = uuid::Uuid::new_v4().to_string();
        let factor = if action == Action::DisableTotp {
            Some(super::totp::factor_binding(&self.0, ctx, &user.id).await?)
        } else {
            None
        };
        let pending = PendingPassword {
            account_id: user.id.clone(),
            session_id: session.id.clone(),
            credential_id: credential.map(|account| account.id.clone()),
            password_hash: String::new(),
            action,
            factor,
        };
        ctx.database
            .create_verification(CreateVerification {
                identifier: format!("password-change:{operation_id}"),
                value: serde_json::to_string(&pending)?,
                expires_at: chrono::Utc::now()
                    + chrono::Duration::minutes(if recovery { 30 } else { 5 }),
            })
            .await?;
        if recovery {
            super::recovery_email::send(&self.0, ctx, &user, &session, &operation_id).await?;
        }
        let response = AuthResponse::json(
            200,
            &json!({"operation_id":operation_id,"email_proof":recovery,"password_proof":!recovery && credential.is_some_and(|account| account.password.is_some()),"passkey_proof":!recovery && !ctx.database.list_passkeys_by_user(&user.id).await?.is_empty(),"oidc_proof":!recovery && accounts.iter().any(|account| account.provider_id=="zitadel")}),
        )?;
        transaction.commit().await.map_err(database_error)?;
        Ok(response)
    }
}

pub(super) async fn complete(
    store: &ClinicalStore,
    request: &AuthRequest,
    ctx: &AuthContext<ClinicalAuthSchema>,
    user: &UserView,
    session: &SessionView,
    operation_id: &str,
    pending: PendingPassword,
) -> AuthResult<AuthResponse> {
    validate(store, ctx, user, &pending.action).await?;
    if pending.action == Action::DisableTotp {
        return super::totp::disable_with_proof(
            store,
            ctx,
            user,
            session,
            pending.factor.ok_or(AuthError::InvalidCredentials)?,
        )
        .await;
    }
    if pending.action == Action::LinkProvider {
        ctx.database
            .create_verification(CreateVerification {
                identifier: format!("provider-link:{operation_id}"),
                value: serde_json::to_string(&super::oidc::LinkGrant {
                    account_id: user.id.clone(),
                    session_id: session.id.clone(),
                })?,
                expires_at: chrono::Utc::now() + chrono::Duration::minutes(5),
            })
            .await?;
        return Ok(AuthResponse::json(
            200,
            &json!({"status":true,"redirect":format!("/account/security/provider/link?link_id={operation_id}")}),
        )?);
    }
    if let Action::CreateApiKey { key } = &pending.action {
        return super::personal_keys::create(store, ctx, user, key).await;
    }
    if let Action::PlatformAdministrator { account_id, grant } = &pending.action {
        let transaction = store.transaction().await?;
        let (session_reference, request_id, ip) = platform_audit(session, request);
        crate::models::platform::change_rights(
            &transaction,
            clinical_id(&user.id)?,
            *account_id,
            *grant,
            session_reference.as_deref(),
            request_id.as_deref(),
            ip.as_deref(),
        )
        .await
        .map_err(platform_error)?;
        transaction.commit().await.map_err(database_error)?;
        return Ok(AuthResponse::json(
            200,
            &json!({"status":true,"redirect":"/platform/users"}),
        )?);
    }
    if let Action::PlatformUser { account_id, active } = &pending.action {
        let transaction = store.transaction().await?;
        let (session_reference, request_id, ip) = platform_audit(session, request);
        crate::models::platform::change_user(
            &transaction,
            clinical_id(&user.id)?,
            *account_id,
            *active,
            session_reference.as_deref(),
            request_id.as_deref(),
            ip.as_deref(),
        )
        .await
        .map_err(platform_error)?;
        transaction.commit().await.map_err(database_error)?;
        return Ok(AuthResponse::json(
            200,
            &json!({"status":true,"redirect":"/platform/users"}),
        )?);
    }
    if let Action::PlatformSettings {
        invite_only,
        medicine_lookup_base_url,
        medicine_lookup_token_url,
        medicine_lookup_source_priority,
    } = &pending.action
    {
        let transaction = store.transaction().await?;
        let meta = request_meta(session, request);
        crate::models::platform::change_settings(
            &transaction,
            clinical_id(&user.id)?,
            *invite_only,
            medicine_lookup_base_url,
            medicine_lookup_token_url,
            medicine_lookup_source_priority,
            &meta,
        )
        .await
        .map_err(platform_error)?;
        transaction.commit().await.map_err(database_error)?;
        return Ok(AuthResponse::json(
            200,
            &json!({"status":true,"redirect":"/platform/settings"}),
        )?);
    }
    if let Action::PlatformOwnerRecovery {
        household_id,
        membership_id,
        reason,
    } = &pending.action
    {
        let transaction = store.transaction().await?;
        crate::models::platform::recovery::promote(
            &transaction,
            clinical_id(&user.id)?,
            *household_id,
            *membership_id,
            reason,
            &request_meta(session, request),
        )
        .await
        .map_err(platform_error)?;
        transaction.commit().await.map_err(database_error)?;
        return Ok(AuthResponse::json(
            200,
            &json!({"status":true,"redirect":"/platform/owner-recovery"}),
        )?);
    }
    if let Action::SupportRequest {
        household_id,
        reason,
    } = &pending.action
    {
        let transaction = store.transaction().await?;
        crate::models::platform::support::request(
            &transaction,
            clinical_id(&user.id)?,
            *household_id,
            reason,
            &request_meta(session, request),
        )
        .await
        .map_err(platform_error)?;
        transaction.commit().await.map_err(database_error)?;
        return Ok(AuthResponse::json(
            200,
            &json!({"status":true,"redirect":"/platform/support"}),
        )?);
    }
    if let Action::SupportApprove { support_id } = &pending.action {
        let transaction = store.transaction().await?;
        let result = crate::models::platform::support::approve(
            &transaction,
            clinical_id(&user.id)?,
            *support_id,
            &request_meta(session, request),
        )
        .await;
        match result {
            Ok(crate::models::platform::support::SupportDecision::Applied) => {
                transaction.commit().await.map_err(database_error)?;
                return Ok(AuthResponse::json(
                    200,
                    &json!({"status":true,"redirect":"/account/support"}),
                )?);
            }
            Ok(crate::models::platform::support::SupportDecision::Expired) => {
                transaction.commit().await.map_err(database_error)?;
                super::request::persist_support_expiry()?;
                return Ok(AuthResponse::json(
                    403,
                    &json!({"code":"FORBIDDEN","message":"The requested change is not permitted."}),
                )?);
            }
            Err(error) => return Err(platform_error(error)),
        }
    }
    if let Action::SupportActivate { support_id } = &pending.action {
        let transaction = store.transaction().await?;
        let result = crate::models::platform::support::activate(
            &transaction,
            clinical_id(&user.id)?,
            *support_id,
            &request_meta(session, request),
        )
        .await;
        match result {
            Ok(crate::models::platform::support::SupportDecision::Applied) => {
                transaction.commit().await.map_err(database_error)?;
                return Ok(AuthResponse::json(
                    200,
                    &json!({"status":true,"redirect":"/platform/support"}),
                )?);
            }
            Ok(crate::models::platform::support::SupportDecision::Expired) => {
                transaction.commit().await.map_err(database_error)?;
                super::request::persist_support_expiry()?;
                return Ok(AuthResponse::json(
                    403,
                    &json!({"code":"FORBIDDEN","message":"The requested change is not permitted."}),
                )?);
            }
            Err(error) => return Err(platform_error(error)),
        }
    }
    if pending.action == Action::CloseAccount {
        let transaction = store.transaction().await?;
        crate::models::platform::guard_closure(&transaction, clinical_id(&user.id)?)
            .await
            .map_err(platform_error)?;
        transaction.commit().await.map_err(database_error)?;
        store.close_account(&user.id).await?;
        return Ok(
            AuthResponse::json(200, &json!({"status":true,"redirect":"/login"}))?.with_header(
                "Set-Cookie",
                better_auth_core::utils::cookie_utils::create_clear_cookie(
                    &ctx.config.session.cookie_name,
                    &ctx.config,
                ),
            ),
        );
    }
    if let Action::ChangeEmail { new_email } = &pending.action {
        return super::email_change::send(
            store,
            request,
            ctx,
            user,
            session,
            operation_id,
            new_email,
        )
        .await;
    }
    if pending.action == Action::RegenerateRecovery {
        return super::regeneration::issue(store, ctx, user, session).await;
    }
    if pending.action == Action::AddPasskey {
        ctx.database
            .create_verification(CreateVerification {
                identifier: format!("passkey-add:{operation_id}"),
                value: serde_json::to_string(&RegistrationGrant {
                    account_id: user.id.clone(),
                    session_id: session.id.clone(),
                    recovery: session
                        .additional_fields
                        .get("authentication_method")
                        .and_then(serde_json::Value::as_str)
                        == Some("recovery"),
                })?,
                expires_at: chrono::Utc::now() + chrono::Duration::minutes(5),
            })
            .await?;
        return Ok(AuthResponse::json(
            200,
            &json!({"status":true,"redirect":format!("/account/security/passkey?operation_id={operation_id}")}),
        )?);
    }
    let transaction = store.transaction().await?;
    let (kind, message) = match pending.action {
        Action::UnlinkProvider => {
            let accounts = ctx.database.get_user_accounts(&user.id).await?;
            for account in accounts
                .iter()
                .filter(|account| account.provider_id == "zitadel")
            {
                store.set_provider_enabled(&account.id, false).await?;
            }
            (
                "provider",
                "ZITADEL was unlinked from your MedTracker account. Your local sign-in methods remain available.",
            )
        }
        Action::RemovePassword => {
            transaction.execute_raw(statement("UPDATE public.identity_provider_accounts SET password=NULL,updated_at=clock_timestamp() WHERE id=$1 AND account_id=$2 AND provider_id='credential'", [pending.credential_id.ok_or(AuthError::InvalidCredentials)?.into(), clinical_id(&user.id)?.into()])).await.map_err(database_error)?;
            ("password", "Your MedTracker password was removed.")
        }
        Action::RemovePasskey { target } => {
            ctx.database.delete_passkey(&target).await?;
            (
                "passkey",
                "A passkey was removed from your MedTracker account.",
            )
        }
        _ => return Err(AuthError::InvalidCredentials),
    };
    store
        .audit(&transaction, clinical_id(&user.id)?, kind, "removed")
        .await?;
    super::mail::notice(store, user, "Sign-in method removed", message).await?;
    transaction.commit().await.map_err(database_error)?;
    Ok(AuthResponse::json(200, &json!({"status":true}))?)
}

pub(super) async fn registration_grant(
    ctx: &AuthContext<ClinicalAuthSchema>,
    session: &SessionView,
    operation_id: &str,
) -> AuthResult<RegistrationGrant> {
    if uuid::Uuid::parse_str(operation_id).is_err() {
        return Err(AuthError::InvalidCredentials);
    }
    let record = ctx
        .database
        .get_verification_by_identifier(&format!("passkey-add:{operation_id}"))
        .await?
        .ok_or(AuthError::InvalidCredentials)?;
    if record.expires_at <= chrono::Utc::now() {
        return Err(AuthError::InvalidCredentials);
    }
    let grant: RegistrationGrant = serde_json::from_str(&record.value)?;
    if grant.account_id != session.user_id
        || grant.session_id != session.id
        || grant.recovery
            != (session
                .additional_fields
                .get("authentication_method")
                .and_then(serde_json::Value::as_str)
                == Some("recovery"))
    {
        return Err(AuthError::InvalidCredentials);
    }
    Ok(grant)
}

#[async_trait]
impl AuthPlugin<ClinicalAuthSchema> for Credentials {
    fn name(&self) -> &'static str {
        "clinical-credential-lifecycle"
    }
    fn routes(&self) -> Vec<AuthRoute> {
        vec![AuthRoute::post(
            "/security/operation/start",
            "start_account_operation",
        )]
    }
    async fn on_request(
        &self,
        request: &AuthRequest,
        ctx: &AuthContext<ClinicalAuthSchema>,
    ) -> AuthResult<Option<AuthResponse>> {
        if request.method == HttpMethod::Post && request.path == "/security/operation/start" {
            self.start(request, ctx).await.map(Some)
        } else {
            Ok(None)
        }
    }
}
