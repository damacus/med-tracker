mod audit_logs;
mod bulk_members;
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
        .add(
            "/{slug}/admin/users/bulk/preview",
            post(bulk_members::preview),
        )
        .add("/{slug}/admin/users/bulk", post(bulk_members::apply))
        .add("/{slug}/admin/memberships/{id}", get(member_detail))
        .add("/{slug}/admin/users/{id}/edit", get(role))
        .add("/{slug}/admin/users/{id}/membership_role", post(save_role))
        .add("/{slug}/admin/memberships/{id}/edit", get(membership_role))
        .add(
            "/{slug}/admin/memberships/{id}/membership_role",
            post(save_membership_role),
        )
        .add("/{slug}/admin/users/{id}/suspend", post(suspend_member))
        .add("/{slug}/admin/users/{id}/restore", post(restore_member))
        .add("/{slug}/admin/users/{id}/revoke", post(revoke_member))
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
        .add("/{slug}/admin/carer_relationships/recover", get(recovery))
        .add(
            "/{slug}/admin/carer_relationships/recover/{id}",
            get(recovery_detail).post(save_recovery),
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
    let (query, explicit_language) =
        match administration::member_browser::MemberQuery::parse(incoming.uri().query()) {
            Ok(value) => value,
            Err(_) => {
                return rendering::invalid_link(&view, fallback_language(incoming.headers()));
            }
        };
    let language = match explicit_language.as_deref() {
        Some(value) => match supported_language(value) {
            Some(language) => language,
            None => return rendering::invalid_link(&view, fallback_language(incoming.headers())),
        },
        None => fallback_language(incoming.headers()),
    };
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .with_language(language)
        .with_explicit_language(explicit_language.is_some())
        .with_member_query(query)
        .show(page::Screen::Users, None, None)
        .await
}
async fn member_detail(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, i64)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    incoming: Request,
) -> Response {
    let id = match administration::member_browser::MembershipId::new(id) {
        Ok(id) => id,
        Err(error) => return presentation_error(&view, "en", error),
    };
    let mut cursor = None;
    let mut duplicate = false;
    let mut pairs = Vec::new();
    for (key, value) in
        url::form_urlencoded::parse(incoming.uri().query().unwrap_or_default().as_bytes())
    {
        if key == "history_cursor" {
            if cursor.replace(value.into_owned()).is_some() {
                duplicate = true;
            }
        } else {
            pairs.push((key.into_owned(), value.into_owned()));
        }
    }
    let raw = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(pairs)
        .finish();
    let (query, explicit) = match administration::member_browser::MemberQuery::parse(Some(&raw)) {
        Ok(value) => value,
        Err(_) => return rendering::invalid_link(&view, fallback_language(incoming.headers())),
    };
    let language = match explicit.as_deref() {
        Some(value) => match supported_language(value) {
            Some(lang) => lang,
            None => return rendering::invalid_link(&view, "en"),
        },
        None => fallback_language(incoming.headers()),
    };
    let cursor = match administration::member_history::HistoryCursor::parse(cursor.as_deref(), id) {
        Ok(cursor) if !duplicate => cursor,
        _ => return rendering::invalid_link(&view, language),
    };
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .with_language(language)
        .with_explicit_language(explicit.is_some())
        .with_member_query(query)
        .show(page::Screen::Member(id, cursor), None, None)
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
    role_page(
        page::Page::new(&ctx, &session, &token, &view, &slug, request),
        incoming,
        administration::member_browser::MemberLocator::legacy_user(id),
    )
    .await
}
async fn membership_role(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, i64)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    incoming: Request,
) -> Response {
    role_page(
        page::Page::new(&ctx, &session, &token, &view, &slug, request),
        incoming,
        administration::member_browser::MemberLocator::membership(id),
    )
    .await
}
async fn role_page(
    page: page::Page<'_>,
    incoming: Request,
    locator: Result<administration::member_browser::MemberLocator, OperationError>,
) -> Response {
    let locator = match locator {
        Ok(locator) => locator,
        Err(error) => return presentation_error(page.view(), "en", error),
    };
    let (query, explicit_language) =
        match administration::member_browser::MemberQuery::parse(incoming.uri().query()) {
            Ok(value) => value,
            Err(_) => {
                return rendering::invalid_link(page.view(), fallback_language(incoming.headers()));
            }
        };
    let language = match explicit_language.as_deref() {
        Some(value) => match supported_language(value) {
            Some(language) => language,
            None => {
                return rendering::invalid_link(page.view(), fallback_language(incoming.headers()));
            }
        },
        None => fallback_language(incoming.headers()),
    };
    page.with_language(language)
        .with_explicit_language(explicit_language.is_some())
        .with_member_query(query)
        .show(page::Screen::Role(locator), None, None)
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

async fn recovery(
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
        Err(_) => return rendering::invalid_link(&view, fallback_language(incoming.headers())),
    };
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .with_language(language)
        .with_explicit_language(incoming.uri().query().is_some())
        .show(page::Screen::Recovery, None, None)
        .await
}

async fn recovery_detail(
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
        Err(_) => return rendering::invalid_link(&view, fallback_language(incoming.headers())),
    };
    page::Page::new(&ctx, &session, &token, &view, &slug, request)
        .with_language(language)
        .with_explicit_language(incoming.uri().query().is_some())
        .show(page::Screen::RecoveryDetail(id), None, None)
        .await
}

async fn save_recovery(
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
        .save(page::Change::Recovery(id), draft)
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
    save_role_page(
        page::Page::new(&ctx, &session, &token, &view, &slug, request),
        administration::member_browser::MemberLocator::legacy_user(id),
        draft,
    )
    .await
}
async fn save_membership_role(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, i64)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    save_role_page(
        page::Page::new(&ctx, &session, &token, &view, &slug, request),
        administration::member_browser::MemberLocator::membership(id),
        draft,
    )
    .await
}
async fn save_role_page(
    page: page::Page<'_>,
    locator: Result<administration::member_browser::MemberLocator, OperationError>,
    draft: HashMap<String, String>,
) -> Response {
    let locator = match locator {
        Ok(locator) => locator,
        Err(error) => return presentation_error(page.view(), "en", error),
    };
    let language = match draft.get("lang") {
        Some(value) => match supported_language(value) {
            Some(language) => language,
            None => return rendering::invalid_link(page.view(), "en"),
        },
        None => "en",
    };
    let query = match administration::member_browser::MemberQuery::from_form(&draft) {
        Ok(query) => query,
        Err(_) => return rendering::invalid_link(page.view(), language),
    };
    page.with_language(language)
        .with_explicit_language(draft.contains_key("lang"))
        .with_member_query(query)
        .save(page::Change::Role(locator), draft)
        .await
}

async fn suspend_member(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, i64)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    member_action(
        page::Page::new(&ctx, &session, &token, &view, &slug, request),
        id,
        page::MembershipAction::Suspend,
        draft,
    )
    .await
}
async fn restore_member(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, i64)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    member_action(
        page::Page::new(&ctx, &session, &token, &view, &slug, request),
        id,
        page::MembershipAction::Restore,
        draft,
    )
    .await
}
async fn revoke_member(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, i64)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    member_action(
        page::Page::new(&ctx, &session, &token, &view, &slug, request),
        id,
        page::MembershipAction::Revoke,
        draft,
    )
    .await
}
async fn member_action(
    page: page::Page<'_>,
    id: i64,
    action: page::MembershipAction,
    draft: HashMap<String, String>,
) -> Response {
    let id = match administration::member_browser::MembershipId::new(id) {
        Ok(id) => id,
        Err(error) => return presentation_error(page.view(), "en", error),
    };
    let language = match draft.get("lang") {
        Some(value) => match supported_language(value) {
            Some(language) => language,
            None => return rendering::invalid_link(page.view(), "en"),
        },
        None => "en",
    };
    let query = match administration::member_browser::MemberQuery::from_form(&draft) {
        Ok(query) => query,
        Err(_) => return rendering::invalid_link(page.view(), language),
    };
    page.with_language(language)
        .with_explicit_language(draft.contains_key("lang"))
        .with_member_query(query)
        .save(page::Change::Membership(id, action), draft)
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
