use super::*;
use crate::models::{
    access::{self, PersonAccess},
    care::{report_pdf, reports as report_data},
    entities::person,
};
use axum::{
    extract::{Query, rejection::QueryRejection},
    http::{HeaderValue, header},
};
use chrono::{Months, NaiveDate};
use loco_rs::controller::views::engines::TeraView;
use serde::Deserialize;

#[derive(Default, Deserialize)]
pub(super) struct ReportQuery {
    person_id: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
    include_medication_takes: Option<String>,
    status: Option<String>,
    format: Option<String>,
}

#[derive(Clone, Copy)]
enum Kind {
    Health,
    Reviews,
}

#[derive(Clone, Copy)]
struct ReportMode {
    kind: Kind,
    pdf: bool,
}

fn today(zone: chrono_tz::Tz) -> NaiveDate {
    Utc::now().with_timezone(&zone).date_naive()
}

fn iso_date(value: &str) -> Option<NaiveDate> {
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
}

fn dates(query: &ReportQuery, today: NaiveDate) -> Option<(NaiveDate, NaiveDate)> {
    let end = query.end_date.as_deref().map_or(Some(today), iso_date)?;
    let start = match query.start_date.as_deref() {
        Some(value) => iso_date(value)?,
        None => end.checked_sub_months(Months::new(12))?,
    };
    (start <= end && (end - start).num_days() <= 366).then_some((start, end))
}

async fn selected_person(
    tenant: &TenantTransaction,
    id: &str,
    manage: bool,
) -> Result<person::Model, OperationError> {
    let mut query =
        person::Entity::find().filter(person::Column::HouseholdId.eq(tenant.scope().household_id));
    query = if let Ok(id) = id.parse::<i64>() {
        query.filter(person::Column::Id.eq(id))
    } else {
        query.filter(person::Column::PortableId.eq(id))
    };
    let person = query
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    let access = if manage {
        PersonAccess::Manage
    } else {
        PersonAccess::View
    };
    match access::require_person_access(tenant, person.id, access).await {
        Ok(()) => Ok(person),
        Err(OperationError::Forbidden) => Err(OperationError::NotFound),
        Err(error) => Err(error),
    }
}

fn validation() -> response::Failure {
    response::Failure::validation("Report parameters are invalid")
}

fn parameters(query: &ReportQuery, kind: Kind, pdf: bool) -> Result<(), response::Failure> {
    if query
        .format
        .as_deref()
        .is_some_and(|value| value != if pdf { "pdf" } else { "json" })
    {
        return Err(validation());
    }
    match kind {
        Kind::Health
            if query.status.is_some()
                || query
                    .include_medication_takes
                    .as_deref()
                    .is_some_and(|value| !matches!(value, "0" | "1")) =>
        {
            Err(validation())
        }
        Kind::Reviews
            if query.start_date.is_some()
                || query.end_date.is_some()
                || query.include_medication_takes.is_some()
                || query.status.as_deref().is_some_and(|value| {
                    !matches!(
                        value,
                        "needs_review"
                            | "reviewed_with_practitioner"
                            | "expected_prescribed_combination"
                            | "not_relevant"
                            | "hidden_low_signal"
                    )
                }) =>
        {
            Err(validation())
        }
        _ => Ok(()),
    }
}

fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

async fn execute(
    ctx: AppContext,
    household_id: i64,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    query: std::result::Result<Query<ReportQuery>, QueryRejection>,
    view: TeraView,
    mode: ReportMode,
) -> Response {
    let ReportMode { kind, pdf } = mode;
    let request_id = administration::request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return no_store(response::error(error, &request_id)),
    };
    let today = today(principal.time_zone());
    let result = async {
        let Query(query) =
            query.map_err(|_| response::Failure::bad_request("Report parameters are invalid"))?;
        let is_adult = report_data::actor_adult(&tenant, today)
            .await
            .map_err(response::operation)?;
        let permitted = match kind {
            Kind::Health => {
                matches!(tenant.membership().role.as_str(), "owner" | "administrator") || is_adult
            }
            Kind::Reviews => is_adult,
        };
        if !permitted {
            return Err(response::operation(OperationError::Forbidden));
        }
        let id = query
            .person_id
            .as_deref()
            .filter(|id| !id.is_empty())
            .ok_or_else(|| response::Failure::bad_request("person_id is required"))?;
        let person = selected_person(&tenant, id, matches!(kind, Kind::Health))
            .await
            .map_err(response::operation)?;
        parameters(&query, kind, pdf)?;
        let generated_at = Utc::now();
        let (data, filename, render_kind, metadata, person_id) = match kind {
            Kind::Health => {
                let (start, end) = dates(&query, today).ok_or_else(validation)?;
                let include_takes = query.include_medication_takes.as_deref() == Some("1");
                let mut data = report_data::gp_history(
                    &tenant,
                    &person,
                    start,
                    end,
                    include_takes,
                    generated_at,
                    principal.time_zone(),
                )
                .await
                .map_err(response::operation)?;
                if pdf {
                    data = report_data::gp_history_pdf(data, principal.time_zone(), include_takes);
                }
                (
                    data,
                    format!("medtracker-health-history-{start}-to-{end}.pdf"),
                    report_pdf::Kind::GpHistory,
                    json!({"person_id":person.id,"start_date":start.to_string(),"end_date":end.to_string(),"include_medication_takes":include_takes,"format":"pdf","outcome":"success"}),
                    person.id,
                )
            }
            Kind::Reviews => {
                let mut data = report_data::medication_reviews(
                    &tenant,
                    &person,
                    query.status.as_deref(),
                    generated_at,
                    today,
                )
                .await
                .map_err(response::operation)?;
                if pdf {
                    data = report_data::medication_review_pdf(
                        data,
                        std::slice::from_ref(&person),
                        principal.time_zone(),
                    );
                }
                (
                    data,
                    format!(
                        "medtracker-medication-review-{}.pdf",
                        generated_at.date_naive()
                    ),
                    report_pdf::Kind::MedicationReview,
                    json!({"person_id":person.id,"status":query.status,"format":"pdf","outcome":"success"}),
                    person.id,
                )
            }
        };
        Ok::<_, response::Failure>((data, filename, render_kind, metadata, person_id))
    }
    .await;
    let audit_request = audit::RequestAudit::report(matches!(kind, Kind::Reviews));
    let (data, filename, render_kind, metadata, person_id) = match result {
        Ok(value) => value,
        Err(error) => {
            return no_store(
                finish(
                    tenant,
                    principal.provenance(),
                    audit_request,
                    Err(error),
                    &request_id,
                )
                .await,
            );
        }
    };
    if !pdf {
        return no_store(
            finish(
                tenant,
                principal.provenance(),
                audit_request,
                Ok((StatusCode::OK, json!({"data":data}), None)),
                &request_id,
            )
            .await,
        );
    }
    let snapshot_account = principal.account_id();
    if tenant.commit().await.is_err() {
        return no_store(response::error(response::unavailable(), &request_id));
    }
    let locale = headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("en")
        .to_owned();
    let rendered =
        tokio::task::spawn_blocking(move || report_pdf::render(&view, render_kind, data, &locale))
            .await;
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return no_store(response::error(error, &request_id)),
    };
    if principal.account_id() != snapshot_account {
        return no_store(response::error(
            response::operation(OperationError::Forbidden),
            &request_id,
        ));
    }
    let bytes = match rendered {
        Ok(Ok(bytes)) => bytes,
        _ => {
            return no_store(
                finish(
                    tenant,
                    principal.provenance(),
                    audit_request,
                    Err(response::Failure::report_unavailable()),
                    &request_id,
                )
                .await,
            );
        }
    };
    let current_day = Utc::now()
        .with_timezone(&principal.time_zone())
        .date_naive();
    let is_adult = match report_data::actor_adult(&tenant, current_day).await {
        Ok(value) => value,
        Err(error) => {
            return no_store(
                finish(
                    tenant,
                    principal.provenance(),
                    audit_request,
                    Err(response::operation(error)),
                    &request_id,
                )
                .await,
            );
        }
    };
    let permitted = match kind {
        Kind::Health => {
            matches!(tenant.membership().role.as_str(), "owner" | "administrator") || is_adult
        }
        Kind::Reviews => is_adult,
    };
    if !permitted {
        return no_store(
            finish(
                tenant,
                principal.provenance(),
                audit_request,
                Err(response::operation(OperationError::Forbidden)),
                &request_id,
            )
            .await,
        );
    }
    let level = if matches!(kind, Kind::Health) {
        PersonAccess::Manage
    } else {
        PersonAccess::View
    };
    if let Err(error) = access::require_person_access(&tenant, person_id, level).await {
        let failure = response::operation(match error {
            OperationError::Forbidden => OperationError::NotFound,
            other => other,
        });
        return no_store(
            finish(
                tenant,
                principal.provenance(),
                audit_request,
                Err(failure),
                &request_id,
            )
            .await,
        );
    }
    let event_type = if matches!(kind, Kind::Health) {
        "health_history_report.downloaded"
    } else {
        "medication_review_report.downloaded"
    };
    if report_data::record_download(&tenant, event_type, metadata)
        .await
        .is_err()
        || audit::record(
            &tenant,
            principal.provenance(),
            audit_request,
            StatusCode::OK,
        )
        .await
        .is_err()
        || tenant.commit().await.is_err()
    {
        return no_store(response::error(response::unavailable(), &request_id));
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
        HeaderValue::from_str(&request_id).expect("request UUID"),
    );
    no_store(response)
}

pub(super) async fn health_json(
    State(ctx): State<AppContext>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    query: std::result::Result<Query<ReportQuery>, QueryRejection>,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    execute(
        ctx,
        household_id,
        headers,
        request,
        query,
        view,
        ReportMode {
            kind: Kind::Health,
            pdf: false,
        },
    )
    .await
}
pub(super) async fn health_pdf(
    State(ctx): State<AppContext>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    query: std::result::Result<Query<ReportQuery>, QueryRejection>,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    execute(
        ctx,
        household_id,
        headers,
        request,
        query,
        view,
        ReportMode {
            kind: Kind::Health,
            pdf: true,
        },
    )
    .await
}
pub(super) async fn reviews_json(
    State(ctx): State<AppContext>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    query: std::result::Result<Query<ReportQuery>, QueryRejection>,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    execute(
        ctx,
        household_id,
        headers,
        request,
        query,
        view,
        ReportMode {
            kind: Kind::Reviews,
            pdf: false,
        },
    )
    .await
}
pub(super) async fn reviews_pdf(
    State(ctx): State<AppContext>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    query: std::result::Result<Query<ReportQuery>, QueryRejection>,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    execute(
        ctx,
        household_id,
        headers,
        request,
        query,
        view,
        ReportMode {
            kind: Kind::Reviews,
            pdf: true,
        },
    )
    .await
}
