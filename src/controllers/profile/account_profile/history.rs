use super::*;
use crate::models::identity::resource::{HistoryActor, deny_personal_history};
use axum::extract::{Query, rejection::QueryRejection};
use serde::Deserialize;

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Pagination {
    before_id: Option<i64>,
    before_taken_at: Option<chrono::NaiveDateTime>,
}

pub(super) async fn index(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    headers: HeaderMap,
    ViewEngine(view): ViewEngine<TeraView>,
    request: Option<Extension<LocoRequestId>>,
    page: Result<Query<Pagination>, QueryRejection>,
) -> Response {
    let request_id = request_id(request);
    let mut response = read(&ctx, &session, headers, &view, &request_id, page).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    if let Ok(value) = axum::http::HeaderValue::from_str(&request_id) {
        response.headers_mut().insert("x-request-id", value);
    }
    response
}

async fn read(
    ctx: &AppContext,
    session: &Session<SessionPgPool>,
    headers: HeaderMap,
    view: &TeraView,
    request_id: &str,
    page: Result<Query<Pagination>, QueryRejection>,
) -> Response {
    let principal =
        match browser::authenticate(&ctx.db, session, ctx.config.settings.as_ref()).await {
            Ok(value) => value,
            Err(error) => return refused(ctx, None, request_id, authentication_error(error)).await,
        };
    let actor = HistoryActor::browser(&principal);
    let transaction = match principal.account_transaction(&ctx.db, false).await {
        Ok(value) => value,
        Err(error) => {
            return refused(ctx, Some(&actor), request_id, authentication_error(error)).await;
        }
    };
    let before = match page.map(|Query(page)| (page.before_id, page.before_taken_at)) {
        Ok((None, None)) => None,
        Ok((Some(id), Some(taken_at))) if id > 0 => {
            Some(crate::models::profile::personal_history::Cursor { taken_at, id })
        }
        _ => {
            if let Err(error) = transaction.rollback().await {
                return operation_error(error.into());
            }
            return refused(
                ctx,
                Some(&actor),
                request_id,
                StatusCode::BAD_REQUEST.into_response(),
            )
            .await;
        }
    };
    let page =
        crate::models::profile::personal_history::page(&transaction, &actor, before, request_id)
            .await;
    let page = match page {
        Ok(value) => value,
        Err(error) => {
            if let Err(error) = transaction.rollback().await {
                return operation_error(error.into());
            }
            return refused(ctx, Some(&actor), request_id, operation_error(error)).await;
        }
    };
    if let Err(error) = transaction.commit().await {
        return refused(ctx, Some(&actor), request_id, operation_error(error.into())).await;
    }
    let next = page.next.map(|cursor| {
        format!(
            "/profile/history?before_id={}&before_taken_at={}",
            cursor.id,
            cursor.taken_at.format("%Y-%m-%dT%H:%M:%S%.f")
        )
    });
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
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = labels["profiles"]["personal"]["history"].clone();
    data["labels"] = labels;
    data["lang"] = json!(locale);
    data["records"] = json!(page.records.into_iter().map(|record|json!({"medicine":record.medicine,"amount":record.dose_amount.map(|amount|amount.to_string()),"unit":record.dose_unit,"taken_at":record.taken_at.and_utc().with_timezone(&principal.time_zone()).format("%Y-%m-%d %H:%M %Z").to_string()})).collect::<Vec<_>>());
    data["next_page"] = json!(next);
    match format::render().view(view, "profile/history.html", data) {
        Ok(response) => ([(header::CACHE_CONTROL, "no-store")], response).into_response(),
        Err(error) => {
            tracing::warn!(error = %error, "Personal history rendering failed");
            unavailable()
        }
    }
}

async fn refused(
    ctx: &AppContext,
    actor: Option<&HistoryActor>,
    request_id: &str,
    response: Response,
) -> Response {
    match deny_personal_history(&ctx.db, actor, request_id).await {
        Ok(()) => response,
        Err(error) => operation_error(error),
    }
}
