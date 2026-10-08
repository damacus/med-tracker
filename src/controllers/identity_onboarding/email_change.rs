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
pub(super) struct Start {
    new_email: String,
    authenticity_token: String,
}
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

fn render(view: &TeraView, csrf: CsrfToken, mode: &str, link: Option<&Link>) -> Response {
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
    data["operation_id"] = json!(link.map(|value| &value.operation_id));
    data["proof"] = json!(link.map(|value| &value.token));
    match format::render().view(view, "identity_onboarding/email_change.html", data) {
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
    render(&view, csrf, "start", None)
}
pub(super) async fn pending(csrf: CsrfToken, ViewEngine(view): ViewEngine<TeraView>) -> Response {
    render(&view, csrf, "pending", None)
}
pub(super) async fn confirmation(
    Query(link): Query<Link>,
    csrf: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    render(&view, csrf, "confirm", Some(&link))
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
    request.body = serde_json::to_vec(
        &json!({"action":"change_email","new_email":form.new_email.trim().to_lowercase()}),
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
            let Ok(body) = serde_json::from_slice::<serde_json::Value>(&response.body) else {
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
                "changing your email address",
            )
        }
        _ => (
            StatusCode::BAD_REQUEST,
            "Email change is unavailable. Check your address and sign-in session.",
        )
            .into_response(),
    }
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
    request.path = "/security/email/confirm".into();
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
        Ok(response) if response.status == 200 => super::rendering::redirect("/account/security"),
        _ => (
            StatusCode::UNAUTHORIZED,
            "Email confirmation expired or does not belong to this session.",
        )
            .into_response(),
    }
}
