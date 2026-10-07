use axum::{Extension, extract::{Form, Query}, http::{HeaderMap, StatusCode, header}, response::IntoResponse};
use axum_csrf::CsrfToken;
use loco_rs::prelude::*;
use serde::Deserialize;
use serde_json::json;

use crate::models::identity::better_auth::{IdentityService, confirmation_email, dispatch};

#[derive(Deserialize)]
pub(super) struct Link { token: String }

#[derive(Deserialize)]
pub(super) struct Confirm { token: String, authenticity_token: String }

pub(super) async fn show(Extension(service): Extension<IdentityService>, Query(link): Query<Link>, token: CsrfToken, ViewEngine(view): ViewEngine<TeraView>) -> Response {
    let Ok(email) = confirmation_email(&service, &link.token).await else { return super::rendering::redirect("/login"); };
    let Ok(authenticity_token) = token.authenticity_token() else { return super::rendering::unavailable(); };
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = json!("Verify your account");
    data["email"] = json!(email);
    data["token"] = json!(link.token);
    data["authenticity_token"] = json!(authenticity_token);
    match format::render().view(&view, "identity_onboarding/confirm_email.html", data) {
        Ok(response) => (token, [(header::CACHE_CONTROL, "no-store"), (header::REFERRER_POLICY, "strict-origin")], response).into_response(),
        Err(_) => super::rendering::unavailable(),
    }
}

pub(super) async fn confirm(Extension(service): Extension<IdentityService>, headers: HeaderMap, request_id: Option<Extension<loco_rs::controller::middleware::request_id::LocoRequestId>>, token: CsrfToken, Form(form): Form<Confirm>) -> Response {
    if token.verify(&form.authenticity_token).is_err() { return StatusCode::FORBIDDEN.into_response(); }
    let mut request = super::passkeys::request(&headers);
    request.method = better_auth_core::HttpMethod::Post;
    request.path = "/onboarding/confirm-email".into();
    request.headers.insert("content-type".into(), "application/json".into());
    request.body = serde_json::to_vec(&json!({"token":form.token,"confirmed":true})).ok();
    match dispatch(&service, request, crate::controllers::medications::request_id(request_id)).await {
        Ok(response) if response.status < 400 => {
            let mut redirect = super::rendering::redirect("/auth/passkey/setup");
            for cookie in response.headers.get_all("set-cookie") {
                let Ok(value) = cookie.parse() else { return super::rendering::unavailable(); };
                redirect.headers_mut().append(header::SET_COOKIE, value);
            }
            redirect
        }
        Ok(response) => {
            tracing::warn!(status = response.status, "Account confirmation rejected by framework");
            super::rendering::redirect("/login")
        }
        Err(error) => {
            tracing::warn!(status = error.status_code(), error = %error, "Account confirmation failed");
            super::rendering::redirect("/login")
        }
    }
}
