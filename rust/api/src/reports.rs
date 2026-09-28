use crate::audit;
use crate::entities::{person, review_prompt, security_audit_event};
use crate::medication_management::{error_response, request_context};
use crate::review_prompts;
use crate::{database_error, ApiError, AppState, AuthContext};
use axum::extract::{rejection::QueryRejection, Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::{extract::Request, routing::get, Json, Router};
use chrono::{DateTime, Months, NaiveDate, Utc};
use chrono_tz::Tz;
use printpdf::{
    Mm, Op, ParsedFont, PdfDocument, PdfFontHandle, PdfPage, PdfSaveOptions, Point, Pt, TextItem,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, DbBackend, EntityTrait,
    QueryFilter, QueryOrder, Set, Statement,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

const HEALTH_CONTROLLER: &str = "api/v1/health_history_reports";
const REVIEW_CONTROLLER: &str = "api/v1/medication_review_reports";
const FONT_PATH: &str = "/src/vendor/fonts/NotoSans-Regular.ttf";

#[derive(Default, Deserialize)]
struct ReportQuery {
    person_id: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
    include_medication_takes: Option<String>,
    status: Option<String>,
    format: Option<String>,
}

#[derive(Clone, Copy)]
enum ReportKind {
    Health,
    Reviews,
}

impl ReportKind {
    fn controller(self) -> &'static str {
        match self {
            Self::Health => HEALTH_CONTROLLER,
            Self::Reviews => REVIEW_CONTROLLER,
        }
    }

    fn policy(self) -> &'static str {
        match self {
            Self::Health => "ReportPolicy",
            Self::Reviews => "MedicationReviewPromptPolicy",
        }
    }

    fn event(self) -> &'static str {
        match self {
            Self::Health => "health_history_report.downloaded",
            Self::Reviews => "medication_review_report.downloaded",
        }
    }
}

fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

async fn no_store_middleware(request: Request, next: Next) -> Response {
    no_store(next.run(request).await)
}

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/households/{household_id}/reports/health_history",
            get(health_json),
        )
        .route(
            "/api/v1/households/{household_id}/reports/health_history.pdf",
            get(health_pdf),
        )
        .route(
            "/api/v1/households/{household_id}/reports/medication_reviews",
            get(reviews_json),
        )
        .route(
            "/api/v1/households/{household_id}/reports/medication_reviews.pdf",
            get(reviews_pdf),
        )
        .layer(middleware::from_fn(no_store_middleware))
}

async fn health_json(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    query: Result<Query<ReportQuery>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    report(
        state,
        household_id,
        query,
        headers,
        ReportKind::Health,
        false,
    )
    .await
}

async fn health_pdf(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    query: Result<Query<ReportQuery>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    report(
        state,
        household_id,
        query,
        headers,
        ReportKind::Health,
        true,
    )
    .await
}

async fn reviews_json(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    query: Result<Query<ReportQuery>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    report(
        state,
        household_id,
        query,
        headers,
        ReportKind::Reviews,
        false,
    )
    .await
}

async fn reviews_pdf(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    query: Result<Query<ReportQuery>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    report(
        state,
        household_id,
        query,
        headers,
        ReportKind::Reviews,
        true,
    )
    .await
}

async fn reject(
    db: DatabaseTransaction,
    context: &AuthContext,
    kind: ReportKind,
    status: StatusCode,
    code: &str,
    message: &str,
) -> Result<Response, ApiError> {
    error_response(
        db,
        context,
        "GET",
        kind.controller(),
        kind.policy(),
        "show",
        status,
        code,
        message,
        None,
    )
    .await
}

fn today() -> NaiveDate {
    let timezone: Tz = std::env::var("TZ")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(chrono_tz::UTC);
    Utc::now().with_timezone(&timezone).date_naive()
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

fn dates(query: &ReportQuery) -> Option<(NaiveDate, NaiveDate)> {
    let end = match query.end_date.as_deref() {
        Some(value) => iso_date(value)?,
        None => today(),
    };
    let start = match query.start_date.as_deref() {
        Some(value) => iso_date(value)?,
        None => end.checked_sub_months(Months::new(12))?,
    };
    (start <= end && (end - start).num_days() <= 366).then_some((start, end))
}

async fn selected_person(
    db: &DatabaseTransaction,
    context: &AuthContext,
    identifier: &str,
    manage: bool,
) -> Result<Option<person::Model>, ApiError> {
    let mut query = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(context.membership.household_id));
    query = if let Ok(id) = identifier.parse::<i64>() {
        query.filter(person::Column::Id.eq(id))
    } else {
        query.filter(person::Column::PortableId.eq(identifier))
    };
    let found = query.one(db).await.map_err(database_error)?;
    let Some(found) = found else {
        return Ok(None);
    };
    if !review_prompts::person_access(db, context, found.id, manage).await? {
        return Ok(None);
    }
    Ok(Some(found))
}

async fn report(
    state: AppState,
    household_id: i64,
    query: Result<Query<ReportQuery>, QueryRejection>,
    headers: HeaderMap,
    kind: ReportKind,
    pdf: bool,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let query = match query {
        Ok(Query(query)) => query,
        Err(_) => {
            return reject(
                db,
                &context,
                kind,
                StatusCode::BAD_REQUEST,
                "bad_request",
                "Report parameters are invalid",
            )
            .await;
        }
    };
    let adult = review_prompts::actor_adult(&db, &context).await?;
    let permitted = match kind {
        ReportKind::Health => {
            matches!(context.membership.role.as_str(), "owner" | "administrator") || adult
        }
        ReportKind::Reviews => adult,
    };
    if !permitted {
        return reject(
            db,
            &context,
            kind,
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
        )
        .await;
    }
    let Some(identifier) = query.person_id.as_deref().filter(|value| !value.is_empty()) else {
        return reject(
            db,
            &context,
            kind,
            StatusCode::BAD_REQUEST,
            "bad_request",
            "person_id is required",
        )
        .await;
    };
    let Some(person) = selected_person(
        &db,
        &context,
        identifier,
        matches!(kind, ReportKind::Health),
    )
    .await?
    else {
        return reject(
            db,
            &context,
            kind,
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
        )
        .await;
    };
    let format = if pdf { "pdf" } else { "json" };
    if query.format.as_deref().is_some_and(|value| value != format) {
        return reject(
            db,
            &context,
            kind,
            StatusCode::UNPROCESSABLE_ENTITY,
            "unprocessable_content",
            "Report parameters are invalid",
        )
        .await;
    }
    let generated_at = Utc::now();
    let (data, filename, metadata) = match kind {
        ReportKind::Health => {
            let Some((start, end)) = dates(&query) else {
                return reject(
                    db,
                    &context,
                    kind,
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "unprocessable_content",
                    "Report parameters are invalid",
                )
                .await;
            };
            if query.status.is_some()
                || query
                    .include_medication_takes
                    .as_deref()
                    .is_some_and(|value| !matches!(value, "0" | "1"))
            {
                return reject(
                    db,
                    &context,
                    kind,
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "unprocessable_content",
                    "Report parameters are invalid",
                )
                .await;
            }
            let include_takes = query.include_medication_takes.as_deref() == Some("1");
            let data = health_data(&db, &person, start, end, include_takes, generated_at).await?;
            let filename = format!("medtracker-health-history-{start}-to-{end}.pdf");
            let metadata = json!({"person_id": person.id, "start_date": start.to_string(), "end_date": end.to_string(), "include_medication_takes": include_takes, "format": format, "outcome": "success"});
            (data, filename, metadata)
        }
        ReportKind::Reviews => {
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
                })
            {
                return reject(
                    db,
                    &context,
                    kind,
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "unprocessable_content",
                    "Report parameters are invalid",
                )
                .await;
            }
            let data = review_data(
                &db,
                &context,
                &person,
                query.status.as_deref(),
                generated_at,
            )
            .await?;
            let filename = format!(
                "medtracker-medication-review-{}.pdf",
                generated_at.date_naive()
            );
            let metadata = json!({"person_id": person.id, "status": query.status, "format": format, "outcome": "success"});
            (data, filename, metadata)
        }
    };
    let pdf_bytes = if pdf {
        let lines = report_lines(kind, &data);
        match tokio::task::spawn_blocking(move || render_pdf(&lines)).await {
            Ok(Ok(bytes)) => Some(bytes),
            _ => {
                return reject(
                    db,
                    &context,
                    kind,
                    StatusCode::SERVICE_UNAVAILABLE,
                    "report_unavailable",
                    "Report is temporarily unavailable",
                )
                .await;
            }
        }
    } else {
        None
    };
    complete(db, &context, kind, data, pdf_bytes, &filename, metadata).await
}

async fn json_rows(
    db: &DatabaseTransaction,
    sql: &str,
    values: Vec<sea_orm::Value>,
) -> Result<Value, ApiError> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            values,
        ))
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::internal)?;
    row.try_get("", "payload").map_err(database_error)
}

async fn health_data(
    db: &DatabaseTransaction,
    person: &person::Model,
    start: NaiveDate,
    end: NaiveDate,
    include_takes: bool,
    generated_at: DateTime<Utc>,
) -> Result<Value, ApiError> {
    let medicines = json_rows(
        db,
        "SELECT COALESCE(jsonb_agg(jsonb_build_object('id', m.id::text, 'name', COALESCE(NULLIF(m.friendly_name, ''), m.name)) ORDER BY lower(COALESCE(NULLIF(m.friendly_name, ''), m.name)), m.id), '[]'::jsonb) AS payload FROM medications m WHERE m.household_id = $1 AND m.id IN (SELECT s.medication_id FROM schedules s WHERE s.person_id = $2 AND s.household_id = $1 AND s.active AND s.retired_at IS NULL AND s.start_date <= $3::date AND s.end_date >= $3::date UNION SELECT pm.medication_id FROM person_medications pm WHERE pm.person_id = $2 AND pm.household_id = $1 AND pm.active AND pm.retired_at IS NULL)",
        vec![person.household_id.into(), person.id.into(), today().to_string().into()],
    )
    .await?;
    let chronology = json_rows(
        db,
        "SELECT COALESCE(jsonb_agg(jsonb_build_object('id', he.id::text, 'event_kind', CASE he.event_kind WHEN 0 THEN 'illness' ELSE 'suspected_side_effect' END, 'title', btrim(he.title), 'started_on', he.started_on, 'ended_on', he.ended_on, 'ongoing', he.ended_on IS NULL, 'duration_days', CASE WHEN he.ended_on IS NULL THEN NULL ELSE he.ended_on - he.started_on + 1 END, 'severity', CASE he.severity WHEN 0 THEN 'mild' WHEN 1 THEN 'moderate' WHEN 2 THEN 'severe' ELSE NULL END, 'notes', he.notes, 'action_taken', he.action_taken, 'medical_help_sought', he.medical_help_sought, 'medication_names', (SELECT COALESCE(jsonb_agg(hem.medication_name ORDER BY hem.id), '[]'::jsonb) FROM health_event_medications hem WHERE hem.health_event_id = he.id AND hem.household_id = $1)) ORDER BY he.started_on DESC, he.id DESC), '[]'::jsonb) AS payload FROM health_events he WHERE he.household_id = $1 AND he.person_id = $2 AND he.started_on <= $4::date AND (he.ended_on IS NULL OR he.ended_on >= $3::date)",
        vec![person.household_id.into(), person.id.into(), start.to_string().into(), end.to_string().into()],
    )
    .await?;
    let takes = if include_takes {
        json_rows(
            db,
            "SELECT COALESCE(jsonb_agg(jsonb_build_object('taken_at', to_char(t.taken_at, 'YYYY-MM-DD\"T\"HH24:MI:SS') || 'Z', 'medication_name', COALESCE(NULLIF(sm.friendly_name, ''), sm.name, NULLIF(dm.friendly_name, ''), dm.name), 'dose_amount', t.dose_amount::text, 'dose_unit', t.dose_unit, 'source_type', CASE WHEN pm.id IS NOT NULL AND pm.administration_kind = 0 THEN 'routine' WHEN pm.id IS NOT NULL OR s.schedule_type = 4 THEN 'as_needed' ELSE 'scheduled' END, 'location_name', COALESCE(tfl.name, dml.name, sml.name)) ORDER BY t.taken_at, t.id), '[]'::jsonb) AS payload FROM medication_takes t LEFT JOIN schedules s ON s.id = t.schedule_id AND s.household_id = $1 LEFT JOIN person_medications pm ON pm.id = t.person_medication_id AND pm.household_id = $1 LEFT JOIN medications sm ON sm.id = COALESCE(s.medication_id, pm.medication_id) AND sm.household_id = $1 LEFT JOIN medications dm ON dm.id = t.taken_from_medication_id AND dm.household_id = $1 LEFT JOIN locations tfl ON tfl.id = t.taken_from_location_id AND tfl.household_id = $1 LEFT JOIN locations dml ON dml.id = dm.location_id AND dml.household_id = $1 LEFT JOIN locations sml ON sml.id = sm.location_id AND sml.household_id = $1 WHERE t.household_id = $1 AND (s.person_id = $2 OR pm.person_id = $2) AND t.taken_at >= $3::timestamp AND t.taken_at < ($4::date + INTERVAL '1 day')",
            vec![person.household_id.into(), person.id.into(), start.to_string().into(), end.to_string().into()],
        )
        .await?
    } else {
        json!([])
    };
    Ok(json!({
        "person": {"id": person.id.to_string(), "name": person.name, "date_of_birth": person.date_of_birth.map(|date| date.to_string())},
        "start_date": start.to_string(),
        "end_date": end.to_string(),
        "generated_at": generated_at.to_rfc3339(),
        "current_medicines": medicines,
        "chronology": chronology,
        "medication_takes": takes
    }))
}

async fn review_data(
    db: &DatabaseTransaction,
    context: &AuthContext,
    person: &person::Model,
    status: Option<&str>,
    generated_at: DateTime<Utc>,
) -> Result<Value, ApiError> {
    review_prompts::sync(db, context).await?;
    let mut query = review_prompt::Entity::find()
        .filter(review_prompt::Column::HouseholdId.eq(person.household_id))
        .filter(review_prompt::Column::PersonId.eq(person.id));
    query = if let Some(status) = status {
        query.filter(review_prompt::Column::Status.eq(status))
    } else {
        query.filter(review_prompt::Column::Status.ne("hidden_low_signal"))
    };
    let rows = query
        .order_by_asc(review_prompt::Column::CreatedAt)
        .order_by_asc(review_prompt::Column::Id)
        .all(db)
        .await
        .map_err(database_error)?;
    Ok(json!({
        "person": {"id": person.id.to_string(), "name": person.name},
        "generated_at": generated_at.to_rfc3339(),
        "prompts": rows.iter().map(review_prompts::value).collect::<Vec<_>>()
    }))
}

fn unavailable_response(request_id: &str) -> Response {
    let mut response = (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({"error": {"code": "report_unavailable", "message": "Report is temporarily unavailable", "request_id": request_id}})),
    )
        .into_response();
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(request_id).expect("UUID request ID"),
    );
    response
}

async fn complete(
    db: DatabaseTransaction,
    context: &AuthContext,
    kind: ReportKind,
    data: Value,
    pdf: Option<Vec<u8>>,
    filename: &str,
    metadata: Value,
) -> Result<Response, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let now = Utc::now().naive_utc();
    let download = security_audit_event::ActiveModel {
        household_id: Set(context.membership.household_id),
        actor_account_id: Set(Some(context.account_id)),
        actor_membership_id: Set(Some(context.membership.id)),
        event_type: Set(kind.event().to_owned()),
        request_id: Set(Some(request_id.clone())),
        metadata: Set(metadata),
        audit_context: Set(
            json!({"actor_account_id": context.account_id, "actor_user_id": context.user_id, "actor_membership_id": context.membership.id, "household_id": context.membership.household_id, "request_id": request_id}),
        ),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    if download.insert(&db).await.is_err()
        || audit::record_resource_request_with_id(
            &db,
            context,
            &request_id,
            "GET",
            kind.controller(),
            kind.policy(),
            "show",
            StatusCode::OK,
            true,
        )
        .await
        .is_err()
        || db.commit().await.is_err()
    {
        return Ok(unavailable_response(&request_id));
    }
    let mut response = if let Some(bytes) = pdf {
        let disposition = format!("attachment; filename=\"{filename}\"");
        (
            StatusCode::OK,
            [
                (
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/pdf"),
                ),
                (
                    header::CONTENT_DISPOSITION,
                    HeaderValue::from_str(&disposition).map_err(|_| ApiError::internal())?,
                ),
            ],
            bytes,
        )
            .into_response()
    } else {
        (StatusCode::OK, Json(json!({"data": data}))).into_response()
    };
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&request_id).expect("UUID request ID"),
    );
    Ok(response)
}

fn text(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}

fn report_lines(kind: ReportKind, data: &Value) -> Vec<String> {
    let mut lines = Vec::new();
    match kind {
        ReportKind::Health => {
            lines.push("Health history".to_owned());
            lines.push(format!("Person: {}", text(&data["person"]["name"])));
            lines.push(format!(
                "Date of birth: {}",
                data["person"]["date_of_birth"]
                    .as_str()
                    .unwrap_or("Not recorded")
            ));
            lines.push(format!(
                "Period: {} to {}",
                text(&data["start_date"]),
                text(&data["end_date"])
            ));
            lines.push(format!("Generated: {}", text(&data["generated_at"])));
            lines.push(String::new());
            lines.push("Current medicines".to_owned());
            if let Some(medicines) = data["current_medicines"].as_array() {
                for medicine in medicines {
                    lines.push(format!("- {}", text(&medicine["name"])));
                }
            }
            lines.push(String::new());
            lines.push("Health events".to_owned());
            if let Some(events) = data["chronology"].as_array() {
                if events.is_empty() {
                    lines.push("None".to_owned());
                }
                for event in events {
                    lines.push(format!(
                        "- {}: {} ({})",
                        text(&event["started_on"]),
                        text(&event["title"]),
                        text(&event["event_kind"])
                    ));
                    if event["ongoing"] == true {
                        lines.push("  Duration: Ongoing".to_owned());
                    } else {
                        lines.push(format!(
                            "  Ended: {}; Duration: {} days",
                            text(&event["ended_on"]),
                            event["duration_days"].as_i64().unwrap_or(0)
                        ));
                    }
                    lines.push(format!(
                        "  Medical help sought: {}",
                        if event["medical_help_sought"] == true {
                            "Yes"
                        } else {
                            "No"
                        }
                    ));
                    for key in ["severity", "notes", "action_taken"] {
                        if let Some(value) = event[key].as_str() {
                            if !value.is_empty() {
                                lines.push(format!("  {key}: {value}"));
                            }
                        }
                    }
                    if let Some(names) = event["medication_names"].as_array() {
                        for name in names {
                            lines.push(format!("  Medicine: {}", text(name)));
                        }
                    }
                }
            }
            lines.push(String::new());
            lines.push("Disclaimer".to_owned());
            lines.push("This report reflects information entered into MedTracker.".to_owned());
            lines.push("Suspected side-effect links do not establish causation.".to_owned());
            lines.push(
                "This report is not a diagnosis or a substitute for professional medical advice."
                    .to_owned(),
            );
            if let Some(takes) = data["medication_takes"].as_array() {
                if !takes.is_empty() {
                    lines.push(String::new());
                    lines.push("Medication takes".to_owned());
                    for take in takes {
                        lines.push(format!(
                            "- {}: {} {} {} ({}); Location: {}",
                            text(&take["taken_at"]),
                            text(&take["medication_name"]),
                            text(&take["dose_amount"]),
                            text(&take["dose_unit"]),
                            text(&take["source_type"]),
                            text(&take["location_name"])
                        ));
                    }
                }
            }
        }
        ReportKind::Reviews => {
            lines.push("Medication review".to_owned());
            lines.push(format!("Person: {}", text(&data["person"]["name"])));
            lines.push(format!("Generated: {}", text(&data["generated_at"])));
            lines.push("This record organises public medicine-label evidence for discussion with a practitioner. It does not replace clinical judgement or tell someone to change a medicine.".to_owned());
            lines.push(String::new());
            if let Some(prompts) = data["prompts"].as_array() {
                if prompts.is_empty() {
                    lines.push("No medicine review items matched the selected filters.".to_owned());
                }
                for prompt in prompts {
                    lines.push(format!(
                        "- {} and {}",
                        text(&prompt["primary_medication_name"]),
                        text(&prompt["interacting_medication_name"])
                    ));
                    lines.push(format!("  Risk: {}", text(&prompt["risk_level"])));
                    lines.push(format!(
                        "  Match confidence: {}",
                        text(&prompt["match_confidence"])
                    ));
                    lines.push(format!("  Status: {}", text(&prompt["status"])));
                    lines.push(format!("  Evidence: {}", text(&prompt["evidence_text"])));
                    lines.push(format!("  Match reason: {}", text(&prompt["match_reason"])));
                    lines.push(format!(
                        "  Matched term: {} ({})",
                        text(&prompt["matched_term"]),
                        text(&prompt["match_type"])
                    ));
                    lines.push(format!(
                        "  Source instruction: {}",
                        text(&prompt["source_instruction"])
                    ));
                    lines.push(format!(
                        "  Source: {}",
                        text(&prompt["evidence_source_name"])
                    ));
                    lines.push(format!(
                        "  Label version: {}; effective: {}; checked: {}",
                        text(&prompt["evidence_source_version"]),
                        text(&prompt["evidence_source_effective_on"]),
                        text(&prompt["evidence_source_checked_on"])
                    ));
                    lines.push(format!(
                        "  Source URL: {}",
                        text(&prompt["evidence_source_url"])
                    ));
                    if let Some(name) = prompt["practitioner_name"].as_str() {
                        lines.push(format!("  Reviewed with: {name}"));
                    }
                    if let Some(role) = prompt["practitioner_role"].as_str() {
                        lines.push(format!("  Practitioner role: {role}"));
                    }
                    if let Some(date) = prompt["reviewed_on"].as_str() {
                        lines.push(format!("  Reviewed on: {date}"));
                    }
                    if let Some(note) = prompt["review_note"].as_str() {
                        if !note.is_empty() {
                            lines.push(format!("  Review note: {note}"));
                        }
                    }
                    lines.push(String::new());
                }
            }
        }
    }
    lines
}

fn wrapped_lines(lines: &[String]) -> Vec<String> {
    let mut wrapped = Vec::new();
    for line in lines {
        let mut segment = String::new();
        for word in line.split_whitespace() {
            if word.chars().count() > 50 {
                if !segment.is_empty() {
                    wrapped.push(std::mem::take(&mut segment));
                }
                let characters: Vec<char> = word.chars().collect();
                for chunk in characters.chunks(50) {
                    wrapped.push(chunk.iter().collect());
                }
                continue;
            }
            if !segment.is_empty() && segment.chars().count() + word.chars().count() + 1 > 50 {
                wrapped.push(std::mem::take(&mut segment));
            }
            if !segment.is_empty() {
                segment.push(' ');
            }
            segment.push_str(word);
        }
        wrapped.push(segment);
    }
    wrapped
}

fn render_pdf(lines: &[String]) -> Result<Vec<u8>, ()> {
    let font_path = std::env::var("REPORT_PDF_FONT_PATH").unwrap_or_else(|_| FONT_PATH.to_owned());
    let font_bytes = std::fs::read(font_path).map_err(|_| ())?;
    let mut font_warnings = Vec::new();
    let font = ParsedFont::from_bytes(&font_bytes, 0, &mut font_warnings).ok_or(())?;
    let mut document = PdfDocument::new("MedTracker report");
    let font_id = document.add_font(&font);
    let mut pages = Vec::new();
    let lines = wrapped_lines(lines);
    for chunk in lines.chunks(53) {
        let mut ops = vec![
            Op::StartTextSection,
            Op::SetTextCursor {
                pos: Point {
                    x: Mm(12.0).into(),
                    y: Mm(280.0).into(),
                },
            },
            Op::SetLineHeight { lh: Pt(14.0) },
            Op::SetFont {
                font: PdfFontHandle::External(font_id.clone()),
                size: Pt(10.0),
            },
        ];
        for line in chunk {
            ops.push(Op::ShowText {
                items: vec![TextItem::Text(line.clone())],
            });
            ops.push(Op::AddLineBreak);
        }
        ops.push(Op::EndTextSection);
        pages.push(PdfPage::new(Mm(210.0), Mm(297.0), ops));
    }
    let mut warnings = Vec::new();
    let bytes = document.with_pages(pages).save(
        &PdfSaveOptions {
            subset_fonts: true,
            ..Default::default()
        },
        &mut warnings,
    );
    (bytes.starts_with(b"%PDF-") && bytes.len() > 500)
        .then_some(bytes)
        .ok_or(())
}
