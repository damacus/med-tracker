use super::*;
use crate::models::identity::browser::passkeys;
use axum::{Extension, extract::Query};
use loco_rs::controller::middleware::request_id::LocoRequestId;

#[derive(Deserialize)]
pub(super) struct Registration {
    webauthn_setup: String,
    nickname: String,
    password: String,
    authenticity_token: String,
}

#[derive(Deserialize)]
pub(super) struct Assertion {
    webauthn_auth: String,
    authenticity_token: String,
}

#[derive(Deserialize)]
pub(super) struct Removal {
    webauthn_remove: String,
    password: String,
    authenticity_token: String,
}

#[derive(Deserialize)]
pub(super) struct Selected {
    webauthn_remove: String,
}

fn page(
    token: &CsrfToken,
    view: &TeraView,
    template: &str,
    mut data: serde_json::Value,
    status: StatusCode,
) -> Response {
    let Ok(csrf) = token.authenticity_token() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    data["authenticity_token"] = csrf.into();
    data["allow_palette"] = false.into();
    data["appearances"] = serde_json::json!([{"id":"light","label":"Light"},{"id":"dark","label":"Dark"},{"id":"system","label":"System"}]);
    data["palettes"] = serde_json::json!([]);
    match format::render().view(view, template, data) {
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

fn denied(error: AuthenticationError) -> Response {
    match error {
        AuthenticationError::Unauthenticated => redirect("/login"),
        AuthenticationError::Forbidden | AuthenticationError::InsufficientScope { .. } => {
            StatusCode::FORBIDDEN.into_response()
        }
        AuthenticationError::Unavailable => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

pub(super) async fn settings(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    match passkeys::list(&ctx.db, &session).await {
        Ok(keys) => page(
            &token,
            &view,
            "auth/security.html",
            serde_json::json!({"title":"Security settings","passkeys":keys}),
            StatusCode::OK,
        ),
        Err(error) => denied(error),
    }
}

pub(super) async fn setup(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    setup_page(&ctx, &session, &token, &view, None, StatusCode::OK).await
}

async fn setup_page(
    ctx: &AppContext,
    session: &Session<SessionPgPool>,
    token: &CsrfToken,
    view: &TeraView,
    error: Option<&str>,
    status: StatusCode,
) -> Response {
    match passkeys::start_registration(&ctx.db, session, &ctx.config.server.full_url()).await {
        Ok(options) => page(
            token,
            view,
            "auth/passkey_setup.html",
            serde_json::json!({"title":"Set Up Passkey Authentication","options":options.to_string(),"error":error}),
            status,
        ),
        Err(error) => denied(error),
    }
}

pub(super) async fn register(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    request: Option<Extension<LocoRequestId>>,
    AxumForm(form): AxumForm<Registration>,
) -> Response {
    if token.verify(&form.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let request_id = request.map(|Extension(value)| value.get().to_owned());
    let input = passkeys::RegistrationInput {
        credential: form.webauthn_setup,
        nickname: form.nickname,
        password: form.password,
    };
    match passkeys::finish_registration(
        &ctx.db,
        &session,
        &ctx.config.server.full_url(),
        input,
        request_id.as_deref(),
    )
    .await
    {
        Ok(()) => redirect("/multifactor-manage"),
        Err(AuthenticationError::Unavailable) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        Err(_) => {
            setup_page(
                &ctx,
                &session,
                &token,
                &view,
                Some("Unable to register the passkey. Check your password and try again."),
                StatusCode::UNAUTHORIZED,
            )
            .await
        }
    }
}

pub(super) async fn options(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
) -> Response {
    match passkeys::start_authentication(&ctx.db, &session, &ctx.config.server.full_url()).await {
        Ok(options) => ([(header::CACHE_CONTROL, "no-store")], axum::Json(options)).into_response(),
        Err(error) => denied(error),
    }
}

pub(super) async fn login(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    request: Option<Extension<LocoRequestId>>,
    AxumForm(form): AxumForm<Assertion>,
) -> Response {
    if token.verify(&form.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let request_id = request.map(|Extension(value)| value.get().to_owned());
    match passkeys::finish_authentication(
        &ctx.db,
        &session,
        &ctx.config.server.full_url(),
        &form.webauthn_auth,
        request_id.as_deref(),
    )
    .await
    {
        Ok(()) => authenticated_redirect(&session),
        Err(AuthenticationError::Unavailable) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        Err(_) => {
            login_form(
                &ctx.db,
                &token,
                &view,
                Some("Unable to sign in with this passkey."),
                StatusCode::UNAUTHORIZED,
            )
            .await
        }
    }
}

pub(super) async fn factor(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    factor_page(&ctx, &session, &token, &view, None, StatusCode::OK).await
}

async fn factor_page(
    ctx: &AppContext,
    session: &Session<SessionPgPool>,
    token: &CsrfToken,
    view: &TeraView,
    error: Option<&str>,
    status: StatusCode,
) -> Response {
    match passkeys::start_factor(&ctx.db, session, &ctx.config.server.full_url()).await {
        Ok(options) => page(
            token,
            view,
            "auth/passkey_auth.html",
            serde_json::json!({"title":"Verify your identity", "options":options.to_string(), "error":error}),
            status,
        ),
        Err(AuthenticationError::Forbidden) => {
            match browser::recovery_challenge(&ctx.db, session).await {
                Ok(otp_available) => page(
                    token,
                    view,
                    "auth/passkey_replacement.html",
                    serde_json::json!({"title":"Replace your older passkey", "otp_available":otp_available}),
                    StatusCode::OK,
                ),
                Err(error) => denied(error),
            }
        }
        Err(error) => denied(error),
    }
}

pub(super) async fn verify_factor(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    request: Option<Extension<LocoRequestId>>,
    AxumForm(form): AxumForm<Assertion>,
) -> Response {
    if token.verify(&form.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let request_id = request.map(|Extension(value)| value.get().to_owned());
    match passkeys::finish_factor(
        &ctx.db,
        &session,
        &ctx.config.server.full_url(),
        &form.webauthn_auth,
        request_id.as_deref(),
    )
    .await
    {
        Ok(()) => authenticated_redirect(&session),
        Err(AuthenticationError::Unavailable) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        Err(_) => {
            factor_page(
                &ctx,
                &session,
                &token,
                &view,
                Some("Unable to verify this passkey. Try again."),
                StatusCode::UNAUTHORIZED,
            )
            .await
        }
    }
}

pub(super) async fn confirm_remove(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Query(selected): Query<Selected>,
) -> Response {
    removal_page(
        &ctx,
        &session,
        &token,
        &view,
        &selected.webauthn_remove,
        None,
        StatusCode::OK,
    )
    .await
}

async fn removal_page(
    ctx: &AppContext,
    session: &Session<SessionPgPool>,
    token: &CsrfToken,
    view: &TeraView,
    id: &str,
    error: Option<&str>,
    status: StatusCode,
) -> Response {
    match passkeys::list(&ctx.db, session).await {
        Ok(keys) if keys.iter().any(|key| key.id == id) => page(
            token,
            view,
            "auth/passkey_remove.html",
            serde_json::json!({"title":"Remove Passkey","credential_id":id,"error":error}),
            status,
        ),
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => denied(error),
    }
}

pub(super) async fn remove(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    request: Option<Extension<LocoRequestId>>,
    AxumForm(form): AxumForm<Removal>,
) -> Response {
    if token.verify(&form.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let request_id = request.map(|Extension(value)| value.get().to_owned());
    match passkeys::remove(
        &ctx.db,
        &session,
        &form.webauthn_remove,
        form.password,
        request_id.as_deref(),
    )
    .await
    {
        Ok(()) => redirect("/login"),
        Err(AuthenticationError::Unavailable) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        Err(_) => {
            removal_page(
                &ctx,
                &session,
                &token,
                &view,
                &form.webauthn_remove,
                Some("Unable to remove the passkey. Check your password and try again."),
                StatusCode::UNAUTHORIZED,
            )
            .await
        }
    }
}
