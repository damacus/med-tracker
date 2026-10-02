use crate::entities::{
    grant, medication, person, person_medication, review_evidence, review_prompt, schedule,
    security_audit_event,
};
use crate::medication_management::{
    error_response, finish, finish_with_request_id, request_context,
};
use crate::mutation_idempotency::{self, Lookup, StoredResponse};
use crate::sync_batch::{SyncOperation, SyncResult};
use crate::{database_error, granted_people, ApiError, AppState, AuthContext};
use axum::extract::{
    rejection::{JsonRejection, QueryRejection},
    Path, Query, State,
};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::Response;
use axum::{extract::Request, middleware, middleware::Next, routing::get, Json, Router};
use chrono::{Datelike, NaiveDate, Utc};
use chrono_tz::Tz;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseTransaction, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

const CONTROLLER: &str = "api/v1/medication_review_prompts";
const POLICY: &str = "MedicationReviewPromptPolicy";

#[derive(Default, Deserialize)]
pub(super) struct ListQuery {
    page: Option<String>,
    per_page: Option<String>,
    review_status: Option<String>,
    priority: Option<String>,
    show_hidden: Option<String>,
}

struct Filters {
    page: u64,
    size: u64,
    review_status: String,
    priority: String,
    show_hidden: bool,
}

impl Filters {
    fn parse(query: ListQuery) -> Option<Self> {
        let page = query
            .page
            .map_or(Some(1), |value| value.parse::<u64>().ok())?;
        let size = query
            .per_page
            .map_or(Some(20), |value| value.parse::<u64>().ok())?;
        let review_status = query
            .review_status
            .unwrap_or_else(|| "needs_review".to_owned());
        let priority = query.priority.unwrap_or_else(|| "all".to_owned());
        let show_hidden = query.show_hidden.unwrap_or_else(|| "0".to_owned());
        if page == 0
            || size == 0
            || size > 100
            || page
                .checked_sub(1)
                .and_then(|page| page.checked_mul(size))
                .is_none_or(|offset| offset > i64::MAX as u64)
            || !matches!(review_status.as_str(), "needs_review" | "reviewed" | "all")
            || !matches!(
                priority.as_str(),
                "all" | "discuss_soon" | "ask_when_convenient" | "low_confidence"
            )
            || !matches!(show_hidden.as_str(), "0" | "1")
        {
            return None;
        }
        Some(Self {
            page,
            size,
            review_status,
            priority,
            show_hidden: show_hidden == "1",
        })
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
            "/api/v1/households/{household_id}/medication_review_prompts",
            get(index),
        )
        .route(
            "/api/v1/households/{household_id}/medication_review_prompts/{id}",
            get(show).patch(patch).put(put),
        )
        .layer(middleware::from_fn(no_store_middleware))
}

fn tag(record: &review_prompt::Model) -> String {
    let seconds = record.updated_at.and_utc().timestamp();
    let micros = record.updated_at.and_utc().timestamp_subsec_micros();
    let canonical = format!("MedicationReviewPrompt:{}:{seconds}.{micros:06}", record.id);
    format!("\"{}\"", hex::encode(Sha256::digest(canonical.as_bytes())))
}

pub(super) fn value(record: &review_prompt::Model) -> Value {
    json!({
        "id": record.id.to_string(),
        "person_id": record.person_id.to_string(),
        "primary_medication_id": record.primary_medication_id.to_string(),
        "interacting_medication_id": record.interacting_medication_id.to_string(),
        "evidence_record_id": record.evidence_record_id.to_string(),
        "primary_medication_name": record.primary_medication_name,
        "interacting_medication_name": record.interacting_medication_name,
        "evidence_source_name": record.evidence_source_name,
        "evidence_source_url": record.evidence_source_url,
        "evidence_source_version": record.evidence_source_version,
        "matched_term": record.matched_term,
        "match_type": record.match_type,
        "source_instruction": record.source_instruction,
        "match_reason": record.match_reason,
        "evidence_text": record.evidence_text,
        "etag": tag(record),
        "evidence_source_checked_on": record.evidence_source_checked_on.to_string(),
        "evidence_source_effective_on": record.evidence_source_effective_on.to_string(),
        "risk_level": record.risk_level,
        "match_confidence": record.match_confidence,
        "status": record.status,
        "practitioner_name": record.practitioner_name,
        "practitioner_role": record.practitioner_role,
        "review_note": record.review_note,
        "reviewed_on": record.reviewed_on.map(|date| date.to_string()),
        "reviewed_by_membership_id": record.reviewed_by_membership_id.map(|id| id.to_string()),
        "updated_at": record.updated_at.and_utc().to_rfc3339(),
    })
}

pub(super) async fn actor_adult(
    db: &DatabaseTransaction,
    context: &AuthContext,
) -> Result<bool, ApiError> {
    let Some(actor_id) = context.membership.person_id else {
        return Ok(false);
    };
    let Some(actor) = person::Entity::find_by_id(actor_id)
        .one(db)
        .await
        .map_err(database_error)?
    else {
        return Ok(false);
    };
    let adult_by_age = actor.date_of_birth.is_some_and(|born| {
        let today = today_in_app_zone();
        let mut age = today.year() - born.year();
        if (today.month(), today.day()) < (born.month(), born.day()) {
            age -= 1
        }
        age >= 18
    });
    Ok(actor.household_id == context.membership.household_id
        && (actor.person_type == 0 || adult_by_age))
}

pub(super) async fn person_access(
    db: &DatabaseTransaction,
    context: &AuthContext,
    person_id: i64,
    manage: bool,
) -> Result<bool, ApiError> {
    let mut query = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(context.membership.household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(context.membership.id))
        .filter(grant::Column::PersonId.eq(person_id))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(
            Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(grant::Column::ExpiresAt.gt(Utc::now().naive_utc())),
        );
    query = if manage {
        query.filter(grant::Column::AccessLevel.eq("manage"))
    } else {
        query.filter(grant::Column::AccessLevel.is_in(["view", "record", "manage"]))
    };
    Ok(query.one(db).await.map_err(database_error)?.is_some())
}

fn display_name(record: &medication::Model) -> String {
    record
        .friendly_name
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .or(record.name.as_deref())
        .unwrap_or("")
        .to_owned()
}

fn today_in_app_zone() -> NaiveDate {
    let timezone: Tz = std::env::var("TZ")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(chrono_tz::UTC);
    Utc::now().with_timezone(&timezone).date_naive()
}

pub(super) async fn sync(db: &DatabaseTransaction, context: &AuthContext) -> Result<(), ApiError> {
    let household_id = context.membership.household_id;
    let people = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(household_id))
        .filter(person::Column::Id.in_subquery(granted_people(&context.membership)))
        .all(db)
        .await
        .map_err(database_error)?;
    if people.is_empty() {
        return Ok(());
    }
    let person_ids: Vec<i64> = people.iter().map(|row| row.id).collect();
    let assignments = person_medication::Entity::find()
        .filter(person_medication::Column::PersonId.is_in(person_ids.clone()))
        .filter(person_medication::Column::Active.eq(true))
        .filter(person_medication::Column::RetiredAt.is_null())
        .all(db)
        .await
        .map_err(database_error)?;
    let today = today_in_app_zone();
    let schedules = schedule::Entity::find()
        .filter(schedule::Column::PersonId.is_in(person_ids))
        .filter(schedule::Column::Active.eq(true))
        .filter(schedule::Column::RetiredAt.is_null())
        .filter(schedule::Column::StartDate.lte(today))
        .filter(schedule::Column::EndDate.gte(today))
        .all(db)
        .await
        .map_err(database_error)?;
    let medications: HashMap<i64, medication::Model> = medication::Entity::find()
        .filter(medication::Column::HouseholdId.eq(household_id))
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|row| (row.id, row))
        .collect();
    let evidence = review_evidence::Entity::find()
        .filter(review_evidence::Column::MatchStatus.is_in(["unreviewed", "reviewed_pair"]))
        .order_by_asc(review_evidence::Column::Id)
        .all(db)
        .await
        .map_err(database_error)?;
    let mut existing: HashSet<(i64, i64, i64, i64)> = review_prompt::Entity::find()
        .filter(review_prompt::Column::HouseholdId.eq(household_id))
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|row| {
            (
                row.person_id,
                row.primary_medication_id,
                row.interacting_medication_id,
                row.evidence_record_id,
            )
        })
        .collect();
    for person in people {
        let mut medication_ids: Vec<i64> = assignments
            .iter()
            .filter(|row| row.person_id == person.id)
            .map(|row| row.medication_id)
            .chain(
                schedules
                    .iter()
                    .filter(|row| row.person_id == person.id)
                    .map(|row| row.medication_id),
            )
            .collect();
        medication_ids.sort_unstable();
        medication_ids.dedup();
        for (first_index, first_id) in medication_ids.iter().enumerate() {
            for second_id in medication_ids.iter().skip(first_index + 1) {
                let (Some(first), Some(second)) =
                    (medications.get(first_id), medications.get(second_id))
                else {
                    continue;
                };
                let first_name = display_name(first);
                let second_name = display_name(second);
                for matched in crate::review_evidence::matches(&first_name, &second_name, &evidence)
                {
                    let (primary, interacting, primary_name, interacting_name) =
                        if matched.primary_is_first {
                            (first, second, &first_name, &second_name)
                        } else {
                            (second, first, &second_name, &first_name)
                        };
                    if !existing.insert((
                        person.id,
                        primary.id,
                        interacting.id,
                        matched.evidence.id,
                    )) {
                        continue;
                    }
                    let now = Utc::now().naive_utc();
                    review_prompt::ActiveModel {
                        household_id: Set(household_id),
                        person_id: Set(person.id),
                        primary_medication_id: Set(primary.id),
                        interacting_medication_id: Set(interacting.id),
                        evidence_record_id: Set(matched.evidence.id),
                        primary_medication_name: Set(primary_name.clone()),
                        interacting_medication_name: Set(interacting_name.clone()),
                        evidence_source_name: Set(matched.evidence.source_name.clone()),
                        evidence_source_url: Set(matched.evidence.source_url.clone()),
                        evidence_source_checked_on: Set(matched.evidence.retrieved_on),
                        evidence_source_effective_on: Set(matched
                            .evidence
                            .source_effective_on
                            .unwrap_or(matched.evidence.retrieved_on)),
                        evidence_source_version: Set(matched
                            .evidence
                            .source_version
                            .clone()
                            .unwrap_or_else(|| "unknown".to_owned())),
                        evidence_text: Set(matched.excerpt),
                        matched_term: Set(matched.matched_term),
                        match_type: Set(matched.match_type.to_owned()),
                        source_instruction: Set(matched.instruction.to_owned()),
                        match_reason: Set(matched.reason),
                        risk_level: Set(matched.risk.clone()),
                        match_confidence: Set(matched.confidence.clone()),
                        status: Set(if matched.risk == "low" || matched.confidence == "low" {
                            "hidden_low_signal"
                        } else {
                            "needs_review"
                        }
                        .to_owned()),
                        created_at: Set(now),
                        updated_at: Set(now),
                        ..Default::default()
                    }
                    .insert(db)
                    .await
                    .map_err(database_error)?;
                }
            }
        }
    }
    Ok(())
}

async fn failure(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    action: &str,
    status: StatusCode,
    code: &str,
    message: &str,
) -> Result<Response, ApiError> {
    Ok(no_store(
        error_response(
            db, context, method, CONTROLLER, POLICY, action, status, code, message, None,
        )
        .await?,
    ))
}

pub(super) async fn index(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    query: Result<Query<ListQuery>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    if !actor_adult(&db, &context).await? {
        return failure(
            db,
            &context,
            "GET",
            "index",
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
        )
        .await;
    }
    let filters = match query.ok().and_then(|Query(value)| Filters::parse(value)) {
        Some(filters) => filters,
        None => {
            return failure(
                db,
                &context,
                "GET",
                "index",
                StatusCode::UNPROCESSABLE_ENTITY,
                "unprocessable_content",
                "Review filters are invalid",
            )
            .await
        }
    };
    let (_, context) = mutation_idempotency::lock_household_and_reauthenticate(
        &state,
        &db,
        &headers,
        household_id,
    )
    .await?;
    if !actor_adult(&db, &context).await? {
        return failure(
            db,
            &context,
            "GET",
            "index",
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
        )
        .await;
    }
    sync(&db, &context).await?;
    let mut query = review_prompt::Entity::find()
        .filter(review_prompt::Column::HouseholdId.eq(household_id))
        .filter(review_prompt::Column::PersonId.in_subquery(granted_people(&context.membership)));
    if !filters.show_hidden {
        query = query.filter(review_prompt::Column::Status.ne("hidden_low_signal"));
    }
    query = match filters.review_status.as_str() {
        "needs_review" if filters.show_hidden => {
            query.filter(review_prompt::Column::Status.is_in(["needs_review", "hidden_low_signal"]))
        }
        "needs_review" => query.filter(review_prompt::Column::Status.eq("needs_review")),
        "reviewed" => query
            .filter(review_prompt::Column::Status.is_not_in(["needs_review", "hidden_low_signal"])),
        _ => query,
    };
    query = match filters.priority.as_str() {
        "discuss_soon" => query.filter(review_prompt::Column::RiskLevel.eq("high")),
        "ask_when_convenient" => query.filter(review_prompt::Column::RiskLevel.eq("moderate")),
        "low_confidence" => query.filter(
            Condition::any()
                .add(review_prompt::Column::RiskLevel.is_in(["low", "unknown"]))
                .add(review_prompt::Column::MatchConfidence.is_in(["low", "unknown"])),
        ),
        _ => query,
    };
    let total = query.clone().count(&db).await.map_err(database_error)?;
    let rows = query
        .order_by_asc(review_prompt::Column::PersonId)
        .order_by_asc(review_prompt::Column::CreatedAt)
        .order_by_asc(review_prompt::Column::Id)
        .offset((filters.page - 1) * filters.size)
        .limit(filters.size)
        .all(&db)
        .await
        .map_err(database_error)?;
    let body = json!({"data": rows.iter().map(value).collect::<Vec<_>>(), "meta": {
        "page": filters.page, "per_page": filters.size, "total_count": total
    }});
    Ok(no_store(
        finish(
            db,
            &context,
            "GET",
            CONTROLLER,
            POLICY,
            "index",
            StatusCode::OK,
            true,
            body,
            None,
        )
        .await?,
    ))
}

async fn visible_prompt(
    db: &DatabaseTransaction,
    context: &AuthContext,
    id: &str,
) -> Result<Option<review_prompt::Model>, ApiError> {
    let Some(id) = id
        .parse::<i64>()
        .ok()
        .filter(|value| *value > 0 && id == value.to_string())
    else {
        return Ok(None);
    };
    review_prompt::Entity::find_by_id(id)
        .filter(review_prompt::Column::HouseholdId.eq(context.membership.household_id))
        .filter(review_prompt::Column::PersonId.in_subquery(granted_people(&context.membership)))
        .one(db)
        .await
        .map_err(database_error)
}

pub(super) async fn show(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    if !actor_adult(&db, &context).await? {
        return failure(
            db,
            &context,
            "GET",
            "show",
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
        )
        .await;
    }
    let Some(record) = visible_prompt(&db, &context, &id).await? else {
        return failure(
            db,
            &context,
            "GET",
            "show",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
        )
        .await;
    };
    let etag = tag(&record);
    Ok(no_store(
        finish(
            db,
            &context,
            "GET",
            CONTROLLER,
            POLICY,
            "show",
            StatusCode::OK,
            true,
            json!({"data": value(&record)}),
            Some(&etag),
        )
        .await?,
    ))
}

struct Changes {
    status: Option<String>,
    practitioner_name: Option<String>,
    practitioner_role: Option<String>,
    reviewed_on: Option<NaiveDate>,
    review_note: Option<String>,
}

impl Changes {
    fn parse(body: &Value) -> Result<Self, StatusCode> {
        let root = body.as_object().ok_or(StatusCode::BAD_REQUEST)?;
        let Some(inner) = root
            .get("medication_review_prompt")
            .and_then(Value::as_object)
        else {
            return Err(StatusCode::BAD_REQUEST);
        };
        if root.len() != 1
            || inner.is_empty()
            || inner.keys().any(|field| {
                !matches!(
                    field.as_str(),
                    "status"
                        | "practitioner_name"
                        | "practitioner_role"
                        | "reviewed_on"
                        | "review_note"
                )
            })
        {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let text = |field: &str| -> Result<Option<String>, StatusCode> {
            match inner.get(field) {
                None => Ok(None),
                Some(Value::String(value)) => Ok(Some(value.clone())),
                _ => Err(StatusCode::UNPROCESSABLE_ENTITY),
            }
        };
        let status = text("status")?;
        if status.as_deref().is_some_and(|status| {
            !matches!(
                status,
                "needs_review"
                    | "reviewed_with_practitioner"
                    | "expected_prescribed_combination"
                    | "not_relevant"
                    | "hidden_low_signal"
            )
        }) {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let reviewed_on = text("reviewed_on")?
            .map(|date| {
                let bytes = date.as_bytes();
                if bytes.len() != 10
                    || bytes[4] != b'-'
                    || bytes[7] != b'-'
                    || bytes
                        .iter()
                        .enumerate()
                        .any(|(index, byte)| index != 4 && index != 7 && !byte.is_ascii_digit())
                {
                    return Err(StatusCode::UNPROCESSABLE_ENTITY);
                }
                NaiveDate::parse_from_str(&date, "%Y-%m-%d")
                    .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)
            })
            .transpose()?;
        Ok(Self {
            status,
            practitioner_name: text("practitioner_name")?,
            practitioner_role: text("practitioner_role")?,
            reviewed_on,
            review_note: text("review_note")?,
        })
    }

    fn apply(
        self,
        record: &review_prompt::Model,
        membership_id: i64,
    ) -> Option<review_prompt::ActiveModel> {
        let status = self.status.clone().unwrap_or_else(|| record.status.clone());
        let name = self
            .practitioner_name
            .as_deref()
            .or(record.practitioner_name.as_deref());
        let role = self
            .practitioner_role
            .as_deref()
            .or(record.practitioner_role.as_deref());
        let reviewed_on = self.reviewed_on.or(record.reviewed_on);
        if matches!(
            status.as_str(),
            "reviewed_with_practitioner" | "expected_prescribed_combination"
        ) && (name.is_none_or(|value| value.trim().is_empty())
            || role.is_none_or(|value| value.trim().is_empty())
            || reviewed_on.is_none())
        {
            return None;
        }
        let mut changed = record.clone().into_active_model();
        let mut dirty = false;
        if let Some(status) = self.status {
            dirty |= status != record.status;
            changed.status = Set(status);
        }
        if let Some(name) = self.practitioner_name {
            dirty |= Some(&name) != record.practitioner_name.as_ref();
            changed.practitioner_name = Set(Some(name));
        }
        if let Some(role) = self.practitioner_role {
            dirty |= Some(&role) != record.practitioner_role.as_ref();
            changed.practitioner_role = Set(Some(role));
        }
        if let Some(date) = self.reviewed_on {
            dirty |= Some(date) != record.reviewed_on;
            changed.reviewed_on = Set(Some(date));
        }
        if let Some(note) = self.review_note {
            dirty |= Some(&note) != record.review_note.as_ref();
            changed.review_note = Set(Some(note));
        }
        if matches!(
            status.as_str(),
            "reviewed_with_practitioner" | "expected_prescribed_combination"
        ) {
            dirty |= Some(membership_id) != record.reviewed_by_membership_id;
            changed.reviewed_by_membership_id = Set(Some(membership_id));
        }
        if dirty {
            changed.updated_at = Set(Utc::now().naive_utc());
        }
        Some(changed)
    }
}

fn path(household_id: i64, id: i64) -> String {
    format!("/api/v1/households/{household_id}/medication_review_prompts/{id}")
}

#[allow(clippy::too_many_arguments)]
async fn keyed_finish(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    status: StatusCode,
    authorized: bool,
    body: Value,
    etag: Option<&str>,
    key_digest: Option<(&str, &str)>,
    id: i64,
) -> Result<Response, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let mut body = body;
    if status.is_client_error() && body.get("error").is_some() {
        body["error"]["request_id"] = json!(request_id);
    }
    if let Some((key, digest)) = key_digest {
        mutation_idempotency::store(
            &db,
            context,
            StoredResponse {
                key,
                method,
                path: &path(context.membership.household_id, id),
                digest,
                status,
                body: body.clone(),
                request_id: &request_id,
                etag,
            },
        )
        .await?;
    }
    Ok(no_store(
        finish_with_request_id(
            db,
            context,
            &request_id,
            method,
            CONTROLLER,
            POLICY,
            "update",
            status,
            authorized,
            body,
            etag,
        )
        .await?,
    ))
}

async fn update(
    state: AppState,
    household_id: i64,
    id: String,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
    method: &str,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    if !actor_adult(&db, &context).await? {
        return failure(
            db,
            &context,
            method,
            "update",
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
        )
        .await;
    }
    let Some(record) = visible_prompt(&db, &context, &id).await? else {
        return failure(
            db,
            &context,
            method,
            "update",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
        )
        .await;
    };
    if !person_access(&db, &context, record.person_id, true).await? {
        return failure(
            db,
            &context,
            method,
            "update",
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
        )
        .await;
    }
    let (_, context) = mutation_idempotency::lock_household_and_reauthenticate(
        &state,
        &db,
        &headers,
        household_id,
    )
    .await?;
    if !actor_adult(&db, &context).await?
        || !person_access(&db, &context, record.person_id, true).await?
    {
        return failure(
            db,
            &context,
            method,
            "update",
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
        )
        .await;
    }
    let Some(record) = visible_prompt(&db, &context, &id).await? else {
        return failure(
            db,
            &context,
            method,
            "update",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
        )
        .await;
    };
    let (body, rejection) = match payload {
        Ok(Json(body)) => (body, false),
        Err(_) => (Value::Null, true),
    };
    let key = mutation_idempotency::key(&headers);
    let digest =
        key.map(|_| mutation_idempotency::digest(method, &path(household_id, record.id), &body));
    if let (Some(key), Some(digest)) = (key, digest.as_deref()) {
        match mutation_idempotency::lookup(
            &db,
            &context,
            key,
            method,
            &path(household_id, record.id),
            digest,
        )
        .await?
        {
            Lookup::New => {}
            Lookup::Conflict => {
                return failure(
                    db,
                    &context,
                    method,
                    "update",
                    StatusCode::CONFLICT,
                    "idempotency_key_reused",
                    "Idempotency key was reused for a different request",
                )
                .await
            }
            Lookup::Replay(saved) => {
                let replay_status = StatusCode::from_u16(saved.response_status as u16)
                    .map_err(|_| ApiError::internal())?;
                let request_id = Uuid::new_v4().to_string();
                crate::audit::record_resource_request_with_id(
                    &db,
                    &context,
                    &request_id,
                    method,
                    CONTROLLER,
                    POLICY,
                    "update",
                    replay_status,
                    replay_status.is_success(),
                )
                .await
                .map_err(database_error)?;
                db.commit().await.map_err(database_error)?;
                let mut response = mutation_idempotency::replay(*saved)?;
                response.headers_mut().insert(
                    "x-request-id",
                    HeaderValue::from_str(&request_id).map_err(|_| ApiError::internal())?,
                );
                return Ok(no_store(response));
            }
        }
    }
    let key_digest = key.zip(digest.as_deref());
    if rejection {
        return keyed_finish(
            db,
            &context,
            method,
            StatusCode::BAD_REQUEST,
            false,
            json!({"error": {"code": "bad_request", "message": "JSON is invalid"}}),
            None,
            None,
            record.id,
        )
        .await;
    }
    let changes = match Changes::parse(&body) {
        Ok(changes) => changes,
        Err(status) => {
            let (code, message) = if status == StatusCode::BAD_REQUEST {
                ("bad_request", "medication_review_prompt is required")
            } else {
                ("validation_failed", "Validation failed")
            };
            return keyed_finish(
                db,
                &context,
                method,
                status,
                false,
                json!({"error": {"code": code, "message": message}}),
                None,
                key_digest,
                record.id,
            )
            .await;
        }
    };
    let expected = headers
        .get(header::IF_MATCH)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty());
    let Some(expected) = expected else {
        return keyed_finish(db, &context, method, StatusCode::PRECONDITION_REQUIRED, false,
            json!({"error": {"code": "precondition_required", "message": "A current review version is required"}}), None, key_digest, record.id).await;
    };
    if expected != tag(&record) {
        return keyed_finish(db, &context, method, StatusCode::CONFLICT, false,
            json!({"error": {"code": "conflict", "message": "A current review version is required"}}), None, None, record.id).await;
    }
    let Some(active) = changes.apply(&record, context.membership.id) else {
        return keyed_finish(
            db,
            &context,
            method,
            StatusCode::UNPROCESSABLE_ENTITY,
            false,
            json!({"error": {"code": "validation_failed", "message": "Review could not be saved"}}),
            None,
            key_digest,
            record.id,
        )
        .await;
    };
    let updated = active.update(&db).await.map_err(database_error)?;
    let now = Utc::now().naive_utc();
    security_audit_event::ActiveModel {
        household_id: Set(household_id),
        actor_account_id: Set(Some(context.account_id)),
        actor_membership_id: Set(Some(context.membership.id)),
        event_type: Set("medication_review_prompt.updated".to_owned()),
        request_id: Set(None),
        ip: Set(None),
        audit_context: Set(json!({})),
        metadata: Set(
            json!({"prompt_id": updated.id, "person_id": updated.person_id,
            "previous_status": record.status, "status": updated.status}),
        ),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .map_err(database_error)?;
    let etag = tag(&updated);
    keyed_finish(
        db,
        &context,
        method,
        StatusCode::OK,
        true,
        json!({"data": value(&updated)}),
        Some(&etag),
        key_digest,
        updated.id,
    )
    .await
}

pub(super) async fn patch(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    update(state, household_id, id, headers, payload, "PATCH").await
}

pub(super) async fn put(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    update(state, household_id, id, headers, payload, "PUT").await
}

fn sync_error(status: StatusCode) -> ApiError {
    let (code, message) = match status {
        StatusCode::PRECONDITION_REQUIRED => (
            "precondition_required",
            "A current review version is required",
        ),
        StatusCode::CONFLICT => ("sync_conflict", "A current review version is required"),
        StatusCode::NOT_FOUND => ("not_found", "Record not found"),
        StatusCode::FORBIDDEN => (
            "forbidden",
            "You are not authorized to perform this action.",
        ),
        _ => ("unprocessable_content", "Review could not be saved"),
    };
    ApiError {
        status,
        code,
        message,
        preserve_activity: false,
    }
}

pub(super) async fn apply_sync_operation(
    db: &DatabaseTransaction,
    context: &AuthContext,
    operation: &SyncOperation,
    request_id: &str,
) -> Result<SyncResult, ApiError> {
    if operation.action != "update" || !actor_adult(db, context).await? {
        return Err(sync_error(StatusCode::FORBIDDEN));
    }
    let id = operation
        .id
        .as_deref()
        .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?;
    let record = visible_prompt(db, context, id)
        .await?
        .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?;
    if !person_access(db, context, record.person_id, true).await? {
        return Err(sync_error(StatusCode::FORBIDDEN));
    }
    let expected = operation
        .if_match
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| sync_error(StatusCode::PRECONDITION_REQUIRED))?;
    if expected != tag(&record) {
        return Err(sync_error(StatusCode::CONFLICT));
    }
    let changes = Changes::parse(&json!({"medication_review_prompt": operation.attributes}))
        .map_err(sync_error)?;
    let active = changes
        .apply(&record, context.membership.id)
        .ok_or_else(|| sync_error(StatusCode::UNPROCESSABLE_ENTITY))?;
    let updated = active.update(db).await.map_err(database_error)?;
    let now = Utc::now().naive_utc();
    security_audit_event::ActiveModel {
        household_id: Set(context.membership.household_id),
        actor_account_id: Set(Some(context.account_id)),
        actor_membership_id: Set(Some(context.membership.id)),
        event_type: Set("medication_review_prompt.updated".to_owned()),
        request_id: Set(Some(request_id.to_owned())),
        ip: Set(None),
        audit_context: Set(json!({})),
        metadata: Set(
            json!({"prompt_id": updated.id, "person_id": updated.person_id,
            "previous_status": record.status, "status": updated.status}),
        ),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(database_error)?;
    Ok(SyncResult {
        record_type: "MedicationReviewPrompt",
        record_id: Some(updated.id),
        record_portable_id: None,
        etag: Some(tag(&updated)),
        replayed: None,
    })
}

pub(super) async fn authorize_sync_replay(
    db: &DatabaseTransaction,
    context: &AuthContext,
    operation: &SyncOperation,
    saved: &Value,
) -> Result<(), ApiError> {
    if operation.action != "update"
        || saved.get("action").and_then(Value::as_str) != Some("update")
        || saved.get("record_type").and_then(Value::as_str) != Some("MedicationReviewPrompt")
        || !actor_adult(db, context).await?
    {
        return Err(ApiError::forbidden());
    }
    let id = operation.id.as_deref().ok_or_else(ApiError::forbidden)?;
    let record = visible_prompt(db, context, id)
        .await?
        .ok_or_else(ApiError::forbidden)?;
    if saved.get("record_id").and_then(Value::as_str) != Some(record.id.to_string().as_str())
        || !person_access(db, context, record.person_id, true).await?
    {
        return Err(ApiError::forbidden());
    }
    Ok(())
}
