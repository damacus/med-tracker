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

pub fn routes() -> Routes {
    Routes::new()
        .add("/login", get(login).post(sign_in))
        .add("/logout", post(sign_out))
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
        Ok(()) => {
            let destination = session
                .get::<crate::models::identity::authorization::AuthorizationInput>("oauth_pending")
                .and_then(|input| serde_urlencoded::to_string(input.0).ok())
                .map_or_else(|| "/".to_owned(), |query| format!("/authorize?{query}"));
            (
                StatusCode::SEE_OTHER,
                [
                    (header::LOCATION, destination.as_str()),
                    (header::CACHE_CONTROL, "no-store"),
                ],
            )
                .into_response()
        }
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
