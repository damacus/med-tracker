use crate::{api_router, oauth, restricted_role, AppState};
use axum::body::{to_bytes, Body};
use axum::extract::{Form, Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use chrono::{DateTime, LocalResult, NaiveDate, NaiveDateTime, TimeZone, Utc};
use medtracker_web::{DoseFormState, DoseSource, HistoryRow, MedicationCard, MedicationDetail};
use sea_orm::TransactionTrait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use tower::ServiceExt;

const BODY_LIMIT: usize = 1_048_576;
const COLLECTION_LIMIT: usize = 500;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/households/{slug}/dashboard", get(dashboard))
        .route("/households/{slug}/medications", get(medications))
        .route("/households/{slug}/medications/{id}", get(medication))
        .route(
            "/households/{slug}/medications/{id}/doses",
            post(record_dose),
        )
        .route("/medication.css", get(styles))
        .route("/medication.js", get(script))
}

async fn styles() -> Response {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        medtracker_web::medication_stylesheet(),
    )
        .into_response()
}

async fn script() -> Response {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        medtracker_web::medication_script(),
    )
        .into_response()
}

fn page(body: String, cookie: Option<HeaderValue>) -> Response {
    page_status(body, cookie, StatusCode::OK)
}

fn page_status(body: String, cookie: Option<HeaderValue>, status: StatusCode) -> Response {
    let mut response = (
        status,
        [
            (header::CACHE_CONTROL, "no-store"),
            (header::CONTENT_SECURITY_POLICY, "default-src 'none'; style-src 'self'; script-src 'self'; form-action 'self'; base-uri 'none'; frame-ancestors 'none'"),
        ],
        axum::response::Html(body),
    ).into_response();
    if let Some(cookie) = cookie {
        response.headers_mut().append(header::SET_COOKIE, cookie);
    }
    response
}

fn failure(status: StatusCode) -> Response {
    (status, [(header::CACHE_CONTROL, "no-store")]).into_response()
}

fn login_redirect() -> Response {
    (
        StatusCode::FOUND,
        [
            (header::LOCATION, "/login"),
            (header::CACHE_CONTROL, "no-store"),
        ],
    )
        .into_response()
}

enum PageError {
    Status(StatusCode),
    Login,
}

impl PageError {
    fn response(self) -> Response {
        match self {
            Self::Status(status) => failure(status),
            Self::Login => login_redirect(),
        }
    }
}

fn error(status: StatusCode) -> PageError {
    PageError::Status(status)
}

fn html_cookie_only(headers: &HeaderMap) -> bool {
    !headers.contains_key(header::AUTHORIZATION)
}

struct ApiReply {
    status: StatusCode,
    value: Value,
}

struct WebApi {
    state: AppState,
    headers: HeaderMap,
    csrf: String,
    cookie: Option<HeaderValue>,
}

impl WebApi {
    async fn authenticated(state: AppState, headers: HeaderMap) -> Result<Self, PageError> {
        if !html_cookie_only(&headers) {
            return Err(PageError::Status(StatusCode::UNAUTHORIZED));
        }
        let db = state
            .db
            .begin()
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        restricted_role(&db)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        let session = oauth::browser_session(&state, &db, &headers)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?
            .ok_or(PageError::Login)?;
        db.commit()
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        Ok(Self {
            state,
            headers,
            csrf: session.csrf,
            cookie: None,
        })
    }

    async fn call(
        &mut self,
        method: Method,
        path: &str,
        body: Option<Value>,
        csrf: Option<&str>,
    ) -> Result<ApiReply, PageError> {
        let mut request = Request::builder().method(method).uri(path);
        for name in [
            header::COOKIE,
            header::AUTHORIZATION,
            header::ORIGIN,
            header::REFERER,
        ] {
            if let Some(value) = self.headers.get(&name) {
                request = request.header(name, value);
            }
        }
        if let Some(csrf) = csrf {
            request = request.header("x-csrf-token", csrf);
        }
        let encoded = if let Some(body) = body {
            request = request.header(header::CONTENT_TYPE, "application/json");
            serde_json::to_vec(&body).map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?
        } else {
            Vec::new()
        };
        let request = request
            .body(Body::from(encoded))
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        let response = api_router(self.state.clone())
            .oneshot(request)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        if let Some(cookie) = response.headers().get(header::SET_COOKIE) {
            self.cookie = Some(cookie.clone());
        }
        let status = response.status();
        let bytes = to_bytes(response.into_body(), BODY_LIMIT)
            .await
            .map_err(|_| error(StatusCode::BAD_GATEWAY))?;
        let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        Ok(ApiReply { status, value })
    }

    async fn get(&mut self, path: &str) -> Result<Value, PageError> {
        let reply = self.call(Method::GET, path, None, None).await?;
        if reply.status == StatusCode::UNAUTHORIZED {
            return Err(PageError::Login);
        }
        if !reply.status.is_success() {
            return Err(error(reply.status));
        }
        Ok(reply.value)
    }

    async fn collection(&mut self, base: &str) -> Result<Vec<Value>, PageError> {
        let mut records = Vec::new();
        for page in 1..=5 {
            let value = self
                .get(&format!("{base}?page={page}&per_page=100"))
                .await?;
            let count = value
                .pointer("/meta/total_count")
                .and_then(Value::as_u64)
                .ok_or_else(|| error(StatusCode::BAD_GATEWAY))? as usize;
            let data = value
                .get("data")
                .and_then(Value::as_array)
                .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
            records.extend(data.iter().cloned());
            if records.len() >= count {
                return Ok(records);
            }
            if data.is_empty() || records.len() >= COLLECTION_LIMIT {
                break;
            }
        }
        Err(error(StatusCode::SERVICE_UNAVAILABLE))
    }

    async fn household(&mut self, slug: &str) -> Result<(i64, String), PageError> {
        let value = self.get("/api/v1/auth/households").await?;
        value
            .get("data")
            .and_then(Value::as_array)
            .and_then(|rows| rows.iter().find(|row| field(row, "slug") == slug))
            .and_then(|row| Some((row.get("id")?.as_i64()?, field(row, "name").to_owned())))
            .ok_or_else(|| error(StatusCode::NOT_FOUND))
    }
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}

fn numeric(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(Value::as_i64)
}

fn card(value: &Value) -> Option<MedicationCard> {
    Some(MedicationCard {
        id: numeric(value, "id")?,
        name: value
            .get("display_name")
            .and_then(Value::as_str)
            .or_else(|| value.get("name").and_then(Value::as_str))
            .unwrap_or("Medication")
            .to_owned(),
        supply: field(value, "current_supply")
            .strip_suffix(".0")
            .unwrap_or(field(value, "current_supply"))
            .to_owned(),
        unit: field(value, "dose_unit").to_owned(),
    })
}

async fn medications(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Response {
    let mut api = match WebApi::authenticated(state, headers).await {
        Ok(api) => api,
        Err(response) => return response.response(),
    };
    let (household_id, household_name) = match api.household(&slug).await {
        Ok(value) => value,
        Err(response) => return response.response(),
    };
    let records = match api
        .collection(&format!("/api/v1/households/{household_id}/medications"))
        .await
    {
        Ok(value) => value,
        Err(response) => return response.response(),
    };
    page(
        medtracker_web::render_medication_list(
            &household_name,
            &slug,
            &api.csrf,
            records.iter().filter_map(card).collect(),
        ),
        api.cookie,
    )
}

async fn detail(
    api: &mut WebApi,
    household_id: i64,
    id: &str,
) -> Result<(MedicationDetail, Vec<MedicationCard>), PageError> {
    let base = format!("/api/v1/households/{household_id}");
    let medication = api.get(&format!("{base}/medications/{id}")).await?;
    let row = medication
        .get("data")
        .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
    let selected = card(row).ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
    let inventory = api.collection(&format!("{base}/medications")).await?;
    let inventory: HashMap<i64, MedicationCard> = inventory
        .iter()
        .filter_map(card)
        .map(|candidate| (candidate.id, candidate))
        .collect();
    let people = api.collection(&format!("{base}/people")).await?;
    let people: HashMap<i64, String> = people
        .iter()
        .filter_map(|person| Some((numeric(person, "id")?, field(person, "name").to_owned())))
        .collect();
    let mut sources = Vec::new();
    for (kind, resource) in [
        ("person_medication", "person_medications"),
        ("schedule", "schedules"),
    ] {
        for source in api.collection(&format!("{base}/{resource}")).await? {
            if numeric(&source, "medication_id") != Some(selected.id)
                || source.get("active").and_then(Value::as_bool) != Some(true)
                || source.get("paused").and_then(Value::as_bool) == Some(true)
            {
                continue;
            }
            let Some(source_id) = numeric(&source, "id") else {
                continue;
            };
            let Some(person_id) = numeric(&source, "person_id") else {
                continue;
            };
            let Some(person_name) = people.get(&person_id) else {
                continue;
            };
            let eligible_stock_ids: Vec<i64> = source
                .get("eligible_stock_medication_ids")
                .and_then(Value::as_array)
                .map(|ids| ids.iter().filter_map(Value::as_i64).collect())
                .unwrap_or_default();
            if eligible_stock_ids
                .iter()
                .any(|id| !inventory.contains_key(id))
            {
                return Err(error(StatusCode::BAD_GATEWAY));
            }
            sources.push(DoseSource {
                id: source_id,
                kind: kind.to_owned(),
                person_name: person_name.clone(),
                amount: if kind == "schedule" {
                    String::new()
                } else {
                    field(&source, "dose_amount").to_owned()
                },
                unit: if kind == "schedule" {
                    String::new()
                } else {
                    field(&source, "dose_unit").to_owned()
                },
                portable_id: field(&source, "portable_id").to_owned(),
                can_record: source.get("can_record").and_then(Value::as_bool) == Some(true),
                eligible_stock_ids,
            });
        }
    }
    let mut stock_ids = Vec::new();
    for source in &sources {
        for id in &source.eligible_stock_ids {
            if !stock_ids.contains(id) {
                stock_ids.push(*id);
            }
        }
    }
    let options: Vec<MedicationCard> = stock_ids
        .into_iter()
        .filter_map(|id| inventory.get(&id).cloned())
        .collect();
    Ok((
        MedicationDetail {
            id: selected.id,
            name: selected.name,
            description: field(row, "description").to_owned(),
            supply: selected.supply,
            unit: selected.unit,
            location: "Household inventory".to_owned(),
            sources,
        },
        options,
    ))
}

fn now_local() -> String {
    let timezone = configured_timezone();
    Utc::now()
        .with_timezone(&timezone)
        .format("%Y-%m-%dT%H:%M")
        .to_string()
}

fn taken_at(value: &str) -> Option<String> {
    let time = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M").ok()?;
    let timezone = configured_timezone();
    let time = match timezone.from_local_datetime(&time) {
        LocalResult::Single(value) => value,
        _ => return None,
    };
    Some(time.to_rfc3339())
}

fn configured_timezone() -> chrono_tz::Tz {
    std::env::var("TZ")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(chrono_tz::UTC)
}

fn is_today_in_zone(taken_at: &str, today: NaiveDate, timezone: chrono_tz::Tz) -> bool {
    DateTime::parse_from_rfc3339(taken_at)
        .ok()
        .is_some_and(|time| time.with_timezone(&timezone).date_naive() == today)
}

async fn medication(
    State(state): State<AppState>,
    Path((slug, id)): Path<(String, String)>,
    Query(query): Query<MedicationQuery>,
    headers: HeaderMap,
) -> Response {
    render_detail(
        state,
        slug,
        id,
        headers,
        DetailOutcome::normal(query.logged),
    )
    .await
}

#[derive(Deserialize)]
struct MedicationQuery {
    logged: Option<String>,
}

struct DetailOutcome {
    logged: Option<String>,
    client_uuid: Option<String>,
    error_state: Option<(String, DoseFormState)>,
    status: StatusCode,
}

impl DetailOutcome {
    fn normal(logged: Option<String>) -> Self {
        Self {
            logged,
            client_uuid: None,
            error_state: None,
            status: StatusCode::OK,
        }
    }

    fn rejected(
        status: StatusCode,
        client_uuid: Option<String>,
        message: &str,
        form: DoseFormState,
    ) -> Self {
        Self {
            logged: None,
            client_uuid,
            error_state: Some((message.to_owned(), form)),
            status,
        }
    }
}

async fn render_detail(
    state: AppState,
    slug: String,
    id: String,
    headers: HeaderMap,
    outcome: DetailOutcome,
) -> Response {
    let mut api = match WebApi::authenticated(state, headers).await {
        Ok(api) => api,
        Err(response) => return response.response(),
    };
    let (household_id, household_name) = match api.household(&slug).await {
        Ok(value) => value,
        Err(response) => return response.response(),
    };
    let (medication, options) = match detail(&mut api, household_id, &id).await {
        Ok(value) => value,
        Err(response) => return response.response(),
    };
    let notice = if let Some(take_id) = outcome.logged {
        let takes = match api
            .collection(&format!(
                "/api/v1/households/{household_id}/medication_takes"
            ))
            .await
        {
            Ok(value) => value,
            Err(response) => return response.response(),
        };
        takes
            .iter()
            .any(|take| {
                field(take, "portable_id") == take_id
                    && numeric(take, "medication_id") == Some(medication.id)
            })
            .then_some("Medication taken successfully.".to_owned())
    } else {
        outcome
            .error_state
            .as_ref()
            .map(|(message, _)| message.clone())
    };
    let form_state = outcome.error_state.map(|(_, state)| state);
    let uuid = outcome
        .client_uuid
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    page_status(
        medtracker_web::render_medication_detail(
            &household_name,
            &slug,
            &api.csrf,
            medication,
            options,
            &now_local(),
            &uuid,
            notice.as_deref(),
            form_state,
        ),
        api.cookie,
        outcome.status,
    )
}

#[derive(Deserialize)]
struct DashboardQuery {
    dashboard_person_id: Option<i64>,
}

async fn dashboard(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<DashboardQuery>,
    headers: HeaderMap,
) -> Response {
    let mut api = match WebApi::authenticated(state, headers).await {
        Ok(api) => api,
        Err(response) => return response.response(),
    };
    let (household_id, household_name) = match api.household(&slug).await {
        Ok(value) => value,
        Err(response) => return response.response(),
    };
    let base = format!("/api/v1/households/{household_id}");
    let takes = match api.collection(&format!("{base}/medication_takes")).await {
        Ok(value) => value,
        Err(response) => return response.response(),
    };
    let inventory = match api.collection(&format!("{base}/medications")).await {
        Ok(value) => value,
        Err(response) => return response.response(),
    };
    let names: HashMap<i64, String> = inventory
        .iter()
        .filter_map(card)
        .map(|row| (row.id, row.name))
        .collect();
    let timezone = configured_timezone();
    let today = Utc::now().with_timezone(&timezone).date_naive();
    let history = takes
        .iter()
        .filter(|row| {
            query
                .dashboard_person_id
                .is_none_or(|id| numeric(row, "person_id") == Some(id))
        })
        .filter(|row| is_today_in_zone(field(row, "taken_at"), today, timezone))
        .filter_map(|row| {
            Some(HistoryRow {
                medication_name: names.get(&numeric(row, "medication_id")?)?.clone(),
                amount: field(row, "dose_amount").to_owned(),
                unit: field(row, "dose_unit").to_owned(),
            })
        })
        .collect();
    page(
        medtracker_web::render_journey_dashboard(&household_name, &slug, &api.csrf, history),
        api.cookie,
    )
}

async fn record_dose(
    State(state): State<AppState>,
    Path((slug, id)): Path<(String, String)>,
    headers: HeaderMap,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    if !oauth::trusted_cookie_origin(&state, &headers) {
        return failure(StatusCode::FORBIDDEN);
    }
    let mut api = match WebApi::authenticated(state.clone(), headers.clone()).await {
        Ok(api) => api,
        Err(response) => return response.response(),
    };
    if fields.get("authenticity_token") != Some(&api.csrf) {
        return failure(StatusCode::FORBIDDEN);
    }
    let (household_id, _) = match api.household(&slug).await {
        Ok(value) => value,
        Err(response) => return response.response(),
    };
    let bad = || failure(StatusCode::UNPROCESSABLE_ENTITY);
    let amount = fields.get("dose_amount").map(String::as_str).unwrap_or("");
    let form_state = DoseFormState {
        source_type: fields.get("source_type").cloned().unwrap_or_default(),
        source_id: fields.get("source_id").cloned().unwrap_or_default(),
        dose_amount: amount.to_owned(),
        dose_unit: fields.get("dose_unit").cloned().unwrap_or_default(),
        taken_at: fields.get("taken_at").cloned().unwrap_or_default(),
        stock_id: fields
            .get("taken_from_medication_id")
            .cloned()
            .unwrap_or_default(),
    };
    let Some(time) = fields.get("taken_at").and_then(|value| taken_at(value)) else {
        return render_detail(
            state,
            slug,
            id,
            headers,
            DetailOutcome::rejected(
                StatusCode::UNPROCESSABLE_ENTITY,
                fields.get("client_uuid").cloned(),
                "Taken at is invalid.",
                form_state,
            ),
        )
        .await;
    };
    let Some(csrf) = fields.get("authenticity_token") else {
        return failure(StatusCode::FORBIDDEN);
    };
    let Some(source_type) = fields.get("source_type") else {
        return bad();
    };
    let Some(source_id) = fields.get("source_id") else {
        return bad();
    };
    let Some(client_uuid) = fields.get("client_uuid") else {
        return bad();
    };
    let source_resource = match source_type.as_str() {
        "person_medication" => "person_medications",
        "schedule" => "schedules",
        _ => return bad(),
    };
    let medication = match api
        .get(&format!(
            "/api/v1/households/{household_id}/medications/{id}"
        ))
        .await
    {
        Ok(value) => value,
        Err(response) => return response.response(),
    };
    let source = match api
        .get(&format!(
            "/api/v1/households/{household_id}/{source_resource}/{source_id}"
        ))
        .await
    {
        Ok(value) => value,
        Err(response) => return response.response(),
    };
    if numeric(&source["data"], "medication_id") != numeric(&medication["data"], "id") {
        return render_detail(
            state,
            slug,
            id,
            headers,
            DetailOutcome::rejected(
                StatusCode::FORBIDDEN,
                Some(client_uuid.clone()),
                "This source does not belong to this medication.",
                form_state,
            ),
        )
        .await;
    }
    let stock = fields
        .get("taken_from_medication_id")
        .and_then(|value| value.parse::<i64>().ok());
    let mut attributes = json!({
        "client_uuid": client_uuid, "source_type": source_type, "source_id": source_id,
        "taken_at": time,
        "taken_from_medication_id": stock
    });
    if source_type == "person_medication" {
        attributes["dose_amount"] = json!(amount);
        attributes["dose_unit"] = json!(fields.get("dose_unit"));
    }
    let body = json!({"medication_take": attributes});
    let reply = match api
        .call(
            Method::POST,
            &format!("/api/v1/households/{household_id}/medication_takes"),
            Some(body),
            Some(csrf),
        )
        .await
    {
        Ok(reply) => reply,
        Err(response) => return response.response(),
    };
    if reply.status.is_success() {
        let take_id = field(&reply.value["data"], "portable_id");
        if take_id.is_empty() {
            return failure(StatusCode::BAD_GATEWAY);
        }
        let location = format!("/households/{slug}/medications/{id}?logged={take_id}");
        let mut response = (
            StatusCode::SEE_OTHER,
            [
                (header::LOCATION, location),
                (header::CACHE_CONTROL, "no-store".to_owned()),
            ],
        )
            .into_response();
        if let Some(cookie) = api.cookie {
            response.headers_mut().append(header::SET_COOKIE, cookie);
        }
        return response;
    }
    let notice = if reply.status == StatusCode::UNPROCESSABLE_ENTITY {
        "Invalid dose configured"
    } else if reply.status == StatusCode::FORBIDDEN {
        "You cannot record this dose."
    } else if reply.status == StatusCode::CONFLICT {
        "This dose request conflicts with a previous record."
    } else {
        return failure(reply.status);
    };
    render_detail(
        state,
        slug,
        id,
        headers,
        DetailOutcome::rejected(reply.status, Some(client_uuid.clone()), notice, form_state),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::{html_cookie_only, is_today_in_zone, page_status};
    use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
    use chrono::NaiveDate;

    #[test]
    fn london_today_includes_late_utc_previous_day() {
        let today = NaiveDate::from_ymd_opt(2026, 3, 30).unwrap();
        assert!(is_today_in_zone(
            "2026-03-29T23:30:00Z",
            today,
            chrono_tz::Europe::London
        ));
        assert!(!is_today_in_zone(
            "2026-03-29T22:30:00Z",
            today,
            chrono_tz::Europe::London
        ));
    }

    #[test]
    fn rejected_form_keeps_its_validation_status() {
        for status in [
            StatusCode::UNPROCESSABLE_ENTITY,
            StatusCode::FORBIDDEN,
            StatusCode::CONFLICT,
        ] {
            let response = page_status("Invalid dose configured".to_owned(), None, status);
            assert_eq!(response.status(), status);
        }
    }

    #[test]
    fn html_cookie_session_rejects_any_explicit_authorization() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("medtracker_session=fake"),
        );
        assert!(html_cookie_only(&headers));
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer fake"),
        );
        assert!(!html_cookie_only(&headers));
    }
}
