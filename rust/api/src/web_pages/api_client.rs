use super::response::{error, PageError};
use super::time::dashboard_time;
use super::{field, numeric};
use crate::{api_router, oauth, restricted_role, AppState};
use axum::body::{to_bytes, Body};
use axum::http::{header, HeaderMap, HeaderValue, Method, Request, StatusCode};
use chrono::{DateTime, Datelike, Duration, Utc};
use medtracker_web::household_i18n::Locale;
use sea_orm::TransactionTrait;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use tower::ServiceExt;

const BODY_LIMIT: usize = 1_048_576;

enum BrowserWriteIntent {
    Scalar(crate::medication_management::ScalarAdjustment),
    Pause(crate::pause_lifecycle::BrowserSourceGuard),
}

pub(super) fn html_cookie_only(headers: &HeaderMap) -> bool {
    !headers.contains_key(header::AUTHORIZATION)
}

pub(super) struct ApiReply {
    pub(super) status: StatusCode,
    pub(super) value: Value,
    pub(super) etag: Option<String>,
}

pub(super) struct WebApi {
    state: AppState,
    headers: HeaderMap,
    pub(super) csrf: String,
    pub(super) cookie: Option<HeaderValue>,
    pub(super) locale: Locale,
}

impl WebApi {
    pub(super) async fn authenticated(
        state: AppState,
        headers: HeaderMap,
    ) -> Result<Self, PageError> {
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

    pub(super) async fn call(
        &mut self,
        method: Method,
        path: &str,
        body: Option<Value>,
        csrf: Option<&str>,
    ) -> Result<ApiReply, PageError> {
        self.call_with_headers(method, path, body, csrf, &HeaderMap::new())
            .await
    }

    pub(super) async fn call_with_headers(
        &mut self,
        method: Method,
        path: &str,
        body: Option<Value>,
        csrf: Option<&str>,
        extra: &HeaderMap,
    ) -> Result<ApiReply, PageError> {
        self.call_inner(method, path, body, csrf, extra, None).await
    }

    pub(super) async fn adjust_scalar_stock(
        &mut self,
        path: &str,
        body: Value,
        csrf: &str,
        original_etag: String,
    ) -> Result<ApiReply, PageError> {
        self.call_inner(
            Method::PATCH,
            path,
            Some(body),
            Some(csrf),
            &HeaderMap::new(),
            Some(BrowserWriteIntent::Scalar(
                crate::medication_management::ScalarAdjustment { original_etag },
            )),
        )
        .await
    }

    pub(super) async fn pause_with_original_source(
        &mut self,
        path: &str,
        body: Value,
        extra: &HeaderMap,
        guard: crate::pause_lifecycle::BrowserSourceGuard,
    ) -> Result<ApiReply, PageError> {
        let csrf = self.csrf.clone();
        self.call_inner(
            Method::POST,
            path,
            Some(body),
            Some(&csrf),
            extra,
            Some(BrowserWriteIntent::Pause(guard)),
        )
        .await
    }

    async fn call_inner(
        &mut self,
        method: Method,
        path: &str,
        body: Option<Value>,
        csrf: Option<&str>,
        extra: &HeaderMap,
        intent: Option<BrowserWriteIntent>,
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
        let mut request = request
            .body(Body::from(encoded))
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        match intent {
            Some(BrowserWriteIntent::Scalar(scalar)) => {
                request.extensions_mut().insert(scalar);
            }
            Some(BrowserWriteIntent::Pause(guard)) => {
                request.extensions_mut().insert(guard);
            }
            None => {}
        }
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

    pub(super) async fn get(&mut self, path: &str) -> Result<Value, PageError> {
        Ok(self.get_reply(path).await?.value)
    }

    pub(super) async fn capabilities(&mut self, household_id: i64) -> Result<Value, PageError> {
        self.get(&format!(
            "/api/v1/households/{household_id}/ui_capabilities"
        ))
        .await
    }

    pub(super) async fn get_reply(&mut self, path: &str) -> Result<ApiReply, PageError> {
        let reply = self.call(Method::GET, path, None, None).await?;
        if reply.status == StatusCode::UNAUTHORIZED {
            return Err(PageError::Login);
        }
        if !reply.status.is_success() {
            return Err(error(reply.status));
        }
        Ok(reply)
    }

    pub(super) async fn collection(&mut self, base: &str) -> Result<Vec<Value>, PageError> {
        let mut records = Vec::new();
        let mut ids = HashSet::new();
        let mut total = None;
        let mut page = 1_u64;
        loop {
            let value = self
                .get(&format!("{base}?page={page}&per_page=100"))
                .await?;
            let count = value
                .pointer("/meta/total_count")
                .and_then(Value::as_u64)
                .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
            if value.pointer("/meta/page").and_then(Value::as_u64) != Some(page)
                || value.pointer("/meta/per_page").and_then(Value::as_u64) != Some(100)
            {
                return Err(error(StatusCode::BAD_GATEWAY));
            }
            if total.is_some_and(|total| total != count) {
                return Err(error(StatusCode::SERVICE_UNAVAILABLE));
            }
            total = Some(count);
            let data = value
                .get("data")
                .and_then(Value::as_array)
                .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
            for row in data {
                let id = row
                    .get("id")
                    .and_then(|value| {
                        value
                            .as_i64()
                            .filter(|id| *id > 0)
                            .map(|id| id.to_string())
                            .or_else(|| {
                                value
                                    .as_str()
                                    .filter(|id| !id.is_empty())
                                    .map(str::to_owned)
                            })
                    })
                    .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
                if !ids.insert(id) {
                    return Err(error(StatusCode::SERVICE_UNAVAILABLE));
                }
                records.push(row.clone());
            }
            if records.len() as u64 == count {
                return Ok(records);
            }
            if data.is_empty() || records.len() as u64 > count {
                return Err(error(StatusCode::SERVICE_UNAVAILABLE));
            }
            page = page
                .checked_add(1)
                .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
        }
    }

    pub(super) async fn dashboard_takes(
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

    pub(super) async fn household(&mut self, slug: &str) -> Result<(i64, String), PageError> {
        let value = self.get("/api/v1/auth/households").await?;
        value
            .get("data")
            .and_then(Value::as_array)
            .and_then(|rows| rows.iter().find(|row| field(row, "slug") == slug))
            .and_then(|row| Some((row.get("id")?.as_i64()?, field(row, "name").to_owned())))
            .ok_or_else(|| error(StatusCode::NOT_FOUND))
    }
}
