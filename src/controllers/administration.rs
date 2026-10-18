mod audit_logs;
mod page;
mod rendering;

use super::medications::{authentication_error, begin, forms, request_id, unavailable};
use crate::models::{
    access::TenantTransaction, care::administration, errors::OperationError,
    identity::resource::AuthenticationError,
};
use axum::{
    Extension,
    extract::{Form as AxumForm, Request},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use axum_csrf::CsrfToken;
use axum_session::Session;
use axum_session_sqlx::SessionPgPool;
use loco_rs::{controller::middleware::request_id::LocoRequestId, prelude::*};
use serde_json::{Value, json};
use std::collections::HashMap;

pub(super) fn presentation_language(
    raw: Option<&str>,
    headers: &HeaderMap,
) -> Result<&'static str, OperationError> {
    let mut language = None;
    for (key, value) in url::form_urlencoded::parse(raw.unwrap_or_default().as_bytes()) {
        if key != "lang" || language.is_some() {
            return Err(OperationError::Validation {
                details: json!({"errors":{"lang":["is invalid"]}}),
            });
        }
        language = supported_language(&value);
        if language.is_none() {
            return Err(OperationError::Validation {
                details: json!({"errors":{"lang":["is invalid"]}}),
            });
        }
    }
    Ok(language.unwrap_or_else(|| {
        crate::models::care::report_pdf::locale(
            headers
                .get(header::ACCEPT_LANGUAGE)
                .and_then(|value| value.to_str().ok())
                .unwrap_or("en"),
        )
    }))
}

fn supported_language(value: &str) -> Option<&'static str> {
    match value {
        "en" => Some("en"),
        "cy" => Some("cy"),
        "ga" => Some("ga"),
        "pt" => Some("pt"),
        "es" => Some("es"),
        _ => None,
    }
}

pub(super) fn presentation_error(
    view: &TeraView,
    language: &str,
    error: OperationError,
) -> Response {
    rendering::error_page(view, language, &error)
}

pub(super) fn presentation_authentication_error(
    view: &TeraView,
    language: &str,
    error: AuthenticationError,
) -> Response {
    match error {
        AuthenticationError::Unauthenticated => authentication_error(error),
        AuthenticationError::Unavailable => {
            presentation_error(view, language, OperationError::Unavailable)
        }
        AuthenticationError::Forbidden
        | AuthenticationError::NotFound
        | AuthenticationError::InsufficientScope { .. } => {
            presentation_error(view, language, OperationError::Forbidden)
        }
    }
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/households")
        .add("/{slug}/admin", get(index))
        .add("/{slug}/admin/household/edit", get(settings))
        .add("/{slug}/admin/household", post(save_settings))
        .add("/{slug}/admin/users", get(users))
        .add("/{slug}/admin/users/{id}/edit", get(role))
        .add("/{slug}/admin/users/{id}/membership_role", post(save_role))
        .add("/{slug}/admin/audit_logs", get(audit_logs::index))
        .add("/{slug}/admin/audit_logs/{id}", get(audit_logs::detail))
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
    incoming: Request,
) -> Response {
    let language = match presentation_language(incoming.uri().query(), incoming.headers()) {
        Ok(language) => language,
        Err(_) => {
            return rendering::invalid_link(&view, fallback_language(incoming.headers()));
        }
    };
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .with_language(language)
        .with_explicit_language(incoming.uri().query().is_some())
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
    incoming: Request,
) -> Response {
    let language = match presentation_language(incoming.uri().query(), incoming.headers()) {
        Ok(language) => language,
        Err(_) => {
            return rendering::invalid_link(&view, fallback_language(incoming.headers()));
        }
    };
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .with_language(language)
        .with_explicit_language(incoming.uri().query().is_some())
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
    incoming: Request,
) -> Response {
    let language = match presentation_language(incoming.uri().query(), incoming.headers()) {
        Ok(language) => language,
        Err(_) => {
            return rendering::invalid_link(&view, fallback_language(incoming.headers()));
        }
    };
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .with_language(language)
        .with_explicit_language(incoming.uri().query().is_some())
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
    incoming: Request,
) -> Response {
    let language = match presentation_language(incoming.uri().query(), incoming.headers()) {
        Ok(language) => language,
        Err(_) => {
            return rendering::invalid_link(&view, fallback_language(incoming.headers()));
        }
    };
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .with_language(language)
        .with_explicit_language(incoming.uri().query().is_some())
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
    incoming: Request,
) -> Response {
    let language = match presentation_language(incoming.uri().query(), incoming.headers()) {
        Ok(language) => language,
        Err(_) => {
            return rendering::invalid_link(&view, fallback_language(incoming.headers()));
        }
    };
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .with_language(language)
        .with_explicit_language(incoming.uri().query().is_some())
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
    incoming: Request,
) -> Response {
    let language = match presentation_language(incoming.uri().query(), incoming.headers()) {
        Ok(language) => language,
        Err(_) => {
            return rendering::invalid_link(&view, fallback_language(incoming.headers()));
        }
    };
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .with_language(language)
        .with_explicit_language(incoming.uri().query().is_some())
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
        .with_language(draft.get("lang").map_or("en", String::as_str))
        .with_explicit_language(draft.contains_key("lang"))
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
        .with_language(draft.get("lang").map_or("en", String::as_str))
        .with_explicit_language(draft.contains_key("lang"))
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
        .with_language(draft.get("lang").map_or("en", String::as_str))
        .with_explicit_language(draft.contains_key("lang"))
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
        .with_language(draft.get("lang").map_or("en", String::as_str))
        .with_explicit_language(draft.contains_key("lang"))
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
        .with_language(draft.get("lang").map_or("en", String::as_str))
        .with_explicit_language(draft.contains_key("lang"))
        .save(page::Change::Relationship(id, true), draft)
        .await
}

pub(super) fn language_suffix(language: &str, explicit: bool) -> String {
    if language == "en" && !explicit {
        String::new()
    } else {
        format!("?lang={language}")
    }
}

pub(super) fn fallback_language(headers: &HeaderMap) -> &'static str {
    crate::models::care::report_pdf::locale(
        headers
            .get(header::ACCEPT_LANGUAGE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("en"),
    )
}

pub(super) fn presentation_invalid_link(view: &TeraView, headers: &HeaderMap) -> Response {
    rendering::invalid_link(view, fallback_language(headers))
}
