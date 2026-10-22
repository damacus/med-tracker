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
        .with_query(query)
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
    let query = match form_context(&draft) {
        Ok(query) => query,
        Err(_) => {
            return super::administration::presentation_invalid_link(
                &view,
                &axum::http::HeaderMap::new(),
            );
        }
    };
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .with_query(query)
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
    let query = match form_context(&draft) {
        Ok(query) => query,
        Err(_) => {
            return super::administration::presentation_invalid_link(
                &view,
                &axum::http::HeaderMap::new(),
            );
        }
    };
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .with_query(query)
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
    let query = match form_context(&draft) {
        Ok(query) => query,
        Err(_) => {
            return super::administration::presentation_invalid_link(
                &view,
                &axum::http::HeaderMap::new(),
            );
        }
    };
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .with_query(query)
        .save(page::Change::Cancel(id), draft)
        .await
}

fn field<'a>(draft: &'a [(String, String)], name: &str) -> &'a str {
    draft
        .iter()
        .find(|(key, _)| key == name)
        .map_or("", |(_, value)| value)
}

pub(crate) fn supported_language(value: &str) -> Option<&'static str> {
    match value {
        "en" => Some("en"),
        "cy" => Some("cy"),
        "ga" => Some("ga"),
        "pt" => Some("pt"),
        "es" => Some("es"),
        _ => None,
    }
}

pub(crate) fn fallback_language(headers: &axum::http::HeaderMap) -> &'static str {
    crate::models::care::report_pdf::locale(
        headers
            .get(header::ACCEPT_LANGUAGE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("en"),
    )
}

pub(crate) fn invalid_link(view: &TeraView, language: &str, status: StatusCode) -> Response {
    let Ok(labels) = crate::models::care::report_pdf::translations(language) else {
        return unavailable();
    };
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["lang"] = json!(language);
    data["labels"] = labels.clone();
    data["invitation_labels"] = labels["admin"]["invitations"]["acceptance"].clone();
    data["title"] = data["invitation_labels"]["invalid_title"].clone();
    match format::render().view(view, "invitations/error.html", data) {
        Ok(response) => (status, [(header::CACHE_CONTROL, "no-store")], response).into_response(),
        Err(_) => unavailable(),
    }
}

pub(crate) fn acceptance_data(
    language: &'static str,
    explicit_language: bool,
    invitation_token: &str,
    details: Value,
    signed_in: bool,
    authenticity_token: &str,
) -> Option<Value> {
    let Ok(labels) = crate::models::care::report_pdf::translations(language) else {
        return None;
    };
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = labels["admin"]["invitations"]["acceptance"]["title"].clone();
    data["lang"] = json!(language);
    data["labels"] = labels.clone();
    data["invitation_labels"] = labels["admin"]["invitations"]["acceptance"].clone();
    data["role_label"] = labels["admin"]["labels"]["membership_roles"]
        [details["membership_role"].as_str().unwrap_or("member")]
    .clone();
    data["invitation"] = details;
    data["invitation_token"] = json!(invitation_token);
    data["explicit_language"] = json!(explicit_language);
    data["signed_in"] = json!(signed_in);
    data["authenticity_token"] = json!(authenticity_token);
    Some(data)
}

struct IndexQuery {
    language: &'static str,
    explicit_language: bool,
    cursor: Option<invitations::browser::BrowseCursor>,
    notice: Option<page::Notice>,
    filters: invitations::browser::BrowseFilters,
}

fn index_query(
    raw: Option<&str>,
    headers: &axum::http::HeaderMap,
) -> Result<IndexQuery, OperationError> {
    let mut language = None;
    let mut cursor = None;
    let mut notice = None;
    let mut search = None;
    let mut role = None;
    let mut status = None;
    for (key, value) in url::form_urlencoded::parse(raw.unwrap_or_default().as_bytes()) {
        if value.len() > 256 {
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
            "q" if search.is_none() => search = Some(value.into_owned()),
            "role" if role.is_none() => role = Some(value.into_owned()),
            "status" if status.is_none() => status = Some(value.into_owned()),
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
        filters: invitations::browser::BrowseFilters::parse(
            search.as_deref().unwrap_or_default(),
            role.as_deref().unwrap_or_default(),
            status.as_deref().unwrap_or_default(),
        )?,
    })
}

fn form_context(draft: &[(String, String)]) -> Result<IndexQuery, OperationError> {
    let mut query = url::form_urlencoded::Serializer::new(String::new());
    for (name, value) in draft
        .iter()
        .filter(|(name, _)| matches!(name.as_str(), "q" | "role" | "status" | "cursor" | "lang"))
    {
        query.append_pair(name, value);
    }
    index_query(Some(&query.finish()), &axum::http::HeaderMap::new())
}
