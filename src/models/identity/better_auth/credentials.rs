use super::{
    ClinicalAuthSchema, IdentityPolicy,
    operations::PendingPassword,
    policy::{clinical_id, database_error},
};
use async_trait::async_trait;
use better_auth_core::{
    AuthContext, AuthError, AuthPlugin, AuthRequest, AuthResponse, AuthResult, AuthRoute,
    CreateVerification, HttpMethod,
    wire::{SessionView, UserView},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::models::errors::OperationError;

#[derive(Serialize, Deserialize, PartialEq)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Action {
    CloseAccount,
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

pub(super) struct Credentials(pub Arc<IdentityPolicy>);

pub(super) async fn validate_in(
    store: &IdentityPolicy,
    transaction: &sea_orm::DatabaseTransaction,
    user: &UserView,
    action: &Action,
) -> AuthResult<()> {
    match action {
        Action::PlatformAdministrator { account_id, grant } => {
            crate::models::platform::check_rights(
                transaction,
                clinical_id(&user.id.display_string()?)?,
                *account_id,
                *grant,
            )
            .await
            .map_err(platform_error)
        }
        Action::PlatformUser { account_id, active } => crate::models::platform::check_user(
            transaction,
            clinical_id(&user.id.display_string()?)?,
            *account_id,
            *active,
        )
        .await
        .map_err(platform_error),
        Action::PlatformSettings {
            invite_only,
            medicine_lookup_base_url,
            medicine_lookup_token_url,
            medicine_lookup_source_priority,
        } => crate::models::platform::check_settings(
            transaction,
            clinical_id(&user.id.display_string()?)?,
            *invite_only,
            medicine_lookup_base_url,
            medicine_lookup_token_url,
            medicine_lookup_source_priority,
            store.settings(),
        )
        .await
        .map_err(platform_error),
        Action::PlatformOwnerRecovery {
            household_id,
            membership_id,
            reason,
        } => crate::models::platform::recovery::check(
            transaction,
            clinical_id(&user.id.display_string()?)?,
            *household_id,
            *membership_id,
            reason,
        )
        .await
        .map_err(platform_error),
        Action::SupportRequest {
            household_id,
            reason,
        } => crate::models::platform::support::check_request(
            transaction,
            clinical_id(&user.id.display_string()?)?,
            *household_id,
            reason,
        )
        .await
        .map_err(platform_error),
        Action::SupportApprove { support_id } => crate::models::platform::support::check_approve(
            transaction,
            clinical_id(&user.id.display_string()?)?,
            *support_id,
        )
        .await
        .map_err(platform_error),
        Action::SupportActivate { support_id } => crate::models::platform::support::check_activate(
            transaction,
            clinical_id(&user.id.display_string()?)?,
            *support_id,
        )
        .await
        .map_err(platform_error),
        _ => Ok(()),
    }
}

fn request_meta(
    session: &SessionView,
    request: &AuthRequest,
) -> AuthResult<crate::models::platform::RequestMeta> {
    let (session_reference, request_id, ip) = platform_audit(session, request)?;
    Ok(crate::models::platform::RequestMeta {
        session_reference,
        request_id,
        ip,
    })
}

pub(super) fn platform_error(error: OperationError) -> AuthError {
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
) -> AuthResult<(Option<String>, Option<String>, Option<String>)> {
    Ok((
        Some(crate::models::identity::store::digest(
            session.token.typed()?,
        )),
        super::request::request_id(),
        request
            .headers
            .get("x-forwarded-for")
            .map(std::string::ToString::to_string),
    ))
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
        let transaction = self.0.transaction().await?;
        self.0.lock_security_session(&transaction, &session).await?;
        validate_in(&self.0, &transaction, &user, &action).await?;
        let accounts = ctx
            .database
            .get_user_accounts(&user.id.display_string()?)
            .await?;
        let credential = accounts
            .iter()
            .find(|account| account.provider_id.as_str() == Some("credential"));
        let operation_id = uuid::Uuid::new_v4().to_string();
        let pending = PendingPassword {
            account_id: user.id.display_string()?.clone(),
            session_id: session.id.display_string()?.clone(),
            credential_id: credential
                .map(|account| account.id.display_string())
                .transpose()?,
            action,
        };
        ctx.database
            .create_verification(CreateVerification {
                identifier: format!("password-change:{operation_id}").into(),
                value: serde_json::to_string(&pending)?.into(),
                expires_at: (chrono::Utc::now() + chrono::Duration::minutes(5)).into(),
                ..Default::default()
            })
            .await?;
        let response = AuthResponse::json(
            200,
            &json!({"operation_id":operation_id,"password_proof":credential.is_some_and(|account| account.password.typed().is_ok_and(Option::is_some)),"passkey_proof":!ctx.database.list_passkeys_by_user(&user.id.display_string()?).await?.is_empty(),"oidc_proof":accounts.iter().any(|account| account.provider_id.as_str()==Some("zitadel"))}),
        )?;
        transaction.commit().await.map_err(database_error)?;
        Ok(response)
    }
}

pub(super) async fn complete_in(
    store: &IdentityPolicy,
    transaction: &sea_orm::DatabaseTransaction,
    request: &AuthRequest,
    user: &UserView,
    session: &SessionView,
    pending: PendingPassword,
) -> AuthResult<AuthResponse> {
    validate_in(store, transaction, user, &pending.action).await?;
    if let Action::PlatformAdministrator { account_id, grant } = &pending.action {
        let (session_reference, request_id, ip) = platform_audit(session, request)?;
        crate::models::platform::change_rights(
            transaction,
            clinical_id(&user.id.display_string()?)?,
            *account_id,
            *grant,
            session_reference.as_deref(),
            request_id.as_deref(),
            ip.as_deref(),
        )
        .await
        .map_err(platform_error)?;
        return AuthResponse::json(200, &json!({"status":true,"redirect":"/platform/users"}));
    }
    if let Action::PlatformUser { account_id, active } = &pending.action {
        let (session_reference, request_id, ip) = platform_audit(session, request)?;
        crate::models::platform::change_user(
            transaction,
            clinical_id(&user.id.display_string()?)?,
            *account_id,
            *active,
            session_reference.as_deref(),
            request_id.as_deref(),
            ip.as_deref(),
        )
        .await
        .map_err(platform_error)?;
        return AuthResponse::json(200, &json!({"status":true,"redirect":"/platform/users"}));
    }
    if let Action::PlatformSettings {
        invite_only,
        medicine_lookup_base_url,
        medicine_lookup_token_url,
        medicine_lookup_source_priority,
    } = &pending.action
    {
        let meta = request_meta(session, request)?;
        crate::models::platform::change_settings(
            transaction,
            clinical_id(&user.id.display_string()?)?,
            *invite_only,
            medicine_lookup_base_url,
            medicine_lookup_token_url,
            medicine_lookup_source_priority,
            &meta,
            store.settings(),
        )
        .await
        .map_err(platform_error)?;
        return AuthResponse::json(200, &json!({"status":true,"redirect":"/platform/settings"}));
    }
    if let Action::PlatformOwnerRecovery {
        household_id,
        membership_id,
        reason,
    } = &pending.action
    {
        crate::models::platform::recovery::promote(
            transaction,
            clinical_id(&user.id.display_string()?)?,
            *household_id,
            *membership_id,
            reason,
            &request_meta(session, request)?,
        )
        .await
        .map_err(platform_error)?;
        return AuthResponse::json(
            200,
            &json!({"status":true,"redirect":"/platform/owner-recovery"}),
        );
    }
    if let Action::SupportRequest {
        household_id,
        reason,
    } = &pending.action
    {
        crate::models::platform::support::request(
            transaction,
            clinical_id(&user.id.display_string()?)?,
            *household_id,
            reason,
            &request_meta(session, request)?,
        )
        .await
        .map_err(platform_error)?;
        return AuthResponse::json(200, &json!({"status":true,"redirect":"/platform/support"}));
    }
    if let Action::SupportApprove { support_id } = &pending.action {
        let result = crate::models::platform::support::approve(
            transaction,
            clinical_id(&user.id.display_string()?)?,
            *support_id,
            &request_meta(session, request)?,
        )
        .await;
        match result {
            Ok(crate::models::platform::support::SupportDecision::Applied) => {
                return AuthResponse::json(
                    200,
                    &json!({"status":true,"redirect":"/account/support"}),
                );
            }
            Ok(crate::models::platform::support::SupportDecision::Expired) => {
                return AuthResponse::json(
                    403,
                    &json!({"code":"FORBIDDEN","message":"The requested change is not permitted."}),
                );
            }
            Err(error) => return Err(platform_error(error)),
        }
    }
    if let Action::SupportActivate { support_id } = &pending.action {
        let result = crate::models::platform::support::activate(
            transaction,
            clinical_id(&user.id.display_string()?)?,
            *support_id,
            &request_meta(session, request)?,
        )
        .await;
        match result {
            Ok(crate::models::platform::support::SupportDecision::Applied) => {
                return AuthResponse::json(
                    200,
                    &json!({"status":true,"redirect":"/platform/support"}),
                );
            }
            Ok(crate::models::platform::support::SupportDecision::Expired) => {
                return AuthResponse::json(
                    403,
                    &json!({"code":"FORBIDDEN","message":"The requested change is not permitted."}),
                );
            }
            Err(error) => return Err(platform_error(error)),
        }
    }
    if pending.action == Action::CloseAccount {
        return Err(AuthError::internal(
            "Account closure must own its operation transaction",
        ));
    }
    Err(AuthError::InvalidCredentials)
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
