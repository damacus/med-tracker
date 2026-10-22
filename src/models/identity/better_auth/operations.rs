use better_auth_core::{
    AuthContext, AuthError, AuthRequest, AuthResponse, AuthResult, HttpMethod,
    wire::{SessionView, UserView},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{
    ClinicalAuthSchema, IdentityPolicy,
    policy::{clinical_id, database_error},
};

#[derive(Serialize, Deserialize)]
pub(super) struct PendingPassword {
    pub account_id: String,
    pub session_id: String,
    pub credential_id: Option<String>,
    pub action: super::credentials::Action,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfirmPassword {
    operation_id: String,
    password: String,
    totp_code: Option<String>,
}

pub(super) async fn pending_password(
    ctx: &AuthContext<ClinicalAuthSchema>,
    session: &SessionView,
    operation_id: &str,
) -> AuthResult<PendingPassword> {
    if uuid::Uuid::parse_str(operation_id).is_err() {
        return Err(AuthError::InvalidCredentials);
    }
    let verification = ctx
        .database
        .get_verification_by_identifier(&format!("password-change:{operation_id}"))
        .await?
        .ok_or(AuthError::InvalidCredentials)?;
    if verification
        .expires_at
        .typed()?
        .to_datetime()?
        .ok_or(AuthError::InvalidCredentials)?
        <= chrono::Utc::now()
    {
        return Err(AuthError::InvalidCredentials);
    }
    let pending: PendingPassword = serde_json::from_str(verification.value.typed()?)?;
    let accounts = ctx
        .database
        .get_user_accounts(&session.user_id.display_string()?)
        .await?;
    let credential = accounts
        .iter()
        .find(|account| account.provider_id == "credential");
    if pending.account_id != session.user_id.display_string()?
        || pending.session_id != session.id.display_string()?
        || pending.credential_id
            != credential
                .map(|account| account.id.display_string())
                .transpose()?
    {
        return Err(AuthError::InvalidCredentials);
    }
    Ok(pending)
}

pub(super) async fn complete_password(
    store: &IdentityPolicy,
    request: &AuthRequest,
    ctx: &AuthContext<ClinicalAuthSchema>,
    user: &UserView,
    session: &SessionView,
    operation_id: &str,
) -> AuthResult<AuthResponse> {
    let pending = pending_password(ctx, session, operation_id).await?;
    complete_proven_operation(store, request, ctx, user, session, operation_id, pending).await
}

pub(super) async fn complete_proven_operation(
    store: &IdentityPolicy,
    request: &AuthRequest,
    ctx: &AuthContext<ClinicalAuthSchema>,
    user: &UserView,
    session: &SessionView,
    operation_id: &str,
    pending: PendingPassword,
) -> AuthResult<AuthResponse> {
    let user_id = user.id.display_string()?;
    if pending.account_id != user_id || session.user_id.display_string()? != user_id {
        return Err(AuthError::InvalidCredentials);
    }
    let record = ctx
        .database
        .get_verification_by_identifier(&format!("password-change:{operation_id}"))
        .await?
        .ok_or(AuthError::InvalidCredentials)?;
    if record.value.typed()? != &serde_json::to_string(&pending)?
        || record
            .expires_at
            .typed()?
            .to_datetime()?
            .is_none_or(|expires| expires <= chrono::Utc::now())
    {
        return Err(AuthError::InvalidCredentials);
    }
    let credential = ctx
        .database
        .get_user_accounts(&user_id)
        .await?
        .into_iter()
        .find(|account| account.provider_id == "credential");
    if pending.credential_id
        != credential
            .map(|account| account.id.display_string())
            .transpose()?
    {
        return Err(AuthError::InvalidCredentials);
    }
    if pending.action == super::credentials::Action::CloseAccount {
        use super::schema::verification::{Column, Entity};
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
        let transaction = store.account_transaction().await?;
        store.lock_security_session(&transaction, session).await?;
        crate::models::platform::guard_closure(
            &transaction,
            clinical_id(&user.id.display_string()?)?,
        )
        .await
        .map_err(super::credentials::platform_error)?;
        let deleted = Entity::delete_many()
            .filter(Column::Identifier.eq(format!("password-change:{operation_id}")))
            .filter(Column::Value.eq(serde_json::to_string(&pending)?))
            .filter(Column::ExpiresAt.gt(chrono::Utc::now()))
            .exec(&transaction)
            .await
            .map_err(database_error)?;
        if deleted.rows_affected != 1 {
            return Err(AuthError::InvalidCredentials);
        }
        store.close_account_in(&transaction, user).await?;
        transaction.commit().await.map_err(database_error)?;
        let mut response = AuthResponse::json(200, &json!({"status":true,"redirect":"/login"}))?;
        better_auth_core::utils::cookie_utils::delete_session_cookies(
            request,
            &ctx.config,
            false,
            Some(&mut response.headers),
        )?;
        return Ok(response);
    }
    use super::schema::verification::{Column, Entity};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let transaction = store.account_transaction().await?;
    store
        .require_security_session(&transaction, session)
        .await?;
    let deleted = Entity::delete_many()
        .filter(Column::Identifier.eq(format!("password-change:{operation_id}")))
        .filter(Column::Value.eq(serde_json::to_string(&pending)?))
        .filter(Column::ExpiresAt.gt(chrono::Utc::now()))
        .exec(&transaction)
        .await
        .map_err(database_error)?;
    if deleted.rows_affected != 1 {
        return Err(AuthError::InvalidCredentials);
    }
    let response =
        super::credentials::complete_in(store, &transaction, request, user, session, pending)
            .await?;
    transaction.commit().await.map_err(database_error)?;
    Ok(response)
}

pub(super) async fn handle(
    service: &super::IdentityService,
    request: &AuthRequest,
) -> AuthResult<AuthResponse> {
    let ctx = service.context();
    let policy = &service.policy;
    let (user, session) = ctx.require_authoritative_session(request).await?;
    if session
        .additional_fields
        .get("clinical_session_purpose")
        .and_then(better_auth_core::FieldValue::as_str)
        != Some("authenticated")
    {
        return Err(AuthError::Unauthenticated);
    }
    let user = ctx
        .database
        .get_user_by_id(&user.id.display_string()?)
        .await?
        .ok_or(AuthError::Unauthenticated)?;
    let accounts = ctx
        .database
        .get_user_accounts(&user.id.display_string()?)
        .await?;
    let credential = accounts
        .iter()
        .find(|account| account.provider_id == "credential");
    let input: ConfirmPassword = request
        .body_as_json()
        .map_err(|_| AuthError::bad_request("Invalid password operation"))?;
    if credential.is_none_or(|account| {
        !matches!(
            account.password,
            better_auth_core::SchemaValue::Typed(Some(_))
        )
    }) {
        return Err(AuthError::InvalidCredentials);
    }
    pending_password(ctx, &session, &input.operation_id).await?;
    let result = service
        .call_endpoint(
            HttpMethod::Post,
            "/verify-password",
            better_auth::server_api::EndpointInput {
                request: Some(request.clone()),
                headers: Some(request.headers.clone()),
                body: Some(json!({"password":input.password})),
                ..Default::default()
            },
        )
        .await?;
    if result.status >= 400 {
        return Err(AuthError::InvalidCredentials);
    }
    let factor_enabled = user.two_factor_enabled.typed()?.unwrap_or(false);
    if factor_enabled {
        let code = input
            .totp_code
            .as_deref()
            .ok_or(AuthError::InvalidCredentials)?;
        let rejected = super::totp::verify_operation_factor(service, request, code).await?;
        if let Some(rejected) = rejected {
            return Ok(rejected);
        }
    }
    complete_password(policy, request, ctx, &user, &session, &input.operation_id).await
}
