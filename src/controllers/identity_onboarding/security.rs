use axum::{
    Extension,
    extract::Form,
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use axum_csrf::CsrfToken;
use loco_rs::prelude::*;
use serde_json::json;

use crate::models::{
    identity::better_auth::{BrowserIdentity, IdentityService, browser_identity, clinical_id},
    platform,
};

pub(super) async fn settings(
    State(ctx): State<AppContext>,
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    let identity = browser_identity(&service, &super::passkeys::request(&headers)).await;
    let Ok(BrowserIdentity::Authenticated { user, session }) = identity else {
        return super::rendering::redirect("/login");
    };
    let Ok(sessions) = service.context().database.get_user_sessions(&user.id).await else {
        return super::rendering::unavailable();
    };
    let Ok(accounts) = service.context().database.get_user_accounts(&user.id).await else {
        return super::rendering::unavailable();
    };
    let Ok(passkeys) = service
        .context()
        .database
        .list_passkeys_by_user(&user.id)
        .await
    else {
        return super::rendering::unavailable();
    };
    let Ok(authenticity_token) = token.authenticity_token() else {
        return super::rendering::unavailable();
    };
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = json!("Account security");
    data["email"] = json!(user.email);
    let Ok(retained_factor) = service.store.retained_factor_enabled(&user.id).await else {
        return super::rendering::unavailable();
    };
    data["totp"] = json!(user.two_factor_enabled || retained_factor);
    data["password"] = json!(
        accounts
            .iter()
            .any(|account| account.provider_id == "credential" && account.password.is_some())
    );
    data["zitadel"] = json!(
        accounts
            .iter()
            .any(|account| account.provider_id == "zitadel")
    );
    data["provider_available"] = json!(service.provider.is_some());
    data["passkeys"] = json!(
        passkeys
            .into_iter()
            .map(|key| json!({"id":key.id,"name":key.name.unwrap_or_else(|| "Passkey".into())}))
            .collect::<Vec<_>>()
    );
    data["sessions"] = json!(sessions.into_iter().map(|entry| json!({"id":entry.id,"current":entry.id == session.id,"created":entry.created_at.format("%d %b %Y %H:%M UTC").to_string(),"expires":entry.expires_at.format("%d %b %Y %H:%M UTC").to_string()})).collect::<Vec<_>>());
    let Ok(account_id) = clinical_id(&user.id) else {
        return super::rendering::unavailable();
    };
    let (mut platform_admin, mut support_access) = (false, false);
    if let Ok(transaction) = platform::begin(&ctx.db).await {
        platform_admin = platform::administrator(&transaction, account_id)
            .await
            .is_ok();
        support_access = platform::support::support_link(&transaction, account_id)
            .await
            .unwrap_or(false);
        let _ = transaction.rollback().await;
    }
    data["platform_admin"] = json!(platform_admin);
    data["support_access"] = json!(support_access);
    data["authenticity_token"] = json!(authenticity_token);
    match format::render().view(&view, "identity_onboarding/security.html", data) {
        Ok(response) => (
            StatusCode::OK,
            token,
            [(header::CACHE_CONTROL, "no-store")],
            response,
        )
            .into_response(),
        Err(_) => super::rendering::unavailable(),
    }
}

#[derive(serde::Deserialize)]
pub(super) struct Revoke {
    session_id: String,
    authenticity_token: String,
}

pub(super) async fn revoke(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    request_id: Option<Extension<loco_rs::controller::middleware::request_id::LocoRequestId>>,
    csrf: CsrfToken,
    Form(form): Form<Revoke>,
) -> Response {
    if csrf.verify(&form.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let mut request = super::passkeys::request(&headers);
    request.method = better_auth_core::HttpMethod::Post;
    request.path = "/security/session/revoke".into();
    request
        .headers
        .insert("content-type".into(), "application/json".into());
    request.body = serde_json::to_vec(&json!({"session_id":form.session_id})).ok();
    match crate::models::identity::better_auth::dispatch(
        &service,
        request,
        crate::controllers::medications::request_id(request_id),
    )
    .await
    {
        Ok(response) if response.status == 200 => {
            let body =
                serde_json::from_slice::<serde_json::Value>(&response.body).unwrap_or_default();
            let mut redirect =
                super::rendering::redirect(if body["current"].as_bool() == Some(true) {
                    "/login"
                } else {
                    "/account/security"
                });
            for cookie in response.headers.get_all("set-cookie") {
                let Ok(value) = cookie.parse() else {
                    return super::rendering::unavailable();
                };
                redirect.headers_mut().append(header::SET_COOKIE, value);
            }
            redirect
        }
        _ => StatusCode::UNAUTHORIZED.into_response(),
    }
}
