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

use crate::models::identity::better_auth::{
    BrowserIdentity, IdentityService, browser_identity, dispatch, validate_new_password,
};

#[derive(Deserialize)]
pub(super) struct Start {
    new_password: String,
    authenticity_token: String,
}

#[derive(Deserialize)]
pub(super) struct Confirm {
    operation_id: String,
    password: String,
    totp_code: Option<String>,
    authenticity_token: String,
}

pub(super) fn render(
    view: &TeraView,
    token: CsrfToken,
    operation_id: Option<&str>,
    error: Option<&str>,
    status: StatusCode,
    methods: (bool, bool, bool),
) -> Response {
    render_named(
        view,
        token,
        operation_id,
        error,
        status,
        methods,
        "password change",
    )
}

pub(super) fn render_named(
    view: &TeraView,
    token: CsrfToken,
    operation_id: Option<&str>,
    error: Option<&str>,
    status: StatusCode,
    methods: (bool, bool, bool),
    operation_label: &str,
) -> Response {
    let Ok(authenticity_token) = token.authenticity_token() else {
        return super::rendering::unavailable();
    };
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = json!("Change password");
    data["authenticity_token"] = json!(authenticity_token);
    data["operation_id"] = json!(operation_id);
    data["error"] = json!(error);
    data["password_proof"] = json!(methods.0);
    data["passkey_proof"] = json!(methods.1);
    data["oidc_proof"] = json!(methods.2);
    data["operation_label"] = json!(operation_label);
    data["recovery_codes"] = json!([]);
    data["generation"] = json!("");
    match super::security_locale::view(view, "identity_onboarding/password.html", data) {
        Ok(response) => (
            status,
            token,
            [(header::CACHE_CONTROL, "no-store")],
            response,
        )
            .into_response(),
        Err(_) => super::rendering::unavailable(),
    }
}

pub(super) async fn show(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    if !matches!(
        browser_identity(&service, &super::passkeys::request(&headers)).await,
        Ok(BrowserIdentity::Authenticated { .. })
    ) {
        return super::rendering::redirect("/login");
    }
    render(
        &view,
        token,
        None,
        None,
        StatusCode::OK,
        (true, false, false),
    )
}

fn request(
    headers: &HeaderMap,
    path: &str,
    data: serde_json::Value,
) -> better_auth_core::AuthRequest {
    let mut request = super::passkeys::request(headers);
    request.method = better_auth_core::HttpMethod::Post;
    request.path = path.into();
    request
        .headers
        .insert("content-type".into(), "application/json".into());
    request.body = serde_json::to_vec(&data).ok();
    request
}

pub(super) async fn start(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    request_id: Option<Extension<loco_rs::controller::middleware::request_id::LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(form): Form<Start>,
) -> Response {
    if token.verify(&form.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    if let Err(message) = validate_new_password(&form.new_password) {
        return render(
            &view,
            token,
            None,
            Some(message),
            StatusCode::UNPROCESSABLE_ENTITY,
            (true, false, false),
        );
    }
    let request = request(
        &headers,
        "/security/password/start",
        json!({"new_password":form.new_password}),
    );
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
            let Some(id) = body.get("operation_id").and_then(serde_json::Value::as_str) else {
                return super::rendering::unavailable();
            };
            if body["email_proof"].as_bool() == Some(true) {
                return super::password_email::render(
                    &view,
                    token,
                    id,
                    None,
                    None,
                    "Confirm password change",
                );
            }
            render(
                &view,
                token,
                Some(id),
                None,
                StatusCode::OK,
                (
                    body["password_proof"].as_bool() == Some(true),
                    body["passkey_proof"].as_bool() == Some(true),
                    body["oidc_proof"].as_bool() == Some(true),
                ),
            )
        }
        _ => render(
            &view,
            token,
            None,
            Some("Password change is unavailable. Sign in again and retry."),
            StatusCode::BAD_REQUEST,
            (true, false, false),
        ),
    }
}

pub(super) async fn confirm(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    request_id: Option<Extension<loco_rs::controller::middleware::request_id::LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(form): Form<Confirm>,
) -> Response {
    if token.verify(&form.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let request = request(
        &headers,
        "/security/password/confirm",
        json!({"operation_id":form.operation_id,"password":form.password,"totp_code":form.totp_code.filter(|code| !code.is_empty())}),
    );
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
            if body["recoveryCodes"].is_array() {
                return super::credentials::recovery_codes(&view, &body);
            }
            if body["apiKey"].is_string() {
                return super::keys::result(&view, &body);
            }
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
        _ => render(
            &view,
            token,
            Some(&form.operation_id),
            Some("Authentication failed. Check your current sign-in details and retry."),
            StatusCode::UNAUTHORIZED,
            (true, false, false),
        ),
    }
}
