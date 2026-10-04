use super::api_client::{decode_api_body, download_headers};
use super::*;
use axum::extract::Query;
use axum::response::IntoResponse;
use chrono::{Months, NaiveDate};
use medtracker_web::household::path_segment;
use medtracker_web::reports::{render_reports, ReportChoice, ReportDraft};

const PDF_HEADER: &[u8] = b"%PDF-";

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/households/{slug}/reports", get(index))
        .route(
            "/households/{slug}/reports/health-history.pdf",
            get(download),
        )
}

#[derive(Debug)]
pub(super) struct Filters {
    pub(super) person_id: String,
    pub(super) start: NaiveDate,
    pub(super) end: NaiveDate,
    pub(super) include_takes: bool,
}

fn iso_date(value: &str) -> Option<NaiveDate> {
    if value.len() != 10
        || !value.bytes().enumerate().all(|(index, byte)| {
            if index == 4 || index == 7 {
                byte == b'-'
            } else {
                byte.is_ascii_digit()
            }
        })
    {
        return None;
    }
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

pub(super) fn filters(
    query: &HashMap<String, String>,
    today: NaiveDate,
) -> Result<Filters, Vec<(String, String)>> {
    let mut errors = Vec::new();
    let person_id = query.get("person_id").cloned().unwrap_or_default();
    if person_id.is_empty() {
        errors.push(("person_id".to_owned(), "person_required".to_owned()));
    }
    let end = match query.get("end_date") {
        Some(value) => match iso_date(value) {
            Some(date) => Some(date),
            None => {
                errors.push(("end_date".to_owned(), "end_date_invalid".to_owned()));
                None
            }
        },
        None => Some(today),
    };
    let start = match query.get("start_date") {
        Some(value) => match iso_date(value) {
            Some(date) => Some(date),
            None => {
                errors.push(("start_date".to_owned(), "start_date_invalid".to_owned()));
                None
            }
        },
        None => match end.and_then(|end| end.checked_sub_months(Months::new(12))) {
            Some(date) => Some(date),
            None => {
                errors.push(("start_date".to_owned(), "date_range".to_owned()));
                None
            }
        },
    };
    if let (Some(start), Some(end)) = (start, end) {
        if start > end {
            errors.push(("end_date".to_owned(), "date_order".to_owned()));
        } else if (end - start).num_days() > 366 {
            errors.push(("end_date".to_owned(), "date_range".to_owned()));
        }
    }
    let include_takes = match query.get("include_medication_takes") {
        Some(value) if value == "1" => true,
        None => false,
        Some(value) if value == "0" => false,
        Some(_) => {
            errors.push((
                "include_medication_takes".to_owned(),
                "takes_invalid".to_owned(),
            ));
            false
        }
    };
    if errors.is_empty() {
        Ok(Filters {
            person_id,
            start: start.unwrap_or(today),
            end: end.unwrap_or(today),
            include_takes,
        })
    } else {
        Err(errors)
    }
}

pub(super) fn download_error_key(status: StatusCode) -> &'static str {
    match status {
        StatusCode::FORBIDDEN => "forbidden",
        StatusCode::NOT_FOUND => "not_found",
        StatusCode::SERVICE_UNAVAILABLE => "unavailable",
        _ => "failed",
    }
}

pub(super) fn default_draft() -> ReportDraft {
    let today = crate::reports::today();
    ReportDraft {
        person_id: String::new(),
        start_date: today
            .checked_sub_months(Months::new(12))
            .map(|date| date.to_string())
            .unwrap_or_default(),
        end_date: today.to_string(),
        include_medication_takes: false,
    }
}

fn draft_from(query: &HashMap<String, String>) -> ReportDraft {
    let mut draft = default_draft();
    if let Some(value) = query.get("start_date") {
        draft.start_date = value.clone();
    }
    if let Some(value) = query.get("end_date") {
        draft.end_date = value.clone();
    }
    if let Some(value) = query.get("person_id") {
        draft.person_id = value.clone();
    }
    draft.include_medication_takes =
        query.get("include_medication_takes").map(String::as_str) == Some("1");
    draft
}

async fn context(
    api: &mut WebApi,
    slug: &str,
) -> Result<(i64, String, Vec<ReportChoice>), PageError> {
    let (household_id, name) = api.household(slug).await?;
    let value = api.capabilities(household_id).await?;
    let manageable = value
        .pointer("/data/people/manage_ids")
        .and_then(Value::as_array)
        .map(|ids| {
            ids.iter()
                .filter_map(|item| item.as_i64().or_else(|| item.as_str()?.parse().ok()))
                .collect::<Vec<i64>>()
        })
        .unwrap_or_default();
    let records = api
        .collection(&format!("/api/v1/households/{household_id}/people"))
        .await?;
    let choices = records
        .iter()
        .filter(|row| numeric(row, "id").is_some_and(|id| manageable.contains(&id)))
        .map(|row| {
            numeric(row, "id")
                .ok_or_else(|| error(StatusCode::INTERNAL_SERVER_ERROR))
                .map(|id| ReportChoice {
                    id: id.to_string(),
                    name: field(row, "name").to_owned(),
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((household_id, name, choices))
}

async fn index(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Response {
    let mut api = match WebApi::authenticated(state, headers).await {
        Ok(api) => api,
        Err(error) => return error.response(),
    };
    let result = async {
        let (household_id, name, choices) = context(&mut api, &slug).await?;
        let notifications_visible = api.notifications_visible(household_id).await;
        render_reports(
            &name,
            &slug,
            api.locale,
            choices,
            default_draft(),
            vec![],
            notifications_visible,
        )
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))
    }
    .await;
    match result {
        Ok(body) => page(body, api.cookie.take()),
        Err(error) => error.response(),
    }
}

async fn download(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    let mut api = match WebApi::authenticated(state, headers).await {
        Ok(api) => api,
        Err(error) => return error.response(),
    };
    let today = crate::reports::today();
    let draft = draft_from(&query);
    let result = async {
        match filters(&query, today) {
            Err(errors) => {
                let (household_id, name, choices) = context(&mut api, &slug).await?;
                let notifications_visible = api.notifications_visible(household_id).await;
                let body = render_reports(
                    &name,
                    &slug,
                    api.locale,
                    choices,
                    draft,
                    errors,
                    notifications_visible,
                )
                .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
                Ok(page_status(
                    body,
                    api.cookie.take(),
                    StatusCode::UNPROCESSABLE_ENTITY,
                ))
            }
            Ok(filters) => {
                let (household_id, name, choices) = context(&mut api, &slug).await?;
                let path = format!(
                    "/api/v1/households/{household_id}/reports/health_history.pdf?person_id={}&start_date={}&end_date={}&include_medication_takes={}",
                    path_segment(&filters.person_id),
                    filters.start,
                    filters.end,
                    if filters.include_takes { "1" } else { "0" }
                );
                let reply = api.download(&path).await?;
                if reply.status.is_success() {
                    if !reply.body.starts_with(PDF_HEADER) {
                        return Err(error(StatusCode::BAD_GATEWAY));
                    }
                    let mut response = (StatusCode::OK, download_headers(&reply.headers), reply.body)
                        .into_response();
                    response.headers_mut().insert(
                        header::CACHE_CONTROL,
                        HeaderValue::from_static("no-store"),
                    );
                    if let Some(cookie) = api.cookie.take() {
                        response.headers_mut().append(header::SET_COOKIE, cookie);
                    }
                    return Ok(response);
                }
                let value = decode_api_body(reply.status, &reply.body)?;
                if value.get("error").is_none() {
                    return Err(error(StatusCode::BAD_GATEWAY));
                }
                let notifications_visible = api.notifications_visible(household_id).await;
                let body = render_reports(
                    &name,
                    &slug,
                    api.locale,
                    choices,
                    draft,
                    vec![(
                        "download".to_owned(),
                        download_error_key(reply.status).to_owned(),
                    )],
                    notifications_visible,
                )
                .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
                Ok(page_status(body, api.cookie.take(), reply.status))
            }
        }
    }
    .await;
    match result {
        Ok(response) => response,
        Err(error) => error.response(),
    }
}
