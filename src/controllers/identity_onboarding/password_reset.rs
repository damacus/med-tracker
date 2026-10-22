use crate::models::identity::better_auth::{IdentityService, dispatch};
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
pub(super) struct Link {
    token: String,
}
#[derive(Deserialize)]
pub(super) struct Request {
    authenticity_token: String,
    email: String,
}
#[derive(Deserialize)]
pub(super) struct Reset {
    authenticity_token: String,
    token: String,
    password: String,
}

fn render(
    view: &TeraView,
    csrf: CsrfToken,
    token: Option<&str>,
    sent: bool,
    error: Option<&str>,
) -> Response {
    let Ok(authenticity_token) = csrf.authenticity_token() else {
        return super::rendering::unavailable();
    };
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = json!("Reset your password");
    data["authenticity_token"] = json!(authenticity_token);
    data["token"] = json!(token);
    data["sent"] = json!(sent);
    data["error"] = json!(error);
    match format::render().view(view, "identity_onboarding/password_reset.html", data) {
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

pub(super) async fn show(csrf: CsrfToken, ViewEngine(view): ViewEngine<TeraView>) -> Response {
    render(&view, csrf, None, false, None)
}
pub(super) async fn confirmation(
    Query(link): Query<Link>,
    csrf: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    if link.token.len() > 4096 {
        return StatusCode::BAD_REQUEST.into_response();
    }
    render(&view, csrf, Some(&link.token), false, None)
}
pub(super) async fn request(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    csrf: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(form): Form<Request>,
) -> Response {
    if csrf.verify(&form.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let mut request = super::passkeys::request(&headers);
    request.method = better_auth_core::HttpMethod::Post;
    request
        .headers
        .insert("content-type".into(), "application/json".into());
    request.path = "/request-password-reset".into();
    request.body =
        serde_json::to_vec(&json!({"email":form.email,"redirectTo":"/reset-password/confirm"}))
            .ok();
    match dispatch(&service, request, uuid::Uuid::new_v4().to_string()).await {
        Ok(response) if response.status < 400 => render(&view, csrf, None, true, None),
        _ => render(
            &view,
            csrf,
            None,
            false,
            Some("Your request could not be completed. Try again shortly."),
        ),
    }
}
pub(super) async fn reset(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    csrf: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(form): Form<Reset>,
) -> Response {
    if csrf.verify(&form.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    if form.token.len() > 4096 {
        return StatusCode::BAD_REQUEST.into_response();
    }
    if let Err(message) =
        crate::models::identity::better_auth::validate_new_password(&form.password)
    {
        return render(&view, csrf, Some(&form.token), false, Some(message));
    }
    let mut request = super::passkeys::request(&headers);
    request.method = better_auth_core::HttpMethod::Post;
    request
        .headers
        .insert("content-type".into(), "application/json".into());
    request.path = "/reset-password".into();
    request.body =
        serde_json::to_vec(&json!({"token":form.token,"newPassword":form.password})).ok();
    match dispatch(&service, request, uuid::Uuid::new_v4().to_string()).await {
        Ok(response) if response.status < 400 => super::rendering::redirect("/login"),
        _ => render(
            &view,
            csrf,
            Some(&form.token),
            false,
            Some("This reset link is invalid or expired. Request a new link."),
        ),
    }
}
