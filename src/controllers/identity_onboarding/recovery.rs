use axum::{Extension, extract::Form, http::{HeaderMap, StatusCode, header}, response::IntoResponse};
use axum_csrf::CsrfToken;
use loco_rs::prelude::*;
use serde::Deserialize;

use crate::models::identity::better_auth::{IdentityService, dispatch};

#[derive(Deserialize)]
pub(super) struct RecoveryForm { authenticity_token: String, code: String }

fn render(view: &TeraView, token: CsrfToken, invalid: bool, status: StatusCode) -> Response {
    let Ok(authenticity_token) = token.authenticity_token() else { return super::rendering::unavailable(); };
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = serde_json::json!("Sign in with recovery code");
    data["authenticity_token"] = serde_json::json!(authenticity_token);
    data["invalid"] = serde_json::json!(invalid);
    match format::render().view(view, "identity_onboarding/recovery.html", data) {
        Ok(response) => (status, token, [(header::CACHE_CONTROL, "no-store")], response).into_response(),
        Err(_) => super::rendering::unavailable(),
    }
}

pub(super) async fn form(token: CsrfToken, ViewEngine(view): ViewEngine<TeraView>) -> Response {
    render(&view, token, false, StatusCode::OK)
}

pub(super) async fn login(Extension(service): Extension<IdentityService>, headers: HeaderMap, request_id: Option<Extension<loco_rs::controller::middleware::request_id::LocoRequestId>>, token: CsrfToken, ViewEngine(view): ViewEngine<TeraView>, Form(form): Form<RecoveryForm>) -> Response {
    if token.verify(&form.authenticity_token).is_err() { return StatusCode::FORBIDDEN.into_response(); }
    let mut request = super::passkeys::request(&headers);
    request.method = better_auth_core::HttpMethod::Post;
    request.path = "/recovery/login".into();
    request.body = serde_json::to_vec(&serde_json::json!({"code":form.code.trim()})).ok();
    match dispatch(&service, request, crate::controllers::medications::request_id(request_id)).await {
        Ok(response) if response.status < 400 => {
            let mut redirect = super::rendering::redirect("/");
            for cookie in response.headers.get_all("set-cookie") {
                let Ok(value) = cookie.parse() else { return super::rendering::unavailable(); };
                redirect.headers_mut().append(header::SET_COOKIE, value);
            }
            redirect
        }
        Ok(response) if response.status < 500 => render(&view, token, true, StatusCode::UNAUTHORIZED),
        Err(better_auth_core::AuthError::InvalidCredentials | better_auth_core::AuthError::Unauthenticated) => render(&view, token, true, StatusCode::UNAUTHORIZED),
        _ => super::rendering::unavailable(),
    }
}
