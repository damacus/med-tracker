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
    link_id: Option<String>,
}
#[derive(Deserialize)]
pub(super) struct Link {
    link_id: String,
}
#[derive(Deserialize)]
pub(super) struct Callback {
    code: Option<String>,
    state: Option<String>,
}
#[derive(Deserialize)]
pub(super) struct RegistrationLink {
    registration_id: String,
}
#[derive(Deserialize)]
pub(super) struct RegistrationForm {
    authenticity_token: String,
    registration_id: String,
    #[serde(flatten)]
    profile: crate::models::identity::better_auth::oidc::Profile,
}

fn limited_response(response: better_auth_core::AuthResponse) -> Response {
    let mut builder = axum::http::Response::builder().status(response.status);
    for (name, value) in response.headers.iter() {
        builder = builder.header(name, value);
    }
    builder
        .body(axum::body::Body::from(response.body))
        .unwrap_or_else(|_| super::rendering::unavailable())
}

async fn preflight(service: &IdentityService, headers: &HeaderMap) -> Option<Response> {
    let provider = service.provider.as_ref()?;
    match provider
        .preflight(service, &super::passkeys::request(headers))
        .await
    {
        Ok(Some(response)) => Some(limited_response(response)),
        Ok(None) => None,
        Err(_) => Some(super::rendering::unavailable()),
    }
}

pub(super) async fn link(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    csrf: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Query(link): Query<Link>,
) -> Response {
    let Some(provider) = service.provider.as_ref() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if provider
        .link_available(&service, &super::passkeys::request(&headers), &link.link_id)
        .await
        .is_err()
    {
        return unavailable(&view);
    }
    let Ok(token) = csrf.authenticity_token() else {
        return super::rendering::unavailable();
    };
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = json!("Link ZITADEL");
    data["authenticity_token"] = json!(token);
    data["link_id"] = json!(link.link_id);
    match super::security_locale::view(&view, "identity_onboarding/provider_link.html", data) {
        Ok(response) => (csrf, [(header::CACHE_CONTROL, "no-store")], response).into_response(),
        Err(_) => super::rendering::unavailable(),
    }
}

pub(super) async fn registration(
    Extension(service): Extension<IdentityService>,
    csrf: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Query(link): Query<RegistrationLink>,
) -> Response {
    let Some(provider) = service.provider.as_ref() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Ok(profile) = provider
        .registration(&service, &link.registration_id, |binding| {
            csrf.verify(binding).is_ok()
        })
        .await
    else {
        return unavailable(&view);
    };
    let Ok(token) = csrf.authenticity_token() else {
        return super::rendering::unavailable();
    };
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = json!("Complete your account");
    data["email"] = profile["email"].clone();
    data["name"] = profile["name"].clone();
    data["authenticity_token"] = json!(token);
    data["registration_id"] = json!(link.registration_id);
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
    headers: HeaderMap,
    request_id: Option<Extension<loco_rs::controller::middleware::request_id::LocoRequestId>>,
    csrf: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(form): Form<RegistrationForm>,
) -> Response {
    if csrf.verify(&form.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Some(provider) = service.provider.as_ref() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match provider
        .register(
            &service,
            &super::passkeys::request(&headers),
            crate::controllers::medications::request_id(request_id),
            &form.registration_id,
            form.profile,
            |binding| csrf.verify(binding).is_ok(),
        )
        .await
    {
        Ok(response) if response.status == 200 => successful(response, &view),
        Ok(response) if response.status == 429 => limited_response(response),
        _ => unavailable(&view),
    }
}

fn successful(response: better_auth_core::AuthResponse, view: &TeraView) -> Response {
    let body = serde_json::from_slice::<serde_json::Value>(&response.body).unwrap_or_default();
    if body["apiKey"].is_string() {
        return super::keys::result(view, &body);
    }
    if body["recoveryCodes"].is_array() {
        return super::credentials::recovery_codes(view, &body);
    }
    let mut redirect =
        super::rendering::redirect(body["redirect"].as_str().unwrap_or("/account/security"));
    redirect.headers_mut().insert(
        header::REFERRER_POLICY,
        header::HeaderValue::from_static("strict-origin"),
    );
    for cookie in response.headers.get_all("set-cookie") {
        let Ok(value) = cookie.parse() else {
            return super::rendering::unavailable();
        };
        redirect.headers_mut().append(header::SET_COOKIE, value);
    }
    redirect
}

fn unavailable(view: &TeraView) -> Response {
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = json!("ZITADEL sign-in unavailable");
    match super::security_locale::view(view, "identity_onboarding/provider_error.html", data) {
        Ok(response) => (
            StatusCode::UNAUTHORIZED,
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
pub(super) async fn start(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    csrf: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(form): Form<Start>,
) -> Response {
    if csrf.verify(&form.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Some(provider) = service.provider.as_ref() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if let Some(response) = preflight(&service, &headers).await {
        return response;
    }
    let Ok(binding) = csrf.authenticity_token() else {
        return super::rendering::unavailable();
    };
    match provider
        .begin(
            &service,
            &super::passkeys::request(&headers),
            binding,
            form.operation_id,
            form.link_id,
        )
        .await
    {
        Ok(url) => (csrf, super::rendering::redirect(&url)).into_response(),
        Err(_) => unavailable(&view),
    }
}
pub(super) async fn callback(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    request_id: Option<Extension<loco_rs::controller::middleware::request_id::LocoRequestId>>,
    csrf: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Query(query): Query<Callback>,
) -> Response {
    let Some(provider) = service.provider.as_ref() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if let Some(response) = preflight(&service, &headers).await {
        return response;
    }
    let (Some(code), Some(state)) = (query.code, query.state) else {
        return unavailable(&view);
    };
    match provider
        .finish(
            &service,
            &super::passkeys::request(&headers),
            crate::controllers::medications::request_id(request_id),
            &code,
            &state,
            |binding| csrf.verify(binding).is_ok(),
        )
        .await
    {
        Ok(response) if response.status == 200 => successful(response, &view),
        _ => unavailable(&view),
    }
}
