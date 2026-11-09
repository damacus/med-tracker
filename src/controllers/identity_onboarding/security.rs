use axum::{
    Extension,
    extract::Form,
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use axum_csrf::CsrfToken;
use loco_rs::prelude::*;
use serde_json::json;

use crate::models::identity::better_auth::{BrowserIdentity, IdentityService, browser_identity};

pub(super) async fn settings(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
) -> Response {
    match browser_identity(&service, &super::passkeys::request(&headers)).await {
        Ok(BrowserIdentity::Authenticated { .. }) => {
            super::rendering::redirect("/profile#security")
        }
        _ => super::rendering::redirect("/login"),
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
            let body = response.body.json().ok().flatten().unwrap_or_default();
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
