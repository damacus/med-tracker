use super::{IdentityService, policy::database_error};
use better_auth_core::{AuthError, AuthRequest, AuthResponse, AuthResult, HttpMethod};
use serde_json::json;

pub(super) async fn verify_operation_factor(
    service: &IdentityService,
    request: &AuthRequest,
    code: &str,
) -> AuthResult<Option<AuthResponse>> {
    let (user, _) = service
        .context()
        .require_authoritative_session(request)
        .await?;
    if !user.two_factor_enabled.typed()?.unwrap_or(false) {
        return Err(AuthError::InvalidCredentials);
    }
    let response = service
        .call_endpoint(
            HttpMethod::Post,
            "/two-factor/verify-totp",
            better_auth::server_api::EndpointInput {
                request: Some(request.clone()),
                headers: Some(request.headers.clone()),
                body: Some(json!({"code":code,"trustDevice":false})),
                ..Default::default()
            },
        )
        .await?;
    Ok((response.status >= 400).then_some(response))
}

pub(super) async fn handle(
    service: &IdentityService,
    request: &AuthRequest,
) -> AuthResult<AuthResponse> {
    let body: serde_json::Value = request.body_as_json()?;
    let path = match request.path() {
        "/security/totp/start" => "/two-factor/enable",
        "/security/totp/finish" => "/two-factor/verify-totp",
        "/security/totp/login"
            if body.get("backup").and_then(serde_json::Value::as_bool) == Some(true) =>
        {
            "/two-factor/verify-backup-code"
        }
        "/security/totp/login" => "/two-factor/verify-totp",
        "/security/totp/disable/confirm" => "/two-factor/disable",
        _ => return Err(AuthError::not_found("Authenticator operation unavailable")),
    };
    if request.path() != "/security/totp/login" {
        let (_, session) = service
            .context()
            .require_authoritative_session(request)
            .await?;
        let transaction = service.policy.transaction().await?;
        service
            .policy
            .require_security_session(&transaction, &session)
            .await?;
        transaction.commit().await.map_err(database_error)?;
    }
    let body = match path {
        "/two-factor/enable" | "/two-factor/disable" => {
            json!({"password":body.get("password").and_then(serde_json::Value::as_str).unwrap_or_default()})
        }
        "/two-factor/verify-backup-code" => {
            json!({"code":body.get("code").and_then(serde_json::Value::as_str).unwrap_or_default(),"trustDevice":false})
        }
        _ => {
            json!({"code":body.get("code").and_then(serde_json::Value::as_str).unwrap_or_default(),"trustDevice":false})
        }
    };
    service
        .call_endpoint(
            HttpMethod::Post,
            path,
            better_auth::server_api::EndpointInput {
                request: Some(request.clone()),
                headers: Some(request.headers.clone()),
                body: Some(body),
                ..Default::default()
            },
        )
        .await
}
