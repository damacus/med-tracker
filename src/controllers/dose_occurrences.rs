mod forms;
mod page;
mod rendering;

use super::medications::{self as browser, forms as browser_forms};
use crate::models::{
    access::TenantTransaction,
    care::{assignments, browser_query, dose_occurrences, people, treatments},
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
use serde_json::{Value, json};
use std::{collections::HashMap, result::Result};

#[derive(Clone, Copy)]
enum Kind {
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
    fn model(self) -> &'static str {
        match self {
            Self::Schedule => "schedule",
            Self::Assignment => "person_medication",
        }
    }
    async fn read(self, tenant: &TenantTransaction, id: &str) -> Result<Value, OperationError> {
        let (body, _) = match self {
            Self::Schedule => treatments::lifecycle::read(tenant, id).await?,
            Self::Assignment => assignments::read(tenant, id).await?,
        };
        Ok(body["data"].clone())
    }
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/households")
        .add(
            "/{slug}/people/{person}/treatments/{kind}/{id}/doses",
            get(index),
        )
        .add(
            "/{slug}/people/{person}/treatments/{kind}/{id}/doses/{action}",
            post(change),
        )
}

async fn index(
    State(ctx): State<AppContext>,
    Path(path): Path<(String, String, String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Query(query): Query<dose_occurrences::RangeQuery>,
) -> Response {
    page::Page::new(
        &ctx,
        &session,
        &token,
        &view,
        &path,
        browser::request_id(request),
    )
    .index(query)
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
    let path = (slug, person, kind, id);
    page::Page::new(
        &ctx,
        &session,
        &token,
        &view,
        &path,
        browser::request_id(request),
    )
    .change(&action, draft)
    .await
}
