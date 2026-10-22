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

use crate::models::identity::better_auth::IdentityService;

#[derive(Deserialize)]
pub(super) struct Link {
    token: String,
    #[serde(rename = "callbackURL")]
    callback_url: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct Confirm {
    token: String,
    #[serde(rename = "callbackURL")]
    callback_url: Option<String>,
    authenticity_token: String,
}

pub(super) async fn show(
    Query(link): Query<Link>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    if link.token.len() > 4096 {
        return super::rendering::redirect("/login");
    }
    let Ok(authenticity_token) = token.authenticity_token() else {
        return super::rendering::unavailable();
    };
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = json!("Verify your account");
    data["token"] = json!(link.token);
    data["callback_url"] = json!(link.callback_url);
    data["authenticity_token"] = json!(authenticity_token);
    match format::render().view(&view, "identity_onboarding/confirm_email.html", data) {
        Ok(response) => (
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

pub(super) async fn confirm(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    token: CsrfToken,
    Form(form): Form<Confirm>,
) -> Response {
    if token.verify(&form.authenticity_token).is_err() || form.token.len() > 4096 {
        return StatusCode::FORBIDDEN.into_response();
    }
    let mut request = super::passkeys::request(&headers);
    request.method = better_auth_core::HttpMethod::Post;
    request.path = "/onboarding/confirm-email".into();
    request
        .headers
        .insert("content-type".into(), "application/json".into());
    request.body = serde_json::to_vec(&json!({"token":form.token,"confirmed":true})).ok();
    let mut native_query = json!({"token":form.token});
    if let Some(callback_url) = form.callback_url.filter(|url| !url.is_empty()) {
        native_query["callbackURL"] = json!(callback_url);
    }
    match crate::models::identity::better_auth::trusted(
        &service,
        &request,
        uuid::Uuid::new_v4().to_string(),
        async {
            service
                .call_endpoint(
                    better_auth_core::HttpMethod::Get,
                    "/verify-email",
                    better_auth::server_api::EndpointInput {
                        headers: Some(request.headers.clone()),
                        request: Some(request.clone()),
                        query: Some(native_query),
                        ..Default::default()
                    },
                )
                .await
        },
    )
    .await
    {
        Ok(response) if response.status < 400 => {
            better_auth_core::error::materialize_axum_response(response)
                .unwrap_or_else(|_| super::rendering::unavailable())
        }
        Ok(response) => {
            tracing::warn!(
                status = response.status,
                "Account confirmation rejected by framework"
            );
            super::rendering::redirect("/login")
        }
        Err(error) => {
            tracing::warn!(status = ?error.status_code(), error = %error, "Account confirmation failed");
            super::rendering::redirect("/login")
        }
    }
}
