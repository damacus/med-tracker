use super::{
    IdentityService,
    policy::{clinical_id, database_error},
};
use better_auth_core::{AuthError, AuthRequest, AuthResponse, AuthResult, HttpMethod};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Revoke {
    session_id: String,
}

pub(super) async fn revoke(
    service: &IdentityService,
    request: &AuthRequest,
) -> AuthResult<AuthResponse> {
    let input: Revoke = request
        .body_as_json()
        .map_err(|_| AuthError::bad_request("Invalid session operation"))?;
    let (_, current) = service
        .context()
        .require_authoritative_session(request)
        .await?;
    let transaction = service.policy.transaction().await?;
    service
        .policy
        .require_security_session(&transaction, &current)
        .await?;
    transaction.commit().await.map_err(database_error)?;
    use super::schema::session::{Column, Entity};
    let target = Entity::find_by_id(clinical_id(&input.session_id)?)
        .filter(Column::UserId.eq(clinical_id(&current.user_id.display_string()?)?))
        .one(service.policy.database())
        .await
        .map_err(database_error)?
        .ok_or(AuthError::InvalidCredentials)?;
    let native = service
        .call_endpoint(
            HttpMethod::Post,
            "/revoke-session",
            better_auth::server_api::EndpointInput {
                request: Some(request.clone()),
                headers: Some(request.headers.clone()),
                body: Some(serde_json::json!({"token":target.token})),
                ..Default::default()
            },
        )
        .await?;
    if native.status >= 400 {
        return Ok(native);
    }
    let is_current = target.id == clinical_id(&current.id.display_string()?)?;
    let mut response = AuthResponse::json(
        200,
        &serde_json::json!({"status":true,"current":is_current}),
    )?;
    for (name, value) in native.headers {
        response = response.with_appended_header(name, value);
    }
    if is_current {
        better_auth_core::utils::cookie_utils::delete_session_cookies(
            request,
            &service.context().config,
            false,
            Some(&mut response.headers),
        )?;
    }
    Ok(response)
}
