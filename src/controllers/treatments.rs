pub(crate) mod forms;
mod page;
mod rendering;

use super::medications::{self as browser, forms as browser_forms};
use crate::models::{
    care::{assignments, browser_treatments, pause_periods, treatments},
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
use loco_rs::{controller::middleware::request_id::LocoRequestId, prelude::*};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::result::Result;

#[derive(Clone, Copy)]
pub(crate) enum Kind {
    Schedule,
    Assignment,
}
impl Kind {
    fn parse(value: &str) -> Result<Self, OperationError> {
        match value {
            "schedules" => Ok(Self::Schedule),
            "assignments" => Ok(Self::Assignment),
            _ => Err(OperationError::NotFound),
        }
    }
    fn path(self) -> &'static str {
        match self {
            Self::Schedule => "schedules",
            Self::Assignment => "assignments",
        }
    }
    fn envelope(self) -> &'static str {
        match self {
            Self::Schedule => "schedule",
            Self::Assignment => "person_medication",
        }
    }
    async fn read(
        self,
        tenant: &crate::models::access::TenantTransaction,
        id: &str,
    ) -> Result<(Value, String), OperationError> {
        match self {
            Self::Schedule => treatments::lifecycle::read(tenant, id).await,
            Self::Assignment => assignments::read(tenant, id).await,
        }
    }
}
#[derive(Default, Deserialize)]
struct Pagination {
    page: Option<i64>,
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/households")
        .add("/{slug}/people/{person}/treatments", get(index))
        .add("/{slug}/people/{person}/treatments/{kind}/new", get(new))
        .add("/{slug}/people/{person}/treatments/{kind}", post(create))
        .add(
            "/{slug}/people/{person}/treatments/{kind}/{id}/edit",
            get(edit),
        )
        .add(
            "/{slug}/people/{person}/treatments/{kind}/{id}",
            post(update),
        )
        .add(
            "/{slug}/people/{person}/treatments/{kind}/{id}/{action}",
            post(change),
        )
}

async fn index(
    State(ctx): State<AppContext>,
    Path((slug, person)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Query(page): Query<Pagination>,
) -> Response {
    page::Page::new(&ctx, &session, &token, &view, &slug, &person, request)
        .index(page.page.unwrap_or(1))
        .await
}
async fn new(
    State(ctx): State<AppContext>,
    Path((slug, person, kind)): Path<(String, String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    page::Page::new(&ctx, &session, &token, &view, &slug, &person, request)
        .form(&kind, None)
        .await
}
async fn edit(
    State(ctx): State<AppContext>,
    Path((slug, person, kind, id)): Path<(String, String, String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    page::Page::new(&ctx, &session, &token, &view, &slug, &person, request)
        .form(&kind, Some(&id))
        .await
}
async fn create(
    State(ctx): State<AppContext>,
    Path((slug, person, kind)): Path<(String, String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    page::Page::new(&ctx, &session, &token, &view, &slug, &person, request)
        .save(&kind, None, draft)
        .await
}
async fn update(
    State(ctx): State<AppContext>,
    Path((slug, person, kind, id)): Path<(String, String, String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    page::Page::new(&ctx, &session, &token, &view, &slug, &person, request)
        .save(&kind, Some(&id), draft)
        .await
}
async fn change(
    State(ctx): State<AppContext>,
    Path((slug, person, kind, id, action)): Path<(String, String, String, String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    page::Page::new(&ctx, &session, &token, &view, &slug, &person, request)
        .change(&kind, &id, &action, draft)
        .await
}
