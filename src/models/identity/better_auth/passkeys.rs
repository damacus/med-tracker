use better_auth_core::{AuthError, AuthRequest, AuthResponse, AuthResult, HttpMethod};
use serde::Deserialize;
use serde_json::json;

use super::IdentityService;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Operation {
    operation_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Proof {
    operation_id: String,
    response: serde_json::Value,
}

pub(super) async fn start(
    service: &IdentityService,
    request: &AuthRequest,
) -> AuthResult<AuthResponse> {
    let input: Operation = request.body_as_json()?;
    let (_, session) = service
        .context()
        .require_authoritative_session(request)
        .await?;
    super::operations::pending_password(service.context(), &session, &input.operation_id).await?;
    service
        .call_endpoint(
            HttpMethod::Get,
            "/passkey/generate-authenticate-options",
            better_auth::server_api::EndpointInput {
                request: Some(request.clone()),
                headers: Some(request.headers.clone()),
                ..Default::default()
            },
        )
        .await
}

pub(super) async fn confirm(
    service: &IdentityService,
    request: &AuthRequest,
) -> AuthResult<AuthResponse> {
    let input: Proof = request.body_as_json()?;
    let (target, previous) = service
        .context()
        .require_authoritative_session(request)
        .await?;
    let pending =
        super::operations::pending_password(service.context(), &previous, &input.operation_id)
            .await?;
    let native = service
        .call_endpoint(
            HttpMethod::Post,
            "/passkey/verify-authentication",
            better_auth::server_api::EndpointInput {
                request: Some(request.clone()),
                headers: Some(request.headers.clone()),
                body: Some(json!({"response":input.response})),
                ..Default::default()
            },
        )
        .await?;
    if native.status >= 400 {
        return Ok(native);
    }
    let body = native.body.field_value()?;
    let native_session = body.model_property("session")?;
    let native_user = body.model_property("user")?;
    let target_id = target.id.display_string()?;
    let token = native_session.model_property("token")?;
    let token = token.as_str().ok_or(AuthError::InvalidCredentials)?;
    let session = service
        .context()
        .database
        .get_session(token)
        .await?
        .ok_or(AuthError::Unauthenticated)?;
    if session.user_id.display_string()? != target_id
        || better_auth_core::SchemaValue::<String>::from_field(native_user.model_property("id")?)
            .display_string()?
            != target_id
        || better_auth_core::SchemaValue::<String>::from_field(
            native_session.model_property("userId")?,
        )
        .display_string()?
            != target_id
        || better_auth_core::SchemaValue::<String>::from_field(native_session.model_property("id")?)
            .display_string()?
            != session.id.display_string()?
        || session.id == previous.id
        || !session.active
    {
        return Err(AuthError::InvalidCredentials);
    }
    let mut response = super::operations::complete_proven_operation(
        &service.policy,
        request,
        service.context(),
        &target,
        &session,
        &input.operation_id,
        pending,
    )
    .await?;
    let domain_headers = std::mem::replace(&mut response.headers, native.headers);
    for (name, value) in domain_headers {
        response = response.with_appended_header(name, value);
    }
    Ok(response)
}
