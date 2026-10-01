use crate::{api_router, oauth, restricted_role, AppState};
use axum::body::{to_bytes, Body};
use axum::extract::{Form, Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use chrono::{DateTime, Datelike, Duration, LocalResult, NaiveDateTime, TimeZone, Timelike, Utc};
use medtracker_web::dashboard::{
    calculate_metrics, calculate_prn, is_same_local_day, DashboardHistory, DashboardPage,
    DashboardPerson, DashboardStock, DashboardTask, DashboardTaskRow, PrnInput, TaskState,
};
use medtracker_web::household_i18n::Locale;
use medtracker_web::{DoseFormState, DoseSource, MedicationCard, MedicationDetail};
use sea_orm::TransactionTrait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use tower::ServiceExt;

const BODY_LIMIT: usize = 1_048_576;
const COLLECTION_LIMIT: usize = 500;

mod locations;
mod medications;
mod people;

pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(people::routes())
        .merge(locations::routes())
        .merge(medications::routes())
        .route("/household.css", get(household_styles))
        .route("/households/{slug}/dashboard", get(dashboard))
        .route("/households/{slug}/medications", get(medications))
        .route("/households/{slug}/medications/{id}", get(medication))
        .route(
            "/households/{slug}/medications/{id}/doses",
            post(record_dose),
        )
        .route("/medication.css", get(styles))
        .route("/medication.js", get(script))
        .route("/dashboard.css", get(dashboard_styles))
        .route("/dashboard.js", get(dashboard_script))
        .route("/leptodon.css", get(leptodon_styles))
        .route("/dashboard-hydrate.js", get(dashboard_hydrate_script))
        .route("/dashboard-hydrate-pkg.js", get(dashboard_hydrate_package))
        .route("/dashboard-hydrate.wasm", get(dashboard_hydrate_wasm))
        .route("/sw.js", get(dashboard_worker))
        .route("/manifest.webmanifest", get(dashboard_manifest))
        .route("/offline", get(dashboard_offline))
        .route("/reconnect", get(dashboard_reconnect))
        .route("/icons/icon-192.png", get(dashboard_icon_192))
        .route("/icons/icon-512.png", get(dashboard_icon_512))
        .route("/fonts/inter-regular.woff2", get(inter_regular))
        .route("/fonts/inter-500.woff2", get(inter_500))
        .route("/fonts/inter-800.woff2", get(inter_800))
        .route("/fonts/inter-600.woff2", get(inter_600))
        .route("/fonts/inter-700.woff2", get(inter_700))
}

async fn household_styles() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        include_str!("../../web/src/household.css"),
    )
        .into_response()
}

fn redirect(location: impl AsRef<str>, cookie: Option<HeaderValue>) -> Response {
    let mut response = (
        StatusCode::SEE_OTHER,
        [
            (header::LOCATION, location.as_ref().to_owned()),
            (header::CACHE_CONTROL, "no-store".to_owned()),
        ],
    )
        .into_response();
    if let Some(cookie) = cookie {
        response.headers_mut().append(header::SET_COOKIE, cookie);
    }
    response
}

async fn dashboard_styles() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        include_str!("../../web/src/dashboard.css"),
    )
        .into_response()
}

async fn leptodon_styles() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        include_str!("../../ui-preview/public/preview.css"),
    )
        .into_response()
}

async fn dashboard_hydrate_script() -> Response {
    let script = include_str!("../../web/src/assets/dashboard-hydrate.js")
        .replace(
            "__PKG_VERSION__",
            &format!(
                "{:016x}",
                medtracker_web::dashboard::DASHBOARD_HYDRATE_PKG_VERSION
            ),
        )
        .replace(
            "__WASM_VERSION__",
            &format!(
                "{:016x}",
                *medtracker_web::dashboard::DASHBOARD_HYDRATE_WASM_VERSION
            ),
        );
    (
        [
            (header::CONTENT_TYPE, "text/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        script,
    )
        .into_response()
}

async fn dashboard_hydrate_package() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        include_str!("../../ui-preview/public/pkg/medtracker_ui_preview.js"),
    )
        .into_response()
}

async fn dashboard_hydrate_wasm() -> Response {
    (
        [
            (header::CONTENT_TYPE, "application/wasm"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        include_bytes!("../../ui-preview/public/pkg/medtracker_ui_preview_bg.wasm").as_slice(),
    )
        .into_response()
}

async fn dashboard_script() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        include_str!("../../web/src/assets/dashboard-register.js"),
    )
        .into_response()
}

async fn dashboard_worker() -> Response {
    let worker = include_str!("../../web/src/assets/dashboard-sw.js")
        .replace(
            "__CSS_VERSION__",
            &format!("{:016x}", medtracker_web::dashboard::DASHBOARD_CSS_VERSION),
        )
        .replace(
            "__JS_VERSION__",
            &format!("{:016x}", medtracker_web::dashboard::DASHBOARD_JS_VERSION),
        )
        .replace(
            "__SW_VERSION__",
            &format!("{:016x}", medtracker_web::dashboard::DASHBOARD_SW_VERSION),
        );
    (
        [
            (header::CONTENT_TYPE, "text/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        worker,
    )
        .into_response()
}

async fn dashboard_manifest() -> Response {
    (
        [
            (header::CONTENT_TYPE, "application/manifest+json"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        include_str!("../../web/src/assets/dashboard-manifest.webmanifest"),
    )
        .into_response()
}

async fn dashboard_offline() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        include_str!("../../web/src/assets/dashboard-offline.html"),
    )
        .into_response()
}

async fn dashboard_reconnect() -> Response {
    (
        StatusCode::FOUND,
        [
            (header::LOCATION, "/login"),
            (header::CACHE_CONTROL, "no-store"),
        ],
    )
        .into_response()
}

async fn dashboard_icon_192() -> Response {
    (
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        include_bytes!("../../web/src/assets/dashboard-icon-192.png").as_slice(),
    )
        .into_response()
}

async fn dashboard_icon_512() -> Response {
    (
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        include_bytes!("../../web/src/assets/dashboard-icon-512.png").as_slice(),
    )
        .into_response()
}

async fn inter_regular() -> Response {
    (
        [
            (header::CONTENT_TYPE, "font/woff2"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        include_bytes!("../../web/src/assets/inter-v20-latin-regular.woff2").as_slice(),
    )
        .into_response()
}

async fn inter_500() -> Response {
    (
        [
            (header::CONTENT_TYPE, "font/woff2"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        include_bytes!("../../web/src/assets/inter-v20-latin-500.woff2").as_slice(),
    )
        .into_response()
}

async fn inter_800() -> Response {
    (
        [
            (header::CONTENT_TYPE, "font/woff2"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        include_bytes!("../../web/src/assets/inter-v20-latin-800.woff2").as_slice(),
    )
        .into_response()
}

async fn inter_600() -> Response {
    (
        [
            (header::CONTENT_TYPE, "font/woff2"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        include_bytes!("../../web/src/assets/inter-v20-latin-600.woff2").as_slice(),
    )
        .into_response()
}

async fn inter_700() -> Response {
    (
        [
            (header::CONTENT_TYPE, "font/woff2"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        include_bytes!("../../web/src/assets/inter-v20-latin-700.woff2").as_slice(),
    )
        .into_response()
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

fn dashboard_page(body: String, cookie: Option<HeaderValue>) -> Response {
    let mut response = page(body, cookie);
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'none'; style-src 'self'; script-src 'self' 'wasm-unsafe-eval'; connect-src 'self'; font-src 'self'; worker-src 'self'; manifest-src 'self'; img-src 'self'; form-action 'self'; base-uri 'none'; frame-ancestors 'none'"),
    );
    response
}

fn page_status(body: String, cookie: Option<HeaderValue>, status: StatusCode) -> Response {
    let mut response = (
        status,
        [
            (header::CACHE_CONTROL, "private, no-store"),
            (header::CONTENT_SECURITY_POLICY, "default-src 'none'; style-src 'self'; script-src 'self'; font-src 'self'; worker-src 'self'; form-action 'self'; base-uri 'none'; frame-ancestors 'none'"),
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
    etag: Option<String>,
}

struct WebApi {
    state: AppState,
    headers: HeaderMap,
    csrf: String,
    cookie: Option<HeaderValue>,
    locale: Locale,
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
        let locale_cookie = headers
            .get(header::COOKIE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| {
                value
                    .split(';')
                    .find_map(|entry| entry.trim().strip_prefix("medtracker_locale="))
            });
        let locale = Locale::resolve(
            locale_cookie,
            headers
                .get(header::ACCEPT_LANGUAGE)
                .and_then(|value| value.to_str().ok()),
        );
        Ok(Self {
            state,
            headers,
            csrf: session.csrf,
            cookie: None,
            locale,
        })
    }

    async fn call(
        &mut self,
        method: Method,
        path: &str,
        body: Option<Value>,
        csrf: Option<&str>,
    ) -> Result<ApiReply, PageError> {
        self.call_with_headers(method, path, body, csrf, &HeaderMap::new())
            .await
    }

    async fn call_with_headers(
        &mut self,
        method: Method,
        path: &str,
        body: Option<Value>,
        csrf: Option<&str>,
        extra: &HeaderMap,
    ) -> Result<ApiReply, PageError> {
        let mut request = Request::builder().method(method).uri(path);
        for name in [
            header::IF_MATCH,
            header::HeaderName::from_static("idempotency-key"),
        ] {
            if let Some(value) = extra.get(&name) {
                request = request.header(name, value);
            }
        }
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
        let etag = response
            .headers()
            .get(header::ETAG)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let bytes = to_bytes(response.into_body(), BODY_LIMIT)
            .await
            .map_err(|_| error(StatusCode::BAD_GATEWAY))?;
        let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        Ok(ApiReply {
            status,
            value,
            etag,
        })
    }

    async fn get(&mut self, path: &str) -> Result<Value, PageError> {
        Ok(self.get_reply(path).await?.value)
    }

    async fn capabilities(&mut self, household_id: i64) -> Result<Value, PageError> {
        self.get(&format!(
            "/api/v1/households/{household_id}/ui_capabilities"
        ))
        .await
    }

    async fn get_reply(&mut self, path: &str) -> Result<ApiReply, PageError> {
        let reply = self.call(Method::GET, path, None, None).await?;
        if reply.status == StatusCode::UNAUTHORIZED {
            return Err(PageError::Login);
        }
        if !reply.status.is_success() {
            return Err(error(reply.status));
        }
        Ok(reply)
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

    async fn dashboard_takes(
        &mut self,
        base: &str,
        selected: &[i64],
        now: DateTime<Utc>,
        timezone: chrono_tz::Tz,
    ) -> Result<Vec<Value>, PageError> {
        let today = now.with_timezone(&timezone).date_naive();
        let week_start = today - Duration::days(i64::from(today.weekday().num_days_from_monday()));
        let month_start = today.with_day(1).unwrap();
        let cutoff = week_start.min(month_start);
        let mut recent = Vec::new();
        let mut latest_older: HashMap<(bool, i64), (DateTime<Utc>, Value)> = HashMap::new();
        let mut seen = 0usize;
        let mut page = 1usize;
        loop {
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
            for row in data {
                if !numeric(row, "person_id").is_some_and(|id| selected.contains(&id)) {
                    continue;
                }
                let Some(timestamp) = dashboard_time(field(row, "taken_at")) else {
                    continue;
                };
                if timestamp.with_timezone(&timezone).date_naive() >= cutoff {
                    recent.push(row.clone());
                    continue;
                }
                let source = numeric(row, "schedule_id")
                    .map(|id| (true, id))
                    .or_else(|| numeric(row, "person_medication_id").map(|id| (false, id)));
                if let Some(source) = source {
                    let replace = latest_older
                        .get(&source)
                        .is_none_or(|(latest, _)| timestamp > *latest);
                    if replace {
                        latest_older.insert(source, (timestamp, row.clone()));
                    }
                }
            }
            seen += data.len();
            if seen >= count {
                recent.extend(latest_older.into_values().map(|(_, row)| row));
                return Ok(recent);
            }
            if data.is_empty() {
                return Err(error(StatusCode::SERVICE_UNAVAILABLE));
            }
            page = page
                .checked_add(1)
                .ok_or_else(|| error(StatusCode::SERVICE_UNAVAILABLE))?;
        }
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
    let capabilities = match api.capabilities(household_id).await {
        Ok(value) => value,
        Err(response) => return response.response(),
    };
    let can_create = match capabilities
        .pointer("/data/medications/create")
        .and_then(Value::as_bool)
    {
        Some(value) => value,
        None => return failure(StatusCode::BAD_GATEWAY),
    };
    page(
        medtracker_web::render_medication_list_with_management(
            &household_name,
            &slug,
            &api.csrf,
            records.iter().filter_map(card).collect(),
            can_create,
            api.locale,
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
    let capabilities = match api.capabilities(household_id).await {
        Ok(value) => value,
        Err(response) => return response.response(),
    };
    let can_edit = match capabilities
        .pointer("/data/medications/update")
        .and_then(Value::as_bool)
    {
        Some(value) => value,
        None => return failure(StatusCode::BAD_GATEWAY),
    };
    page_status(
        medtracker_web::render_medication_detail_with_management(
            medtracker_web::MedicationDetailRender {
                household_name: &household_name,
                slug: &slug,
                csrf: &api.csrf,
                medication,
                stock_options: options,
                taken_at: &now_local(),
                client_uuid: &uuid,
                notice: notice.as_deref(),
                form_state,
            },
            can_edit,
            api.locale,
        ),
        api.cookie,
        outcome.status,
    )
}

#[derive(Deserialize)]
struct DashboardQuery {
    dashboard_person_id: Option<String>,
    contract_fail_dashboard_read: Option<String>,
}

fn dashboard_read_error(cookie: Option<HeaderValue>) -> Response {
    page_status(format!("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><link rel=\"stylesheet\" href=\"/dashboard.css?v={:016x}\"><title>Dashboard unavailable | MedTracker</title></head><body><main class=\"dashboard-error\"><h1>Dashboard unavailable</h1><p>We could not load your dashboard. Please reconnect and try again.</p><a href=\"/reconnect\">Try again</a></main></body></html>", medtracker_web::dashboard::DASHBOARD_CSS_VERSION), cookie, StatusCode::SERVICE_UNAVAILABLE)
}

pub(crate) fn dashboard_now() -> DateTime<Utc> {
    if cfg!(debug_assertions) && std::env::var_os("CONTRACT_PROJECT").is_some() {
        if let Some(now) = std::env::var("CONTRACT_DASHBOARD_NOW")
            .ok()
            .and_then(|value| DateTime::parse_from_rfc3339(&value).ok())
        {
            return now.with_timezone(&Utc);
        }
    }
    Utc::now()
}

fn dashboard_time(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

#[cfg(test)]
fn is_today_in_zone(value: &str, today: chrono::NaiveDate, timezone: chrono_tz::Tz) -> bool {
    dashboard_time(value)
        .is_some_and(|timestamp| timestamp.with_timezone(&timezone).date_naive() == today)
}

fn dashboard_dose(source: &Value) -> String {
    let amount = field(source, "dose_amount");
    let unit = field(source, "dose_unit");
    format!("{amount} {unit}").trim().to_owned()
}

fn schedule_in_local_range(source: &Value, today: chrono::NaiveDate) -> bool {
    let start = chrono::NaiveDate::parse_from_str(field(source, "start_date"), "%Y-%m-%d").ok();
    let end = chrono::NaiveDate::parse_from_str(field(source, "end_date"), "%Y-%m-%d").ok();
    start
        .zip(end)
        .is_some_and(|(start, end)| (start..=end).contains(&today))
}

fn taper_step_on(source: &Value, today: chrono::NaiveDate) -> Option<&Value> {
    source
        .pointer("/schedule_config/taper_steps")
        .and_then(Value::as_array)?
        .iter()
        .find(|step| {
            let start =
                chrono::NaiveDate::parse_from_str(field(step, "start_date"), "%Y-%m-%d").ok();
            let end = chrono::NaiveDate::parse_from_str(field(step, "end_date"), "%Y-%m-%d").ok();
            start
                .zip(end)
                .is_some_and(|(start, end)| (start..=end).contains(&today))
        })
}

fn schedule_applies_on(source: &Value, today: chrono::NaiveDate) -> bool {
    schedule_in_local_range(source, today)
        && (field(source, "schedule_type") != "tapering" || taper_step_on(source, today).is_some())
}

fn source_active_on(kind: &str, source: &Value, today: chrono::NaiveDate) -> bool {
    if kind == "schedules" {
        source.get("paused").and_then(Value::as_bool) == Some(false)
            && schedule_applies_on(source, today)
    } else {
        source.get("active").and_then(Value::as_bool) == Some(true)
    }
}

fn dashboard_sources(
    schedules: Vec<Value>,
    assignments: Vec<Value>,
    visible: &[i64],
    today: chrono::NaiveDate,
) -> Vec<(String, Value)> {
    schedules
        .into_iter()
        .map(|value| ("schedules".to_owned(), value))
        .chain(
            assignments
                .into_iter()
                .map(|value| ("person_medications".to_owned(), value)),
        )
        .filter(|(_, source)| numeric(source, "person_id").is_some_and(|id| visible.contains(&id)))
        .filter(|(kind, source)| {
            source_active_on(kind, source, today)
                || (source
                    .get("current_pause_period")
                    .is_some_and(|value| !value.is_null())
                    && (kind != "schedules" || schedule_applies_on(source, today)))
        })
        .collect()
}

fn source_takes(takes: &[Value], kind: &str, id: i64) -> Vec<DateTime<Utc>> {
    let key = if kind == "schedules" {
        "schedule_id"
    } else {
        "person_medication_id"
    };
    takes
        .iter()
        .filter(|row| numeric(row, key) == Some(id))
        .filter_map(|row| dashboard_time(field(row, "taken_at")))
        .collect()
}

fn stock_matches(
    source: &Value,
    permitted_medications: &HashMap<i64, std::collections::HashSet<i64>>,
    medications: &HashMap<i64, &Value>,
    household_manager: bool,
) -> bool {
    let Some(source_medication) =
        numeric(source, "medication_id").and_then(|id| medications.get(&id).copied())
    else {
        return false;
    };
    let allowed_ids = numeric(source, "person_id").and_then(|id| permitted_medications.get(&id));
    medications.iter().any(|(id, row)| {
        (household_manager || allowed_ids.is_some_and(|ids| ids.contains(id)))
            && field(row, "name") == field(source_medication, "name")
            && field(row, "dose_amount") == field(source_medication, "dose_amount")
            && field(row, "dose_unit") == field(source_medication, "dose_unit")
            && row.get("out_of_stock").and_then(Value::as_bool) != Some(true)
    })
}

fn source_config_on(source: &Value, today: chrono::NaiveDate) -> Option<&Value> {
    let config = source.get("schedule_config")?;
    if field(source, "schedule_type") != "tapering" {
        return Some(config);
    }
    taper_step_on(source, today).or(Some(config))
}

fn source_limit(source: &Value, today: chrono::NaiveDate) -> Option<usize> {
    let count = |value: &Value| {
        value
            .as_u64()
            .and_then(|value| usize::try_from(value).ok())
            .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
    };
    source_config_on(source, today)
        .and_then(|config| {
            ["max_daily_doses", "max_doses", "max"]
                .iter()
                .find_map(|name| config.get(*name).and_then(count))
        })
        .or_else(|| source.get("max_daily_doses").and_then(count))
}

fn source_interval(source: &Value, today: chrono::NaiveDate) -> Option<f64> {
    let hours = |value: &Value| {
        value
            .as_f64()
            .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
    };
    source_config_on(source, today)
        .and_then(|config| {
            ["min_hours_between_doses", "min_hours", "minimum_hours"]
                .iter()
                .find_map(|name| config.get(*name).and_then(hours))
        })
        .or_else(|| source.get("min_hours_between_doses").and_then(hours))
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
    if cfg!(debug_assertions)
        && std::env::var_os("CONTRACT_PROJECT").is_some()
        && query.contract_fail_dashboard_read.as_deref() == Some("people")
    {
        return dashboard_read_error(api.cookie);
    }
    let base = format!("/api/v1/households/{household_id}");
    let visible = match api.collection(&format!("{base}/people")).await {
        Ok(value) => value,
        Err(_) => return dashboard_read_error(api.cookie),
    };
    let selectable_people: Vec<(i64, String)> = visible
        .iter()
        .filter_map(|row| Some((numeric(row, "id")?, field(row, "name").to_owned())))
        .collect();
    let profile = match api.get(&format!("{base}/profile")).await {
        Ok(value) => value,
        Err(_) => return dashboard_read_error(api.cookie),
    };
    let me = match api.get(&format!("{base}/me")).await {
        Ok(value) => value,
        Err(_) => return dashboard_read_error(api.cookie),
    };
    let household_manager = matches!(
        me.pointer("/data/membership_role").and_then(Value::as_str),
        Some("owner" | "administrator")
    );
    let mobile_shortcuts = profile
        .pointer("/data/mobile_shortcuts")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_else(|| vec!["dashboard".into(), "inventory".into(), "finder".into()]);
    let account_person_id = profile
        .pointer("/data/person_id")
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<i64>().ok());
    let selection = query.dashboard_person_id.as_deref().unwrap_or("");
    let selected = if selection == "all" {
        selectable_people
            .iter()
            .map(|(id, _)| *id)
            .collect::<Vec<_>>()
    } else if selection.is_empty() {
        account_person_id
            .filter(|id| {
                selectable_people
                    .iter()
                    .any(|(visible_id, _)| visible_id == id)
            })
            .or_else(|| selectable_people.first().map(|(id, _)| *id))
            .map(|id| vec![id])
            .unwrap_or_default()
    } else if let Ok(id) = selection.parse::<i64>() {
        if selectable_people
            .iter()
            .any(|(candidate, _)| *candidate == id)
        {
            vec![id]
        } else {
            return failure(StatusCode::NOT_FOUND);
        }
    } else {
        return failure(StatusCode::BAD_REQUEST);
    };
    let selected_id = if selection.is_empty() {
        selected
            .first()
            .map(i64::to_string)
            .unwrap_or_else(|| "all".to_owned())
    } else {
        selection.to_owned()
    };
    let selected_name = if selected_id == "all" {
        "All Family".to_owned()
    } else {
        selectable_people
            .iter()
            .find(|(id, _)| id.to_string() == selected_id)
            .map(|(_, name)| name.clone())
            .unwrap_or_else(|| "All Family".to_owned())
    };
    let account_name = selectable_people
        .iter()
        .find(|(id, _)| Some(*id) == account_person_id)
        .map(|(_, name)| name.as_str())
        .unwrap_or("there");
    let timezone = profile
        .pointer("/data/time_zone")
        .and_then(Value::as_str)
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(configured_timezone);
    let schedules = match api.collection(&format!("{base}/schedules")).await {
        Ok(value) => value,
        Err(_) => return dashboard_read_error(api.cookie),
    };
    let assignments = match api.collection(&format!("{base}/person_medications")).await {
        Ok(value) => value,
        Err(_) => return dashboard_read_error(api.cookie),
    };
    let now = dashboard_now();
    let takes = match api
        .dashboard_takes(
            &format!("{base}/medication_takes"),
            &selected,
            now,
            timezone,
        )
        .await
    {
        Ok(value) => value,
        Err(_) => return dashboard_read_error(api.cookie),
    };
    let inventory = match api.collection(&format!("{base}/medications")).await {
        Ok(value) => value,
        Err(_) => return dashboard_read_error(api.cookie),
    };
    let medications: HashMap<i64, &Value> = inventory
        .iter()
        .filter_map(|row| Some((numeric(row, "id")?, row)))
        .collect();
    let today = now.with_timezone(&timezone).date_naive();
    let mut permitted_medications: HashMap<i64, std::collections::HashSet<i64>> = HashMap::new();
    for source in schedules.iter().chain(&assignments) {
        if let (Some(person_id), Some(medication_id)) = (
            numeric(source, "person_id"),
            numeric(source, "medication_id"),
        ) {
            permitted_medications
                .entry(person_id)
                .or_default()
                .insert(medication_id);
        }
    }
    let sources = dashboard_sources(schedules, assignments, &selected, today);
    let mut people: Vec<DashboardPerson> = selectable_people
        .iter()
        .filter(|(id, _)| selected.contains(id))
        .map(|(id, name)| DashboardPerson {
            id: *id,
            name: name.clone(),
            tasks: Vec::new(),
            outcomes: Vec::new(),
        })
        .collect();
    let mut metric_tasks = Vec::new();
    let mut stock_ids = Vec::new();
    for (kind, source) in &sources {
        let (Some(id), Some(person_id), Some(medication_id)) = (
            numeric(source, "id"),
            numeric(source, "person_id"),
            numeric(source, "medication_id"),
        ) else {
            return failure(StatusCode::BAD_GATEWAY);
        };
        stock_ids.push(medication_id);
        let path =
            format!("{base}/{kind}/{id}/dose_occurrences?start_date={today}&end_date={today}");
        let occurrences = match crate::dose_occurrences::with_dashboard_timezone(
            timezone,
            api.get(&path),
        )
        .await
        {
            Ok(value) => value,
            Err(_) => return dashboard_read_error(api.cookie),
        };
        let Some(rows) = occurrences.get("data").and_then(Value::as_array) else {
            return failure(StatusCode::BAD_GATEWAY);
        };
        let Some(person) = people.iter_mut().find(|person| person.id == person_id) else {
            continue;
        };
        let medication = medications.get(&medication_id).copied();
        let medication_name = medication
            .map(|row| field(row, "display_name"))
            .filter(|name| !name.is_empty())
            .or_else(|| medication.map(|row| field(row, "name")))
            .unwrap_or("Medication")
            .to_owned();
        let prn = field(source, "administration_kind") == "as_needed"
            || field(source, "schedule_type") == "prn"
            || field(source, "frequency").eq_ignore_ascii_case("as needed")
            || source
                .pointer("/schedule_config/as_needed")
                .and_then(Value::as_bool)
                == Some(true);
        if !prn
            && rows.is_empty()
            && source
                .get("current_pause_period")
                .is_some_and(|value| !value.is_null())
        {
            person.tasks.push(DashboardTaskRow {
                medication_name: medication_name.clone(),
                dose: dashboard_dose(source),
                time: "—".to_owned(),
                scheduled_at: None,
                state: TaskState::Paused,
                routine: true,
            });
        }
        if prn {
            let source_takes = source_takes(&takes, kind, id);
            let projection = calculate_prn(PrnInput {
                now,
                timezone,
                paused: source
                    .get("current_pause_period")
                    .is_some_and(|value| !value.is_null()),
                active: source_active_on(kind, source, today),
                stock_available: stock_matches(
                    source,
                    &permitted_medications,
                    &medications,
                    household_manager,
                ),
                max_doses: source_limit(source, today),
                min_hours_between_doses: source_interval(source, today),
                dose_cycle: field(source, "dose_cycle"),
                takes: &source_takes,
            });
            let time = projection
                .next_available_at
                .map(|value| value.with_timezone(&timezone).format("%H:%M").to_string())
                .unwrap_or_else(|| "Anytime".to_owned());
            person.tasks.push(DashboardTaskRow {
                medication_name: medication_name.clone(),
                dose: dashboard_dose(source),
                time,
                scheduled_at: projection.next_available_at,
                state: projection.state,
                routine: false,
            });
            metric_tasks.push(DashboardTask {
                state: projection.state,
                scheduled_at: projection.next_available_at,
            });
        }
        for row in rows {
            if row.get("expected").and_then(Value::as_bool) == Some(false)
                && field(row, "outcome") == "open"
            {
                continue;
            }
            let scheduled_at = dashboard_time(field(row, "scheduled_at"));
            let state = match field(row, "outcome") {
                "taken" => TaskState::Taken,
                "not_taken" => TaskState::NotTaken,
                _ if source
                    .get("current_pause_period")
                    .is_some_and(|value| !value.is_null()) =>
                {
                    TaskState::Paused
                }
                _ if medication.is_some_and(|row| {
                    row.get("out_of_stock").and_then(Value::as_bool) == Some(true)
                }) =>
                {
                    TaskState::OutOfStock
                }
                _ if prn => TaskState::Unknown,
                _ if row.get("due").and_then(Value::as_bool) == Some(true) => TaskState::Available,
                _ => TaskState::Upcoming,
            };
            let time = scheduled_at
                .map(|value| value.with_timezone(&timezone).format("%H:%M").to_string())
                .unwrap_or_else(|| "Anytime".to_owned());
            let task = DashboardTaskRow {
                medication_name: medication_name.clone(),
                dose: dashboard_dose(source),
                time,
                scheduled_at,
                state,
                routine: !prn,
            };
            if state == TaskState::NotTaken {
                person.outcomes.push(task);
            } else if state != TaskState::Taken && !prn {
                person.tasks.push(task);
                metric_tasks.push(DashboardTask {
                    state,
                    scheduled_at,
                });
            }
        }
    }
    for person in &mut people {
        person
            .tasks
            .sort_by_key(|row| (row.scheduled_at.is_none(), row.scheduled_at));
        person
            .outcomes
            .sort_by_key(|row| (row.scheduled_at.is_none(), row.scheduled_at));
    }
    let history = takes
        .iter()
        .filter(|row| numeric(row, "person_id").is_some_and(|id| selected.contains(&id)))
        .filter_map(|row| {
            let timestamp = dashboard_time(field(row, "taken_at"))?;
            if !is_same_local_day(timestamp, now, timezone) {
                return None;
            }
            let name = selectable_people
                .iter()
                .find(|(id, _)| Some(*id) == numeric(row, "person_id"))?
                .1
                .clone();
            let medication = medications.get(&numeric(row, "medication_id")?)?;
            Some(DashboardHistory {
                person_name: name,
                medication_name: field(medication, "display_name").to_owned(),
                dose: dashboard_dose(row),
                time: timestamp
                    .with_timezone(&timezone)
                    .format("%H:%M")
                    .to_string(),
            })
        })
        .collect();
    stock_ids.sort_unstable();
    stock_ids.dedup();
    let stock = stock_ids
        .into_iter()
        .filter_map(|id| {
            let row = medications.get(&id)?;
            Some(DashboardStock {
                name: field(row, "display_name").to_owned(),
                amount: field(row, "current_supply").to_owned(),
                unit: field(row, "dose_unit").to_owned(),
                low: row.get("low_stock").and_then(Value::as_bool) == Some(true),
                out: row.get("out_of_stock").and_then(Value::as_bool) == Some(true),
            })
        })
        .collect();
    let greeting = format!(
        "Good {}, {}",
        if now.with_timezone(&timezone).hour() < 12 {
            "morning"
        } else if now.with_timezone(&timezone).hour() < 18 {
            "afternoon"
        } else {
            "evening"
        },
        account_name.split_whitespace().next().unwrap_or("there")
    );
    dashboard_page(
        medtracker_web::dashboard::render_dashboard(DashboardPage {
            household_name,
            slug,
            csrf: api.csrf.clone(),
            timezone,
            greeting,
            date: now
                .with_timezone(&timezone)
                .format("%A, %b %d")
                .to_string()
                .to_uppercase(),
            selected_id,
            selected_name,
            mobile_shortcuts,
            household_manager,
            people,
            selectable_people,
            metrics: calculate_metrics(&metric_tasks, now),
            stock,
            history,
        }),
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
    use super::{dashboard_now, html_cookie_only, is_today_in_zone, page_status};
    use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
    use chrono::NaiveDate;

    #[test]
    fn dashboard_clock_uses_real_utc_outside_the_isolated_contract_runner() {
        if std::env::var_os("CONTRACT_PROJECT").is_some() {
            return;
        }
        let before = chrono::Utc::now();
        let actual = dashboard_now();
        let after = chrono::Utc::now();
        assert!(before <= actual && actual <= after);
    }

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
