use crate::models::identity::better_auth::{IdentityService, dispatch};
use axum::{
    Extension,
    extract::Form,
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use axum_csrf::CsrfToken;
use loco_rs::prelude::*;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub(super) struct Start {
    action: String,
    target: Option<String>,
    authenticity_token: String,
}
pub(super) async fn start(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    request_id: Option<Extension<loco_rs::controller::middleware::request_id::LocoRequestId>>,
    csrf: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(form): Form<Start>,
) -> Response {
    if csrf.verify(&form.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let mut request = super::passkeys::request(&headers);
    request.method = better_auth_core::HttpMethod::Post;
    request.path = "/security/operation/start".into();
    request
        .headers
        .insert("content-type".into(), "application/json".into());
    if matches!(
        form.action.as_str(),
        "remove_password" | "remove_passkey" | "unlink_provider"
    ) {
        let native = async {
            let (user, _) = service.context().require_authoritative_session(&request).await?;
            let (path, body) = if form.action == "remove_passkey" {
                ("/passkey/delete-passkey", json!({"id":form.target.as_ref().ok_or(better_auth_core::AuthError::InvalidCredentials)?}))
            } else {
                let provider = if form.action == "remove_password" { "credential" } else { "zitadel" };
                let account = service.context().database.get_user_accounts(&user.id.display_string()?).await?.into_iter().find(|account| account.provider_id.as_str() == Some(provider)).ok_or(better_auth_core::AuthError::InvalidCredentials)?;
                ("/unlink-account", json!({"accountId":account.id.display_string()?}))
            };
            request.path=path.into();request.body=Some(serde_json::to_vec(&body)?);
            dispatch(&service, request, crate::controllers::medications::request_id(request_id)).await
        }.await;
        return match native {
            Ok(response) if response.status < 400 => {
                let mut redirect = super::rendering::redirect("/account/security");
                for cookie in response.headers.get_all("set-cookie") {
                    let Ok(cookie) = cookie.parse() else {
                        return super::rendering::unavailable();
                    };
                    redirect.headers_mut().append(header::SET_COOKIE, cookie);
                }
                redirect
            }
            _ => (
                StatusCode::BAD_REQUEST,
                "This sign-in method could not be removed. Check your session and try again.",
            )
                .into_response(),
        };
    }
    let label = match form.action.as_str() {
        "unlink_provider" => "unlinking ZITADEL",
        "close_account" => "closing your account",
        "remove_password" => "removing your password",
        "remove_passkey" => "removing this passkey",
        _ => "account change",
    };
    let mut body = json!({"action":form.action});
    if let Some(target) = form.target {
        body["target"] = json!(target);
    }
    request.body = serde_json::to_vec(&body).ok();
    match dispatch(
        &service,
        request,
        crate::controllers::medications::request_id(request_id),
    )
    .await
    {
        Ok(response) if response.status == 200 => {
            let Ok(Some(body)) = response.body.json() else {
                return super::rendering::unavailable();
            };
            let Some(id) = body["operation_id"].as_str() else {
                return super::rendering::unavailable();
            };
            super::password::render_named(
                &view,
                csrf,
                Some(id),
                None,
                StatusCode::OK,
                (
                    body["password_proof"].as_bool() == Some(true),
                    body["passkey_proof"].as_bool() == Some(true),
                    body["oidc_proof"].as_bool() == Some(true),
                ),
                label,
            )
        }
        _ => (
            StatusCode::BAD_REQUEST,
            "This account operation is unavailable. Check your session and permissions.",
        )
            .into_response(),
    }
}

pub(super) async fn passkey(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    if !matches!(
        crate::models::identity::better_auth::browser_identity(
            &service,
            &super::passkeys::request(&headers)
        )
        .await,
        Ok(crate::models::identity::better_auth::BrowserIdentity::Authenticated { .. })
    ) {
        return super::rendering::redirect("/login");
    }
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = json!("Add passkey");
    match super::security_locale::view(&view, "identity_onboarding/add_passkey.html", data) {
        Ok(response) => ([(header::CACHE_CONTROL, "no-store")], response).into_response(),
        Err(_) => super::rendering::unavailable(),
    }
}
