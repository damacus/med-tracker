use super::medications::{
    authentication_error, begin, operation_error, rendering::appearance_context, request_id,
    unavailable,
};
use crate::models::{
    access::{self, PersonAccess, TenantTransaction},
    care::{report_pdf, reports as report_data},
    entities::person,
    errors::OperationError,
};
use axum::{
    Extension,
    extract::{Query, rejection::QueryRejection},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Redirect},
};
use axum_csrf::CsrfToken;
use axum_session::Session;
use axum_session_sqlx::SessionPgPool;
use chrono::{Duration, Months, NaiveDate, Utc};
use loco_rs::{controller::middleware::request_id::LocoRequestId, prelude::*};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Default, Deserialize)]
struct Filters {
    person_id: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
    include_medication_takes: Option<String>,
    status: Option<String>,
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/households")
        .add("/{slug}/reports", get(index))
        .add("/{slug}/reports/health-history", get(gp_download))
        .add("/{slug}/medicine-reviews/report", get(review_download))
}

fn date(value: Option<&str>, fallback: NaiveDate) -> Option<NaiveDate> {
    value.map_or(Some(fallback), |value| {
        (value.len() == 10
            && value.bytes().enumerate().all(|(index, byte)| {
                if index == 4 || index == 7 {
                    byte == b'-'
                } else {
                    byte.is_ascii_digit()
                }
            }))
        .then(|| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
        .flatten()
    })
}

fn ordinary_dates(filters: &Filters, today: NaiveDate) -> Option<(NaiveDate, NaiveDate)> {
    let end = date(filters.end_date.as_deref(), today)?;
    let start = date(filters.start_date.as_deref(), end - Duration::days(6))?;
    (start <= end && (end - start).num_days() <= 180).then_some((start, end))
}

fn gp_dates(filters: &Filters, today: NaiveDate) -> Option<(NaiveDate, NaiveDate)> {
    let end = date(filters.end_date.as_deref(), today)?;
    let start = date(
        filters.start_date.as_deref(),
        end.checked_sub_months(Months::new(12))?,
    )?;
    (start <= end && (end - start).num_days() <= 366).then_some((start, end))
}

fn page(
    view: &TeraView,
    token: &CsrfToken,
    slug: &str,
    mut data: Value,
    status: StatusCode,
    language: &str,
) -> Response {
    let selected = report_pdf::locale(language);
    let Ok(labels) = report_pdf::translations(selected) else {
        return unavailable();
    };
    if data["error_key"] == "end_before_start" {
        data["error"] = labels["reports"]["end_before_start"].clone();
    }
    let mut context = appearance_context();
    context["slug"] = json!(slug);
    context["title"] = json!(labels["reports"]["index"]["title"]);
    context["lang"] = json!(selected);
    context["labels"] = labels;
    context["report"] = data;
    match format::render().view(view, "reports/index.html", context) {
        Ok(response) => (
            status,
            token.clone(),
            [(header::CACHE_CONTROL, "no-store")],
            response,
        )
            .into_response(),
        Err(_) => unavailable(),
    }
}

async fn invalid_page(
    view: &TeraView,
    token: &CsrfToken,
    slug: &str,
    language: &str,
    tenant: TenantTransaction,
    filters: &Filters,
) -> Response {
    let people = match report_data::accessible_people(&tenant, PersonAccess::View).await {
        Ok(people) => people,
        Err(error) => return operation_error(error),
    };
    let manageable = match report_data::accessible_people(&tenant, PersonAccess::Manage).await {
        Ok(people) => people,
        Err(error) => return operation_error(error),
    };
    if tenant.commit().await.is_err() {
        return unavailable();
    }
    page(
        view,
        token,
        slug,
        json!({"error_key":"end_before_start","people":people.iter().map(|person| json!({"id":person.id,"id_string":person.id.to_string(),"name":person.name})).collect::<Vec<_>>(),"manageable_people":manageable.iter().map(|person| json!({"id":person.id,"id_string":person.id.to_string(),"name":person.name})).collect::<Vec<_>>(),"start_date":filters.start_date,"end_date":filters.end_date,"selected_person_id":filters.person_id,"include_medication_takes":filters.include_medication_takes.as_deref()==Some("1")}),
        StatusCode::UNPROCESSABLE_ENTITY,
        language,
    )
}

fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn report_redirect(slug: &str) -> Response {
    no_store(Redirect::to(&format!("/households/{slug}/reports")).into_response())
}

async fn selection(
    tenant: &TenantTransaction,
    id: &str,
    level: PersonAccess,
) -> Result<person::Model, OperationError> {
    let Some(id) = id.parse::<i64>().ok().filter(|id| *id > 0) else {
        return Err(OperationError::NotFound);
    };
    let person = person::Entity::find_by_id(id)
        .filter(person::Column::HouseholdId.eq(tenant.scope().household_id))
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    match access::require_person_access(tenant, person.id, level).await {
        Ok(()) => Ok(person),
        Err(OperationError::Forbidden) => Err(OperationError::NotFound),
        Err(error) => Err(error),
    }
}

#[allow(clippy::too_many_arguments)]
async fn index(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    query: std::result::Result<Query<Filters>, QueryRejection>,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &session, &slug, &request_id).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let today = Utc::now()
        .with_timezone(&principal.time_zone())
        .date_naive();
    let language = headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("en");
    let Query(filters) = match query {
        Ok(value) => value,
        Err(_) => return report_redirect(&slug),
    };
    let result = async {
        let allowed = access::can_manage_household(&tenant)
            || report_data::actor_adult(&tenant, today).await?;
        if !allowed {
            return Err(OperationError::Forbidden);
        }
        let (start, end) = ordinary_dates(&filters, today).ok_or(OperationError::Validation {
            details: json!({"error":"Report date range is invalid"}),
        })?;
        let people = report_data::accessible_people(&tenant, PersonAccess::View).await?;
        let manageable = report_data::accessible_people(&tenant, PersonAccess::Manage).await?;
        let selected: Vec<person::Model> = match filters.person_id.as_deref() {
            Some(id) if !id.is_empty() => people
                .iter()
                .filter(|person| person.id.to_string() == id)
                .cloned()
                .collect(),
            _ => people.clone(),
        };
        let mut data = report_data::ordinary_history(
            &tenant,
            &selected,
            start,
            end,
            Utc::now(),
            principal.time_zone(),
        )
        .await?;
        data["people"] = json!(
            people
                .iter()
                .map(|row| json!({"id":row.id,"id_string":row.id.to_string(),"name":row.name}))
                .collect::<Vec<_>>()
        );
        data["manageable_people"] = json!(
            manageable
                .iter()
                .map(|row| json!({"id":row.id,"id_string":row.id.to_string(),"name":row.name}))
                .collect::<Vec<_>>()
        );
        data["selected_person_id"] = json!(filters.person_id);
        data["include_medication_takes"] =
            json!(filters.include_medication_takes.as_deref() == Some("1"));
        Ok::<_, OperationError>(data)
    }
    .await;
    let data = match result {
        Ok(data) => data,
        Err(OperationError::Validation { .. }) => {
            let end = date(filters.end_date.as_deref(), today);
            let start =
                end.and_then(|end| date(filters.start_date.as_deref(), end - Duration::days(6)));
            if end.is_none()
                || start.is_none()
                || start.is_some_and(|start| end.is_some_and(|end| (end - start).num_days() > 180))
            {
                return report_redirect(&slug);
            }
            return invalid_page(&view, &token, &slug, language, tenant, &filters).await;
        }
        Err(error) => return operation_error(error),
    };
    if tenant.commit().await.is_err() {
        return unavailable();
    }
    page(&view, &token, &slug, data, StatusCode::OK, language)
}

async fn gp_download(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    ViewEngine(view): ViewEngine<TeraView>,
    query: std::result::Result<Query<Filters>, QueryRejection>,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &session, &slug, &request_id).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let Query(filters) = match query {
        Ok(value) => value,
        Err(_) => return report_redirect(&slug),
    };
    let today = Utc::now()
        .with_timezone(&principal.time_zone())
        .date_naive();
    let result = async {
        let allowed = access::can_manage_household(&tenant) || report_data::actor_adult(&tenant, today).await?;
        if !allowed { return Err(OperationError::Forbidden); }
        let id = filters.person_id.as_deref().filter(|id| id.parse::<i64>().is_ok_and(|value| value > 0)).ok_or(OperationError::Validation { details: json!({"error":"Select one person"}) })?;
        let person = selection(&tenant, id, PersonAccess::Manage).await?;
        let (start, end) = gp_dates(&filters, today).ok_or(OperationError::Validation { details: json!({"error":"Report date range is invalid"}) })?;
        let include_takes = filters.include_medication_takes.as_deref() == Some("1");
        let data = report_data::gp_history_pdf(report_data::gp_history(&tenant, &person, start, end, include_takes, Utc::now(), principal.time_zone()).await?, principal.time_zone(), include_takes);
        Ok::<_, OperationError>((data, format!("medtracker-health-history-{start}-to-{end}.pdf"), json!({"person_id":person.id,"start_date":start.to_string(),"end_date":end.to_string(),"include_medication_takes":include_takes}), vec![person.id]))
    }.await;
    let (data, filename, metadata, people) = match result {
        Ok(value) => value,
        Err(OperationError::Validation { .. }) => return report_redirect(&slug),
        Err(error) => return operation_error(error),
    };
    let language = headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("en");
    download(
        DownloadContext {
            ctx: &ctx,
            session: &session,
            slug: &slug,
        },
        tenant,
        &view,
        DownloadPayload {
            kind: report_pdf::Kind::GpHistory,
            data,
            filename: &filename,
            event_type: "health_history_report.downloaded",
            metadata,
            account: principal.account_id(),
            level: PersonAccess::Manage,
            people,
        },
        &request_id,
        language,
    )
    .await
}

async fn review_download(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    ViewEngine(view): ViewEngine<TeraView>,
    query: std::result::Result<Query<Filters>, QueryRejection>,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &session, &slug, &request_id).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let Query(filters) = match query {
        Ok(value) => value,
        Err(_) => {
            return operation_error(OperationError::Validation {
                details: json!({"error":"Report parameters are invalid"}),
            });
        }
    };
    let today = Utc::now()
        .with_timezone(&principal.time_zone())
        .date_naive();
    let result = async {
        if !report_data::actor_adult(&tenant, today).await? {
            return Err(OperationError::Forbidden);
        }
        if filters.start_date.is_some()
            || filters.end_date.is_some()
            || filters.include_medication_takes.is_some()
        {
            return Err(OperationError::Validation {
                details: json!({"error":"Report parameters are invalid"}),
            });
        }
        let people = report_data::accessible_people(&tenant, PersonAccess::View).await?;
        let selected: Vec<person::Model> = match filters.person_id.as_deref() {
            Some(id) if !id.is_empty() => people
                .into_iter()
                .filter(|person| person.id.to_string() == id)
                .collect(),
            _ => people,
        };
        let people: Vec<i64> = selected.iter().map(|person| person.id).collect();
        let data = report_data::household_medication_reviews(
            &tenant,
            &selected,
            filters.status.as_deref(),
            Utc::now(),
            today,
        )
        .await?;
        Ok::<_, OperationError>((
            data,
            format!("medtracker-medication-review-{today}.pdf"),
            json!({"status":filters.status}),
            people,
        ))
    }
    .await;
    let (data, filename, metadata, people) = match result {
        Ok(value) => value,
        Err(error) => return operation_error(error),
    };
    let language = headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("en");
    download(
        DownloadContext {
            ctx: &ctx,
            session: &session,
            slug: &slug,
        },
        tenant,
        &view,
        DownloadPayload {
            kind: report_pdf::Kind::MedicationReview,
            data,
            filename: &filename,
            event_type: "medication_review_report.downloaded",
            metadata,
            account: principal.account_id(),
            level: PersonAccess::View,
            people,
        },
        &request_id,
        language,
    )
    .await
}

struct DownloadPayload<'a> {
    kind: report_pdf::Kind,
    data: Value,
    filename: &'a str,
    event_type: &'a str,
    metadata: Value,
    account: i64,
    level: PersonAccess,
    people: Vec<i64>,
}

struct DownloadContext<'a> {
    ctx: &'a AppContext,
    session: &'a Session<SessionPgPool>,
    slug: &'a str,
}

async fn download(
    context: DownloadContext<'_>,
    tenant: TenantTransaction,
    view: &TeraView,
    payload: DownloadPayload<'_>,
    request_id: &str,
    language: &str,
) -> Response {
    let DownloadPayload {
        kind,
        data,
        filename,
        event_type,
        metadata,
        account,
        level,
        people,
    } = payload;
    if tenant.commit().await.is_err() {
        return unavailable();
    }
    let view = view.clone();
    let language = language.to_owned();
    let rendered =
        tokio::task::spawn_blocking(move || report_pdf::render(&view, kind, data, &language)).await;
    let bytes = match rendered {
        Ok(Ok(bytes)) => bytes,
        _ => {
            return no_store(
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Report is temporarily unavailable",
                )
                    .into_response(),
            );
        }
    };
    let (principal, tenant) =
        match begin(context.ctx, context.session, context.slug, request_id).await {
            Ok(value) => value,
            Err(error) => return authentication_error(error),
        };
    if principal.account_id() != account {
        return operation_error(OperationError::Forbidden);
    }
    let today = Utc::now()
        .with_timezone(&principal.time_zone())
        .date_naive();
    let permitted = match kind {
        report_pdf::Kind::GpHistory | report_pdf::Kind::OrdinaryHistory => {
            if access::can_manage_household(&tenant) {
                true
            } else {
                match report_data::actor_adult(&tenant, today).await {
                    Ok(adult) => adult,
                    Err(error) => return operation_error(error),
                }
            }
        }
        report_pdf::Kind::MedicationReview => {
            match report_data::actor_adult(&tenant, today).await {
                Ok(adult) => adult,
                Err(error) => return operation_error(error),
            }
        }
    };
    if !permitted {
        return operation_error(OperationError::Forbidden);
    }
    let accessible = match report_data::accessible_people(&tenant, level).await {
        Ok(people) => people,
        Err(error) => return operation_error(error),
    };
    let accessible: std::collections::HashSet<i64> =
        accessible.into_iter().map(|person| person.id).collect();
    if people.iter().any(|id| !accessible.contains(id)) {
        return operation_error(OperationError::Forbidden);
    }
    let mut metadata = metadata;
    metadata["format"] = json!("pdf");
    metadata["outcome"] = json!("success");
    if report_data::record_download(&tenant, event_type, metadata)
        .await
        .is_err()
        || tenant.commit().await.is_err()
    {
        return unavailable();
    }
    let disposition = format!("attachment; filename=\"{filename}\"");
    let mut response = (
        StatusCode::OK,
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/pdf"),
            ),
            (
                header::CONTENT_DISPOSITION,
                HeaderValue::from_str(&disposition).expect("controlled filename"),
            ),
        ],
        bytes,
    )
        .into_response();
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(request_id).expect("request UUID"),
    );
    no_store(response)
}
