mod page;
mod rendering;

use super::medications::{
    authentication_error, begin, forms, operation_error, request_id, unavailable,
};
use crate::models::{
    access::TenantTransaction,
    care::{browser_query, locations},
    entities::location,
    errors::OperationError,
    identity::browser::BrowserPrincipal,
};
use axum::{
    Extension,
    extract::Form as AxumForm,
    http::{StatusCode, header},
    response::IntoResponse,
};
use axum_csrf::CsrfToken;
use axum_session::Session;
use axum_session_sqlx::SessionPgPool;
use loco_rs::controller::middleware::request_id::LocoRequestId;
use loco_rs::prelude::*;
use serde_json::{Value, json};
use std::collections::HashMap;

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/households")
        .add("/{slug}/locations", get(index).post(create))
        .add("/{slug}/locations/new", get(new))
        .add("/{slug}/locations/{id}", get(show).post(update))
        .add("/{slug}/locations/{id}/edit", get(edit))
        .add("/{slug}/locations/{id}/destroy", post(destroy))
}

async fn index(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .index()
        .await
}

async fn show(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .show(&id, None)
        .await
}

async fn new(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .form(None, None, None)
        .await
}

async fn edit(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .form(Some(&id), None, None)
        .await
}

async fn create(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .save(None, draft, false)
        .await
}

async fn update(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .save(Some(&id), draft, false)
        .await
}

async fn destroy(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .save(Some(&id), draft, true)
        .await
}

fn redirect(slug: &str, id: Option<i64>) -> Response {
    let path = id.map_or_else(
        || format!("/households/{slug}/locations"),
        |id| format!("/households/{slug}/locations/{id}"),
    );
    (
        StatusCode::SEE_OTHER,
        [
            (header::LOCATION, path),
            (header::CACHE_CONTROL, "no-store".into()),
        ],
    )
        .into_response()
}
