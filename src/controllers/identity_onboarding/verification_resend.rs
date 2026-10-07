use axum::{
    Extension,
    extract::Form,
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use axum_csrf::CsrfToken;
use loco_rs::prelude::*;
use serde::Deserialize;

use crate::models::identity::better_auth::{IdentityService, dispatch};

#[derive(Deserialize)]
pub(super) struct ResendForm {
    email: String,
    authenticity_token: String,
}

pub(super) async fn show(token: CsrfToken, ViewEngine(view): ViewEngine<TeraView>) -> Response {
    let Ok(authenticity_token) = token.authenticity_token() else {
        return super::rendering::unavailable();
    };
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = serde_json::json!("Resend verification email");
    data["authenticity_token"] = serde_json::json!(authenticity_token);
    match format::render().view(&view, "identity_onboarding/verification_resend.html", data) {
        Ok(response) => (
            StatusCode::OK,
            token,
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

pub(super) async fn resend(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    request_id: Option<Extension<loco_rs::controller::middleware::request_id::LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(form): Form<ResendForm>,
) -> Response {
    if token.verify(&form.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let mut request = super::passkeys::request(&headers);
    request.method = better_auth_core::HttpMethod::Post;
    request.path = "/send-verification-email".into();
    request
        .headers
        .insert("content-type".into(), "application/json".into());
    request.body = serde_json::to_vec(
        &serde_json::json!({"email":form.email.trim(),"callbackURL":"/auth/passkey/setup"}),
    )
    .ok();
    match dispatch(
        &service,
        request,
        crate::controllers::medications::request_id(request_id),
    )
    .await
    {
        Ok(response) if response.status < 500 => super::rendering::sent(&view),
        Err(error) if error.status_code() < 500 => super::rendering::sent(&view),
        Ok(response) => {
            tracing::warn!(
                status = response.status,
                "Verification resend rejected by framework"
            );
            super::rendering::unavailable()
        }
        Err(error) => {
            tracing::warn!(status = error.status_code(), "Verification resend failed");
            super::rendering::unavailable()
        }
    }
}
