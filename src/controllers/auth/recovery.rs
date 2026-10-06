use super::*;

#[derive(Deserialize)]
pub(super) struct Recovery {
    #[serde(rename = "recovery-code")]
    code: String,
    authenticity_token: String,
}

fn form(
    token: &CsrfToken,
    view: &TeraView,
    error: Option<&str>,
    status: StatusCode,
    otp_available: bool,
) -> Response {
    let Ok(authenticity_token) = token.authenticity_token() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let data = serde_json::json!({"title":"Use a recovery code", "allow_palette":false, "appearances":[{"id":"light","label":"Light"},{"id":"dark","label":"Dark"},{"id":"system","label":"System"}], "palettes":[], "authenticity_token":authenticity_token, "error":error, "otp_available":otp_available});
    match format::render().view(view, "auth/recovery.html", data) {
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

pub(super) async fn challenge(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    match browser::recovery_challenge(&ctx.db, &session).await {
        Ok(otp_available) => form(&token, &view, None, StatusCode::OK, otp_available),
        Err(AuthenticationError::Unauthenticated) => redirect("/login"),
        Err(AuthenticationError::Forbidden | AuthenticationError::InsufficientScope { .. }) => {
            StatusCode::FORBIDDEN.into_response()
        }
        Err(AuthenticationError::Unavailable) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

pub(super) async fn verify(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(input): AxumForm<Recovery>,
) -> Response {
    if token.verify(&input.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    match browser::verify_recovery(&ctx.db, &session, &input.code).await {
        Ok(()) => authenticated_redirect(&session),
        Err(AuthenticationError::Unauthenticated) if !browser::has_pending_challenge(&session) => {
            redirect("/login")
        }
        Err(AuthenticationError::Unauthenticated) => {
            let otp_available = match browser::recovery_challenge(&ctx.db, &session).await {
                Ok(available) => available,
                Err(AuthenticationError::Unavailable) => {
                    return StatusCode::SERVICE_UNAVAILABLE.into_response();
                }
                Err(_) => false,
            };
            if !browser::has_pending_challenge(&session) {
                return redirect("/login");
            }
            form(
                &token,
                &view,
                Some("Invalid recovery code"),
                StatusCode::UNAUTHORIZED,
                otp_available,
            )
        }
        Err(AuthenticationError::Forbidden | AuthenticationError::InsufficientScope { .. }) => {
            StatusCode::FORBIDDEN.into_response()
        }
        Err(AuthenticationError::Unavailable) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
