use crate::models::identity::better_auth::IdentityService;
use axum::{
    Extension,
    extract::{Form, Query},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use axum_csrf::CsrfToken;
use loco_rs::prelude::*;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub(super) struct Start {
    authenticity_token: String,
    operation_id: Option<String>,
    #[serde(default)]
    link: bool,
}
#[derive(Deserialize)]
pub(super) struct RegistrationForm {
    authenticity_token: String,
    #[serde(flatten)]
    profile: crate::models::identity::better_auth::oidc::Profile,
}
#[derive(Deserialize)]
pub(super) struct Operation {
    operation_id: String,
}

fn provider_error(error: &better_auth_core::AuthError, operation: &'static str) {
    let status = error
        .status_code()
        .ok()
        .and_then(|value| value.typed().ok().copied())
        .unwrap_or(500);
    let category = match error {
        better_auth_core::AuthError::Database(_) => "database",
        better_auth_core::AuthError::Config(_) => "configuration",
        better_auth_core::AuthError::Internal(_) => "internal",
        _ => "native_endpoint",
    };
    if status >= 500 {
        tracing::error!(
            operation,
            category,
            status,
            "Native provider operation failed"
        );
    } else {
        tracing::debug!(
            operation,
            category,
            status,
            "Native provider operation rejected"
        );
    }
}

fn native_redirect(response: better_auth_core::AuthResponse) -> Response {
    if response.status >= 400 {
        return better_auth_core::error::materialize_axum_response(response)
            .unwrap_or_else(|_| super::rendering::unavailable());
    }
    let value = response.body.json().ok().flatten().unwrap_or_default();
    let Some(url) = value["url"].as_str() else {
        return better_auth_core::error::materialize_axum_response(response)
            .unwrap_or_else(|_| super::rendering::unavailable());
    };
    let mut redirect = super::rendering::redirect(url);
    for cookie in response.headers.get_all("set-cookie") {
        let Ok(cookie) = cookie.parse() else {
            return super::rendering::unavailable();
        };
        redirect.headers_mut().append(header::SET_COOKIE, cookie);
    }
    redirect
}

pub(super) async fn link(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    csrf: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    if service.provider.is_none() {
        return StatusCode::NOT_FOUND.into_response();
    }
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
    let Ok(token) = csrf.authenticity_token() else {
        return super::rendering::unavailable();
    };
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = json!("Link ZITADEL");
    data["authenticity_token"] = json!(token);
    match super::security_locale::view(&view, "identity_onboarding/provider_link.html", data) {
        Ok(response) => (csrf, [(header::CACHE_CONTROL, "no-store")], response).into_response(),
        Err(_) => super::rendering::unavailable(),
    }
}

pub(super) async fn registration(
    Extension(service): Extension<IdentityService>,
    csrf: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    if service.provider.is_none() {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Ok(token) = csrf.authenticity_token() else {
        return super::rendering::unavailable();
    };
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = json!("Create your account with ZITADEL");
    data["authenticity_token"] = json!(token);
    match super::security_locale::view(
        &view,
        "identity_onboarding/provider_registration.html",
        data,
    ) {
        Ok(response) => (
            csrf,
            [
                (header::CACHE_CONTROL, "no-store"),
                (header::REFERRER_POLICY, "strict-origin"),
            ],
            response,
        )
            .into_response(),
        Err(_) => super::rendering::unavailable(),
    }
}

pub(super) async fn register(
    Extension(service): Extension<IdentityService>,
    request_id: Option<Extension<loco_rs::controller::middleware::request_id::LocoRequestId>>,
    headers: HeaderMap,
    csrf: CsrfToken,
    Form(form): Form<RegistrationForm>,
) -> Response {
    if csrf.verify(&form.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Some(provider) = service.provider.as_ref() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let request = super::passkeys::request(&headers);
    match provider.preflight(&service, &request).await {
        Ok(Some(response)) => return native_redirect(response),
        Err(_) => return super::rendering::unavailable(),
        Ok(None) => {}
    }
    match crate::models::identity::better_auth::trusted(
        &service,
        &request,
        crate::controllers::medications::request_id(request_id),
        provider.register(&service, &request, form.profile),
    )
    .await
    {
        Ok(response) => native_redirect(response),
        Err(error) => {
            provider_error(&error, "register");
            super::rendering::redirect("/auth/zitadel/register")
        }
    }
}

pub(super) async fn start(
    Extension(service): Extension<IdentityService>,
    request_id: Option<Extension<loco_rs::controller::middleware::request_id::LocoRequestId>>,
    headers: HeaderMap,
    csrf: CsrfToken,
    Form(form): Form<Start>,
) -> Response {
    if csrf.verify(&form.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Some(provider) = service.provider.as_ref() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let request = super::passkeys::request(&headers);
    match provider.preflight(&service, &request).await {
        Ok(Some(response)) => return native_redirect(response),
        Err(_) => return super::rendering::unavailable(),
        Ok(None) => {}
    }
    match crate::models::identity::better_auth::trusted(
        &service,
        &request,
        crate::controllers::medications::request_id(request_id),
        provider.begin(&service, &request, form.operation_id, form.link),
    )
    .await
    {
        Ok(response) => native_redirect(response),
        Err(error) => {
            provider_error(&error, "start");
            super::rendering::redirect("/login")
        }
    }
}

pub(super) async fn callback(
    Extension(service): Extension<IdentityService>,
    request_id: Option<Extension<loco_rs::controller::middleware::request_id::LocoRequestId>>,
    headers: HeaderMap,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Response {
    let Some(provider) = service.provider.as_ref() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let request = super::passkeys::request(&headers);
    match crate::models::identity::better_auth::trusted(
        &service,
        &request,
        crate::controllers::medications::request_id(request_id),
        provider.finish(&service, &request, json!(query)),
    )
    .await
    {
        Ok(response) => better_auth_core::error::materialize_axum_response(response)
            .unwrap_or_else(|_| super::rendering::unavailable()),
        Err(error) => {
            provider_error(&error, "callback");
            super::rendering::redirect("/login")
        }
    }
}

pub(super) async fn complete_operation(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    ViewEngine(view): ViewEngine<TeraView>,
    Query(operation): Query<Operation>,
) -> Response {
    let Some(provider) = service.provider.as_ref() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let request = super::passkeys::request(&headers);
    match crate::models::identity::better_auth::trusted(
        &service,
        &request,
        uuid::Uuid::new_v4().to_string(),
        provider.complete_operation(&service, &request, &operation.operation_id),
    )
    .await
    {
        Ok(response) if response.status < 400 => {
            let body = response.body.json().ok().flatten().unwrap_or_default();
            if body["apiKey"].is_string() {
                return super::keys::result(&view, &body);
            }
            let mut redirect = super::rendering::redirect(
                body["redirect"].as_str().unwrap_or("/account/security"),
            );
            for cookie in response.headers.get_all("set-cookie") {
                let Ok(cookie) = cookie.parse() else {
                    return super::rendering::unavailable();
                };
                redirect.headers_mut().append(header::SET_COOKIE, cookie);
            }
            redirect
        }
        Ok(response) => native_redirect(response),
        Err(_) => super::rendering::redirect("/account/security"),
    }
}
