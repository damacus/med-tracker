use crate::models::identity::better_auth::{
    BrowserIdentity, IdentityService, browser_identity, dispatch,
};
use axum::{
    Extension,
    extract::Form,
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use axum_csrf::CsrfToken;
use axum_session::Session;
use axum_session_sqlx::SessionPgPool;
use loco_rs::prelude::*;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub(super) struct Input {
    authenticity_token: String,
    password: Option<String>,
    code: Option<String>,
    backup: Option<bool>,
    saved: Option<String>,
}

fn render(
    view: &TeraView,
    csrf: CsrfToken,
    mode: &str,
    setup: Option<&serde_json::Value>,
    error: Option<&str>,
) -> Response {
    let Ok(token) = csrf.authenticity_token() else {
        return super::rendering::unavailable();
    };
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = json!(match mode {
        "login" => "Verify your identity",
        "disable" => "Disable authenticator app",
        "backup" | "regenerate" => "Replace backup codes",
        _ => "Enable authenticator app",
    });
    data["mode"] = json!(mode);
    data["error"] = json!(error);
    data["authenticity_token"] = json!(token);
    let uri = setup
        .and_then(|body| body["totpURI"].as_str())
        .and_then(|value| url::Url::parse(value).ok());
    data["secret"] = json!(uri.as_ref().and_then(|uri| {
        uri.query_pairs()
            .find(|(key, _)| key == "secret")
            .map(|(_, value)| value.into_owned())
    }));
    data["backup_codes"] = setup
        .and_then(|body| body.get("backupCodes"))
        .cloned()
        .unwrap_or_else(|| json!([]));
    match super::security_locale::view(view, "identity_onboarding/totp.html", data) {
        Ok(response) => (csrf, [(header::CACHE_CONTROL, "no-store")], response).into_response(),
        Err(_) => super::rendering::unavailable(),
    }
}

fn cookies(mut response: Response, native: &better_auth_core::AuthResponse) -> Response {
    for value in native.headers.get_all("set-cookie") {
        let Ok(value) = value.parse() else {
            return super::rendering::unavailable();
        };
        response.headers_mut().append(header::SET_COOKIE, value);
    }
    response
}

async fn execute(
    service: &IdentityService,
    headers: &HeaderMap,
    path: &str,
    body: serde_json::Value,
) -> better_auth_core::AuthResult<better_auth_core::AuthResponse> {
    let mut request = super::passkeys::request(headers);
    request.method = better_auth_core::HttpMethod::Post;
    request.path = path.into();
    request
        .headers
        .insert("content-type".into(), "application/json".into());
    request.body = Some(serde_json::to_vec(&body)?);
    dispatch(service, request, uuid::Uuid::new_v4().to_string()).await
}

pub(super) async fn show(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    csrf: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    if !matches!(
        browser_identity(&service, &super::passkeys::request(&headers)).await,
        Ok(BrowserIdentity::Authenticated { .. })
    ) {
        return super::rendering::redirect("/login");
    }
    render(&view, csrf, "start", None, None)
}
pub(super) async fn login_form(
    csrf: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    render(&view, csrf, "login", None, None)
}

pub(super) async fn start(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    csrf: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(input): Form<Input>,
) -> Response {
    if csrf.verify(&input.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    match execute(
        &service,
        &headers,
        "/security/totp/start",
        json!({"password":input.password.unwrap_or_default()}),
    )
    .await
    {
        Ok(native) if native.status < 400 => {
            let Ok(Some(body)) = native.body.json() else {
                return super::rendering::unavailable();
            };
            cookies(render(&view, csrf, "finish", Some(&body), None), &native)
        }
        _ => render(
            &view,
            csrf,
            "start",
            None,
            Some("Authentication failed. Check your current password."),
        ),
    }
}

pub(super) async fn finish(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    csrf: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(input): Form<Input>,
) -> Response {
    if csrf.verify(&input.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    if input.saved.as_deref() != Some("yes") {
        return render(
            &view,
            csrf,
            "finish",
            None,
            Some(
                "Save your backup codes before continuing. Restart setup to generate a new set if necessary.",
            ),
        );
    }
    match execute(
        &service,
        &headers,
        "/security/totp/finish",
        json!({"code":input.code.unwrap_or_default()}),
    )
    .await
    {
        Ok(native) if native.status < 400 => {
            cookies(super::rendering::redirect("/account/security"), &native)
        }
        Ok(native) => cookies(
            render(
                &view,
                csrf,
                "finish",
                None,
                Some("Invalid authentication code. Check your app and retry."),
            ),
            &native,
        ),
        Err(_) => render(
            &view,
            csrf,
            "finish",
            None,
            Some("Invalid authentication code. Check your app and retry."),
        ),
    }
}

pub(super) async fn login(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    session: Session<SessionPgPool>,
    csrf: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(input): Form<Input>,
) -> Response {
    if csrf.verify(&input.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    match execute(
        &service,
        &headers,
        "/security/totp/login",
        json!({"code":input.code.unwrap_or_default(),"backup":input.backup.unwrap_or(false)}),
    )
    .await
    {
        Ok(native) if native.status < 400 => cookies(
            crate::controllers::auth::authenticated_redirect(&session),
            &native,
        ),
        Ok(native) => cookies(
            render(
                &view,
                csrf,
                "login",
                None,
                Some(
                    "The code is invalid or expired. Try a fresh code or another unused backup code.",
                ),
            ),
            &native,
        ),
        Err(_) => render(
            &view,
            csrf,
            "login",
            None,
            Some("The code is invalid or expired. Try a fresh code or another unused backup code."),
        ),
    }
}

pub(super) async fn disable_start(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    csrf: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(input): Form<Input>,
) -> Response {
    if csrf.verify(&input.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    match browser_identity(&service, &super::passkeys::request(&headers)).await {
        Ok(BrowserIdentity::Authenticated { user, .. })
            if user.two_factor_enabled.typed().ok().copied().flatten() == Some(true) =>
        {
            render(&view, csrf, "disable", None, None)
        }
        _ => super::rendering::redirect("/account/security"),
    }
}

pub(super) async fn disable_confirm(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    csrf: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(input): Form<Input>,
) -> Response {
    if csrf.verify(&input.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    match execute(
        &service,
        &headers,
        "/security/totp/disable/confirm",
        json!({"password":input.password.unwrap_or_default()}),
    )
    .await
    {
        Ok(native) if native.status < 400 => {
            cookies(super::rendering::redirect("/account/security"), &native)
        }
        _ => render(
            &view,
            csrf,
            "disable",
            None,
            Some("Authentication failed. Check your current password."),
        ),
    }
}

pub(super) async fn regenerate(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    csrf: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(input): Form<Input>,
) -> Response {
    if csrf.verify(&input.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    match execute(
        &service,
        &headers,
        "/two-factor/generate-backup-codes",
        json!({"password":input.password.unwrap_or_default()}),
    )
    .await
    {
        Ok(native) if native.status < 400 => {
            let Ok(Some(body)) = native.body.json() else {
                return super::rendering::unavailable();
            };
            cookies(render(&view, csrf, "backup", Some(&body), None), &native)
        }
        _ => render(
            &view,
            csrf,
            "regenerate",
            None,
            Some("Backup codes could not be replaced. Check your current password."),
        ),
    }
}
