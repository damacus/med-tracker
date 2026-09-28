use crate::audit;
use crate::dosage_options::valid_identifier;
use crate::entities::{grant, membership, pause_period, person, person_medication, schedule};
use crate::medication_management::{
    error_response, finish_with_request_id, record_version, request_context,
};
use crate::mutation_idempotency::{self, Lookup, StoredResponse};
use crate::read_entities::{
    person as read_person, person_medication as read_assignment, schedule as read_schedule,
};
use crate::read_resources::{serialize_assignments, serialize_schedules};
use crate::sync_batch::{SyncOperation, SyncResult};
use crate::sync_events::{record_change, SyncRecord};
use crate::{database_error, representation_etag, ApiError, AppState, AuthContext};
use axum::body::Bytes;
use axum::extract::{rejection::JsonRejection, Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseTransaction, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

const PERIOD_CONTROLLER: &str = "api/v1/medication_pause_periods";
const PERIOD_POLICY: &str = "MedicationPausePeriodPolicy";
const REASONS: &[&str] = &[
    "out_of_supply",
    "temporarily_not_needed",
    "clinician_advice",
    "side_effects",
    "other",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Schedule,
    Assignment,
}

impl Kind {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "schedule" => Some(Self::Schedule),
            "person_medication" => Some(Self::Assignment),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Schedule => "schedule",
            Self::Assignment => "person_medication",
        }
    }

    fn policy(self) -> &'static str {
        match self {
            Self::Schedule => "SchedulePolicy",
            Self::Assignment => "PersonMedicationPolicy",
        }
    }

    fn controller(self) -> &'static str {
        match self {
            Self::Schedule => "api/v1/schedules",
            Self::Assignment => "api/v1/person_medications",
        }
    }

    fn record_type(self) -> &'static str {
        match self {
            Self::Schedule => "Schedule",
            Self::Assignment => "PersonMedication",
        }
    }
}

enum Source {
    Schedule(schedule::Model),
    Assignment(person_medication::Model),
}

impl Source {
    fn kind(&self) -> Kind {
        match self {
            Self::Schedule(_) => Kind::Schedule,
            Self::Assignment(_) => Kind::Assignment,
        }
    }

    fn id(&self) -> i64 {
        match self {
            Self::Schedule(row) => row.id,
            Self::Assignment(row) => row.id,
        }
    }

    fn person_id(&self) -> i64 {
        match self {
            Self::Schedule(row) => row.person_id,
            Self::Assignment(row) => row.person_id,
        }
    }

    fn portable_id(&self) -> &str {
        match self {
            Self::Schedule(row) => &row.portable_id,
            Self::Assignment(row) => &row.portable_id,
        }
    }

    fn active(&self) -> bool {
        match self {
            Self::Schedule(row) => row.active,
            Self::Assignment(row) => row.active,
        }
    }

    fn retired(&self) -> bool {
        match self {
            Self::Schedule(row) => row.retired_at.is_some(),
            Self::Assignment(row) => row.retired_at.is_some(),
        }
    }
}

enum Failure {
    Malformed,
    Invalid(&'static str, &'static str),
    NotFound,
    Forbidden,
    Conflict,
    KeyConflict,
}

#[derive(Deserialize)]
pub(super) struct ListQuery {
    page: Option<i64>,
    per_page: Option<i64>,
    source_type: Option<String>,
    source_id: Option<String>,
}

fn timestamp(value: chrono::NaiveDateTime) -> String {
    value
        .and_utc()
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

async fn person_access(
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

async fn find_source(
    db: &DatabaseTransaction,
    context: &AuthContext,
    kind: Kind,
    id: &str,
    include_retired: bool,
) -> Result<Option<Source>, ApiError> {
    let household = context.membership.household_id;
    let found = match kind {
        Kind::Schedule => {
            let mut query = schedule::Entity::find()
                .filter(schedule::Column::HouseholdId.eq(household))
                .filter(schedule::Column::PortableId.eq(id));
            if !include_retired {
                query = query.filter(schedule::Column::RetiredAt.is_null());
            }
            query
                .one(db)
                .await
                .map_err(database_error)?
                .map(Source::Schedule)
        }
        Kind::Assignment => {
            let mut query = person_medication::Entity::find()
                .filter(person_medication::Column::HouseholdId.eq(household))
                .filter(person_medication::Column::PortableId.eq(id));
            if !include_retired {
                query = query.filter(person_medication::Column::RetiredAt.is_null());
            }
            query
                .one(db)
                .await
                .map_err(database_error)?
                .map(Source::Assignment)
        }
    };
    match found {
        Some(source) if person_access(db, context, source.person_id(), false).await? => {
            Ok(Some(source))
        }
        _ => Ok(None),
    }
}

async fn find_source_path(
    db: &DatabaseTransaction,
    context: &AuthContext,
    kind: Kind,
    id: &str,
) -> Result<Option<Source>, ApiError> {
    let portable = if valid_identifier(id) {
        if let Ok(numeric) = id.parse::<i64>() {
            match kind {
                Kind::Schedule => schedule::Entity::find_by_id(numeric)
                    .one(db)
                    .await
                    .map_err(database_error)?
                    .map(|row| row.portable_id),
                Kind::Assignment => person_medication::Entity::find_by_id(numeric)
                    .one(db)
                    .await
                    .map_err(database_error)?
                    .map(|row| row.portable_id),
            }
        } else {
            Some(id.to_owned())
        }
    } else {
        None
    };
    let Some(portable) = portable else {
        return Ok(None);
    };
    find_source(db, context, kind, &portable, false).await
}

async fn source_for_period(
    db: &DatabaseTransaction,
    context: &AuthContext,
    period: &pause_period::Model,
    include_retired: bool,
) -> Result<Option<Source>, ApiError> {
    let source = if let Some(id) = period.schedule_id {
        schedule::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(database_error)?
            .map(Source::Schedule)
    } else if let Some(id) = period.person_medication_id {
        person_medication::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(database_error)?
            .map(Source::Assignment)
    } else {
        None
    };
    match source {
        Some(source)
            if source.person_id() > 0
                && (include_retired || !source.retired())
                && person_access(db, context, source.person_id(), false).await? =>
        {
            Ok(Some(source))
        }
        _ => Ok(None),
    }
}

async fn actor_names(
    db: &DatabaseTransaction,
    periods: &[pause_period::Model],
) -> Result<HashMap<i64, String>, ApiError> {
    let ids: Vec<i64> = periods
        .iter()
        .flat_map(|period| {
            [
                period.recorded_by_membership_id,
                period.resumed_by_membership_id,
            ]
        })
        .flatten()
        .collect();
    let actors = if ids.is_empty() {
        Vec::new()
    } else {
        membership::Entity::find()
            .filter(membership::Column::Id.is_in(ids))
            .all(db)
            .await
            .map_err(database_error)?
    };
    let people_ids: Vec<i64> = actors.iter().filter_map(|actor| actor.person_id).collect();
    let people: HashMap<i64, String> = if people_ids.is_empty() {
        HashMap::new()
    } else {
        read_person::Entity::find()
            .filter(read_person::Column::Id.is_in(people_ids))
            .all(db)
            .await
            .map_err(database_error)?
            .into_iter()
            .map(|row| (row.id, row.name))
            .collect()
    };
    Ok(actors
        .into_iter()
        .filter_map(|actor| {
            actor
                .person_id
                .and_then(|id| people.get(&id).cloned())
                .map(|name| (actor.id, name))
        })
        .collect())
}

fn period_row(
    period: &pause_period::Model,
    source: &Source,
    names: &HashMap<i64, String>,
) -> Value {
    json!({
        "id": period.portable_id,
        "portable_id": period.portable_id,
        "source_type": source.kind().name(),
        "source_id": source.portable_id(),
        "reason": period.reason,
        "note": period.note,
        "legacy_context": period.legacy_context,
        "started_at": period.started_at.map(timestamp),
        "ended_at": period.ended_at.map(timestamp),
        "recorded_by_membership_id": period.recorded_by_membership_id.map(|id| id.to_string()),
        "resumed_by_membership_id": period.resumed_by_membership_id.map(|id| id.to_string()),
        "recorded_by_name": period.recorded_by_membership_id.and_then(|id| names.get(&id)),
        "resumed_by_name": period.resumed_by_membership_id.and_then(|id| names.get(&id)),
        "created_at": timestamp(period.created_at),
        "updated_at": timestamp(period.updated_at),
    })
}

fn period_snapshot(period: &pause_period::Model) -> Value {
    json!({
        "household_id": period.household_id,
        "portable_id": period.portable_id,
        "schedule_id": period.schedule_id,
        "person_medication_id": period.person_medication_id,
        "reason": period.reason,
        "note": period.note,
        "legacy_context": period.legacy_context,
        "recorded_by_membership_id": period.recorded_by_membership_id,
        "resumed_by_membership_id": period.resumed_by_membership_id,
        "started_at": period.started_at.map(timestamp),
        "ended_at": period.ended_at.map(timestamp),
    })
}

async fn period_body(
    db: &DatabaseTransaction,
    period: &pause_period::Model,
    source: &Source,
) -> Result<(Value, String), ApiError> {
    let names = actor_names(db, std::slice::from_ref(period)).await?;
    let body = json!({"data": period_row(period, source, &names)});
    let etag = representation_etag(&body);
    Ok((body, etag))
}

pub(super) async fn period_values(
    db: &DatabaseTransaction,
    periods: &[pause_period::Model],
    schedules: &HashMap<i64, schedule::Model>,
    assignments: &HashMap<i64, person_medication::Model>,
) -> Result<Vec<(Value, String)>, ApiError> {
    let names = actor_names(db, periods).await?;
    periods
        .iter()
        .map(|period| {
            let source = if let Some(id) = period.schedule_id {
                schedules.get(&id).cloned().map(Source::Schedule)
            } else {
                period
                    .person_medication_id
                    .and_then(|id| assignments.get(&id).cloned())
                    .map(Source::Assignment)
            }
            .ok_or_else(ApiError::not_found)?;
            let row = period_row(period, &source, &names);
            let etag = representation_etag(&json!({"data": row}));
            Ok((row, etag))
        })
        .collect()
}

async fn source_body(
    db: &DatabaseTransaction,
    context: &AuthContext,
    source: &Source,
) -> Result<Value, ApiError> {
    let row = match source {
        Source::Schedule(source) => {
            let read = read_schedule::Entity::find_by_id(source.id)
                .one(db)
                .await
                .map_err(database_error)?
                .ok_or_else(ApiError::not_found)?;
            serialize_schedules(db, context, vec![read])
                .await?
                .remove(0)
        }
        Source::Assignment(source) => {
            let read = read_assignment::Entity::find_by_id(source.id)
                .one(db)
                .await
                .map_err(database_error)?
                .ok_or_else(ApiError::not_found)?;
            serialize_assignments(db, context, vec![read])
                .await?
                .remove(0)
        }
    };
    Ok(json!({"data": row}))
}

async fn fail(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    controller: &str,
    policy: &str,
    action: &str,
    failure: Failure,
) -> Result<Response, ApiError> {
    let (status, code, message, errors) = match failure {
        Failure::Malformed => (
            StatusCode::BAD_REQUEST,
            "bad_request",
            "Invalid request body",
            None,
        ),
        Failure::Invalid(field, message) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            "Validation failed",
            Some(json!({field: [message]})),
        ),
        Failure::NotFound => (StatusCode::NOT_FOUND, "not_found", "Record not found", None),
        Failure::Forbidden => (
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
            None,
        ),
        Failure::Conflict => (
            StatusCode::CONFLICT,
            "conflict",
            "Record has changed since it was last read",
            None,
        ),
        Failure::KeyConflict => (
            StatusCode::CONFLICT,
            "idempotency_key_reused",
            "Idempotency key has already been used for a different request",
            None,
        ),
    };
    error_response(
        db, context, method, controller, policy, action, status, code, message, errors,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn finish(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    controller: &str,
    policy: &str,
    action: &str,
    status: StatusCode,
    body: Value,
    etag: Option<&str>,
    request_id: &str,
) -> Result<Response, ApiError> {
    finish_with_request_id(
        db, context, request_id, method, controller, policy, action, status, true, body, etag,
    )
    .await
}

enum Replay {
    New,
    Saved(Response),
    Conflict,
}

async fn keyed_replay(
    db: &DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    method: &str,
    path: &str,
    body: &Value,
    action: &str,
) -> Result<Replay, ApiError> {
    let Some(key) = mutation_idempotency::key(headers) else {
        return Ok(Replay::New);
    };
    let digest = mutation_idempotency::digest(method, path, body);
    match mutation_idempotency::lookup(db, context, key, method, path, &digest).await? {
        Lookup::New => Ok(Replay::New),
        Lookup::Conflict => Ok(Replay::Conflict),
        Lookup::Replay(saved) => {
            let request_id = Uuid::new_v4().to_string();
            let status = StatusCode::from_u16(saved.response_status as u16)
                .map_err(|_| ApiError::internal())?;
            audit::record_resource_request_with_id(
                db,
                context,
                &request_id,
                method,
                PERIOD_CONTROLLER,
                PERIOD_POLICY,
                action,
                status,
                status.is_success(),
            )
            .await
            .map_err(database_error)?;
            let mut response = mutation_idempotency::replay(*saved)?;
            response.headers_mut().insert(
                "x-request-id",
                request_id.parse().map_err(|_| ApiError::internal())?,
            );
            Ok(Replay::Saved(response))
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn store_key(
    db: &DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    method: &str,
    path: &str,
    request: &Value,
    status: StatusCode,
    body: &Value,
    request_id: &str,
    etag: Option<&str>,
) -> Result<(), ApiError> {
    let Some(key) = mutation_idempotency::key(headers) else {
        return Ok(());
    };
    let digest = mutation_idempotency::digest(method, path, request);
    mutation_idempotency::store(
        db,
        context,
        StoredResponse {
            key,
            method,
            path,
            digest: &digest,
            status,
            body: body.clone(),
            request_id,
            etag,
        },
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn keyed_invalid(
    db: DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    method: &str,
    action: &str,
    path: &str,
    request: &Value,
    field: &'static str,
    message: &'static str,
) -> Result<Response, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let mut body = json!({"error": {
        "code": "validation_failed",
        "message": "Validation failed",
        "errors": {field: [message]},
    }});
    let mut stored = body.clone();
    stored["error"]["request_id"] = json!(request_id);
    store_key(
        &db,
        context,
        headers,
        method,
        path,
        request,
        StatusCode::UNPROCESSABLE_ENTITY,
        &stored,
        &request_id,
        None,
    )
    .await?;
    finish_with_request_id(
        db,
        context,
        &request_id,
        method,
        PERIOD_CONTROLLER,
        PERIOD_POLICY,
        action,
        StatusCode::UNPROCESSABLE_ENTITY,
        false,
        std::mem::take(&mut body),
        None,
    )
    .await
}

fn create_attributes(body: &Value) -> Result<(Kind, &str, &str, Option<String>), Failure> {
    let outer = body.as_object().ok_or(Failure::Malformed)?;
    let attributes = outer
        .get("medication_pause_period")
        .and_then(Value::as_object)
        .ok_or(Failure::Malformed)?;
    if outer.len() != 1
        || attributes.keys().any(|key| {
            !matches!(
                key.as_str(),
                "source_type" | "source_id" | "reason" | "note"
            )
        })
    {
        return Err(Failure::Invalid(
            "medication_pause_period",
            "contains an unsupported field",
        ));
    }
    let kind = attributes
        .get("source_type")
        .and_then(Value::as_str)
        .and_then(Kind::parse)
        .ok_or(Failure::Invalid("source_type", "is invalid"))?;
    let source_id = attributes
        .get("source_id")
        .and_then(Value::as_str)
        .filter(|id| Uuid::parse_str(id).is_ok())
        .ok_or(Failure::Invalid(
            "source_id",
            "must be a portable identifier",
        ))?;
    let reason = attributes
        .get("reason")
        .and_then(Value::as_str)
        .filter(|reason| REASONS.contains(reason))
        .ok_or(Failure::Invalid("reason", "is invalid"))?;
    let note = match attributes.get("note") {
        None | Some(Value::Null) => None,
        Some(Value::String(note)) => Some(note.clone()),
        _ => return Err(Failure::Invalid("note", "must be a string")),
    };
    Ok((kind, source_id, reason, note))
}

fn source_identity(body: &Value) -> Result<(Kind, &str), Failure> {
    let outer = body.as_object().ok_or(Failure::Malformed)?;
    let attributes = outer
        .get("medication_pause_period")
        .and_then(Value::as_object)
        .ok_or(Failure::Malformed)?;
    let kind = attributes
        .get("source_type")
        .and_then(Value::as_str)
        .and_then(Kind::parse)
        .ok_or(Failure::Invalid("source_type", "is invalid"))?;
    let source_id = attributes
        .get("source_id")
        .and_then(Value::as_str)
        .filter(|id| Uuid::parse_str(id).is_ok())
        .ok_or(Failure::Invalid(
            "source_id",
            "must be a portable identifier",
        ))?;
    Ok((kind, source_id))
}

fn empty_request(payload: &Bytes) -> Result<Value, Failure> {
    if payload.is_empty() {
        return Ok(json!({}));
    }
    let body: Value = serde_json::from_slice(payload).map_err(|_| Failure::Malformed)?;
    if body.as_object().is_none_or(|body| !body.is_empty()) {
        return Err(Failure::Invalid("body", "must be empty"));
    }
    Ok(body)
}

async fn open_period(
    db: &DatabaseTransaction,
    source: &Source,
) -> Result<Option<pause_period::Model>, ApiError> {
    let query = pause_period::Entity::find().filter(pause_period::Column::EndedAt.is_null());
    let query = match source {
        Source::Schedule(row) => query.filter(pause_period::Column::ScheduleId.eq(row.id)),
        Source::Assignment(row) => {
            query.filter(pause_period::Column::PersonMedicationId.eq(row.id))
        }
    };
    query.one(db).await.map_err(database_error)
}

async fn latest_completed(
    db: &DatabaseTransaction,
    source: &Source,
) -> Result<Option<pause_period::Model>, ApiError> {
    let query = pause_period::Entity::find()
        .filter(pause_period::Column::EndedAt.is_not_null())
        .order_by_desc(pause_period::Column::EndedAt)
        .order_by_desc(pause_period::Column::Id);
    let query = match source {
        Source::Schedule(row) => query.filter(pause_period::Column::ScheduleId.eq(row.id)),
        Source::Assignment(row) => {
            query.filter(pause_period::Column::PersonMedicationId.eq(row.id))
        }
    };
    query.one(db).await.map_err(database_error)
}

async fn set_source_active(
    db: &DatabaseTransaction,
    context: &AuthContext,
    source: &Source,
    active: bool,
    request_id: &str,
) -> Result<Source, ApiError> {
    let person_portable_id = person::Entity::find_by_id(source.person_id())
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?
        .portable_id;
    let updated = match source {
        Source::Schedule(row) => {
            let before = json!({"active": row.active});
            let mut model: schedule::ActiveModel = row.clone().into();
            model.active = Set(active);
            model.updated_at = Set(Utc::now().naive_utc());
            let row = model.update(db).await.map_err(database_error)?;
            record_version(
                db,
                context,
                request_id,
                "Schedule",
                row.id,
                "update",
                Some(before),
                Some(json!({"active": row.active})),
            )
            .await?;
            Source::Schedule(row)
        }
        Source::Assignment(row) => {
            let before = json!({"active": row.active});
            let mut model: person_medication::ActiveModel = row.clone().into();
            model.active = Set(active);
            model.updated_at = Set(Utc::now().naive_utc());
            let row = model.update(db).await.map_err(database_error)?;
            record_version(
                db,
                context,
                request_id,
                "PersonMedication",
                row.id,
                "update",
                Some(before),
                Some(json!({"active": row.active})),
            )
            .await?;
            Source::Assignment(row)
        }
    };
    record_change(
        db,
        context,
        request_id,
        SyncRecord {
            record_type: source.kind().record_type(),
            record_id: source.id(),
            portable_id: source.portable_id(),
            action: "update",
            person_portable_id: Some(&person_portable_id),
        },
    )
    .await?;
    Ok(updated)
}

async fn insert_period(
    db: &DatabaseTransaction,
    context: &AuthContext,
    source: &Source,
    reason: &str,
    note: Option<String>,
    request_id: &str,
) -> Result<pause_period::Model, ApiError> {
    let now = Utc::now().naive_utc();
    let legacy = !source.active() || reason == "reason_not_recorded";
    let row = pause_period::ActiveModel {
        household_id: Set(context.membership.household_id),
        portable_id: Set(Uuid::new_v4().to_string()),
        schedule_id: Set((source.kind() == Kind::Schedule).then_some(source.id())),
        person_medication_id: Set((source.kind() == Kind::Assignment).then_some(source.id())),
        reason: Set(if source.active() {
            reason
        } else {
            "reason_not_recorded"
        }
        .to_owned()),
        note: Set(if source.active() { note } else { None }),
        legacy_context: Set(legacy),
        imported_context: Set(false),
        imported_actor_references: Set(json!({})),
        recorded_by_membership_id: Set(source.active().then_some(context.membership.id)),
        resumed_by_membership_id: Set(None),
        started_at: Set(source.active().then_some(now)),
        ended_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(database_error)?;
    record_version(
        db,
        context,
        request_id,
        "MedicationPausePeriod",
        row.id,
        "create",
        None,
        Some(period_snapshot(&row)),
    )
    .await?;
    record_period_change(db, context, source, &row, "create", request_id).await?;
    Ok(row)
}

async fn record_period_change(
    db: &DatabaseTransaction,
    context: &AuthContext,
    source: &Source,
    period: &pause_period::Model,
    action: &str,
    request_id: &str,
) -> Result<(), ApiError> {
    let person = person::Entity::find_by_id(source.person_id())
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?;
    record_change(
        db,
        context,
        request_id,
        SyncRecord {
            record_type: "MedicationPausePeriod",
            record_id: period.id,
            portable_id: &period.portable_id,
            action,
            person_portable_id: Some(&person.portable_id),
        },
    )
    .await
}

async fn pause_source(
    db: &DatabaseTransaction,
    context: &AuthContext,
    source: Source,
    reason: &str,
    note: Option<String>,
    request_id: &str,
) -> Result<(Source, pause_period::Model), ApiError> {
    let period = match open_period(db, &source).await? {
        Some(period) => period,
        None => insert_period(db, context, &source, reason, note, request_id).await?,
    };
    let source = if source.active() {
        set_source_active(db, context, &source, false, request_id).await?
    } else {
        source
    };
    Ok((source, period))
}

async fn close_period(
    db: &DatabaseTransaction,
    context: &AuthContext,
    source: Source,
    requested: Option<pause_period::Model>,
    request_id: &str,
) -> Result<(Source, Option<pause_period::Model>), ApiError> {
    let period = if let Some(requested) = requested {
        Some(requested)
    } else if let Some(open) = open_period(db, &source).await? {
        Some(open)
    } else if source.active() {
        latest_completed(db, &source).await?
    } else {
        Some(
            insert_period(
                db,
                context,
                &source,
                "reason_not_recorded",
                None,
                request_id,
            )
            .await?,
        )
    };
    let Some(period) = period else {
        return Ok((source, None));
    };
    if period.ended_at.is_some() {
        return Ok((source, Some(period)));
    }
    let before = period_snapshot(&period);
    let mut model: pause_period::ActiveModel = period.into();
    model.ended_at = Set(Some(Utc::now().naive_utc()));
    model.resumed_by_membership_id = Set(Some(context.membership.id));
    model.updated_at = Set(Utc::now().naive_utc());
    let period = model.update(db).await.map_err(database_error)?;
    record_version(
        db,
        context,
        request_id,
        "MedicationPausePeriod",
        period.id,
        "update",
        Some(before),
        Some(period_snapshot(&period)),
    )
    .await?;
    record_period_change(db, context, &source, &period, "update", request_id).await?;
    let source = if !source.active() {
        set_source_active(db, context, &source, true, request_id).await?
    } else {
        source
    };
    Ok((source, Some(period)))
}

pub(super) async fn index(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    query: Result<Query<ListQuery>, axum::extract::rejection::QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let Query(query) = match query {
        Ok(query) => query,
        Err(_) => {
            return fail(
                db,
                &context,
                "GET",
                PERIOD_CONTROLLER,
                PERIOD_POLICY,
                "index",
                Failure::Invalid("query", "is invalid"),
            )
            .await;
        }
    };
    let (page, per_page) = (query.page.unwrap_or(1), query.per_page.unwrap_or(20));
    if page < 1 || !(1..=100).contains(&per_page) {
        return fail(
            db,
            &context,
            "GET",
            PERIOD_CONTROLLER,
            PERIOD_POLICY,
            "index",
            Failure::Invalid("pagination", "is invalid"),
        )
        .await;
    }
    let source_filter = match (query.source_type.as_deref(), query.source_id.as_deref()) {
        (None, None) => None,
        (Some(kind), Some(id)) => {
            let Some(kind) = Kind::parse(kind) else {
                return fail(
                    db,
                    &context,
                    "GET",
                    PERIOD_CONTROLLER,
                    PERIOD_POLICY,
                    "index",
                    Failure::Invalid("source_type", "is invalid"),
                )
                .await;
            };
            if Uuid::parse_str(id).is_err() {
                return fail(
                    db,
                    &context,
                    "GET",
                    PERIOD_CONTROLLER,
                    PERIOD_POLICY,
                    "index",
                    Failure::Invalid("source_id", "is invalid"),
                )
                .await;
            }
            let Some(source) = find_source(&db, &context, kind, id, true).await? else {
                return fail(
                    db,
                    &context,
                    "GET",
                    PERIOD_CONTROLLER,
                    PERIOD_POLICY,
                    "index",
                    Failure::NotFound,
                )
                .await;
            };
            Some(source)
        }
        _ => {
            return fail(
                db,
                &context,
                "GET",
                PERIOD_CONTROLLER,
                PERIOD_POLICY,
                "index",
                Failure::Invalid("source", "source_type and source_id are required together"),
            )
            .await;
        }
    };
    let visible_ids: HashSet<i64> = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(context.membership.id))
        .filter(grant::Column::AccessLevel.is_in(["view", "record", "manage"]))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(
            Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(grant::Column::ExpiresAt.gt(Utc::now().naive_utc())),
        )
        .all(&db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|grant| grant.person_id)
        .collect();
    let schedules = schedule::Entity::find()
        .filter(schedule::Column::HouseholdId.eq(household_id))
        .filter(schedule::Column::PersonId.is_in(visible_ids.iter().copied().collect::<Vec<_>>()))
        .all(&db)
        .await
        .map_err(database_error)?;
    let assignments = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(household_id))
        .filter(
            person_medication::Column::PersonId
                .is_in(visible_ids.iter().copied().collect::<Vec<_>>()),
        )
        .all(&db)
        .await
        .map_err(database_error)?;
    let mut sources: HashMap<(String, i64), Source> = HashMap::new();
    for row in schedules {
        sources.insert(("schedule".to_owned(), row.id), Source::Schedule(row));
    }
    for row in assignments {
        sources.insert(
            ("person_medication".to_owned(), row.id),
            Source::Assignment(row),
        );
    }
    let schedule_ids: Vec<i64> = sources
        .iter()
        .filter_map(|((kind, id), _)| (kind == "schedule").then_some(*id))
        .collect();
    let assignment_ids: Vec<i64> = sources
        .iter()
        .filter_map(|((kind, id), _)| (kind == "person_medication").then_some(*id))
        .collect();
    let mut period_query = pause_period::Entity::find()
        .filter(pause_period::Column::HouseholdId.eq(household_id))
        .filter(
            Condition::any()
                .add(pause_period::Column::ScheduleId.is_in(schedule_ids))
                .add(pause_period::Column::PersonMedicationId.is_in(assignment_ids)),
        );
    if let Some(selected) = source_filter.as_ref() {
        period_query = match selected {
            Source::Schedule(source) => {
                period_query.filter(pause_period::Column::ScheduleId.eq(source.id))
            }
            Source::Assignment(source) => {
                period_query.filter(pause_period::Column::PersonMedicationId.eq(source.id))
            }
        };
    }
    let total = period_query
        .clone()
        .count(&db)
        .await
        .map_err(database_error)?;
    let start = ((page - 1) as u64).saturating_mul(per_page as u64);
    let periods = period_query
        .order_by_desc(pause_period::Column::CreatedAt)
        .order_by_desc(pause_period::Column::Id)
        .limit(per_page as u64)
        .offset(start)
        .all(&db)
        .await
        .map_err(database_error)?;
    let names = actor_names(&db, &periods).await?;
    let mut data = Vec::new();
    for period in periods {
        let key = if let Some(id) = period.schedule_id {
            ("schedule".to_owned(), id)
        } else {
            (
                "person_medication".to_owned(),
                period.person_medication_id.unwrap(),
            )
        };
        if let Some(source) = sources.get(&key) {
            data.push(period_row(&period, source, &names));
        }
    }
    let body =
        json!({"data": data, "meta": {"page": page, "per_page": per_page, "total_count": total}});
    let request_id = Uuid::new_v4().to_string();
    finish(
        db,
        &context,
        "GET",
        PERIOD_CONTROLLER,
        PERIOD_POLICY,
        "index",
        StatusCode::OK,
        body,
        None,
        &request_id,
    )
    .await
}

pub(super) async fn create(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    let (db, _) = request_context(&state, &headers, household_id).await?;
    let (_, context) = mutation_idempotency::lock_household_and_reauthenticate(
        &state,
        &db,
        &headers,
        household_id,
    )
    .await?;
    let Json(body) = match payload {
        Ok(body) => body,
        Err(_) => {
            return fail(
                db,
                &context,
                "POST",
                PERIOD_CONTROLLER,
                PERIOD_POLICY,
                "create",
                Failure::Malformed,
            )
            .await
        }
    };
    let (kind, source_id) = match source_identity(&body) {
        Ok(attributes) => attributes,
        Err(failure) => {
            return fail(
                db,
                &context,
                "POST",
                PERIOD_CONTROLLER,
                PERIOD_POLICY,
                "create",
                failure,
            )
            .await
        }
    };
    let Some(source) = find_source(&db, &context, kind, source_id, false).await? else {
        return fail(
            db,
            &context,
            "POST",
            PERIOD_CONTROLLER,
            PERIOD_POLICY,
            "create",
            Failure::NotFound,
        )
        .await;
    };
    if !person_access(&db, &context, source.person_id(), true).await? {
        return fail(
            db,
            &context,
            "POST",
            PERIOD_CONTROLLER,
            PERIOD_POLICY,
            "create",
            Failure::Forbidden,
        )
        .await;
    }
    let path = format!("/api/v1/households/{household_id}/medication_pause_periods");
    match keyed_replay(&db, &context, &headers, "POST", &path, &body, "create").await? {
        Replay::New => {}
        Replay::Saved(response) => {
            db.commit().await.map_err(database_error)?;
            return Ok(response);
        }
        Replay::Conflict => {
            return fail(
                db,
                &context,
                "POST",
                PERIOD_CONTROLLER,
                PERIOD_POLICY,
                "create",
                Failure::KeyConflict,
            )
            .await
        }
    }
    let (_, _, reason, note) = match create_attributes(&body) {
        Ok(attributes) => attributes,
        Err(Failure::Invalid(field, message)) => {
            return keyed_invalid(
                db, &context, &headers, "POST", "create", &path, &body, field, message,
            )
            .await;
        }
        Err(failure) => {
            return fail(
                db,
                &context,
                "POST",
                PERIOD_CONTROLLER,
                PERIOD_POLICY,
                "create",
                failure,
            )
            .await
        }
    };
    let request_id = Uuid::new_v4().to_string();
    let (_, period) = pause_source(&db, &context, source, reason, note, &request_id).await?;
    let source = source_for_period(&db, &context, &period, false)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let (response_body, etag) = period_body(&db, &period, &source).await?;
    store_key(
        &db,
        &context,
        &headers,
        "POST",
        &path,
        &body,
        StatusCode::CREATED,
        &response_body,
        &request_id,
        Some(&etag),
    )
    .await?;
    finish(
        db,
        &context,
        "POST",
        PERIOD_CONTROLLER,
        PERIOD_POLICY,
        "create",
        StatusCode::CREATED,
        response_body,
        Some(&etag),
        &request_id,
    )
    .await
}

pub(super) async fn resume(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Bytes,
) -> Result<Response, ApiError> {
    let (db, _) = request_context(&state, &headers, household_id).await?;
    let (_, context) = mutation_idempotency::lock_household_and_reauthenticate(
        &state,
        &db,
        &headers,
        household_id,
    )
    .await?;
    if Uuid::parse_str(&id).is_err() {
        return fail(
            db,
            &context,
            "POST",
            PERIOD_CONTROLLER,
            PERIOD_POLICY,
            "resume",
            Failure::NotFound,
        )
        .await;
    }
    let Some(period) = pause_period::Entity::find()
        .filter(pause_period::Column::HouseholdId.eq(household_id))
        .filter(pause_period::Column::PortableId.eq(&id))
        .one(&db)
        .await
        .map_err(database_error)?
    else {
        return fail(
            db,
            &context,
            "POST",
            PERIOD_CONTROLLER,
            PERIOD_POLICY,
            "resume",
            Failure::NotFound,
        )
        .await;
    };
    let Some(source) = source_for_period(&db, &context, &period, false).await? else {
        return fail(
            db,
            &context,
            "POST",
            PERIOD_CONTROLLER,
            PERIOD_POLICY,
            "resume",
            Failure::NotFound,
        )
        .await;
    };
    if !person_access(&db, &context, source.person_id(), true).await? {
        return fail(
            db,
            &context,
            "POST",
            PERIOD_CONTROLLER,
            PERIOD_POLICY,
            "resume",
            Failure::Forbidden,
        )
        .await;
    }
    let body = match empty_request(&payload) {
        Ok(body) => body,
        Err(failure) => {
            return fail(
                db,
                &context,
                "POST",
                PERIOD_CONTROLLER,
                PERIOD_POLICY,
                "resume",
                failure,
            )
            .await
        }
    };
    let path = format!("/api/v1/households/{household_id}/medication_pause_periods/{id}/resume");
    match keyed_replay(&db, &context, &headers, "POST", &path, &body, "resume").await? {
        Replay::New => {}
        Replay::Saved(response) => {
            db.commit().await.map_err(database_error)?;
            return Ok(response);
        }
        Replay::Conflict => {
            return fail(
                db,
                &context,
                "POST",
                PERIOD_CONTROLLER,
                PERIOD_POLICY,
                "resume",
                Failure::KeyConflict,
            )
            .await
        }
    }
    let (_, etag) = period_body(&db, &period, &source).await?;
    if headers
        .get(header::IF_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| !value.is_empty() && value != etag)
    {
        return fail(
            db,
            &context,
            "POST",
            PERIOD_CONTROLLER,
            PERIOD_POLICY,
            "resume",
            Failure::Conflict,
        )
        .await;
    }
    let request_id = Uuid::new_v4().to_string();
    let (source, period) = close_period(&db, &context, source, Some(period), &request_id).await?;
    let period = period.ok_or_else(ApiError::not_found)?;
    let (response_body, etag) = period_body(&db, &period, &source).await?;
    store_key(
        &db,
        &context,
        &headers,
        "POST",
        &path,
        &body,
        StatusCode::OK,
        &response_body,
        &request_id,
        Some(&etag),
    )
    .await?;
    finish(
        db,
        &context,
        "POST",
        PERIOD_CONTROLLER,
        PERIOD_POLICY,
        "resume",
        StatusCode::OK,
        response_body,
        Some(&etag),
        &request_id,
    )
    .await
}

async fn legacy_action(
    state: AppState,
    household_id: i64,
    id: String,
    headers: HeaderMap,
    payload: Bytes,
    kind: Kind,
    pause: bool,
) -> Result<Response, ApiError> {
    let (db, _) = request_context(&state, &headers, household_id).await?;
    let (_, context) = mutation_idempotency::lock_household_and_reauthenticate(
        &state,
        &db,
        &headers,
        household_id,
    )
    .await?;
    let action = if pause { "pause" } else { "resume" };
    let Some(source) = find_source_path(&db, &context, kind, &id).await? else {
        return fail(
            db,
            &context,
            "PATCH",
            kind.controller(),
            kind.policy(),
            action,
            Failure::NotFound,
        )
        .await;
    };
    if !person_access(&db, &context, source.person_id(), true).await? {
        return fail(
            db,
            &context,
            "PATCH",
            kind.controller(),
            kind.policy(),
            action,
            Failure::Forbidden,
        )
        .await;
    }
    let _body = match empty_request(&payload) {
        Ok(body) => body,
        Err(failure) => {
            return fail(
                db,
                &context,
                "PATCH",
                kind.controller(),
                kind.policy(),
                action,
                failure,
            )
            .await
        }
    };
    let request_id = Uuid::new_v4().to_string();
    let source = if pause {
        pause_source(
            &db,
            &context,
            source,
            "reason_not_recorded",
            None,
            &request_id,
        )
        .await?
        .0
    } else {
        close_period(&db, &context, source, None, &request_id)
            .await?
            .0
    };
    let response_body = source_body(&db, &context, &source).await?;
    let etag = representation_etag(&response_body);
    finish(
        db,
        &context,
        "PATCH",
        kind.controller(),
        kind.policy(),
        action,
        StatusCode::OK,
        response_body,
        Some(&etag),
        &request_id,
    )
    .await
}

pub(super) async fn pause_schedule(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Bytes,
) -> Result<Response, ApiError> {
    legacy_action(
        state,
        household_id,
        id,
        headers,
        payload,
        Kind::Schedule,
        true,
    )
    .await
}

pub(super) async fn resume_schedule(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Bytes,
) -> Result<Response, ApiError> {
    legacy_action(
        state,
        household_id,
        id,
        headers,
        payload,
        Kind::Schedule,
        false,
    )
    .await
}

pub(super) async fn pause_assignment(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Bytes,
) -> Result<Response, ApiError> {
    legacy_action(
        state,
        household_id,
        id,
        headers,
        payload,
        Kind::Assignment,
        true,
    )
    .await
}

pub(super) async fn resume_assignment(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Bytes,
) -> Result<Response, ApiError> {
    legacy_action(
        state,
        household_id,
        id,
        headers,
        payload,
        Kind::Assignment,
        false,
    )
    .await
}

pub(super) async fn reorder_assignment(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    let (db, _) = request_context(&state, &headers, household_id).await?;
    let (_, context) = mutation_idempotency::lock_household_and_reauthenticate(
        &state,
        &db,
        &headers,
        household_id,
    )
    .await?;
    let Some(Source::Assignment(source)) =
        find_source_path(&db, &context, Kind::Assignment, &id).await?
    else {
        return fail(
            db,
            &context,
            "PATCH",
            Kind::Assignment.controller(),
            Kind::Assignment.policy(),
            "reorder",
            Failure::NotFound,
        )
        .await;
    };
    if !person_access(&db, &context, source.person_id, true).await? {
        return fail(
            db,
            &context,
            "PATCH",
            Kind::Assignment.controller(),
            Kind::Assignment.policy(),
            "reorder",
            Failure::Forbidden,
        )
        .await;
    }
    let Json(body) = match payload {
        Ok(body) => body,
        Err(_) => {
            return fail(
                db,
                &context,
                "PATCH",
                Kind::Assignment.controller(),
                Kind::Assignment.policy(),
                "reorder",
                Failure::Malformed,
            )
            .await
        }
    };
    let Some(map) = body.as_object() else {
        return fail(
            db,
            &context,
            "PATCH",
            Kind::Assignment.controller(),
            Kind::Assignment.policy(),
            "reorder",
            Failure::Malformed,
        )
        .await;
    };
    let direction = map.get("direction").and_then(Value::as_str);
    if map.len() != 1 || !matches!(direction, Some("up" | "down")) {
        return fail(
            db,
            &context,
            "PATCH",
            Kind::Assignment.controller(),
            Kind::Assignment.policy(),
            "reorder",
            Failure::Invalid("direction", "must be up or down"),
        )
        .await;
    }
    let direction = direction.unwrap();
    let mut query = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(household_id))
        .filter(person_medication::Column::PersonId.eq(source.person_id))
        .filter(person_medication::Column::RetiredAt.is_null())
        .filter(if direction == "up" {
            person_medication::Column::Position.lt(source.position)
        } else {
            person_medication::Column::Position.gt(source.position)
        });
    query = if direction == "up" {
        query
            .order_by_desc(person_medication::Column::Position)
            .order_by_desc(person_medication::Column::Id)
    } else {
        query
            .order_by_asc(person_medication::Column::Position)
            .order_by_asc(person_medication::Column::Id)
    };
    let adjacent = query.one(&db).await.map_err(database_error)?;
    let request_id = Uuid::new_v4().to_string();
    let updated = if let Some(adjacent) = adjacent {
        let original_position = source.position;
        let adjacent_position = adjacent.position;
        let mut moving: person_medication::ActiveModel = source.clone().into();
        moving.position = Set(adjacent_position);
        moving.updated_at = Set(Utc::now().naive_utc());
        let moving = moving.update(&db).await.map_err(database_error)?;
        let mut swapping: person_medication::ActiveModel = adjacent.clone().into();
        swapping.position = Set(original_position);
        swapping.updated_at = Set(Utc::now().naive_utc());
        let swapping = swapping.update(&db).await.map_err(database_error)?;
        let person_portable_id = person::Entity::find_by_id(source.person_id)
            .one(&db)
            .await
            .map_err(database_error)?
            .ok_or_else(ApiError::not_found)?
            .portable_id;
        for (before, after) in [(&source, &moving), (&adjacent, &swapping)] {
            record_version(
                &db,
                &context,
                &request_id,
                "PersonMedication",
                after.id,
                "update",
                Some(json!({"position": before.position})),
                Some(json!({"position": after.position})),
            )
            .await?;
            record_change(
                &db,
                &context,
                &request_id,
                SyncRecord {
                    record_type: "PersonMedication",
                    record_id: after.id,
                    portable_id: &after.portable_id,
                    action: "update",
                    person_portable_id: Some(&person_portable_id),
                },
            )
            .await?;
        }
        moving
    } else {
        source
    };
    let response_body = source_body(&db, &context, &Source::Assignment(updated)).await?;
    let etag = representation_etag(&response_body);
    finish(
        db,
        &context,
        "PATCH",
        Kind::Assignment.controller(),
        Kind::Assignment.policy(),
        "reorder",
        StatusCode::OK,
        response_body,
        Some(&etag),
        &request_id,
    )
    .await
}

fn sync_failure(failure: Failure) -> ApiError {
    let (status, code, message) = match failure {
        Failure::Malformed => (
            StatusCode::BAD_REQUEST,
            "bad_request",
            "Invalid request body",
        ),
        Failure::Invalid(_, _) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "unprocessable_content",
            "Attributes are invalid",
        ),
        Failure::NotFound => (StatusCode::NOT_FOUND, "not_found", "Record not found"),
        Failure::Forbidden => (
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
        ),
        Failure::Conflict => (
            StatusCode::CONFLICT,
            "conflict",
            "Record has changed since it was last read",
        ),
        Failure::KeyConflict => (
            StatusCode::CONFLICT,
            "idempotency_key_reused",
            "Idempotency key has already been used for a different request",
        ),
    };
    ApiError {
        status,
        code,
        message,
        preserve_activity: false,
    }
}

fn sync_result(source: &Source, etag: String) -> SyncResult {
    SyncResult {
        record_type: source.kind().record_type(),
        record_id: Some(source.id()),
        record_portable_id: Some(source.portable_id().to_owned()),
        etag: Some(etag),
        replayed: None,
    }
}

fn sync_precondition(actual: &str, expected: Option<&str>) -> Result<(), ApiError> {
    let expected = expected.ok_or(ApiError {
        status: StatusCode::PRECONDITION_REQUIRED,
        code: "precondition_required",
        message: "A current resource version is required",
        preserve_activity: false,
    })?;
    if expected != actual {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "sync_conflict",
            message: "Record has changed since it was last read",
            preserve_activity: false,
        });
    }
    Ok(())
}

pub(super) async fn apply_sync_operation(
    db: &DatabaseTransaction,
    context: &AuthContext,
    operation: &SyncOperation,
    request_id: &str,
) -> Result<SyncResult, ApiError> {
    match operation.resource_type.as_str() {
        "schedule" | "person_medication" => {
            let kind = if operation.resource_type == "schedule" {
                Kind::Schedule
            } else {
                Kind::Assignment
            };
            let id = operation
                .id
                .as_deref()
                .ok_or_else(|| sync_failure(Failure::NotFound))?;
            let source = find_source_path(db, context, kind, id)
                .await?
                .ok_or_else(|| sync_failure(Failure::NotFound))?;
            if !person_access(db, context, source.person_id(), true).await? {
                return Err(sync_failure(Failure::Forbidden));
            }
            let current_etag = representation_etag(&source_body(db, context, &source).await?);
            sync_precondition(&current_etag, operation.if_match.as_deref())?;
            if operation.action == "reorder" {
                let Source::Assignment(source) = source else {
                    return Err(sync_failure(Failure::NotFound));
                };
                return reorder_sync(db, context, source, operation, request_id).await;
            }
            let source = match operation.action.as_str() {
                "pause" => {
                    if operation
                        .attributes
                        .keys()
                        .any(|key| !matches!(key.as_str(), "reason" | "note"))
                    {
                        return Err(sync_failure(Failure::Invalid(
                            "attributes",
                            "contains an unsupported field",
                        )));
                    }
                    let reason = operation
                        .attributes
                        .get("reason")
                        .and_then(Value::as_str)
                        .filter(|reason| REASONS.contains(reason))
                        .ok_or_else(|| sync_failure(Failure::Invalid("reason", "is invalid")))?;
                    let note = match operation.attributes.get("note") {
                        None | Some(Value::Null) => None,
                        Some(Value::String(value)) => Some(value.clone()),
                        _ => {
                            return Err(sync_failure(Failure::Invalid("note", "must be a string")))
                        }
                    };
                    pause_source(db, context, source, reason, note, request_id)
                        .await?
                        .0
                }
                "resume" => {
                    if !operation.attributes.is_empty() {
                        return Err(sync_failure(Failure::Malformed));
                    }
                    close_period(db, context, source, None, request_id).await?.0
                }
                _ => return Err(sync_failure(Failure::Malformed)),
            };
            let etag = representation_etag(&source_body(db, context, &source).await?);
            Ok(sync_result(&source, etag))
        }
        "medication_pause_period" => apply_period_sync(db, context, operation, request_id).await,
        _ => Err(sync_failure(Failure::Malformed)),
    }
}

async fn apply_period_sync(
    db: &DatabaseTransaction,
    context: &AuthContext,
    operation: &SyncOperation,
    request_id: &str,
) -> Result<SyncResult, ApiError> {
    let (source, period, replayed) = if operation.action == "create" {
        let body = json!({"medication_pause_period": operation.attributes});
        if operation
            .attributes
            .get("source_id")
            .and_then(Value::as_str)
            .is_some_and(|id| id.parse::<i64>().is_ok())
        {
            return Err(sync_failure(Failure::NotFound));
        }
        let (kind, id, reason, note) = create_attributes(&body).map_err(sync_failure)?;
        let source = find_source(db, context, kind, id, false)
            .await?
            .ok_or_else(|| sync_failure(Failure::NotFound))?;
        if !person_access(db, context, source.person_id(), true).await? {
            return Err(sync_failure(Failure::Forbidden));
        }
        let existing = open_period(db, &source).await?.is_some();
        let (source, period) = pause_source(db, context, source, reason, note, request_id).await?;
        (source, period, existing)
    } else if operation.action == "close" {
        if !operation.attributes.is_empty() {
            return Err(sync_failure(Failure::Invalid(
                "attributes",
                "must be empty",
            )));
        }
        let id = operation
            .id
            .as_deref()
            .ok_or_else(|| sync_failure(Failure::NotFound))?;
        let period = pause_period::Entity::find()
            .filter(pause_period::Column::HouseholdId.eq(context.membership.household_id))
            .filter(pause_period::Column::PortableId.eq(id))
            .one(db)
            .await
            .map_err(database_error)?
            .ok_or_else(|| sync_failure(Failure::NotFound))?;
        let source = source_for_period(db, context, &period, false)
            .await?
            .ok_or_else(|| sync_failure(Failure::NotFound))?;
        if !person_access(db, context, source.person_id(), true).await? {
            return Err(sync_failure(Failure::Forbidden));
        }
        let (_, etag) = period_body(db, &period, &source).await?;
        sync_precondition(&etag, operation.if_match.as_deref())?;
        let replayed = period.ended_at.is_some();
        let (source, period) = close_period(db, context, source, Some(period), request_id).await?;
        (source, period.ok_or_else(ApiError::not_found)?, replayed)
    } else {
        return Err(sync_failure(Failure::Malformed));
    };
    let (_, etag) = period_body(db, &period, &source).await?;
    Ok(SyncResult {
        record_type: "MedicationPausePeriod",
        record_id: Some(period.id),
        record_portable_id: Some(period.portable_id),
        etag: Some(etag),
        replayed: Some(replayed),
    })
}

async fn reorder_sync(
    db: &DatabaseTransaction,
    context: &AuthContext,
    source: person_medication::Model,
    operation: &SyncOperation,
    request_id: &str,
) -> Result<SyncResult, ApiError> {
    let direction = operation
        .attributes
        .get("direction")
        .and_then(Value::as_str);
    if operation.attributes.len() != 1 || !matches!(direction, Some("up" | "down")) {
        return Err(ApiError {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: "unprocessable_content",
            message: "Direction must be up or down",
            preserve_activity: false,
        });
    }
    let direction = direction.unwrap();
    let mut query = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(context.membership.household_id))
        .filter(person_medication::Column::PersonId.eq(source.person_id))
        .filter(person_medication::Column::RetiredAt.is_null())
        .filter(if direction == "up" {
            person_medication::Column::Position.lt(source.position)
        } else {
            person_medication::Column::Position.gt(source.position)
        });
    query = if direction == "up" {
        query
            .order_by_desc(person_medication::Column::Position)
            .order_by_desc(person_medication::Column::Id)
    } else {
        query
            .order_by_asc(person_medication::Column::Position)
            .order_by_asc(person_medication::Column::Id)
    };
    let adjacent = query.one(db).await.map_err(database_error)?;
    let updated = if let Some(adjacent) = adjacent {
        let mut moving: person_medication::ActiveModel = source.clone().into();
        moving.position = Set(adjacent.position);
        moving.updated_at = Set(Utc::now().naive_utc());
        let moving = moving.update(db).await.map_err(database_error)?;
        let mut swapping: person_medication::ActiveModel = adjacent.clone().into();
        swapping.position = Set(source.position);
        swapping.updated_at = Set(Utc::now().naive_utc());
        let swapping = swapping.update(db).await.map_err(database_error)?;
        let person_portable_id = person::Entity::find_by_id(source.person_id)
            .one(db)
            .await
            .map_err(database_error)?
            .ok_or_else(ApiError::not_found)?
            .portable_id;
        for (before, after) in [(&source, &moving), (&adjacent, &swapping)] {
            record_version(
                db,
                context,
                request_id,
                "PersonMedication",
                after.id,
                "update",
                Some(json!({"position": before.position})),
                Some(json!({"position": after.position})),
            )
            .await?;
            record_change(
                db,
                context,
                request_id,
                SyncRecord {
                    record_type: "PersonMedication",
                    record_id: after.id,
                    portable_id: &after.portable_id,
                    action: "update",
                    person_portable_id: Some(&person_portable_id),
                },
            )
            .await?;
        }
        moving
    } else {
        source
    };
    let source = Source::Assignment(updated);
    let etag = representation_etag(&source_body(db, context, &source).await?);
    Ok(sync_result(&source, etag))
}

pub(super) async fn authorize_sync_replay(
    db: &DatabaseTransaction,
    context: &AuthContext,
    operation: &SyncOperation,
    saved: &Value,
) -> Result<(), ApiError> {
    if saved.get("action").and_then(Value::as_str) != Some(operation.action.as_str()) {
        return Err(ApiError::forbidden());
    }
    let portable_id = saved
        .get("record_portable_id")
        .and_then(Value::as_str)
        .ok_or_else(ApiError::forbidden)?;
    if operation.resource_type == "medication_pause_period" {
        if saved.get("record_type").and_then(Value::as_str) != Some("MedicationPausePeriod") {
            return Err(ApiError::forbidden());
        }
        let period = pause_period::Entity::find()
            .filter(pause_period::Column::HouseholdId.eq(context.membership.household_id))
            .filter(pause_period::Column::PortableId.eq(portable_id))
            .one(db)
            .await
            .map_err(database_error)?
            .ok_or_else(ApiError::forbidden)?;
        if saved.get("record_id").and_then(Value::as_str) != Some(period.id.to_string().as_str())
            || operation
                .id
                .as_deref()
                .is_some_and(|id| id != period.portable_id && id != period.id.to_string())
        {
            return Err(ApiError::forbidden());
        }
        let source = source_for_period(db, context, &period, true)
            .await?
            .ok_or_else(ApiError::forbidden)?;
        if !person_access(db, context, source.person_id(), true).await? {
            return Err(ApiError::forbidden());
        }
        if operation.action == "create"
            && (operation
                .attributes
                .get("source_type")
                .and_then(Value::as_str)
                != Some(source.kind().name())
                || operation
                    .attributes
                    .get("source_id")
                    .and_then(Value::as_str)
                    != Some(source.portable_id()))
        {
            return Err(ApiError::forbidden());
        }
        return Ok(());
    }
    let kind = match operation.resource_type.as_str() {
        "schedule" => Kind::Schedule,
        "person_medication" => Kind::Assignment,
        _ => return Err(ApiError::forbidden()),
    };
    if saved.get("record_type").and_then(Value::as_str) != Some(kind.record_type()) {
        return Err(ApiError::forbidden());
    }
    let source = find_source(db, context, kind, portable_id, true)
        .await?
        .ok_or_else(ApiError::forbidden)?;
    if saved.get("record_id").and_then(Value::as_str) != Some(source.id().to_string().as_str())
        || operation
            .id
            .as_deref()
            .is_none_or(|id| id != source.portable_id() && id != source.id().to_string())
        || !person_access(db, context, source.person_id(), true).await?
    {
        return Err(ApiError::forbidden());
    }
    Ok(())
}
