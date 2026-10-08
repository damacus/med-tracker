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
        .add("/multifactor-manage", get(retired))
        .add("/webauthn-setup", get(retired).post(retired))
        .add("/webauthn-remove", get(retired).post(retired))
        .add("/webauthn-login/options", get(retired))
        .add("/webauthn-login", post(retired))
        .add("/webauthn-auth", get(retired).post(retired))
        .add("/otp-auth", get(retired).post(retired))
        .add("/recovery-auth", get(retired).post(retired))
}

async fn retired() -> Response {
    (
        StatusCode::GONE,
        [(header::CACHE_CONTROL, "no-store")],
        "This sign-in route has been retired. Use /login or /account/security.",
    )
        .into_response()
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
    let data = serde_json::json!({ "title":"Sign in", "allow_palette":false, "appearances":[{"id":"light","label":"Light"},{"id":"dark","label":"Dark"},{"id":"system","label":"System"}], "palettes":[], "authenticity_token":authenticity_token, "error":error, "registration_open":registration_open, "zitadel":std::env::var_os("MEDTRACKER_ZITADEL_ISSUER").is_some() });
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
    (State(ctx), axum::Extension(service)): (
        State<AppContext>,
        axum::Extension<crate::models::identity::better_auth::IdentityService>,
    ),
    headers: axum::http::HeaderMap,
    request_id: Option<axum::Extension<loco_rs::controller::middleware::request_id::LocoRequestId>>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(form): AxumForm<Login>,
) -> Response {
    if token.verify(&form.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let mut request =
        better_auth_core::AuthRequest::new(better_auth_core::HttpMethod::Post, "/sign-in/email");
    request.headers = headers
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.as_str().to_owned(), value.to_owned()))
        })
        .collect();
    request.body =
        serde_json::to_vec(&serde_json::json!({"email":form.email,"password":form.password})).ok();
    let result = crate::models::identity::better_auth::dispatch(
        &service,
        request,
        crate::controllers::medications::request_id(request_id),
    )
    .await;
    if let Ok(ref response) = result
        && response.status == 429
    {
        let mut limited = login_form(
            &ctx.db,
            &token,
            &view,
            Some("Too many sign-in attempts. Try again later."),
            StatusCode::TOO_MANY_REQUESTS,
        )
        .await;
        if let Some(value) = response
            .headers
            .get("retry-after")
            .and_then(|value| value.parse().ok())
        {
            limited.headers_mut().insert(header::RETRY_AFTER, value);
        }
        return limited;
    }
    if let Ok(ref response) = result
        && response.status < 400
    {
        let Ok(body) = serde_json::from_slice::<serde_json::Value>(&response.body) else {
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        };
        let issued = if let Some(token) = body.get("token").and_then(serde_json::Value::as_str) {
            match service.context().database.get_session(token).await {
                Ok(Some(session)) => Some(session),
                _ => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
            }
        } else {
            None
        };
        let requires_totp = body
            .get("twoFactorRedirect")
            .and_then(serde_json::Value::as_bool)
            == Some(true)
            || issued.as_ref().is_some_and(|session| {
                session
                    .additional_fields
                    .get("legacy_factor_pending")
                    .and_then(serde_json::Value::as_bool)
                    == Some(true)
            });
        let enrolment = issued.as_ref().is_some_and(|session| {
            session
                .additional_fields
                .get("clinical_session_purpose")
                .and_then(serde_json::Value::as_str)
                == Some("enrolment")
        });
        let mut redirect = if requires_totp {
            redirect("/auth/totp")
        } else if enrolment {
            redirect("/auth/passkey/setup")
        } else {
            authenticated_redirect(&session)
        };
        for cookie in response.headers.get_all("set-cookie") {
            let Ok(value) = cookie.parse() else {
                return StatusCode::SERVICE_UNAVAILABLE.into_response();
            };
            redirect.headers_mut().append(header::SET_COOKIE, value);
        }
        return redirect;
    }
    let outcome: Result<browser::SignInOutcome, AuthenticationError> = Err(
        if result.as_ref().is_ok_and(|response| response.status < 500)
            || matches!(
                result,
                Err(better_auth_core::AuthError::InvalidCredentials
                    | better_auth_core::AuthError::Unauthenticated)
            )
        {
            AuthenticationError::Unauthenticated
        } else {
            AuthenticationError::Unavailable
        },
    );
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

async fn sign_out(
    axum::Extension(service): axum::Extension<
        crate::models::identity::better_auth::IdentityService,
    >,
    headers: axum::http::HeaderMap,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    AxumForm(form): AxumForm<Logout>,
) -> Response {
    if token.verify(&form.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let mut request =
        better_auth_core::AuthRequest::new(better_auth_core::HttpMethod::Post, "/sign-out");
    request.headers = headers
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.as_str().to_owned(), value.to_owned()))
        })
        .collect();
    request.body = Some(b"{}".to_vec());
    request
        .headers
        .insert("content-type".into(), "application/json".into());
    match crate::models::identity::better_auth::dispatch(
        &service,
        request,
        uuid::Uuid::new_v4().to_string(),
    )
    .await
    {
        Ok(response) if response.status < 400 => {
            session.destroy();
            let mut redirect = redirect("/login");
            for cookie in response.headers.get_all("set-cookie") {
                let Ok(value) = cookie.parse() else {
                    return StatusCode::SERVICE_UNAVAILABLE.into_response();
                };
                redirect.headers_mut().append(header::SET_COOKIE, value);
            }
            redirect
        }
        Ok(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
