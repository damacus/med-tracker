mod carer_relationships;
mod health_events;
mod page;
mod rendering;

use super::medications::{
    authentication_error, begin, forms, operation_error, request_id, unavailable,
};
use crate::models::{
    access::TenantTransaction,
    care::{administration, browser_treatments, people},
    errors::OperationError,
};
use axum::{
    Extension,
    extract::{Form as AxumForm, Query},
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
        .add("/{slug}/people", get(index).post(create))
        .add("/{slug}/people/new", get(new))
        .add("/{slug}/people/{id}", get(show).post(update))
        .add("/{slug}/people/{id}/edit", get(edit))
        .add(
            "/{slug}/people/{id}/carer_relationships",
            get(carer_relationships::index).post(carer_relationships::create),
        )
        .add(
            "/{slug}/people/{id}/carer_relationships/new",
            get(carer_relationships::index),
        )
        .add(
            "/{slug}/people/{id}/carer_relationships/{relationship_id}/remove",
            post(carer_relationships::remove),
        )
        .add(
            "/{slug}/people/{id}/carer_relationships/{relationship_id}/restore",
            post(carer_relationships::restore),
        )
        .add(
            "/{slug}/people/{id}/health_events",
            get(health_events::index).post(health_events::create),
        )
        .add(
            "/{slug}/people/{id}/health_events/new",
            get(health_events::new),
        )
        .add(
            "/{slug}/people/{id}/health_events/{event_id}/edit",
            get(health_events::edit),
        )
        .add(
            "/{slug}/people/{id}/health_events/{event_id}",
            post(health_events::update),
        )
        .add(
            "/{slug}/people/{id}/health_events/{event_id}/delete",
            post(health_events::delete),
        )
}

async fn index(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Query(pagination): Query<people::Pagination>,
) -> Response {
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .index(pagination)
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
        .show(&id)
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
        .save(None, draft)
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
        .save(Some(&id), draft)
        .await
}
