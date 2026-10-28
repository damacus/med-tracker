use super::*;
use axum::extract::Query;
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
    Query(page): Query<Pagination>,
) -> Response {
    let before = match (page.before_id, page.before_taken_at) {
        (None, None) => None,
        (Some(id), Some(taken_at)) => {
            Some(crate::models::profile::personal_history::Cursor { taken_at, id })
        }
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    let (principal, transaction) = match begin(&ctx, &session, false).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let records = crate::models::profile::personal_history::page(
        &transaction,
        AccountId::from_database(principal.account_id()),
        before,
        &request_id(request),
        principal.provenance(),
    )
    .await;
    let mut records = match records {
        Ok(value) => value,
        Err(error) => return operation_error(error),
    };
    if let Err(error) = transaction.commit().await {
        return operation_error(error.into());
    }
    let next = if records.len() > 25 {
        records.truncate(25);
        records.last().map(|record| {
            format!(
                "/profile/history?before_id={}&before_taken_at={}",
                record.id,
                record.taken_at.format("%Y-%m-%dT%H:%M:%S%.f")
            )
        })
    } else {
        None
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
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = labels["profiles"]["personal"]["history"].clone();
    data["labels"] = labels;
    data["lang"] = json!(locale);
    data["records"] = json!(records.into_iter().map(|record|json!({"medicine":record.medicine,"amount":record.dose_amount.map(|amount|amount.to_string()),"unit":record.dose_unit,"taken_at":record.taken_at.and_utc().with_timezone(&principal.time_zone()).format("%Y-%m-%d %H:%M %Z").to_string()})).collect::<Vec<_>>());
    data["next_page"] = json!(next);
    match format::render().view(&view, "profile/history.html", data) {
        Ok(response) => ([(header::CACHE_CONTROL, "no-store")], response).into_response(),
        Err(error) => {
            tracing::warn!(error = %error, "Personal history rendering failed");
            unavailable()
        }
    }
}
