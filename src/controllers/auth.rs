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

async fn login(
    State(ctx): State<AppContext>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    login_form(&ctx.db, &token, &view, None, StatusCode::OK).await
}

async fn login_form(
    db: &sea_orm::DatabaseConnection,
    token: &CsrfToken,
    view: &TeraView,
    error: Option<&str>,
    status: StatusCode,
) -> Response {
    let registration_open = match crate::models::identity::signup::registration_open(db).await {
        Ok(value) => value,
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    let Ok(authenticity_token) = token.authenticity_token() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let data = serde_json::json!({ "title":"Sign in", "allow_palette":false, "appearances":[{"id":"light","label":"Light"},{"id":"dark","label":"Dark"},{"id":"system","label":"System"}], "palettes":[], "authenticity_token":authenticity_token, "error":error, "registration_open":registration_open });
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
    axum::Extension(service): axum::Extension<crate::models::identity::better_auth::IdentityService>,
    headers: axum::http::HeaderMap,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(form): AxumForm<Login>,
) -> Response {
    if token.verify(&form.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let mut request = better_auth_core::AuthRequest::new(better_auth_core::HttpMethod::Post, "/sign-in/email");
    request.headers = headers.iter().filter_map(|(name, value)| value.to_str().ok().map(|value| (name.as_str().to_owned(), value.to_owned()))).collect();
    request.body = serde_json::to_vec(&serde_json::json!({"email":form.email,"password":form.password})).ok();
    let result = crate::models::identity::better_auth::dispatch(&service, request, uuid::Uuid::new_v4().to_string()).await;
    if let Ok(ref response) = result && response.status < 400 {
        let requires_totp = serde_json::from_slice::<serde_json::Value>(&response.body).ok().is_some_and(|body| body.get("twoFactorRedirect").and_then(serde_json::Value::as_bool) == Some(true));
        let mut redirect = if requires_totp { redirect("/auth/totp") } else { authenticated_redirect(&session) };
        for cookie in response.headers.get_all("set-cookie") {
            let Ok(value) = cookie.parse() else { return StatusCode::SERVICE_UNAVAILABLE.into_response(); };
            redirect.headers_mut().append(header::SET_COOKIE, value);
        }
        return redirect;
    }
    let outcome: Result<browser::SignInOutcome, AuthenticationError> = Err(if result.as_ref().is_ok_and(|response| response.status < 500) || matches!(result, Err(better_auth_core::AuthError::InvalidCredentials | better_auth_core::AuthError::Unauthenticated)) { AuthenticationError::Unauthenticated } else { AuthenticationError::Unavailable });
    match outcome {
        Ok(browser::SignInOutcome::Authenticated) => authenticated_redirect(&session),
        Ok(browser::SignInOutcome::OtpRequired) => redirect("/otp-auth"),
        Ok(browser::SignInOutcome::RecoveryRequired) => redirect("/recovery-auth"),
        Ok(browser::SignInOutcome::PasskeyRequired) => redirect("/webauthn-auth"),
        Err(AuthenticationError::Unauthenticated) => {
            login_form(
                &ctx.db,
                &token,
                &view,
                Some("Invalid email or password"),
                StatusCode::UNAUTHORIZED,
            )
            .await
        }
        Err(AuthenticationError::Forbidden | AuthenticationError::InsufficientScope { .. }) => {
            login_form(
                &ctx.db,
                &token,
                &view,
                Some("An additional sign-in factor is required."),
                StatusCode::FORBIDDEN,
            )
            .await
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

pub(crate) fn store_invitation_continuation(session: &Session<SessionPgPool>, token: &str) {
    session.set("invitation_pending", token.to_owned());
    session.set_store(true);
}

pub(crate) fn authenticated_redirect(session: &Session<SessionPgPool>) -> Response {
    if let Some(query) = session
        .get::<crate::models::identity::authorization::AuthorizationInput>("oauth_pending")
        .and_then(|input| serde_urlencoded::to_string(input.0).ok())
    {
        return redirect(&format!("/authorize?{query}"));
    }
    if let Some(token) = session.get::<String>("invitation_pending") {
        session.remove("invitation_pending");
        if let Ok(query) = serde_urlencoded::to_string([("token", token)]) {
            return redirect(&format!("/invitations/accept?{query}"));
        }
    }
    redirect("/")
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
    axum::Extension(service): axum::Extension<crate::models::identity::better_auth::IdentityService>,
    headers: axum::http::HeaderMap,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    AxumForm(form): AxumForm<Logout>,
) -> Response {
    if token.verify(&form.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let mut request = better_auth_core::AuthRequest::new(better_auth_core::HttpMethod::Post, "/sign-out");
    request.headers = headers.iter().filter_map(|(name, value)| value.to_str().ok().map(|value| (name.as_str().to_owned(), value.to_owned()))).collect();
    request.body = Some(b"{}".to_vec());
    request.headers.insert("content-type".into(), "application/json".into());
    match crate::models::identity::better_auth::dispatch(&service, request, uuid::Uuid::new_v4().to_string()).await {
        Ok(response) if response.status < 400 => {
            session.destroy();
            let mut redirect = redirect("/login");
            for cookie in response.headers.get_all("set-cookie") {
                let Ok(value) = cookie.parse() else { return StatusCode::SERVICE_UNAVAILABLE.into_response(); };
                redirect.headers_mut().append(header::SET_COOKIE, value);
            }
            redirect
        }
        Ok(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
