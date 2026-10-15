use crate::{
    controllers::medications::{
        authentication_error, begin, operation_error, request_id, unavailable,
    },
    models::care::{global_search, report_pdf},
};
use axum::{
    Extension,
    extract::Query,
    http::{HeaderMap, header},
    response::IntoResponse,
};
use axum_session::Session;
use axum_session_sqlx::SessionPgPool;
use chrono::Utc;
use loco_rs::controller::middleware::request_id::LocoRequestId;
use loco_rs::prelude::*;
use serde::Deserialize;
use serde_json::json;

#[derive(Default, Deserialize)]
struct SearchQuery {
    q: Option<String>,
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/households")
        .add("/{slug}/search", get(show))
        .add("/{slug}/search.json", get(json_results))
        .add("/{slug}/search/labels.json", get(json_labels))
}

async fn show(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    headers: HeaderMap,
    ViewEngine(view): ViewEngine<TeraView>,
    Query(input): Query<SearchQuery>,
) -> Response {
    let (principal, tenant) = match begin(&ctx, &session, &slug, &request_id(request)).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let locale = report_pdf::locale(
        headers
            .get(header::ACCEPT_LANGUAGE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("en"),
    );
    let labels = match report_pdf::translations(locale) {
        Ok(value) => value,
        Err(_) => return unavailable(),
    };
    let query = input.q.unwrap_or_default().trim().to_owned();
    let results = if query.is_empty() {
        Vec::new()
    } else {
        match global_search::query(
            &tenant,
            &slug,
            &query,
            &labels,
            Utc::now()
                .with_timezone(&principal.time_zone())
                .date_naive(),
        )
        .await
        {
            Ok(value) => value,
            Err(error) => return operation_error(error),
        }
    };
    if tenant.commit().await.is_err() {
        return unavailable();
    }
    let mut data = super::medications::rendering::appearance_context();
    data["title"] = labels["global_search"]["page_title"].clone();
    data["lang"] = json!(locale);
    data["labels"] = labels;
    data["slug"] = json!(slug);
    data["query"] = json!(query);
    data["results"] = json!(results);
    match format::render().view(
        &view,
        "searches/show.html",
        crate::views::template_data(&data),
    ) {
        Ok(response) => ([(header::CACHE_CONTROL, "no-store")], response).into_response(),
        Err(_) => unavailable(),
    }
}

async fn json_results(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    headers: HeaderMap,
    Query(input): Query<SearchQuery>,
) -> Response {
    let (principal, tenant) = match begin(&ctx, &session, &slug, &request_id(request)).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let locale = report_pdf::locale(
        headers
            .get(header::ACCEPT_LANGUAGE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("en"),
    );
    let labels = match report_pdf::translations(locale) {
        Ok(value) => value,
        Err(_) => return unavailable(),
    };
    let query = input.q.unwrap_or_default();
    let results = if query.trim().is_empty() {
        Vec::new()
    } else {
        match global_search::query(
            &tenant,
            &slug,
            &query,
            &labels,
            Utc::now()
                .with_timezone(&principal.time_zone())
                .date_naive(),
        )
        .await
        {
            Ok(value) => value,
            Err(error) => return operation_error(error),
        }
    };
    if tenant.commit().await.is_err() {
        return unavailable();
    }
    (
        [(header::CACHE_CONTROL, "no-store")],
        axum::Json(json!({"results": results})),
    )
        .into_response()
}

async fn json_labels(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    headers: HeaderMap,
) -> Response {
    let (_, tenant) = match begin(&ctx, &session, &slug, &request_id(request)).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let locale = report_pdf::locale(
        headers
            .get(header::ACCEPT_LANGUAGE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("en"),
    );
    let labels = match report_pdf::translations(locale) {
        Ok(value) => value,
        Err(_) => return unavailable(),
    };
    if tenant.commit().await.is_err() {
        return unavailable();
    }
    (
        [(header::CACHE_CONTROL, "no-store")],
        axum::Json(json!({"locale": locale, "labels": labels["global_search"]})),
    )
        .into_response()
}
