use crate::models::identity::{browser, resource::AuthenticationError};
use axum::{
    Extension,
    extract::{Form, FromRequestParts},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use axum_csrf::CsrfToken;
use axum_session::Session;
use axum_session_sqlx::SessionPgPool;
use loco_rs::prelude::*;

pub async fn index(
    State(ctx): State<AppContext>,
    session: std::result::Result<
        Session<SessionPgPool>,
        <Session<SessionPgPool> as FromRequestParts<AppContext>>::Rejection,
    >,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    let mut households = Vec::new();
    let mut signed_in = false;
    let mut auth_notice = None;
    if let Ok(session) = session {
        auth_notice = session.get::<String>("auth_notice");
        session.remove("auth_notice");
        match browser::authenticate(&ctx.db, &session, ctx.config.settings.as_ref()).await {
            Ok(principal) => match principal.households(&ctx.db).await {
                Ok(value) => {
                    households = value;
                    signed_in = true;
                }
                Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
            },
            Err(AuthenticationError::Unauthenticated) => {}
            Err(AuthenticationError::Forbidden | AuthenticationError::InsufficientScope { .. }) => {
                return StatusCode::FORBIDDEN.into_response();
            }
            Err(AuthenticationError::Unavailable) => {
                return StatusCode::SERVICE_UNAVAILABLE.into_response();
            }
        }
    }
    let Ok(authenticity_token) = token.authenticity_token() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    match format::render().view(
        &view,
        "home/index.html",
        crate::views::template_data(&data!({
            "title": "Welcome",
            "allow_palette": signed_in,
            "signed_in": signed_in,
            "auth_notice": auth_notice,
            "households": households,
            "authenticity_token": authenticity_token,
            "appearances": [
                {"id": "light", "label": "Light"},
                {"id": "dark", "label": "Dark"},
                {"id": "system", "label": "System"}
            ],
            "palettes": [
                {"id": "default", "label": "Command Centre"},
                {"id": "serene-sage", "label": "Serene Sage"},
                {"id": "modern-clinical", "label": "Modern Clinical"},
                {"id": "warm-earth", "label": "Warm Earth"},
                {"id": "deep-lavender", "label": "Deep Lavender"},
                {"id": "forest-care", "label": "Forest Care"},
                {"id": "sunset-support", "label": "Sunset Support"},
                {"id": "tech-indigo", "label": "Tech Indigo"},
                {"id": "soft-rose", "label": "Soft Rose"},
                {"id": "minty-fresh", "label": "Minty Fresh"}
            ]
        })),
    ) {
        Ok(response) => (token, [(header::CACHE_CONTROL, "no-store")], response).into_response(),
        Err(error) => error.into_response(),
    }
}

#[derive(serde::Deserialize)]
pub struct Selection {
    household_id: String,
    authenticity_token: String,
}

pub async fn select(
    State(ctx): State<AppContext>,
    Extension(service): Extension<crate::models::identity::better_auth::IdentityService>,
    session: Session<SessionPgPool>,
    headers: HeaderMap,
    request_id: Option<Extension<loco_rs::controller::middleware::request_id::LocoRequestId>>,
    token: CsrfToken,
    Form(form): Form<Selection>,
) -> Response {
    if token.verify(&form.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let principal =
        match browser::authenticate(&ctx.db, &session, ctx.config.settings.as_ref()).await {
            Ok(principal) => principal,
            Err(AuthenticationError::Unauthenticated) => {
                return StatusCode::UNAUTHORIZED.into_response();
            }
            Err(AuthenticationError::Forbidden | AuthenticationError::InsufficientScope { .. }) => {
                return StatusCode::FORBIDDEN.into_response();
            }
            Err(AuthenticationError::Unavailable) => {
                return StatusCode::SERVICE_UNAVAILABLE.into_response();
            }
        };
    let households = match principal.households(&ctx.db).await {
        Ok(households) => households,
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    let Some(household) = households
        .iter()
        .find(|household| household.id.to_string() == form.household_id)
    else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let mut request = better_auth_core::AuthRequest::new(
        better_auth_core::HttpMethod::Post,
        "/organization/set-active",
    );
    request.headers = headers
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.as_str().to_owned(), value.to_owned()))
        })
        .collect();
    request
        .headers
        .insert("content-type".into(), "application/json".into());
    request.body =
        serde_json::to_vec(&serde_json::json!({"organizationId":form.household_id})).ok();
    match crate::models::identity::better_auth::dispatch(
        &service,
        request,
        crate::controllers::medications::request_id(request_id),
    )
    .await
    {
        Ok(response) if response.status == 200 => (
            StatusCode::SEE_OTHER,
            [
                (
                    header::LOCATION,
                    format!("/households/{}/medications", household.slug),
                ),
                (header::CACHE_CONTROL, "no-store".to_owned()),
            ],
        )
            .into_response(),
        Ok(response) => StatusCode::from_u16(response.status)
            .unwrap_or(StatusCode::SERVICE_UNAVAILABLE)
            .into_response(),
        Err(error) => StatusCode::from_u16(error.status_code())
            .unwrap_or(StatusCode::SERVICE_UNAVAILABLE)
            .into_response(),
    }
}
