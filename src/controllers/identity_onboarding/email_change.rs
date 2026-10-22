use crate::models::identity::better_auth::{
    BrowserIdentity, IdentityService, browser_identity, dispatch,
};
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
    new_email: String,
    authenticity_token: String,
}
fn render(view: &TeraView, csrf: CsrfToken, mode: &str) -> Response {
    let Ok(token) = csrf.authenticity_token() else {
        return super::rendering::unavailable();
    };
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = json!(match mode {
        "confirm" => "Verify your new email address",
        "pending" => "Check your new email",
        _ => "Change email",
    });
    data["mode"] = json!(mode);
    data["authenticity_token"] = json!(token);
    match super::security_locale::view(view, "identity_onboarding/email_change.html", data) {
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
pub(super) async fn show(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    csrf: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    if !matches!(
        browser_identity(&service, &super::passkeys::request(&headers)).await,
        Ok(BrowserIdentity::Authenticated { .. })
    ) {
        return super::rendering::redirect("/login");
    }
    render(&view, csrf, "start")
}
pub(super) async fn pending(csrf: CsrfToken, ViewEngine(view): ViewEngine<TeraView>) -> Response {
    render(&view, csrf, "pending")
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
    request.path = "/change-email".into();
    request
        .headers
        .insert("content-type".into(), "application/json".into());
    request.body = serde_json::to_vec(
        &json!({"newEmail":form.new_email.trim().to_lowercase(),"callbackURL":"/account/security"}),
    )
    .ok();
    match dispatch(
        &service,
        request,
        crate::controllers::medications::request_id(request_id),
    )
    .await
    {
        Ok(response) if response.status == 200 => render(&view, csrf, "pending"),
        _ => (
            StatusCode::BAD_REQUEST,
            "Email change is unavailable. Check your address and sign-in session.",
        )
            .into_response(),
    }
}
