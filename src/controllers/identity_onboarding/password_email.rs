use crate::models::identity::better_auth::{
    BrowserIdentity, IdentityService, browser_identity, dispatch,
};
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
    operation_id: String,
    token: String,
}

#[derive(Deserialize)]
pub(super) struct Confirm {
    operation_id: String,
    token: String,
    authenticity_token: String,
}

pub(super) fn render(
    view: &TeraView,
    csrf: CsrfToken,
    operation_id: &str,
    proof: Option<&str>,
    email: Option<&str>,
    label: &str,
) -> Response {
    let Ok(authenticity_token) = csrf.authenticity_token() else {
        return super::rendering::unavailable();
    };
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = json!(label);
    data["operation_id"] = json!(operation_id);
    data["proof"] = json!(proof);
    data["email"] = json!(email);
    data["authenticity_token"] = json!(authenticity_token);
    match format::render().view(view, "identity_onboarding/password_email.html", data) {
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
    Query(link): Query<Link>,
    csrf: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    let Ok(BrowserIdentity::Authenticated { user, session }) =
        browser_identity(&service, &super::passkeys::request(&headers)).await
    else {
        return super::rendering::redirect("/login");
    };
    if session
        .additional_fields
        .get("authentication_method")
        .and_then(serde_json::Value::as_str)
        != Some("recovery")
    {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Ok(label) = crate::models::identity::better_auth::recovery_confirmation_label(
        &service,
        &super::passkeys::request(&headers),
        &link.operation_id,
    )
    .await
    else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    render(
        &view,
        csrf,
        &link.operation_id,
        Some(&link.token),
        user.email.as_deref(),
        label,
    )
}

pub(super) async fn confirm(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    request_id: Option<Extension<loco_rs::controller::middleware::request_id::LocoRequestId>>,
    csrf: CsrfToken,
    Form(form): Form<Confirm>,
) -> Response {
    if csrf.verify(&form.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let mut request = super::passkeys::request(&headers);
    request.method = better_auth_core::HttpMethod::Post;
    request.path = "/security/password/email/confirm".into();
    request
        .headers
        .insert("content-type".into(), "application/json".into());
    request.body = serde_json::to_vec(
        &json!({"operation_id":form.operation_id,"token":form.token,"confirmed":true}),
    )
    .ok();
    match dispatch(
        &service,
        request,
        crate::controllers::medications::request_id(request_id),
    )
    .await
    {
        Ok(response) if response.status == 200 => {
            let body =
                serde_json::from_slice::<serde_json::Value>(&response.body).unwrap_or_default();
            let mut redirect = super::rendering::redirect(
                body["redirect"].as_str().unwrap_or("/account/security"),
            );
            for cookie in response.headers.get_all("set-cookie") {
                let Ok(value) = cookie.parse() else {
                    return super::rendering::unavailable();
                };
                redirect.headers_mut().append(header::SET_COOKIE, value);
            }
            redirect
        }
        _ => (
            StatusCode::UNAUTHORIZED,
            "This confirmation is unavailable or expired. Start the sign-in method change again.",
        )
            .into_response(),
    }
}
