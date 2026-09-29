use crate::dosage_options::valid_identifier;
use crate::entities::{
    api_tombstone, grant, health_event, health_event_medication, medication, person, version,
};
use crate::medication_management::{
    finish, finish_with_request_id, household_manager, record_version, request_context,
    visible_medication,
};
use crate::mutation_idempotency::lock_household_and_reauthenticate;
use crate::sync_batch::{SyncOperation, SyncResult};
use crate::sync_events::{record_change, SyncRecord};
use crate::{database_error, representation_etag, ApiError, AppState, AuthContext, Pagination};
use axum::extract::{rejection::JsonRejection, rejection::QueryRejection, Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::Response;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, Condition, DatabaseTransaction, EntityTrait,
    IntoActiveModel, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, QueryTrait,
};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

const CONTROLLER: &str = "api/v1/health_events";
const POLICY: &str = "HealthEventPolicy";

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/households/{household_id}/health_events",
            get(index).post(create),
        )
        .route(
            "/api/v1/households/{household_id}/health_events/{id}",
            get(show).patch(patch).put(put),
        )
}

struct Attributes {
    person_id: Option<String>,
    event_kind: Option<i32>,
    severity: Option<i32>,
    title: Option<String>,
    notes: Option<String>,
    started_on: Option<NaiveDate>,
    ended_on: Option<NaiveDate>,
    medication_ids: Option<Vec<String>>,
}

fn parse_date(value: &Value) -> Result<NaiveDate, StatusCode> {
    let text = value.as_str().ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
    let bytes = text.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    NaiveDate::parse_from_str(text, "%Y-%m-%d").map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)
}

fn parse_attributes(body: &Value, create: bool) -> Result<Attributes, StatusCode> {
    let outer = body.as_object().ok_or(StatusCode::BAD_REQUEST)?;
    let inner = outer
        .get("health_event")
        .and_then(Value::as_object)
        .ok_or(StatusCode::BAD_REQUEST)?;
    if outer.len() != 1
        || inner.is_empty()
        || inner.keys().any(|key| {
            !matches!(
                key.as_str(),
                "person_id"
                    | "event_kind"
                    | "severity"
                    | "title"
                    | "notes"
                    | "started_on"
                    | "ended_on"
                    | "medication_ids"
            )
        })
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let identifier = |key: &str| -> Result<Option<String>, StatusCode> {
        let Some(value) = inner.get(key) else {
            return Ok(None);
        };
        let text = value.as_str().ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
        if !valid_identifier(text) {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        Ok(Some(text.to_owned()))
    };
    let person_id = identifier("person_id")?;
    let event_kind = match inner.get("event_kind") {
        Some(Value::String(value)) if value == "illness" => Some(0),
        Some(Value::String(value)) if value == "suspected_side_effect" => Some(1),
        Some(_) => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        None => None,
    };
    let severity = match inner.get("severity") {
        Some(Value::String(value)) if value == "mild" => Some(0),
        Some(Value::String(value)) if value == "moderate" => Some(1),
        Some(Value::String(value)) if value == "severe" => Some(2),
        Some(_) => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        None => None,
    };
    let title = match inner.get("title") {
        Some(Value::String(value)) if !value.trim().is_empty() => Some(value.to_owned()),
        Some(_) => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        None => None,
    };
    let notes = match inner.get("notes") {
        Some(Value::String(value)) => Some(value.to_owned()),
        Some(_) => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        None => None,
    };
    let started_on = inner.get("started_on").map(parse_date).transpose()?;
    let ended_on = inner.get("ended_on").map(parse_date).transpose()?;
    let medication_ids = match inner.get("medication_ids") {
        Some(Value::Array(values)) => {
            let mut ids = Vec::with_capacity(values.len());
            let mut unique = HashSet::new();
            for value in values {
                let text = value.as_str().ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
                if !valid_identifier(text) || !unique.insert(text) {
                    return Err(StatusCode::UNPROCESSABLE_ENTITY);
                }
                ids.push(text.to_owned());
            }
            Some(ids)
        }
        Some(_) => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        None => None,
    };
    if create
        && (person_id.is_none() || event_kind.is_none() || title.is_none() || started_on.is_none())
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    Ok(Attributes {
        person_id,
        event_kind,
        severity,
        title,
        notes,
        started_on,
        ended_on,
        medication_ids,
    })
}

fn field_type_errors(body: &Value) -> Option<Value> {
    let inner = body.get("health_event")?.as_object()?;
    if inner
        .get("person_id")
        .is_some_and(|value| !value.is_string())
    {
        return Some(json!({"person_id": ["must be a string"]}));
    }
    if inner
        .get("medication_ids")
        .and_then(Value::as_array)
        .is_some_and(|values| values.iter().any(|value| !value.is_string()))
    {
        return Some(json!({"medication_ids": ["must be a string"]}));
    }
    None
}

fn event_kind(value: i32) -> &'static str {
    match value {
        1 => "suspected_side_effect",
        _ => "illness",
    }
}

fn severity(value: Option<i32>) -> Option<&'static str> {
    value.map(|value| match value {
        2 => "severe",
        1 => "moderate",
        _ => "mild",
    })
}

fn snapshot(record: &health_event::Model) -> Value {
    json!({
        "id": record.id,
        "household_id": record.household_id,
        "person_id": record.person_id,
        "portable_id": record.portable_id,
        "event_kind": record.event_kind,
        "severity": record.severity,
        "title": record.title,
        "notes": record.notes,
        "started_on": record.started_on,
        "ended_on": record.ended_on,
        "updated_at": record.updated_at
    })
}

pub(super) async fn values(
    db: &DatabaseTransaction,
    records: &[health_event::Model],
) -> Result<Vec<Value>, ApiError> {
    if records.is_empty() {
        return Ok(Vec::new());
    }
    let event_ids = records.iter().map(|record| record.id).collect::<Vec<_>>();
    let person_ids = records
        .iter()
        .map(|record| record.person_id)
        .collect::<Vec<_>>();
    let people = person::Entity::find()
        .filter(person::Column::Id.is_in(person_ids))
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|person| (person.id, person.portable_id))
        .collect::<HashMap<_, _>>();
    let links = health_event_medication::Entity::find()
        .filter(health_event_medication::Column::HealthEventId.is_in(event_ids))
        .order_by_asc(health_event_medication::Column::Id)
        .all(db)
        .await
        .map_err(database_error)?;
    let medication_ids = links
        .iter()
        .filter_map(|link| link.medication_id)
        .collect::<Vec<_>>();
    let medications = medication::Entity::find()
        .filter(medication::Column::Id.is_in(medication_ids))
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|medication| (medication.id, medication.portable_id))
        .collect::<HashMap<_, _>>();
    let mut linked = HashMap::<i64, Vec<(i64, String)>>::new();
    for link in links {
        if let Some(id) = link.medication_id {
            if let Some(portable_id) = medications.get(&id) {
                linked
                    .entry(link.health_event_id)
                    .or_default()
                    .push((id, portable_id.clone()));
            }
        }
    }
    Ok(records
        .iter()
        .map(|record| {
            let associated = linked.remove(&record.id).unwrap_or_default();
            let medication_ids = associated.iter().map(|(id, _)| *id).collect::<Vec<_>>();
            let medication_portable_ids = associated
                .into_iter()
                .map(|(_, portable_id)| portable_id)
                .collect::<Vec<_>>();
            json!({
                "id": record.id,
                "portable_id": record.portable_id,
                "person_id": record.person_id,
                "person_portable_id": people.get(&record.person_id),
                "event_kind": event_kind(record.event_kind),
                "severity": severity(record.severity),
                "title": record.title,
                "notes": record.notes,
                "started_on": record.started_on.format("%Y-%m-%d").to_string(),
                "ended_on": record.ended_on.map(|date| date.format("%Y-%m-%d").to_string()),
                "updated_at": record.updated_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                "medication_ids": medication_ids,
                "medication_portable_ids": medication_portable_ids
            })
        })
        .collect())
}

pub(super) async fn representation(
    db: &DatabaseTransaction,
    record: &health_event::Model,
) -> Result<(Value, String), ApiError> {
    let value = values(db, std::slice::from_ref(record))
        .await?
        .into_iter()
        .next()
        .ok_or_else(ApiError::internal)?;
    let body = json!({"data": value});
    let etag = representation_etag(&body);
    Ok((body, etag))
}

async fn access(
    db: &DatabaseTransaction,
    context: &AuthContext,
    person_id: i64,
    level: &str,
) -> Result<bool, ApiError> {
    let levels = match level {
        "manage" => vec!["manage"],
        "record" => vec!["record", "manage"],
        _ => vec!["view", "record", "manage"],
    };
    Ok(grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(context.membership.household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(context.membership.id))
        .filter(grant::Column::PersonId.eq(person_id))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(grant::Column::AccessLevel.is_in(levels))
        .filter(
            Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(grant::Column::ExpiresAt.gt(Utc::now().naive_utc())),
        )
        .one(db)
        .await
        .map_err(database_error)?
        .is_some())
}

async fn visible_person(
    db: &DatabaseTransaction,
    context: &AuthContext,
    id: &str,
) -> Result<Option<person::Model>, ApiError> {
    if !valid_identifier(id) {
        return Ok(None);
    }
    let mut query = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(context.membership.household_id));
    query = if let Ok(id) = id.parse::<i64>() {
        query.filter(person::Column::Id.eq(id))
    } else {
        query.filter(person::Column::PortableId.eq(id))
    };
    let found = query.one(db).await.map_err(database_error)?;
    let Some(found) = found else {
        return Ok(None);
    };
    if access(db, context, found.id, "view").await? {
        Ok(Some(found))
    } else {
        Ok(None)
    }
}

async fn visible_event(
    db: &DatabaseTransaction,
    context: &AuthContext,
    id: &str,
) -> Result<Option<health_event::Model>, ApiError> {
    if !valid_identifier(id) {
        return Ok(None);
    }
    let mut query = health_event::Entity::find()
        .filter(health_event::Column::HouseholdId.eq(context.membership.household_id));
    query = if let Ok(id) = id.parse::<i64>() {
        query.filter(health_event::Column::Id.eq(id))
    } else {
        query.filter(health_event::Column::PortableId.eq(id))
    };
    let found = query.one(db).await.map_err(database_error)?;
    let Some(found) = found else {
        return Ok(None);
    };
    if access(db, context, found.person_id, "view").await? {
        Ok(Some(found))
    } else {
        Ok(None)
    }
}

async fn record_reassignment(
    db: &DatabaseTransaction,
    context: &AuthContext,
    record: &health_event::Model,
    original: &person::Model,
    target: &person::Model,
) -> Result<(), ApiError> {
    if original.id == target.id {
        return Ok(());
    }
    let now = Utc::now().naive_utc();
    api_tombstone::ActiveModel {
        household_id: Set(context.membership.household_id),
        household_membership_id: Set(Some(context.membership.id)),
        account_id: Set(Some(context.account_id)),
        action: Set("delete".to_owned()),
        record_type: Set("HealthEvent".to_owned()),
        record_portable_id: Set(record.portable_id.clone()),
        metadata: Set(json!({"record_type": "HealthEvent", "record_id": record.id,
            "portable_id": record.portable_id, "person_portable_id": original.portable_id,
            "reassigned_to_person_portable_id": target.portable_id})),
        deleted_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(database_error)?;
    Ok(())
}

async fn medications(
    db: &DatabaseTransaction,
    context: &AuthContext,
    identifiers: &[String],
) -> Result<Option<Vec<medication::Model>>, ApiError> {
    let mut result = Vec::with_capacity(identifiers.len());
    let mut unique = HashSet::new();
    for identifier in identifiers {
        let Some(medication) = visible_medication(db, context, identifier).await? else {
            return Ok(None);
        };
        if !unique.insert(medication.id) {
            return Err(ApiError {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                code: "validation_failed",
                message: "Validation failed",
                preserve_activity: false,
            });
        }
        result.push(medication);
    }
    Ok(Some(result))
}

async fn set_medications(
    db: &DatabaseTransaction,
    context: &AuthContext,
    event_id: i64,
    records: &[medication::Model],
) -> Result<(), ApiError> {
    let existing = health_event_medication::Entity::find()
        .filter(health_event_medication::Column::HealthEventId.eq(event_id))
        .all(db)
        .await
        .map_err(database_error)?;
    let selected_ids = records
        .iter()
        .map(|record| record.id)
        .collect::<HashSet<_>>();
    let existing_ids = existing
        .iter()
        .filter_map(|record| record.medication_id)
        .collect::<HashSet<_>>();
    let removed = existing
        .iter()
        .filter(|record| {
            !record
                .medication_id
                .is_some_and(|id| selected_ids.contains(&id))
        })
        .map(|record| record.id)
        .collect::<Vec<_>>();
    if !removed.is_empty() {
        health_event_medication::Entity::delete_many()
            .filter(health_event_medication::Column::Id.is_in(removed))
            .exec(db)
            .await
            .map_err(database_error)?;
    }
    let now = Utc::now().naive_utc();
    for record in records {
        if existing_ids.contains(&record.id) {
            continue;
        }
        health_event_medication::ActiveModel {
            household_id: Set(context.membership.household_id),
            health_event_id: Set(event_id),
            medication_id: Set(Some(record.id)),
            medication_name: Set(record
                .name
                .as_deref()
                .or(record.friendly_name.as_deref())
                .unwrap_or("Medication")
                .to_owned()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(database_error)?;
    }
    Ok(())
}

async fn failure(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    action: &str,
    status: StatusCode,
) -> Result<Response, ApiError> {
    failure_with_errors(db, context, method, action, status, None).await
}

async fn failure_with_errors(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    action: &str,
    status: StatusCode,
    errors: Option<Value>,
) -> Result<Response, ApiError> {
    let (code, message) = match status {
        StatusCode::BAD_REQUEST => ("bad_request", "Invalid request body"),
        StatusCode::FORBIDDEN => (
            "forbidden",
            "You are not authorized to perform this action.",
        ),
        StatusCode::NOT_FOUND => ("not_found", "Record not found"),
        StatusCode::CONFLICT => ("conflict", "Record has changed since it was last read"),
        _ => ("validation_failed", "Validation failed"),
    };
    let mut body = json!({"error": {"code": code, "message": message}});
    if let Some(errors) = errors {
        body["error"]["errors"] = errors;
    }
    finish(
        db,
        context,
        method,
        CONTROLLER,
        POLICY,
        action,
        status,
        status != StatusCode::FORBIDDEN,
        body,
        None,
    )
    .await
}

fn page(pagination: Pagination) -> Result<(u64, u64, Option<chrono::NaiveDateTime>), StatusCode> {
    let page = pagination.page.unwrap_or(1);
    let per_page = pagination.per_page.unwrap_or(20);
    if page < 1 || !(1..=100).contains(&per_page) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let offset = (page - 1)
        .checked_mul(per_page)
        .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
    let updated_since = pagination
        .updated_since
        .map(|value| DateTime::parse_from_rfc3339(&value).map(|time| time.naive_utc()))
        .transpose()
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    Ok((offset as u64, per_page as u64, updated_since))
}

pub(super) async fn index(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    pagination: Result<Query<Pagination>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let Ok(Query(pagination)) = pagination else {
        return failure(
            db,
            &context,
            "GET",
            "index",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await;
    };
    let (offset, per_page, updated_since) = match page(pagination) {
        Ok(page) => page,
        Err(status) => return failure(db, &context, "GET", "index", status).await,
    };
    let mut query =
        health_event::Entity::find().filter(health_event::Column::HouseholdId.eq(household_id));
    if !household_manager(&context) {
        let granted_ids = grant::Entity::find()
            .select_only()
            .column(grant::Column::PersonId)
            .filter(grant::Column::HouseholdId.eq(household_id))
            .filter(grant::Column::HouseholdMembershipId.eq(context.membership.id))
            .filter(grant::Column::RevokedAt.is_null())
            .filter(
                Condition::any()
                    .add(grant::Column::ExpiresAt.is_null())
                    .add(grant::Column::ExpiresAt.gt(Utc::now().naive_utc())),
            )
            .into_query();
        query = query.filter(health_event::Column::PersonId.in_subquery(granted_ids));
    }
    if let Some(since) = updated_since {
        query = query.filter(health_event::Column::UpdatedAt.gte(since));
    }
    let total = query.clone().count(&db).await.map_err(database_error)?;
    let records = query
        .order_by_asc(health_event::Column::Id)
        .offset(offset)
        .limit(per_page)
        .all(&db)
        .await
        .map_err(database_error)?;
    let data = values(&db, &records).await?;
    finish(
        db,
        &context,
        "GET",
        CONTROLLER,
        POLICY,
        "index",
        StatusCode::OK,
        true,
        json!({"data": data, "meta": {"page": offset / per_page + 1, "per_page": per_page, "total_count": total}}),
        None,
    )
    .await
}

pub(super) async fn show(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let Some(record) = visible_event(&db, &context, &id).await? else {
        return failure(db, &context, "GET", "show", StatusCode::NOT_FOUND).await;
    };
    let (body, etag) = representation(&db, &record).await?;
    finish(
        db,
        &context,
        "GET",
        CONTROLLER,
        POLICY,
        "show",
        StatusCode::OK,
        true,
        body,
        Some(&etag),
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
    let (_, context) =
        lock_household_and_reauthenticate(&state, &db, &headers, household_id).await?;
    let Ok(Json(body)) = payload else {
        return failure(db, &context, "POST", "create", StatusCode::BAD_REQUEST).await;
    };
    let attributes = match parse_attributes(&body, true) {
        Ok(attributes) => attributes,
        Err(status) => {
            return failure_with_errors(
                db,
                &context,
                "POST",
                "create",
                status,
                field_type_errors(&body),
            )
            .await;
        }
    };
    let Some(person) = visible_person(
        &db,
        &context,
        attributes
            .person_id
            .as_deref()
            .ok_or_else(ApiError::internal)?,
    )
    .await?
    else {
        return failure(db, &context, "POST", "create", StatusCode::NOT_FOUND).await;
    };
    if !access(&db, &context, person.id, "record").await? {
        return failure(db, &context, "POST", "create", StatusCode::FORBIDDEN).await;
    }
    let selected = match medications(
        &db,
        &context,
        attributes.medication_ids.as_deref().unwrap_or(&[]),
    )
    .await
    {
        Ok(Some(records)) => records,
        Ok(None) => return failure(db, &context, "POST", "create", StatusCode::NOT_FOUND).await,
        Err(error) if error.status == StatusCode::UNPROCESSABLE_ENTITY => {
            return failure(
                db,
                &context,
                "POST",
                "create",
                StatusCode::UNPROCESSABLE_ENTITY,
            )
            .await;
        }
        Err(error) => return Err(error),
    };
    let started_on = attributes.started_on.ok_or_else(ApiError::internal)?;
    if attributes.ended_on.is_some_and(|date| date < started_on) {
        return failure(
            db,
            &context,
            "POST",
            "create",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await;
    }
    let now = Utc::now().naive_utc();
    let record = health_event::ActiveModel {
        household_id: Set(household_id),
        person_id: Set(person.id),
        portable_id: Set(Uuid::new_v4().to_string()),
        event_kind: Set(attributes.event_kind.ok_or_else(ApiError::internal)?),
        severity: Set(attributes.severity),
        title: Set(attributes.title.ok_or_else(ApiError::internal)?),
        notes: Set(attributes.notes),
        started_on: Set(started_on),
        ended_on: Set(attributes.ended_on),
        action_taken: Set(None),
        medical_help_sought: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .map_err(database_error)?;
    set_medications(&db, &context, record.id, &selected).await?;
    let request_id = Uuid::new_v4().to_string();
    record_version(
        &db,
        &context,
        &request_id,
        "HealthEvent",
        record.id,
        "create",
        None,
        Some(snapshot(&record)),
    )
    .await?;
    record_change(
        &db,
        &context,
        &request_id,
        SyncRecord {
            record_type: "HealthEvent",
            record_id: record.id,
            portable_id: &record.portable_id,
            action: "create",
            person_portable_id: Some(&person.portable_id),
        },
    )
    .await?;
    let (body, etag) = representation(&db, &record).await?;
    finish_with_request_id(
        db,
        &context,
        &request_id,
        "POST",
        CONTROLLER,
        POLICY,
        "create",
        StatusCode::CREATED,
        true,
        body,
        Some(&etag),
    )
    .await
}

async fn update(
    state: AppState,
    household_id: i64,
    id: String,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
    method: &str,
) -> Result<Response, ApiError> {
    let (db, _) = request_context(&state, &headers, household_id).await?;
    let (_, context) =
        lock_household_and_reauthenticate(&state, &db, &headers, household_id).await?;
    let Some(record) = visible_event(&db, &context, &id).await? else {
        return failure(db, &context, method, "update", StatusCode::NOT_FOUND).await;
    };
    if !access(&db, &context, record.person_id, "manage").await? {
        return failure(db, &context, method, "update", StatusCode::FORBIDDEN).await;
    }
    let (before_body, current_etag) = representation(&db, &record).await?;
    if headers
        .get(header::IF_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| !value.is_empty() && value != current_etag)
    {
        return failure(db, &context, method, "update", StatusCode::CONFLICT).await;
    }
    let Ok(Json(body)) = payload else {
        return failure(db, &context, method, "update", StatusCode::BAD_REQUEST).await;
    };
    let attributes = match parse_attributes(&body, false) {
        Ok(attributes) => attributes,
        Err(status) => {
            return failure_with_errors(
                db,
                &context,
                method,
                "update",
                status,
                field_type_errors(&body),
            )
            .await;
        }
    };
    let original_person = person::Entity::find_by_id(record.person_id)
        .one(&db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::internal)?;
    let target_person = if let Some(person_id) = attributes.person_id.as_deref() {
        let Some(person) = visible_person(&db, &context, person_id).await? else {
            return failure(db, &context, method, "update", StatusCode::NOT_FOUND).await;
        };
        if !access(&db, &context, person.id, "manage").await? {
            return failure(db, &context, method, "update", StatusCode::FORBIDDEN).await;
        }
        person
    } else {
        original_person.clone()
    };
    let selected = if let Some(ids) = attributes.medication_ids.as_deref() {
        match medications(&db, &context, ids).await {
            Ok(Some(records)) => Some(records),
            Ok(None) => {
                return failure(db, &context, method, "update", StatusCode::NOT_FOUND).await
            }
            Err(error) if error.status == StatusCode::UNPROCESSABLE_ENTITY => {
                return failure(
                    db,
                    &context,
                    method,
                    "update",
                    StatusCode::UNPROCESSABLE_ENTITY,
                )
                .await;
            }
            Err(error) => return Err(error),
        }
    } else {
        None
    };
    let started_on = attributes.started_on.unwrap_or(record.started_on);
    let ended_on = attributes.ended_on.or(record.ended_on);
    if ended_on.is_some_and(|date| date < started_on) {
        return failure(
            db,
            &context,
            method,
            "update",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await;
    }
    let selected_ids = selected
        .as_ref()
        .map(|records| records.iter().map(|record| record.id).collect::<Vec<_>>());
    let previous_ids = before_body["data"]["medication_ids"]
        .as_array()
        .ok_or_else(ApiError::internal)?
        .iter()
        .filter_map(Value::as_i64)
        .collect::<Vec<_>>();
    let unchanged = target_person.id == record.person_id
        && attributes.event_kind.unwrap_or(record.event_kind) == record.event_kind
        && attributes.severity.unwrap_or(record.severity.unwrap_or(-1))
            == record.severity.unwrap_or(-1)
        && attributes.title.as_deref().unwrap_or(&record.title) == record.title
        && attributes.notes.as_deref().or(record.notes.as_deref()) == record.notes.as_deref()
        && started_on == record.started_on
        && ended_on == record.ended_on
        && selected_ids.as_ref().is_none_or(|ids| ids == &previous_ids);
    if unchanged {
        return finish(
            db,
            &context,
            method,
            CONTROLLER,
            POLICY,
            "update",
            StatusCode::OK,
            true,
            before_body,
            Some(&current_etag),
        )
        .await;
    }
    let before = snapshot(&record);
    let mut active = record.clone().into_active_model();
    active.person_id = Set(target_person.id);
    active.event_kind = Set(attributes.event_kind.unwrap_or(record.event_kind));
    active.severity = Set(attributes.severity.or(record.severity));
    active.title = Set(attributes.title.unwrap_or(record.title.clone()));
    active.notes = Set(attributes.notes.or(record.notes.clone()));
    active.started_on = Set(started_on);
    active.ended_on = Set(ended_on);
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active.update(&db).await.map_err(database_error)?;
    if let Some(selected) = selected {
        set_medications(&db, &context, updated.id, &selected).await?;
    }
    let request_id = Uuid::new_v4().to_string();
    record_version(
        &db,
        &context,
        &request_id,
        "HealthEvent",
        updated.id,
        "update",
        Some(before),
        Some(snapshot(&updated)),
    )
    .await?;
    record_reassignment(&db, &context, &updated, &original_person, &target_person).await?;
    record_change(
        &db,
        &context,
        &request_id,
        SyncRecord {
            record_type: "HealthEvent",
            record_id: updated.id,
            portable_id: &updated.portable_id,
            action: "update",
            person_portable_id: Some(&target_person.portable_id),
        },
    )
    .await?;
    let (body, etag) = representation(&db, &updated).await?;
    finish_with_request_id(
        db,
        &context,
        &request_id,
        method,
        CONTROLLER,
        POLICY,
        "update",
        StatusCode::OK,
        true,
        body,
        Some(&etag),
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
        StatusCode::NOT_FOUND => ("not_found", "Record not found"),
        StatusCode::FORBIDDEN => (
            "forbidden",
            "You are not authorized to perform this action.",
        ),
        StatusCode::PRECONDITION_REQUIRED => (
            "precondition_required",
            "A current resource version is required",
        ),
        StatusCode::CONFLICT => ("sync_conflict", "Record has changed since it was last read"),
        StatusCode::BAD_REQUEST => ("bad_request", "Invalid request body"),
        _ => ("unprocessable_content", "Health event is invalid"),
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
    let household_id = context.membership.household_id;
    if operation.action == "create" {
        let attrs = parse_attributes(&json!({"health_event": operation.attributes}), true)
            .map_err(sync_error)?;
        let person = visible_person(db, context, attrs.person_id.as_deref().unwrap())
            .await?
            .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?;
        if !access(db, context, person.id, "record").await? {
            return Err(sync_error(StatusCode::FORBIDDEN));
        }
        let selected = medications(db, context, attrs.medication_ids.as_deref().unwrap_or(&[]))
            .await?
            .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?;
        let started_on = attrs.started_on.unwrap();
        if attrs.ended_on.is_some_and(|date| date < started_on) {
            return Err(sync_error(StatusCode::UNPROCESSABLE_ENTITY));
        }
        let now = Utc::now().naive_utc();
        let record = health_event::ActiveModel {
            household_id: Set(household_id),
            person_id: Set(person.id),
            portable_id: Set(Uuid::new_v4().to_string()),
            event_kind: Set(attrs.event_kind.unwrap()),
            severity: Set(attrs.severity),
            title: Set(attrs.title.unwrap()),
            notes: Set(attrs.notes),
            started_on: Set(started_on),
            ended_on: Set(attrs.ended_on),
            action_taken: Set(None),
            medical_help_sought: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(database_error)?;
        set_medications(db, context, record.id, &selected).await?;
        record_version(
            db,
            context,
            request_id,
            "HealthEvent",
            record.id,
            "create",
            None,
            Some(snapshot(&record)),
        )
        .await?;
        record_change(
            db,
            context,
            request_id,
            SyncRecord {
                record_type: "HealthEvent",
                record_id: record.id,
                portable_id: &record.portable_id,
                action: "create",
                person_portable_id: Some(&person.portable_id),
            },
        )
        .await?;
        let (_, etag) = representation(db, &record).await?;
        return Ok(SyncResult {
            record_type: "HealthEvent",
            record_id: Some(record.id),
            record_portable_id: Some(record.portable_id),
            etag: Some(etag),
            replayed: Some(false),
        });
    }
    let id = operation
        .id
        .as_deref()
        .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?;
    let record = visible_event(db, context, id)
        .await?
        .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?;
    if !access(db, context, record.person_id, "manage").await? {
        return Err(sync_error(StatusCode::FORBIDDEN));
    }
    let (before_body, current_etag) = representation(db, &record).await?;
    let expected = operation
        .if_match
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| sync_error(StatusCode::PRECONDITION_REQUIRED))?;
    if expected != current_etag {
        return Err(sync_error(StatusCode::CONFLICT));
    }
    let person = person::Entity::find_by_id(record.person_id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?;
    match operation.action.as_str() {
        "update" => {
            let attrs = parse_attributes(&json!({"health_event": operation.attributes}), false)
                .map_err(sync_error)?;
            let target_person = if let Some(id) = attrs.person_id.as_deref() {
                let requested = visible_person(db, context, id)
                    .await?
                    .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?;
                if !access(db, context, requested.id, "manage").await? {
                    return Err(sync_error(StatusCode::FORBIDDEN));
                }
                requested
            } else {
                person.clone()
            };
            let selected = attrs
                .medication_ids
                .as_deref()
                .map(|ids| medications(db, context, ids));
            let selected = if let Some(selected) = selected {
                Some(
                    selected
                        .await?
                        .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?,
                )
            } else {
                None
            };
            let started_on = attrs.started_on.unwrap_or(record.started_on);
            let ended_on = attrs.ended_on.or(record.ended_on);
            if ended_on.is_some_and(|date| date < started_on) {
                return Err(sync_error(StatusCode::UNPROCESSABLE_ENTITY));
            }
            let selected_ids = selected
                .as_ref()
                .map(|rows| rows.iter().map(|row| row.id).collect::<Vec<_>>());
            let previous_ids = before_body["data"]["medication_ids"]
                .as_array()
                .ok_or_else(ApiError::internal)?
                .iter()
                .filter_map(Value::as_i64)
                .collect::<Vec<_>>();
            let unchanged = target_person.id == record.person_id
                && attrs.event_kind.unwrap_or(record.event_kind) == record.event_kind
                && attrs.severity.unwrap_or(record.severity.unwrap_or(-1))
                    == record.severity.unwrap_or(-1)
                && attrs.title.as_deref().unwrap_or(&record.title) == record.title
                && attrs.notes.as_deref().or(record.notes.as_deref()) == record.notes.as_deref()
                && started_on == record.started_on
                && ended_on == record.ended_on
                && selected_ids.as_ref().is_none_or(|ids| ids == &previous_ids);
            let updated = if unchanged {
                record
            } else {
                let before = snapshot(&record);
                let mut active = record.clone().into_active_model();
                active.person_id = Set(target_person.id);
                active.event_kind = Set(attrs.event_kind.unwrap_or(record.event_kind));
                active.severity = Set(attrs.severity.or(record.severity));
                active.title = Set(attrs.title.unwrap_or(record.title.clone()));
                active.notes = Set(attrs.notes.or(record.notes.clone()));
                active.started_on = Set(started_on);
                active.ended_on = Set(ended_on);
                active.updated_at = Set(Utc::now().naive_utc());
                let updated = active.update(db).await.map_err(database_error)?;
                if let Some(selected) = selected {
                    set_medications(db, context, updated.id, &selected).await?;
                }
                record_version(
                    db,
                    context,
                    request_id,
                    "HealthEvent",
                    updated.id,
                    "update",
                    Some(before),
                    Some(snapshot(&updated)),
                )
                .await?;
                record_reassignment(db, context, &updated, &person, &target_person).await?;
                record_change(
                    db,
                    context,
                    request_id,
                    SyncRecord {
                        record_type: "HealthEvent",
                        record_id: updated.id,
                        portable_id: &updated.portable_id,
                        action: "update",
                        person_portable_id: Some(&target_person.portable_id),
                    },
                )
                .await?;
                updated
            };
            let (_, etag) = representation(db, &updated).await?;
            Ok(SyncResult {
                record_type: "HealthEvent",
                record_id: Some(updated.id),
                record_portable_id: Some(updated.portable_id),
                etag: Some(etag),
                replayed: Some(unchanged),
            })
        }
        "delete" => {
            if !operation.attributes.is_empty() {
                return Err(sync_error(StatusCode::UNPROCESSABLE_ENTITY));
            }
            health_event_medication::Entity::delete_many()
                .filter(health_event_medication::Column::HealthEventId.eq(record.id))
                .exec(db)
                .await
                .map_err(database_error)?;
            health_event::Entity::delete_by_id(record.id)
                .exec(db)
                .await
                .map_err(database_error)?;
            record_version(
                db,
                context,
                request_id,
                "HealthEvent",
                record.id,
                "destroy",
                Some(snapshot(&record)),
                None,
            )
            .await?;
            let now = Utc::now().naive_utc();
            api_tombstone::ActiveModel {
                household_id: Set(household_id),
                household_membership_id: Set(Some(context.membership.id)),
                account_id: Set(Some(context.account_id)),
                action: Set("delete".to_owned()),
                record_type: Set("HealthEvent".to_owned()),
                record_portable_id: Set(record.portable_id.clone()),
                metadata: Set(json!({"record_type": "HealthEvent", "record_id": record.id,
                    "portable_id": record.portable_id, "person_portable_id": person.portable_id})),
                deleted_at: Set(now),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(database_error)?;
            Ok(SyncResult {
                record_type: "HealthEvent",
                record_id: Some(record.id),
                record_portable_id: Some(record.portable_id),
                etag: None,
                replayed: None,
            })
        }
        _ => Err(sync_error(StatusCode::UNPROCESSABLE_ENTITY)),
    }
}

pub(super) async fn authorize_sync_replay(
    db: &DatabaseTransaction,
    context: &AuthContext,
    operation: &SyncOperation,
    saved: &Value,
    original_request_id: &str,
) -> Result<(), ApiError> {
    if saved.get("action").and_then(Value::as_str) != Some(operation.action.as_str())
        || saved.get("record_type").and_then(Value::as_str) != Some("HealthEvent")
    {
        return Err(ApiError::forbidden());
    }
    let portable_id = saved
        .get("record_portable_id")
        .and_then(Value::as_str)
        .ok_or_else(ApiError::forbidden)?;
    let record_id = saved
        .get("record_id")
        .and_then(Value::as_str)
        .and_then(|id| id.parse::<i64>().ok())
        .ok_or_else(ApiError::forbidden)?;
    if operation
        .id
        .as_deref()
        .is_some_and(|id| id != portable_id && id != record_id.to_string())
    {
        return Err(ApiError::forbidden());
    }
    let record = health_event::Entity::find()
        .filter(health_event::Column::HouseholdId.eq(context.membership.household_id))
        .filter(health_event::Column::PortableId.eq(portable_id))
        .one(db)
        .await
        .map_err(database_error)?;
    let current_person_id = if let Some(record) = record.as_ref() {
        if record.id != record_id {
            return Err(ApiError::forbidden());
        }
        record.person_id
    } else if operation.action == "delete" {
        let tombstone = api_tombstone::Entity::find()
            .filter(api_tombstone::Column::HouseholdId.eq(context.membership.household_id))
            .filter(api_tombstone::Column::RecordType.eq("HealthEvent"))
            .filter(api_tombstone::Column::RecordPortableId.eq(portable_id))
            .all(db)
            .await
            .map_err(database_error)?
            .into_iter()
            .find(|row| {
                row.metadata["reassigned_to_person_portable_id"].is_null()
                    && row.metadata["record_id"].as_i64() == Some(record_id)
            })
            .ok_or_else(ApiError::forbidden)?;
        let person_portable_id = tombstone
            .metadata
            .get("person_portable_id")
            .and_then(Value::as_str)
            .ok_or_else(ApiError::forbidden)?;
        person::Entity::find()
            .filter(person::Column::HouseholdId.eq(context.membership.household_id))
            .filter(person::Column::PortableId.eq(person_portable_id))
            .one(db)
            .await
            .map_err(database_error)?
            .ok_or_else(ApiError::forbidden)?
            .id
    } else {
        return Err(ApiError::forbidden());
    };
    let level = if operation.action == "create" {
        "record"
    } else {
        "manage"
    };
    if !access(db, context, current_person_id, level).await? {
        return Err(ApiError::forbidden());
    }
    let versions = version::Entity::find()
        .filter(version::Column::HouseholdId.eq(context.membership.household_id))
        .filter(version::Column::ActorMembershipId.eq(context.membership.id))
        .filter(version::Column::RequestId.eq(original_request_id))
        .filter(version::Column::ItemType.eq("HealthEvent"))
        .filter(version::Column::ItemId.eq(record_id))
        .all(db)
        .await
        .map_err(database_error)?;
    let mut historical_person_ids = HashSet::new();
    for version in &versions {
        if let Some(object) = version.object.as_deref() {
            let object: Value = serde_json::from_str(object).map_err(|_| ApiError::forbidden())?;
            if let Some(id) = object["person_id"].as_i64() {
                historical_person_ids.insert(id);
            }
        }
        if let Some(changes) = version.object_changes.as_deref() {
            let changes: Value =
                serde_json::from_str(changes).map_err(|_| ApiError::forbidden())?;
            if let Some(values) = changes["person_id"].as_array() {
                for id in values.iter().filter_map(Value::as_i64) {
                    historical_person_ids.insert(id);
                }
            }
        }
    }
    if historical_person_ids.is_empty() {
        let unchanged = operation.action == "update"
            && saved.get("replayed").and_then(Value::as_bool) == Some(true)
            && record
                .as_ref()
                .is_some_and(|record| record.person_id == current_person_id);
        if !unchanged || !versions.is_empty() {
            return Err(ApiError::forbidden());
        }
        let (_, current_etag) = representation(db, record.as_ref().unwrap()).await?;
        if saved.get("etag").and_then(Value::as_str) != Some(current_etag.as_str()) {
            return Err(ApiError::forbidden());
        }
        historical_person_ids.insert(current_person_id);
    }
    for person_id in &historical_person_ids {
        if !access(db, context, *person_id, level).await? {
            return Err(ApiError::forbidden());
        }
    }
    if let Some(id) = operation
        .attributes
        .get("person_id")
        .and_then(Value::as_str)
    {
        if visible_person(db, context, id)
            .await?
            .is_none_or(|person| !historical_person_ids.contains(&person.id))
        {
            return Err(ApiError::forbidden());
        }
    }
    if let Some(ids) = operation
        .attributes
        .get("medication_ids")
        .and_then(Value::as_array)
    {
        for id in ids {
            let id = id.as_str().ok_or_else(ApiError::forbidden)?;
            if visible_medication(db, context, id).await?.is_none() {
                return Err(ApiError::forbidden());
            }
        }
    }
    Ok(())
}
