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
    let invited = invitation.is_some();
    let mut data = if let Some(invitation) = invitation {
        let language = form
            .lang
            .as_deref()
            .and_then(crate::controllers::invitations::supported_language)
            .unwrap_or_else(super::security_locale::current_language);
        let Some(data) = crate::controllers::invitations::acceptance_data(
            language,
            form.lang.is_some(),
            form.invitation_token.as_deref().unwrap_or_default(),
            invitation,
            false,
            &authenticity_token,
        ) else {
            return unavailable();
        };
        data
    } else {
        let mut data = crate::controllers::medications::rendering::appearance_context();
        data["title"] = json!("Create Account");
        data
    };
    data["name"] = json!(form.name);
    data["date_of_birth"] = json!(form.date_of_birth);
    data["email"] = json!(form.email);
    data["errors"] = json!(errors);
    data["authenticity_token"] = json!(authenticity_token);
    (
        token,
        render(
            view,
            if invited {
                "invitations/accept.html"
            } else {
                "identity_onboarding/create.html"
            },
            data,
            status,
        ),
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
