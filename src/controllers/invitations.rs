mod acceptance;
mod page;
mod rendering;

use super::medications::{
    authentication_error, begin, forms, operation_error, request_id, unavailable,
};
use crate::models::{care::invitations, errors::OperationError};
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

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/households")
        .add("/{slug}/admin/invitations", get(index).post(create))
        .add("/{slug}/admin/invitations/{id}/resend", post(resend))
        .add("/{slug}/admin/invitations/{id}/cancel", post(cancel))
}

pub fn acceptance_routes() -> Routes {
    Routes::new().add(
        "/invitations/accept",
        get(acceptance::preview).post(acceptance::accept),
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
        .index(None, None)
        .await
}
async fn create(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<Vec<(String, String)>>,
) -> Response {
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .save(page::Change::Create, draft)
        .await
}
async fn resend(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, i64)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<Vec<(String, String)>>,
) -> Response {
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .save(page::Change::Resend(id), draft)
        .await
}
async fn cancel(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, i64)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<Vec<(String, String)>>,
) -> Response {
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .save(page::Change::Cancel(id), draft)
        .await
}

fn field<'a>(draft: &'a [(String, String)], name: &str) -> &'a str {
    draft
        .iter()
        .find(|(key, _)| key == name)
        .map_or("", |(_, value)| value)
}
