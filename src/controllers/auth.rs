use axum::{
    extract::Form as AxumForm,
    http::{StatusCode, header},
    response::IntoResponse,
};
use axum_csrf::CsrfToken;
use axum_session::Session;
use axum_session_sqlx::SessionPgPool;
use loco_rs::prelude::*;
use serde::Deserialize;

use crate::models::identity::{browser, resource::AuthenticationError};

mod passkeys;
mod recovery;

#[derive(Deserialize)]
struct Login {
    email: String,
    password: String,
    authenticity_token: String,
}

#[derive(Deserialize)]
struct Logout {
    authenticity_token: String,
}

#[derive(Deserialize)]
struct Otp {
    otp: String,
    authenticity_token: String,
}

pub fn routes() -> Routes {
    Routes::new()
        .add("/multifactor-manage", get(passkeys::settings))
        .add(
            "/webauthn-setup",
            get(passkeys::setup).post(passkeys::register),
        )
        .add(
            "/webauthn-remove",
            get(passkeys::confirm_remove).post(passkeys::remove),
        )
        .add("/webauthn-login/options", get(passkeys::options))
        .add("/webauthn-login", post(passkeys::login))
        .add(
            "/webauthn-auth",
            get(passkeys::factor).post(passkeys::verify_factor),
        )
        .add("/login", get(login).post(sign_in))
        .add("/logout", post(sign_out))
        .add("/otp-auth", get(otp_challenge).post(verify_otp))
        .add(
            "/recovery-auth",
            get(recovery::challenge).post(recovery::verify),
        )
}

async fn login(token: CsrfToken, ViewEngine(view): ViewEngine<TeraView>) -> Response {
    login_form(&token, &view, None, StatusCode::OK)
}

fn login_form(
    token: &CsrfToken,
    view: &TeraView,
    error: Option<&str>,
    status: StatusCode,
) -> Response {
    let Ok(authenticity_token) = token.authenticity_token() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let data = serde_json::json!({ "title":"Sign in", "allow_palette":false, "appearances":[{"id":"light","label":"Light"},{"id":"dark","label":"Dark"},{"id":"system","label":"System"}], "palettes":[], "authenticity_token":authenticity_token, "error":error });
    match format::render().view(view, "auth/login.html", data) {
        Ok(response) => (
            status,
            token.clone(),
            [(header::CACHE_CONTROL, "no-store")],
            response,
        )
            .into_response(),
        Err(error) => error.into_response(),
    }
}

async fn sign_in(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(form): AxumForm<Login>,
) -> Response {
    if token.verify(&form.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    match browser::sign_in(&ctx.db, &session, form.email, form.password).await {
        Ok(browser::SignInOutcome::Authenticated) => authenticated_redirect(&session),
        Ok(browser::SignInOutcome::OtpRequired) => redirect("/otp-auth"),
        Ok(browser::SignInOutcome::RecoveryRequired) => redirect("/recovery-auth"),
        Ok(browser::SignInOutcome::PasskeyRequired) => redirect("/webauthn-auth"),
        Err(AuthenticationError::Unauthenticated) => login_form(
            &token,
            &view,
            Some("Invalid email or password"),
            StatusCode::UNAUTHORIZED,
        ),
        Err(AuthenticationError::Forbidden | AuthenticationError::InsufficientScope { .. }) => {
            login_form(
                &token,
                &view,
                Some("An additional sign-in factor is required."),
                StatusCode::FORBIDDEN,
            )
        }
        Err(AuthenticationError::Unavailable) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

fn redirect(destination: &str) -> Response {
    (
        StatusCode::SEE_OTHER,
        [
            (header::LOCATION, destination),
            (header::CACHE_CONTROL, "no-store"),
        ],
    )
        .into_response()
}

fn authenticated_redirect(session: &Session<SessionPgPool>) -> Response {
    let destination = session
        .get::<crate::models::identity::authorization::AuthorizationInput>("oauth_pending")
        .and_then(|input| serde_urlencoded::to_string(input.0).ok())
        .map_or_else(|| "/".to_owned(), |query| format!("/authorize?{query}"));
    redirect(&destination)
}

fn otp_form(
    token: &CsrfToken,
    view: &TeraView,
    error: Option<&str>,
    status: StatusCode,
) -> Response {
    let Ok(authenticity_token) = token.authenticity_token() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let data = serde_json::json!({"title":"Verify your identity", "allow_palette":false, "appearances":[{"id":"light","label":"Light"},{"id":"dark","label":"Dark"},{"id":"system","label":"System"}], "palettes":[], "authenticity_token":authenticity_token, "error":error});
    match format::render().view(view, "auth/otp.html", data) {
        Ok(response) => (
            status,
            token.clone(),
            [(header::CACHE_CONTROL, "no-store")],
            response,
        )
            .into_response(),
        Err(error) => error.into_response(),
    }
}

async fn otp_challenge(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    match browser::challenge(&ctx.db, &session).await {
        Ok(()) => otp_form(&token, &view, None, StatusCode::OK),
        Err(AuthenticationError::Unauthenticated) => redirect("/login"),
        Err(AuthenticationError::Forbidden | AuthenticationError::InsufficientScope { .. }) => {
            otp_form(
                &token,
                &view,
                Some("Authentication codes are unavailable. Sign in using another method."),
                StatusCode::FORBIDDEN,
            )
        }
        Err(AuthenticationError::Unavailable) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

async fn verify_otp(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(form): AxumForm<Otp>,
) -> Response {
    if token.verify(&form.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    match browser::verify_otp(&ctx.db, &session, &form.otp).await {
        Ok(()) => authenticated_redirect(&session),
        Err(AuthenticationError::Unauthenticated) if !browser::has_pending_challenge(&session) => {
            redirect("/login")
        }
        Err(AuthenticationError::Unauthenticated) => otp_form(
            &token,
            &view,
            Some("Invalid or expired authentication code."),
            StatusCode::UNAUTHORIZED,
        ),
        Err(AuthenticationError::Forbidden | AuthenticationError::InsufficientScope { .. }) => {
            otp_form(
                &token,
                &view,
                Some("Authentication codes are unavailable. Sign in using another method."),
                StatusCode::FORBIDDEN,
            )
        }
        Err(AuthenticationError::Unavailable) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

async fn sign_out(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    AxumForm(form): AxumForm<Logout>,
) -> Response {
    if token.verify(&form.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    match browser::sign_out(&ctx.db, &session).await {
        Ok(()) => (
            StatusCode::SEE_OTHER,
            [
                (header::LOCATION, "/login"),
                (header::CACHE_CONTROL, "no-store"),
            ],
        )
            .into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
