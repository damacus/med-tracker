use crate::models::identity::better_auth::{IdentityService, dispatch, personal_keys};
use axum::{
    Extension,
    extract::Form,
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use axum_csrf::CsrfToken;
use loco_rs::prelude::*;
use serde_json::json;
use std::collections::BTreeMap;

pub(super) async fn show(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    request_id: Option<Extension<loco_rs::controller::middleware::request_id::LocoRequestId>>,
    csrf: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    let Ok(listing) = personal_keys::listing(
        &service,
        &super::passkeys::request(&headers),
        crate::controllers::medications::request_id(request_id),
    )
    .await
    else {
        return super::rendering::redirect("/login");
    };
    let Ok(token) = csrf.authenticity_token() else {
        return super::rendering::unavailable();
    };
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = json!("Personal API keys");
    data["authenticity_token"] = json!(token);
    data["keys"] = listing["keys"].clone();
    data["households"] = listing["households"].clone();
    match format::render().view(&view, "identity_onboarding/keys.html", data) {
        Ok(response) => (csrf, [(header::CACHE_CONTROL, "no-store")], response).into_response(),
        Err(_) => super::rendering::unavailable(),
    }
}

pub(super) async fn create(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    request_id: Option<Extension<loco_rs::controller::middleware::request_id::LocoRequestId>>,
    csrf: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(form): Form<BTreeMap<String, String>>,
) -> Response {
    if form
        .get("authenticity_token")
        .is_none_or(|token| csrf.verify(token).is_err())
    {
        return StatusCode::FORBIDDEN.into_response();
    }
    let households = form
        .keys()
        .filter_map(|key| key.strip_prefix("household_"))
        .map(str::parse::<i64>)
        .collect::<std::result::Result<Vec<_>, _>>();
    let Ok(households) = households else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let Some(expires_days) = form
        .get("expires_days")
        .and_then(|value| value.parse::<u16>().ok())
    else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let key = personal_keys::Creation {
        name: form.get("name").cloned().unwrap_or_default(),
        households,
        permissions: form
            .keys()
            .filter_map(|key| key.strip_prefix("permission_").map(str::to_owned))
            .collect(),
        expires_days,
    };
    let mut request = super::passkeys::request(&headers);
    request.path = "/security/operation/start".into();
    request.method = better_auth_core::HttpMethod::Post;
    request
        .headers
        .insert("content-type".into(), "application/json".into());
    request.body = serde_json::to_vec(&json!({"action":"create_api_key","key":key})).ok();
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
                "creating a personal API key",
            )
        }
        _ => (
            StatusCode::BAD_REQUEST,
            "Choose permitted households, explicit permissions and an expiry of 1 to 365 days.",
        )
            .into_response(),
    }
}

#[derive(serde::Deserialize)]
pub(super) struct Revoke {
    key_id: String,
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
    request.path = "/security/api-key/revoke".into();
    request.method = better_auth_core::HttpMethod::Post;
    request
        .headers
        .insert("content-type".into(), "application/json".into());
    request.body = serde_json::to_vec(&json!({"key_id":form.key_id})).ok();
    match dispatch(
        &service,
        request,
        crate::controllers::medications::request_id(request_id),
    )
    .await
    {
        Ok(response) if response.status == 200 => {
            super::rendering::redirect("/account/security/keys")
        }
        _ => StatusCode::UNAUTHORIZED.into_response(),
    }
}

pub(super) fn result(view: &TeraView, body: &serde_json::Value) -> Response {
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = json!("Save your API key");
    data["api_key"] = body["apiKey"].clone();
    data["name"] = body["name"].clone();
    match format::render().view(view, "identity_onboarding/key_result.html", data) {
        Ok(response) => (
            [
                (header::CACHE_CONTROL, "no-store"),
                (header::REFERRER_POLICY, "no-referrer"),
            ],
            response,
        )
            .into_response(),
        Err(_) => super::rendering::unavailable(),
    }
}
