mod page;
mod rendering;

use super::medications::{
    authentication_error, begin, forms, operation_error, request_id, unavailable,
};
use crate::models::{access::TenantTransaction, care::administration, errors::OperationError};
use axum::{
    Extension,
    extract::Form as AxumForm,
    http::{StatusCode, header},
    response::IntoResponse,
};
use axum_csrf::CsrfToken;
use axum_session::Session;
use axum_session_sqlx::SessionPgPool;
use loco_rs::{controller::middleware::request_id::LocoRequestId, prelude::*};
use serde_json::{Value, json};
use std::collections::HashMap;

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/households")
        .add("/{slug}/admin", get(index))
        .add("/{slug}/admin/household/edit", get(settings))
        .add("/{slug}/admin/household", post(save_settings))
        .add("/{slug}/admin/users", get(users))
        .add("/{slug}/admin/users/{id}/edit", get(role))
        .add("/{slug}/admin/users/{id}/membership_role", post(save_role))
        .add(
            "/{slug}/admin/carer_relationships",
            get(relationships).post(create_relationship),
        )
        .add(
            "/{slug}/admin/carer_relationships/new",
            get(new_relationship),
        )
        .add(
            "/{slug}/admin/carer_relationships/{id}/deactivate",
            post(deactivate),
        )
        .add(
            "/{slug}/admin/carer_relationships/{id}/activate",
            post(activate),
        )
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
        .show(page::Screen::Index, None, None)
        .await
}
async fn settings(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .show(page::Screen::Settings, None, None)
        .await
}
async fn users(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .show(page::Screen::Users, None, None)
        .await
}
async fn role(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, i64)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .show(page::Screen::Role(id), None, None)
        .await
}
async fn relationships(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .show(page::Screen::Relationships, None, None)
        .await
}
async fn new_relationship(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .show(page::Screen::NewRelationship, None, None)
        .await
}
async fn save_settings(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .save(page::Change::Settings, draft)
        .await
}
async fn save_role(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, i64)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .save(page::Change::Role(id), draft)
        .await
}
async fn create_relationship(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .save(page::Change::CreateRelationship, draft)
        .await
}
async fn deactivate(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, i64)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .save(page::Change::Relationship(id, false), draft)
        .await
}
async fn activate(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, i64)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .save(page::Change::Relationship(id, true), draft)
        .await
}
