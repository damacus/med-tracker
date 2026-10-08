use axum::{
    http::{StatusCode, header},
    response::IntoResponse,
};
use axum_csrf::CsrfToken;
use loco_rs::prelude::*;
use serde_json::{Value, json};

use super::forms::{Create, Errors};

pub(super) fn account(
    view: &TeraView,
    token: CsrfToken,
    form: &Create,
    invitation: Option<Value>,
    errors: &Errors,
    status: StatusCode,
) -> Response {
    let Ok(authenticity_token) = token.authenticity_token() else {
        return unavailable();
    };
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = json!("Create Account");
    data["name"] = json!(form.name);
    data["date_of_birth"] = json!(form.date_of_birth);
    data["email"] = json!(form.email);
    data["errors"] = json!(errors);
    data["authenticity_token"] = json!(authenticity_token);
    if let Some(invitation) = invitation {
        data["invitation"] = invitation;
        data["invitation_token"] = json!(form.invitation_token);
    }
    (
        token,
        render(view, "identity_onboarding/create.html", data, status),
    )
        .into_response()
}

pub(super) fn sent(view: &TeraView) -> Response {
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = json!("Verify Account");
    render(
        view,
        "identity_onboarding/verification_sent.html",
        data,
        StatusCode::OK,
    )
}

pub(super) fn setup(view: &TeraView, password: bool) -> Response {
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = json!("Set Up Passkey Authentication");
    data["password"] = json!(password);
    render(
        view,
        "identity_onboarding/passkey_setup.html",
        data,
        StatusCode::OK,
    )
}

fn render(view: &TeraView, path: &str, data: Value, status: StatusCode) -> Response {
    match format::render().view(view, path, data) {
        Ok(response) => (status, [(header::CACHE_CONTROL, "no-store")], response).into_response(),
        Err(_) => unavailable(),
    }
}

pub(super) fn redirect(path: &str) -> Response {
    (
        StatusCode::SEE_OTHER,
        [
            (header::LOCATION, path),
            (header::CACHE_CONTROL, "no-store"),
        ],
    )
        .into_response()
}

pub(super) fn unavailable() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        [(header::CACHE_CONTROL, "no-store")],
    )
        .into_response()
}
