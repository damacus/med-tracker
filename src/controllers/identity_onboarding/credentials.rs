use axum::{Extension, extract::{Form, Query}, http::{HeaderMap, StatusCode, header}, response::IntoResponse};
use axum_csrf::CsrfToken;
use loco_rs::prelude::*;
use serde::Deserialize;
use serde_json::json;
use crate::models::identity::better_auth::{IdentityService, dispatch};

#[derive(Deserialize)]
pub(super) struct Start { action: String, target: Option<String>, authenticity_token: String }
#[derive(Deserialize)]
pub(super) struct Link { operation_id: String }

pub(super) async fn start(Extension(service): Extension<IdentityService>, headers: HeaderMap, request_id: Option<Extension<loco_rs::controller::middleware::request_id::LocoRequestId>>, csrf: CsrfToken, ViewEngine(view): ViewEngine<TeraView>, Form(form): Form<Start>) -> Response {
    if csrf.verify(&form.authenticity_token).is_err() { return StatusCode::FORBIDDEN.into_response(); }
    let mut request = super::passkeys::request(&headers);
    request.method = better_auth_core::HttpMethod::Post;
    request.path = "/security/operation/start".into();
    request.headers.insert("content-type".into(), "application/json".into());
    let label = match form.action.as_str() { "add_passkey" => "adding a passkey", "remove_password" => "removing your password", "remove_passkey" => "removing this passkey", "regenerate_recovery" => "replacing your recovery codes", _ => "account change" };
    let mut body = json!({"action":form.action});
    if let Some(target) = form.target { body["target"] = json!(target); }
    request.body = serde_json::to_vec(&body).ok();
    match dispatch(&service, request, crate::controllers::medications::request_id(request_id)).await {
        Ok(response) if response.status == 200 => {
            let Ok(body) = serde_json::from_slice::<serde_json::Value>(&response.body) else { return super::rendering::unavailable(); };
            let Some(id) = body["operation_id"].as_str() else { return super::rendering::unavailable(); };
            super::password::render_named(&view, csrf, Some(id), None, StatusCode::OK, (body["password_proof"].as_bool() == Some(true), body["passkey_proof"].as_bool() == Some(true)), label)
        }
        _ => (StatusCode::BAD_REQUEST, "This change is unavailable. Keep a local sign-in method and explicitly disable your authenticator app before removing a password.").into_response(),
    }
}

pub(super) async fn passkey(Query(link): Query<Link>, ViewEngine(view): ViewEngine<TeraView>) -> Response {
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = json!("Add passkey");
    data["operation_id"] = json!(link.operation_id);
    match format::render().view(&view, "identity_onboarding/add_passkey.html", data) {
        Ok(response) => ([(header::CACHE_CONTROL, "no-store")], response).into_response(),
        Err(_) => super::rendering::unavailable(),
    }
}

pub(super) fn recovery_codes(view: &TeraView, body: &serde_json::Value) -> Response {
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = json!("Save your recovery codes");
    data["recovery_codes"] = body["recoveryCodes"].clone();
    data["generation"] = body["generation"].clone();
    match format::render().view(view, "identity_onboarding/regenerated.html", data) {
        Ok(response) => ([(header::CACHE_CONTROL, "no-store")], response).into_response(),
        Err(_) => super::rendering::unavailable(),
    }
}
