use crate::models::identity::{browser, resource::AuthenticationError};
use axum::{
    extract::FromRequestParts,
    http::{StatusCode, header},
    response::IntoResponse,
};
use axum_session::Session;
use axum_session_sqlx::SessionPgPool;
use loco_rs::prelude::*;

pub async fn index(
    State(ctx): State<AppContext>,
    session: std::result::Result<
        Session<SessionPgPool>,
        <Session<SessionPgPool> as FromRequestParts<AppContext>>::Rejection,
    >,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    let mut households = Vec::new();
    let mut signed_in = false;
    if let Ok(session) = session {
        match browser::authenticate(&ctx.db, &session).await {
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
    match format::render().view(
        &view,
        "home/index.html",
        data!({
            "title": "Welcome",
            "allow_palette": signed_in,
            "signed_in": signed_in,
            "households": households,
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
        }),
    ) {
        Ok(response) => ([(header::CACHE_CONTROL, "no-store")], response).into_response(),
        Err(error) => error.into_response(),
    }
}
