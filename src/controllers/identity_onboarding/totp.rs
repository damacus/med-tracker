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
use loco_rs::{controller::middleware::request_id::LocoRequestId, prelude::*};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub(super) struct Input {
    authenticity_token: String,
    password: Option<String>,
    code: Option<String>,
    operation_id: Option<String>,
}

fn render(
    view: &TeraView,
    token: CsrfToken,
    mode: &str,
    operation: Option<&str>,
    secret: Option<&str>,
    error: Option<&str>,
) -> Response {
    let Ok(authenticity_token) = token.authenticity_token() else {
        return super::rendering::unavailable();
    };
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = json!(match mode {
        "login" => "Verify your identity",
        "disable" => "Disable authenticator app",
        _ => "Enable authenticator app",
    });
    data["mode"] = json!(mode);
    data["operation_id"] = json!(operation);
    data["secret"] = json!(secret);
    data["error"] = json!(error);
    data["authenticity_token"] = json!(authenticity_token);
    match format::render().view(view, "identity_onboarding/totp.html", data) {
        Ok(response) => (token, [(header::CACHE_CONTROL, "no-store")], response).into_response(),
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
    render(&view, token, "start", None, None, None)
}

pub(super) async fn login_form(
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    render(&view, token, "login", None, None, None)
}

async fn execute(
    service: &IdentityService,
    headers: &HeaderMap,
    path: &str,
    data: serde_json::Value,
    request_id: Option<Extension<LocoRequestId>>,
) -> better_auth_core::AuthResult<better_auth_core::AuthResponse> {
    let mut request = super::passkeys::request(headers);
    request.method = better_auth_core::HttpMethod::Post;
    request.path = path.into();
    request
        .headers
        .insert("content-type".into(), "application/json".into());
    request.body = serde_json::to_vec(&data).ok();
    dispatch(
        service,
        request,
        crate::controllers::medications::request_id(request_id),
    )
    .await
}

fn with_cookies(mut response: Response, result: &better_auth_core::AuthResponse) -> Response {
    for cookie in result.headers.get_all("set-cookie") {
        let Ok(value) = cookie.parse() else {
            return super::rendering::unavailable();
        };
        response.headers_mut().append(header::SET_COOKIE, value);
    }
    response
}

pub(super) async fn start(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    request_id: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(input): Form<Input>,
) -> Response {
    if token.verify(&input.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    if let Ok(result) = execute(
        &service,
        &headers,
        "/security/totp/start",
        json!({"password":input.password.unwrap_or_default()}),
        request_id,
    )
    .await
        && result.status == 200
    {
        let Ok(body) = serde_json::from_slice::<serde_json::Value>(&result.body) else {
            return super::rendering::unavailable();
        };
        let Some(id) = body.get("operation_id").and_then(serde_json::Value::as_str) else {
            return super::rendering::unavailable();
        };
        let Some(uri) = body
            .get("totpURI")
            .and_then(serde_json::Value::as_str)
            .and_then(|uri| url::Url::parse(uri).ok())
        else {
            return super::rendering::unavailable();
        };
        let secret = uri
            .query_pairs()
            .find(|(key, _)| key == "secret")
            .map(|(_, value)| value.into_owned());
        return render(&view, token, "finish", Some(id), secret.as_deref(), None);
    }
    render(
        &view,
        token,
        "start",
        None,
        None,
        Some("Authentication failed. Check your current password."),
    )
}

pub(super) async fn finish(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    request_id: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(input): Form<Input>,
) -> Response {
    if token.verify(&input.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    match execute(
        &service,
        &headers,
        "/security/totp/finish",
        json!({"code":input.code.unwrap_or_default(),"operation_id":input.operation_id}),
        request_id,
    )
    .await
    {
        Ok(result) if result.status == 200 => {
            with_cookies(super::rendering::redirect("/account/security"), &result)
        }
        Ok(result) => with_cookies(
            render(
                &view,
                token,
                "finish",
                input.operation_id.as_deref(),
                None,
                Some("Invalid authentication code. Check your app and retry."),
            ),
            &result,
        ),
        Err(_) => render(
            &view,
            token,
            "finish",
            input.operation_id.as_deref(),
            None,
            Some("Invalid authentication code. Check your app and retry."),
        ),
    }
}

pub(super) async fn login(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    request_id: Option<Extension<LocoRequestId>>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(input): Form<Input>,
) -> Response {
    if token.verify(&input.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    match execute(
        &service,
        &headers,
        "/security/totp/login",
        json!({"code":input.code.unwrap_or_default()}),
        request_id,
    )
    .await
    {
        Ok(result) if result.status == 200 => with_cookies(
            crate::controllers::auth::authenticated_redirect(&session),
            &result,
        ),
        Ok(result) => {
            let cleared = result
                .headers
                .get_all("set-cookie")
                .any(|cookie| cookie.contains("Max-Age=0"));
            let response = if result.status == 400 || cleared {
                super::rendering::redirect("/login")
            } else {
                render(
                    &view,
                    token,
                    "login",
                    None,
                    None,
                    Some("Invalid authentication code. Check your app and retry."),
                )
            };
            with_cookies(response, &result)
        }
        Err(
            better_auth_core::AuthError::Unauthenticated
            | better_auth_core::AuthError::SessionNotFound,
        ) => super::rendering::redirect("/login"),
        Err(_) => render(
            &view,
            token,
            "login",
            None,
            None,
            Some("Authentication is unavailable. Try signing in again."),
        ),
    }
}

pub(super) async fn disable_start(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    request_id: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(input): Form<Input>,
) -> Response {
    if token.verify(&input.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    if let Ok(result) = execute(
        &service,
        &headers,
        "/security/totp/disable/start",
        json!({}),
        request_id,
    )
    .await
        && result.status == 200
    {
        let Ok(body) = serde_json::from_slice::<serde_json::Value>(&result.body) else {
            return super::rendering::unavailable();
        };
        let Some(id) = body.get("operation_id").and_then(serde_json::Value::as_str) else {
            return super::rendering::unavailable();
        };
        return render(&view, token, "disable", Some(id), None, None);
    }
    super::rendering::redirect("/account/security")
}

pub(super) async fn disable_confirm(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    request_id: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(input): Form<Input>,
) -> Response {
    if token.verify(&input.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    if let Ok(result) = execute(&service, &headers, "/security/totp/disable/confirm", json!({"code":input.code.unwrap_or_default(),"password":input.password.unwrap_or_default(),"operation_id":input.operation_id}), request_id).await && result.status == 200 { return with_cookies(super::rendering::redirect("/account/security"), &result); }
    render(
        &view,
        token,
        "disable",
        input.operation_id.as_deref(),
        None,
        Some("Authentication failed. Check your current password and authentication code."),
    )
}
