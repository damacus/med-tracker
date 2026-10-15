use crate::models::identity::{
    better_auth::{IdentityService, households},
    browser,
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

#[derive(Deserialize)]
pub struct Draft {
    name: String,
    timezone: String,
    authenticity_token: String,
}

fn render(
    view: &TeraView,
    token: CsrfToken,
    name: &str,
    timezone: &str,
    invalid: bool,
) -> Response {
    let Ok(authenticity_token) = token.authenticity_token() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let mut data = super::medications::rendering::appearance_context();
    data["title"] = serde_json::json!("Create household");
    data["name"] = serde_json::json!(name);
    data["timezone"] = serde_json::json!(timezone);
    data["timezones"] = serde_json::json!(
        chrono_tz::TZ_VARIANTS
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    );
    data["authenticity_token"] = serde_json::json!(authenticity_token);
    data["invalid"] = serde_json::json!(invalid);
    match format::render().view(
        view,
        "households/new.html",
        crate::views::template_data(&data),
    ) {
        Ok(response) => (
            if invalid {
                StatusCode::UNPROCESSABLE_ENTITY
            } else {
                StatusCode::OK
            },
            token,
            [(header::CACHE_CONTROL, "no-store")],
            response,
        )
            .into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

pub async fn new(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    match browser::authenticate(&ctx.db, &session, ctx.config.settings.as_ref()).await {
        Ok(principal) => render(&view, token, "", &principal.time_zone().to_string(), false),
        Err(error) => super::medications::authentication_error(error),
    }
}

pub async fn create(
    State(ctx): State<AppContext>,
    Extension(service): Extension<IdentityService>,
    session: Session<SessionPgPool>,
    headers: HeaderMap,
    request_id: Option<Extension<loco_rs::controller::middleware::request_id::LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(draft): Form<Draft>,
) -> Response {
    if token.verify(&draft.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    if let Err(error) = browser::authenticate(&ctx.db, &session, ctx.config.settings.as_ref()).await
    {
        return super::medications::authentication_error(error);
    }
    let Some(input) = households::NewHousehold::parse(&draft.name, &draft.timezone) else {
        return render(&view, token, &draft.name, &draft.timezone, true);
    };
    let mut auth = better_auth_core::AuthRequest::new(
        better_auth_core::HttpMethod::Post,
        "/organization/set-active",
    );
    auth.headers = headers
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.to_string(), value.to_owned()))
        })
        .collect();
    match households::create(
        &service,
        &auth,
        input,
        super::medications::request_id(request_id),
    )
    .await
    {
        Ok(created) => (
            StatusCode::SEE_OTHER,
            [
                (
                    header::LOCATION,
                    format!("/households/{}/medications", created.slug),
                ),
                (header::CACHE_CONTROL, "no-store".to_owned()),
            ],
        )
            .into_response(),
        Err(error) if error.status_code() < 500 => StatusCode::from_u16(error.status_code())
            .unwrap_or(StatusCode::SERVICE_UNAVAILABLE)
            .into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
