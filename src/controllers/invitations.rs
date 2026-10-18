mod acceptance;
mod page;
mod rendering;

use super::medications::{begin, forms, request_id, unavailable};
use crate::models::{care::invitations, errors::OperationError};
use axum::{
    Extension,
    extract::{Form as AxumForm, Request},
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
    incoming: Request,
) -> Response {
    let query = match index_query(incoming.uri().query(), incoming.headers()) {
        Ok(value) => value,
        Err(_) => {
            return super::administration::presentation_invalid_link(&view, incoming.headers());
        }
    };
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .with_language(query.language)
        .with_explicit_language(query.explicit_language)
        .with_cursor(query.cursor)
        .with_notice(query.notice)
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
        .with_language(field(&draft, "lang"))
        .with_explicit_language(draft.iter().any(|(key, _)| key == "lang"))
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
        .with_language(field(&draft, "lang"))
        .with_explicit_language(draft.iter().any(|(key, _)| key == "lang"))
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
        .with_language(field(&draft, "lang"))
        .with_explicit_language(draft.iter().any(|(key, _)| key == "lang"))
        .save(page::Change::Cancel(id), draft)
        .await
}

fn field<'a>(draft: &'a [(String, String)], name: &str) -> &'a str {
    draft
        .iter()
        .find(|(key, _)| key == name)
        .map_or("", |(_, value)| value)
}

struct IndexQuery {
    language: &'static str,
    explicit_language: bool,
    cursor: Option<invitations::browser::BrowseCursor>,
    notice: Option<page::Notice>,
}

fn index_query(
    raw: Option<&str>,
    headers: &axum::http::HeaderMap,
) -> Result<IndexQuery, OperationError> {
    let mut language = None;
    let mut cursor = None;
    let mut notice = None;
    for (key, value) in url::form_urlencoded::parse(raw.unwrap_or_default().as_bytes()) {
        if value.len() > 128 {
            return Err(OperationError::NotFound);
        }
        match key.as_ref() {
            "lang"
                if language.is_none()
                    && matches!(value.as_ref(), "en" | "cy" | "ga" | "pt" | "es") =>
            {
                language = Some(crate::models::care::report_pdf::locale(&value));
            }
            "cursor" if cursor.is_none() => {
                cursor = Some(invitations::browser::BrowseCursor::parse(&value)?)
            }
            "notice" if notice.is_none() => {
                notice = Some(page::Notice::parse(&value).ok_or(OperationError::NotFound)?);
            }
            _ => return Err(OperationError::NotFound),
        }
    }
    let explicit = language.is_some();
    let language = language.unwrap_or(super::administration::presentation_language(None, headers)?);
    Ok(IndexQuery {
        language,
        explicit_language: explicit,
        cursor,
        notice,
    })
}
