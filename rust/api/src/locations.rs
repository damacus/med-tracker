use crate::entities::{
    api_tombstone, dosage, grant, medication, medication_take, person, person_medication, schedule,
};
use crate::medication_management::{
    error_response, finish_with_request_id, record_version, request_context,
};
use crate::mutation_idempotency::{self, Lookup, StoredResponse};
use crate::read_entities::{dose_occurrence, location_membership, pause_period, stock_location};
use crate::read_resources::location_value;
use crate::sync_events::{record_change, SyncRecord};
use crate::{audit, database_error, representation_etag, ApiError, AppState, AuthContext};
use axum::extract::{rejection::JsonRejection, Path, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseTransaction, DbErr, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

fn manager(context: &AuthContext) -> bool {
    matches!(context.membership.role.as_str(), "owner" | "administrator")
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
    error_response(
        db,
        context,
        method,
        "api/v1/locations",
        "LocationPolicy",
        action,
        status,
        code,
        message,
        None,
    )
    .await
}

async fn person_manage_access(
    db: &DatabaseTransaction,
    context: &AuthContext,
    person_id: i64,
) -> Result<Option<bool>, ApiError> {
    let grant = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(context.membership.household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(context.membership.id))
        .filter(grant::Column::PersonId.eq(person_id))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(
            Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(grant::Column::ExpiresAt.gt(Utc::now().naive_utc())),
        )
        .one(db)
        .await
        .map_err(database_error)?;
    Ok(grant.map(|grant| grant.access_level == "manage"))
}

fn valid_identifier(value: &str) -> bool {
    valid_numeric_id(value)
        || (value.len() == 36
            && value.bytes().enumerate().all(|(index, byte)| match index {
                8 | 13 | 18 | 23 => byte == b'-',
                19 => matches!(byte, b'8' | b'9' | b'a' | b'b' | b'A' | b'B'),
                _ => byte.is_ascii_hexdigit(),
            }))
}

fn valid_numeric_id(value: &str) -> bool {
    value
        .as_bytes()
        .first()
        .is_some_and(|byte| (b'1'..=b'9').contains(byte))
        && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn attributes(body: &Value, create: bool) -> Option<(Option<String>, Option<Option<String>>)> {
    let outer = body.as_object()?;
    if outer.len() != 1 {
        return None;
    }
    let inner = outer.get("location")?.as_object()?;
    if inner
        .keys()
        .any(|key| key != "name" && key != "description")
        || inner.is_empty()
        || (create && !inner.contains_key("name"))
    {
        return None;
    }
    let name = match inner.get("name") {
        Some(value) => {
            let value = value.as_str()?;
            if value.is_empty() {
                return None;
            }
            Some(value.to_owned())
        }
        None => None,
    };
    let description = match inner.get("description") {
        Some(Value::Null) => Some(None),
        Some(Value::String(value)) => Some(Some(value.clone())),
        Some(_) => return None,
        None => None,
    };
    Some((name, description))
}

async fn location(
    db: &DatabaseTransaction,
    household_id: i64,
    id: &str,
    lock: bool,
) -> Result<Option<stock_location::Model>, ApiError> {
    let query =
        stock_location::Entity::find().filter(stock_location::Column::HouseholdId.eq(household_id));
    let query = if let Ok(id) = id.parse::<i64>() {
        query.filter(stock_location::Column::Id.eq(id))
    } else {
        query.filter(stock_location::Column::PortableId.eq(id))
    };
    let query = if lock { query.lock_exclusive() } else { query };
    query.one(db).await.map_err(database_error)
}

fn representation(record: stock_location::Model) -> (Value, String) {
    let body = json!({"data": location_value(record)});
    let etag = representation_etag(&body);
    (body, etag)
}

fn snapshot(record: &stock_location::Model) -> Value {
    json!({"id": record.id, "household_id": record.household_id, "portable_id": record.portable_id, "name": record.name, "description": record.description, "created_at": record.created_at, "updated_at": record.updated_at})
}

async fn tombstone(
    db: &DatabaseTransaction,
    context: &AuthContext,
    record_type: &str,
    record_id: i64,
    portable_id: &str,
    extra_metadata: Value,
) -> Result<(), ApiError> {
    let now = Utc::now().naive_utc();
    let mut metadata =
        json!({"record_type": record_type, "record_id": record_id, "portable_id": portable_id});
    if let (Some(base), Some(extra)) = (metadata.as_object_mut(), extra_metadata.as_object()) {
        base.extend(extra.clone());
    }
    api_tombstone::ActiveModel {
        household_id: Set(context.membership.household_id),
        household_membership_id: Set(Some(context.membership.id)),
        account_id: Set(Some(context.account_id)),
        action: Set("delete".to_owned()),
        record_type: Set(record_type.to_owned()),
        record_portable_id: Set(portable_id.to_owned()),
        metadata: Set(metadata),
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

#[allow(clippy::too_many_arguments)]
async fn location_change(
    db: &DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    record: &stock_location::Model,
    version_event: &str,
    sync_action: &str,
    before: Option<Value>,
    after: Option<Value>,
) -> Result<(), ApiError> {
    record_version(
        db,
        context,
        request_id,
        "Location",
        record.id,
        version_event,
        before,
        after,
    )
    .await?;
    if sync_action == "delete" {
        tombstone(
            db,
            context,
            "Location",
            record.id,
            &record.portable_id,
            json!({}),
        )
        .await
    } else {
        record_change(
            db,
            context,
            request_id,
            SyncRecord {
                record_type: "Location",
                record_id: record.id,
                portable_id: &record.portable_id,
                action: sync_action,
                person_portable_id: None,
            },
        )
        .await
    }
}

#[allow(clippy::too_many_arguments)]
async fn keyed_replay(
    db: &DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    method: &str,
    path: &str,
    body: &Value,
    action: &str,
    controller: &str,
    policy: &str,
) -> Result<Option<Response>, ApiError> {
    let Some(key) = mutation_idempotency::key(headers) else {
        return Ok(None);
    };
    let digest = mutation_idempotency::digest(method, path, body);
    match mutation_idempotency::lookup(db, context, key, method, path, &digest).await? {
        Lookup::New => Ok(None),
        Lookup::Replay(saved) => {
            let mut saved = *saved;
            let status = StatusCode::from_u16(saved.response_status as u16)
                .map_err(|_| ApiError::internal())?;
            let request_id = audit::record_resource_request(
                db, context, method, controller, policy, action, status, true,
            )
            .await
            .map_err(database_error)?;
            if status.is_client_error() && saved.response_body.get("error").is_some() {
                saved.response_body["error"]["request_id"] = json!(request_id);
            }
            let mut response = mutation_idempotency::replay(saved)?;
            response.headers_mut().insert(
                "x-request-id",
                HeaderValue::from_str(&request_id).map_err(|_| ApiError::internal())?,
            );
            Ok(Some(response))
        }
        Lookup::Conflict => {
            let request_id = audit::record_resource_request(
                db,
                context,
                method,
                controller,
                policy,
                action,
                StatusCode::CONFLICT,
                true,
            )
            .await
            .map_err(database_error)?;
            let mut response = (StatusCode::CONFLICT, Json(json!({"error":{"code":"idempotency_key_reused","message":"Idempotency key has already been used for a different request","request_id":request_id}}))).into_response();
            response.headers_mut().insert(
                "x-request-id",
                HeaderValue::from_str(&request_id).map_err(|_| ApiError::internal())?,
            );
            Ok(Some(response))
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn store_keyed(
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
    if let Some(key) = mutation_idempotency::key(headers) {
        mutation_idempotency::store(
            db,
            context,
            StoredResponse {
                key,
                method,
                path,
                digest: &mutation_idempotency::digest(method, path, request),
                status,
                body: body.clone(),
                request_id,
                etag,
            },
        )
        .await?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn keyed_failure(
    db: DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    method: &str,
    path: &str,
    request: &Value,
    controller: &str,
    policy: &str,
    action: &str,
    status: StatusCode,
    code: &str,
    message: &str,
    errors: Option<Value>,
) -> Result<Response, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let mut body = json!({"error": {"code": code, "message": message, "request_id": request_id}});
    if let Some(errors) = errors {
        body["error"]["errors"] = errors;
    }
    store_keyed(
        &db,
        context,
        headers,
        method,
        path,
        request,
        status,
        &body,
        &request_id,
        None,
    )
    .await?;
    finish_with_request_id(
        db,
        context,
        &request_id,
        method,
        controller,
        policy,
        action,
        status,
        true,
        body,
        None,
    )
    .await
}

async fn manager_context(
    state: &AppState,
    headers: &HeaderMap,
    household_id: i64,
    method: &str,
    action: &str,
) -> Result<Result<(DatabaseTransaction, AuthContext), Response>, ApiError> {
    let (db, _) = request_context(state, headers, household_id).await?;
    let (_, context) =
        mutation_idempotency::lock_household_and_reauthenticate(state, &db, headers, household_id)
            .await?;
    if manager(&context) {
        Ok(Ok((db, context)))
    } else {
        let response = failure(
            db,
            &context,
            method,
            action,
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
        )
        .await?;
        Ok(Err(response))
    }
}

pub(super) async fn create(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    let (db, context) =
        match manager_context(&state, &headers, household_id, "POST", "create").await? {
            Ok(value) => value,
            Err(response) => return Ok(response),
        };
    let Json(body) = match payload {
        Ok(body) => body,
        Err(_) => {
            return failure(
                db,
                &context,
                "POST",
                "create",
                StatusCode::BAD_REQUEST,
                "bad_request",
                "Invalid JSON request body",
            )
            .await;
        }
    };
    let path = format!("/api/v1/households/{household_id}/locations");
    if let Some(response) = keyed_replay(
        &db,
        &context,
        &headers,
        "POST",
        &path,
        &body,
        "create",
        "api/v1/locations",
        "LocationPolicy",
    )
    .await?
    {
        db.commit().await.map_err(database_error)?;
        return Ok(response);
    }
    let Some((Some(name), description)) = attributes(&body, true) else {
        return keyed_failure(
            db,
            &context,
            &headers,
            "POST",
            &path,
            &body,
            "api/v1/locations",
            "LocationPolicy",
            "create",
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            "Validation failed",
            Some(json!({"location": ["is invalid"]})),
        )
        .await;
    };
    let now = Utc::now().naive_utc();
    let active = stock_location::ActiveModel {
        household_id: Set(household_id),
        name: Set(name),
        description: Set(description.unwrap_or(None)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    let savepoint = db.begin().await.map_err(database_error)?;
    let record = match active.insert(&savepoint).await {
        Ok(record) => {
            savepoint.commit().await.map_err(database_error)?;
            record
        }
        Err(error)
            if matches!(
                error.sql_err(),
                Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
            ) =>
        {
            savepoint.rollback().await.map_err(database_error)?;
            return keyed_failure(
                db,
                &context,
                &headers,
                "POST",
                &path,
                &body,
                "api/v1/locations",
                "LocationPolicy",
                "create",
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_failed",
                "Validation failed",
                Some(json!({"name": ["has already been taken"]})),
            )
            .await;
        }
        Err(error) => return Err(database_error(error)),
    };
    let request_id = Uuid::new_v4().to_string();
    location_change(
        &db,
        &context,
        &request_id,
        &record,
        "create",
        "create",
        None,
        Some(snapshot(&record)),
    )
    .await?;
    let (response_body, etag) = representation(record);
    store_keyed(
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
    finish_with_request_id(
        db,
        &context,
        &request_id,
        "POST",
        "api/v1/locations",
        "LocationPolicy",
        "create",
        StatusCode::CREATED,
        true,
        response_body,
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
    let (db, context) =
        match manager_context(&state, &headers, household_id, method, "update").await? {
            Ok(value) => value,
            Err(response) => return Ok(response),
        };
    let Json(request) = match payload {
        Ok(body) => body,
        Err(_) => {
            return failure(
                db,
                &context,
                method,
                "update",
                StatusCode::BAD_REQUEST,
                "bad_request",
                "Invalid JSON request body",
            )
            .await;
        }
    };
    let path = format!("/api/v1/households/{household_id}/locations/{id}");
    if let Some(response) = keyed_replay(
        &db,
        &context,
        &headers,
        method,
        &path,
        &request,
        "update",
        "api/v1/locations",
        "LocationPolicy",
    )
    .await?
    {
        db.commit().await.map_err(database_error)?;
        return Ok(response);
    }
    if !valid_identifier(&id) {
        return keyed_failure(
            db,
            &context,
            &headers,
            method,
            &path,
            &request,
            "api/v1/locations",
            "LocationPolicy",
            "update",
            StatusCode::BAD_REQUEST,
            "bad_request",
            "Invalid resource ID",
            None,
        )
        .await;
    }
    let Some(record) = location(&db, household_id, &id, true).await? else {
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
    let Some(if_match) = headers
        .get(header::IF_MATCH)
        .and_then(|value| value.to_str().ok())
    else {
        return keyed_failure(
            db,
            &context,
            &headers,
            method,
            &path,
            &request,
            "api/v1/locations",
            "LocationPolicy",
            "update",
            StatusCode::PRECONDITION_REQUIRED,
            "precondition_required",
            "If-Match is required",
            None,
        )
        .await;
    };
    if if_match.is_empty() {
        return keyed_failure(
            db,
            &context,
            &headers,
            method,
            &path,
            &request,
            "api/v1/locations",
            "LocationPolicy",
            "update",
            StatusCode::PRECONDITION_REQUIRED,
            "precondition_required",
            "If-Match is required",
            None,
        )
        .await;
    }
    let (_, current_etag) = representation(record.clone());
    if if_match != current_etag {
        return failure(
            db,
            &context,
            method,
            "update",
            StatusCode::CONFLICT,
            "conflict",
            "Record has changed since it was last read",
        )
        .await;
    }
    let Some((name, description)) = attributes(&request, false) else {
        return keyed_failure(
            db,
            &context,
            &headers,
            method,
            &path,
            &request,
            "api/v1/locations",
            "LocationPolicy",
            "update",
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            "Validation failed",
            Some(json!({"location": ["is invalid"]})),
        )
        .await;
    };
    let changed = name.as_ref().is_some_and(|value| value != &record.name)
        || description
            .as_ref()
            .is_some_and(|value| value != &record.description);
    if !changed {
        let request_id = Uuid::new_v4().to_string();
        let (response_body, etag) = representation(record);
        store_keyed(
            &db,
            &context,
            &headers,
            method,
            &path,
            &request,
            StatusCode::OK,
            &response_body,
            &request_id,
            Some(&etag),
        )
        .await?;
        return finish_with_request_id(
            db,
            &context,
            &request_id,
            method,
            "api/v1/locations",
            "LocationPolicy",
            "update",
            StatusCode::OK,
            true,
            response_body,
            Some(&etag),
        )
        .await;
    }
    let before = snapshot(&record);
    let mut active: stock_location::ActiveModel = record.into();
    if let Some(name) = name {
        active.name = Set(name);
    }
    if let Some(description) = description {
        active.description = Set(description);
    }
    active.updated_at = Set(Utc::now().naive_utc());
    let savepoint = db.begin().await.map_err(database_error)?;
    let record = match active.update(&savepoint).await {
        Ok(record) => {
            savepoint.commit().await.map_err(database_error)?;
            record
        }
        Err(error)
            if matches!(
                error.sql_err(),
                Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
            ) =>
        {
            savepoint.rollback().await.map_err(database_error)?;
            return keyed_failure(
                db,
                &context,
                &headers,
                method,
                &path,
                &request,
                "api/v1/locations",
                "LocationPolicy",
                "update",
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_failed",
                "Validation failed",
                Some(json!({"name": ["has already been taken"]})),
            )
            .await;
        }
        Err(error) => return Err(database_error(error)),
    };
    let request_id = Uuid::new_v4().to_string();
    location_change(
        &db,
        &context,
        &request_id,
        &record,
        "update",
        "update",
        Some(before),
        Some(snapshot(&record)),
    )
    .await?;
    let (response_body, etag) = representation(record);
    store_keyed(
        &db,
        &context,
        &headers,
        method,
        &path,
        &request,
        StatusCode::OK,
        &response_body,
        &request_id,
        Some(&etag),
    )
    .await?;
    finish_with_request_id(
        db,
        &context,
        &request_id,
        method,
        "api/v1/locations",
        "LocationPolicy",
        "update",
        StatusCode::OK,
        true,
        response_body,
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

struct CascadeSnapshot {
    medications: Vec<medication::Model>,
    dosages: Vec<dosage::Model>,
    schedules: Vec<schedule::Model>,
    assignments: Vec<person_medication::Model>,
    memberships: Vec<location_membership::Model>,
}

async fn record_cascade_effects(
    db: &DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    cascade: &CascadeSnapshot,
) -> Result<(), ApiError> {
    let person_ids: HashSet<i64> = cascade
        .schedules
        .iter()
        .map(|row| row.person_id)
        .chain(cascade.assignments.iter().map(|row| row.person_id))
        .collect();
    let people = if person_ids.is_empty() {
        Vec::new()
    } else {
        person::Entity::find()
            .filter(person::Column::Id.is_in(person_ids))
            .all(db)
            .await
            .map_err(database_error)?
    };
    let person_portable_ids: HashMap<i64, String> = people
        .into_iter()
        .map(|row| (row.id, row.portable_id))
        .collect();
    for row in &cascade.memberships {
        record_version(
            db,
            context,
            request_id,
            "LocationMembership",
            row.id,
            "destroy",
            Some(json!({"id": row.id, "household_id": row.household_id, "location_id": row.location_id, "person_id": row.person_id, "created_at": row.created_at, "updated_at": row.updated_at})),
            None,
        )
        .await?;
    }
    for row in &cascade.schedules {
        record_version(
            db, context, request_id, "Schedule", row.id, "destroy",
            Some(json!({"id":row.id,"household_id":row.household_id,"portable_id":row.portable_id,"person_id":row.person_id,"medication_id":row.medication_id,"source_dosage_option_id":row.source_dosage_option_id,"dose_amount":row.dose_amount,"dose_unit":row.dose_unit,"frequency":row.frequency,"dose_cycle":row.dose_cycle,"start_date":row.start_date,"end_date":row.end_date,"active":row.active,"notes":row.notes,"created_at":row.created_at,"updated_at":row.updated_at,"schedule_type":row.schedule_type,"schedule_config":row.schedule_config,"max_daily_doses":row.max_daily_doses,"min_hours_between_doses":row.min_hours_between_doses,"retired_at":row.retired_at})),
            None,
        ).await?;
        let person_metadata = person_portable_ids.get(&row.person_id).map_or_else(
            || json!({}),
            |portable_id| json!({"person_portable_id": portable_id}),
        );
        tombstone(
            db,
            context,
            "Schedule",
            row.id,
            &row.portable_id,
            person_metadata,
        )
        .await?;
    }
    for row in &cascade.assignments {
        record_version(
            db, context, request_id, "PersonMedication", row.id, "destroy",
            Some(json!({"id":row.id,"household_id":row.household_id,"portable_id":row.portable_id,"person_id":row.person_id,"medication_id":row.medication_id,"source_dosage_option_id":row.source_dosage_option_id,"dose_amount":row.dose_amount,"dose_unit":row.dose_unit,"active":row.active,"notes":row.notes,"created_at":row.created_at,"updated_at":row.updated_at,"dose_cycle":row.dose_cycle,"administration_kind":row.administration_kind,"position":row.position,"max_daily_doses":row.max_daily_doses,"min_hours_between_doses":row.min_hours_between_doses,"retired_at":row.retired_at})),
            None,
        ).await?;
        let person_metadata = person_portable_ids.get(&row.person_id).map_or_else(
            || json!({}),
            |portable_id| json!({"person_portable_id": portable_id}),
        );
        tombstone(
            db,
            context,
            "PersonMedication",
            row.id,
            &row.portable_id,
            person_metadata,
        )
        .await?;
    }
    for row in &cascade.dosages {
        record_version(
            db, context, request_id, "MedicationDosageOption", row.id, "destroy",
            Some(json!({"id":row.id,"household_id":row.household_id,"medication_id":row.medication_id,"portable_id":row.portable_id,"amount":row.amount,"unit":row.unit,"current_supply":row.current_supply,"reorder_threshold":row.reorder_threshold,"frequency":row.frequency,"description":row.description,"default_for_adults":row.default_for_adults,"default_for_children":row.default_for_children,"default_max_daily_doses":row.default_max_daily_doses,"default_min_hours_between_doses":row.default_min_hours_between_doses,"default_dose_cycle":row.default_dose_cycle,"created_at":row.created_at,"updated_at":row.updated_at})),
            None,
        ).await?;
        tombstone(
            db,
            context,
            "MedicationDosageOption",
            row.id,
            &row.portable_id,
            json!({}),
        )
        .await?;
    }
    for row in &cascade.medications {
        let mut visible_people: Vec<&str> = cascade
            .schedules
            .iter()
            .filter(|source| source.medication_id == row.id)
            .filter_map(|source| {
                person_portable_ids
                    .get(&source.person_id)
                    .map(String::as_str)
            })
            .chain(
                cascade
                    .assignments
                    .iter()
                    .filter(|source| source.medication_id == row.id)
                    .filter_map(|source| {
                        person_portable_ids
                            .get(&source.person_id)
                            .map(String::as_str)
                    }),
            )
            .collect();
        visible_people.sort_unstable();
        visible_people.dedup();
        let visibility = if !visible_people.is_empty() {
            json!({"sync_person_portable_ids": visible_people})
        } else if let Some(creator) = row.created_by_membership_id {
            json!({"sync_creator_membership_id": creator.to_string()})
        } else {
            json!({})
        };
        tombstone(
            db,
            context,
            "Medication",
            row.id,
            &row.portable_id,
            visibility,
        )
        .await?;
    }
    Ok(())
}

async fn delete_dependents(
    db: &DatabaseTransaction,
    record: &stock_location::Model,
) -> Result<Option<CascadeSnapshot>, DbErr> {
    let medications = medication::Entity::find()
        .filter(medication::Column::HouseholdId.eq(record.household_id))
        .filter(medication::Column::LocationId.eq(record.id))
        .order_by_asc(medication::Column::Id)
        .lock_exclusive()
        .all(db)
        .await?;
    let medication_ids: Vec<i64> = medications.iter().map(|medication| medication.id).collect();
    let schedules = if medication_ids.is_empty() {
        Vec::new()
    } else {
        schedule::Entity::find()
            .filter(schedule::Column::MedicationId.is_in(medication_ids.clone()))
            .order_by_asc(schedule::Column::Id)
            .lock_exclusive()
            .all(db)
            .await?
    };
    let assignments = if medication_ids.is_empty() {
        Vec::new()
    } else {
        person_medication::Entity::find()
            .filter(person_medication::Column::MedicationId.is_in(medication_ids.clone()))
            .order_by_asc(person_medication::Column::Id)
            .lock_exclusive()
            .all(db)
            .await?
    };
    let schedule_ids: Vec<i64> = schedules.iter().map(|schedule| schedule.id).collect();
    let assignment_ids: Vec<i64> = assignments.iter().map(|assignment| assignment.id).collect();
    let dosages = if medication_ids.is_empty() {
        Vec::new()
    } else {
        dosage::Entity::find()
            .filter(dosage::Column::MedicationId.is_in(medication_ids.clone()))
            .order_by_asc(dosage::Column::Id)
            .lock_exclusive()
            .all(db)
            .await?
    };
    let memberships = location_membership::Entity::find()
        .filter(location_membership::Column::LocationId.eq(record.id))
        .order_by_asc(location_membership::Column::Id)
        .lock_exclusive()
        .all(db)
        .await?;

    let mut takes =
        Condition::any().add(medication_take::Column::TakenFromLocationId.eq(record.id));
    if !medication_ids.is_empty() {
        takes =
            takes.add(medication_take::Column::TakenFromMedicationId.is_in(medication_ids.clone()));
    }
    if !schedule_ids.is_empty() {
        takes = takes.add(medication_take::Column::ScheduleId.is_in(schedule_ids.clone()));
    }
    if !assignment_ids.is_empty() {
        takes =
            takes.add(medication_take::Column::PersonMedicationId.is_in(assignment_ids.clone()));
    }
    if medication_take::Entity::find()
        .filter(takes)
        .one(db)
        .await?
        .is_some()
    {
        return Ok(None);
    }

    let mut sources = Condition::any();
    if !schedule_ids.is_empty() {
        sources = sources.add(dose_occurrence::Column::ScheduleId.is_in(schedule_ids.clone()));
    }
    if !assignment_ids.is_empty() {
        sources =
            sources.add(dose_occurrence::Column::PersonMedicationId.is_in(assignment_ids.clone()));
    }
    if !schedule_ids.is_empty() || !assignment_ids.is_empty() {
        if dose_occurrence::Entity::find()
            .filter(sources)
            .one(db)
            .await?
            .is_some()
        {
            return Ok(None);
        }
        let mut pauses = Condition::any();
        if !schedule_ids.is_empty() {
            pauses = pauses.add(pause_period::Column::ScheduleId.is_in(schedule_ids.clone()));
        }
        if !assignment_ids.is_empty() {
            pauses =
                pauses.add(pause_period::Column::PersonMedicationId.is_in(assignment_ids.clone()));
        }
        if pause_period::Entity::find()
            .filter(pauses)
            .one(db)
            .await?
            .is_some()
        {
            return Ok(None);
        }
    }

    location_membership::Entity::delete_many()
        .filter(location_membership::Column::LocationId.eq(record.id))
        .exec(db)
        .await?;
    if !medication_ids.is_empty() {
        schedule::Entity::delete_many()
            .filter(schedule::Column::MedicationId.is_in(medication_ids.clone()))
            .exec(db)
            .await?;
        person_medication::Entity::delete_many()
            .filter(person_medication::Column::MedicationId.is_in(medication_ids.clone()))
            .exec(db)
            .await?;
        dosage::Entity::delete_many()
            .filter(dosage::Column::MedicationId.is_in(medication_ids.clone()))
            .exec(db)
            .await?;
        medication::Entity::delete_many()
            .filter(medication::Column::Id.is_in(medication_ids))
            .exec(db)
            .await?;
    }
    stock_location::Entity::delete_by_id(record.id)
        .exec(db)
        .await?;
    Ok(Some(CascadeSnapshot {
        medications,
        dosages,
        schedules,
        assignments,
        memberships,
    }))
}

pub(super) async fn delete(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) =
        match manager_context(&state, &headers, household_id, "DELETE", "destroy").await? {
            Ok(value) => value,
            Err(response) => return Ok(response),
        };
    let path = format!("/api/v1/households/{household_id}/locations/{id}");
    let request = json!({});
    if let Some(response) = keyed_replay(
        &db,
        &context,
        &headers,
        "DELETE",
        &path,
        &request,
        "destroy",
        "api/v1/locations",
        "LocationPolicy",
    )
    .await?
    {
        db.commit().await.map_err(database_error)?;
        return Ok(response);
    }
    if !valid_identifier(&id) {
        return keyed_failure(
            db,
            &context,
            &headers,
            "DELETE",
            &path,
            &request,
            "api/v1/locations",
            "LocationPolicy",
            "destroy",
            StatusCode::BAD_REQUEST,
            "bad_request",
            "Invalid resource ID",
            None,
        )
        .await;
    }
    let Some(record) = location(&db, household_id, &id, true).await? else {
        return failure(
            db,
            &context,
            "DELETE",
            "destroy",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
        )
        .await;
    };
    let Some(if_match) = headers
        .get(header::IF_MATCH)
        .and_then(|value| value.to_str().ok())
    else {
        return keyed_failure(
            db,
            &context,
            &headers,
            "DELETE",
            &path,
            &request,
            "api/v1/locations",
            "LocationPolicy",
            "destroy",
            StatusCode::PRECONDITION_REQUIRED,
            "precondition_required",
            "If-Match is required",
            None,
        )
        .await;
    };
    if if_match.is_empty() {
        return keyed_failure(
            db,
            &context,
            &headers,
            "DELETE",
            &path,
            &request,
            "api/v1/locations",
            "LocationPolicy",
            "destroy",
            StatusCode::PRECONDITION_REQUIRED,
            "precondition_required",
            "If-Match is required",
            None,
        )
        .await;
    }
    let (_, current_etag) = representation(record.clone());
    if if_match != current_etag {
        return failure(
            db,
            &context,
            "DELETE",
            "destroy",
            StatusCode::CONFLICT,
            "conflict",
            "Record has changed since it was last read",
        )
        .await;
    }
    let savepoint = db.begin().await.map_err(database_error)?;
    let cascade = match delete_dependents(&savepoint, &record).await {
        Ok(Some(cascade)) => {
            savepoint.commit().await.map_err(database_error)?;
            cascade
        }
        Ok(None) => {
            savepoint.rollback().await.map_err(database_error)?;
            return keyed_failure(
                db,
                &context,
                &headers,
                "DELETE",
                &path,
                &request,
                "api/v1/locations",
                "LocationPolicy",
                "destroy",
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_failed",
                "Location cannot be deleted while administration history exists",
                None,
            )
            .await;
        }
        Err(error)
            if matches!(
                error.sql_err(),
                Some(sea_orm::SqlErr::ForeignKeyConstraintViolation(_))
            ) =>
        {
            savepoint.rollback().await.map_err(database_error)?;
            return keyed_failure(
                db,
                &context,
                &headers,
                "DELETE",
                &path,
                &request,
                "api/v1/locations",
                "LocationPolicy",
                "destroy",
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_failed",
                "Location cannot be deleted while retained records exist",
                None,
            )
            .await;
        }
        Err(error) => return Err(database_error(error)),
    };
    let request_id = Uuid::new_v4().to_string();
    record_cascade_effects(&db, &context, &request_id, &cascade).await?;
    location_change(
        &db,
        &context,
        &request_id,
        &record,
        "destroy",
        "delete",
        Some(snapshot(&record)),
        None,
    )
    .await?;
    let response_body = json!({});
    store_keyed(
        &db,
        &context,
        &headers,
        "DELETE",
        &path,
        &request,
        StatusCode::NO_CONTENT,
        &response_body,
        &request_id,
        None,
    )
    .await?;
    finish_with_request_id(
        db,
        &context,
        &request_id,
        "DELETE",
        "api/v1/locations",
        "LocationPolicy",
        "destroy",
        StatusCode::NO_CONTENT,
        true,
        response_body,
        None,
    )
    .await
}

fn membership_body(
    membership: location_membership::Model,
    location: &stock_location::Model,
    person: &person::Model,
) -> Value {
    json!({"data": {
        "id": membership.id.to_string(),
        "location_id": location.id.to_string(),
        "location_portable_id": location.portable_id,
        "person_id": person.id.to_string(),
        "person_portable_id": person.portable_id,
        "created_at": membership.created_at.and_utc().to_rfc3339()
    }})
}

pub(super) async fn create_membership(
    State(state): State<AppState>,
    Path((household_id, location_id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    let (db, context) =
        match manager_context(&state, &headers, household_id, "POST", "create_membership").await? {
            Ok(value) => value,
            Err(response) => return Ok(response),
        };
    if !valid_identifier(&location_id) {
        return failure(
            db,
            &context,
            "POST",
            "create_membership",
            StatusCode::BAD_REQUEST,
            "bad_request",
            "Invalid location ID",
        )
        .await;
    }
    let Some(location) = location(&db, household_id, &location_id, true).await? else {
        return failure(
            db,
            &context,
            "POST",
            "create_membership",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
        )
        .await;
    };
    let Json(body) = match payload {
        Ok(body) => body,
        Err(_) => {
            return failure(
                db,
                &context,
                "POST",
                "create_membership",
                StatusCode::BAD_REQUEST,
                "bad_request",
                "Invalid JSON request body",
            )
            .await
        }
    };
    let path =
        format!("/api/v1/households/{household_id}/locations/{location_id}/location_memberships");
    if let Some(response) = keyed_replay(
        &db,
        &context,
        &headers,
        "POST",
        &path,
        &body,
        "create",
        "api/v1/location_memberships",
        "LocationMembershipPolicy",
    )
    .await?
    {
        db.commit().await.map_err(database_error)?;
        return Ok(response);
    }
    let Some(outer) = body.as_object() else {
        return keyed_failure(
            db,
            &context,
            &headers,
            "POST",
            &path,
            &body,
            "api/v1/location_memberships",
            "LocationMembershipPolicy",
            "create",
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            "Validation failed",
            Some(json!({"location_membership": ["is invalid"]})),
        )
        .await;
    };
    let Some(inner) = outer.get("location_membership").and_then(Value::as_object) else {
        return keyed_failure(
            db,
            &context,
            &headers,
            "POST",
            &path,
            &body,
            "api/v1/location_memberships",
            "LocationMembershipPolicy",
            "create",
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            "Validation failed",
            Some(json!({"location_membership": ["is invalid"]})),
        )
        .await;
    };
    if outer.len() != 1 || inner.len() != 1 {
        return keyed_failure(
            db,
            &context,
            &headers,
            "POST",
            &path,
            &body,
            "api/v1/location_memberships",
            "LocationMembershipPolicy",
            "create",
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            "Validation failed",
            Some(json!({"location_membership": ["is invalid"]})),
        )
        .await;
    }
    let Some(person_id) = inner.get("person_id").and_then(Value::as_str) else {
        return keyed_failure(
            db,
            &context,
            &headers,
            "POST",
            &path,
            &body,
            "api/v1/location_memberships",
            "LocationMembershipPolicy",
            "create",
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            "Validation failed",
            Some(json!({"location_membership": ["is invalid"]})),
        )
        .await;
    };
    if !valid_identifier(person_id) {
        return keyed_failure(
            db,
            &context,
            &headers,
            "POST",
            &path,
            &body,
            "api/v1/location_memberships",
            "LocationMembershipPolicy",
            "create",
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            "Validation failed",
            Some(json!({"location_membership": ["is invalid"]})),
        )
        .await;
    }
    let query = person::Entity::find().filter(person::Column::HouseholdId.eq(household_id));
    let query = if let Ok(id) = person_id.parse::<i64>() {
        query.filter(person::Column::Id.eq(id))
    } else {
        query.filter(person::Column::PortableId.eq(person_id))
    };
    let Some(person) = query.one(&db).await.map_err(database_error)? else {
        return failure(
            db,
            &context,
            "POST",
            "create_membership",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
        )
        .await;
    };
    match person_manage_access(&db, &context, person.id).await? {
        None => {
            return failure(
                db,
                &context,
                "POST",
                "create_membership",
                StatusCode::NOT_FOUND,
                "not_found",
                "Record not found",
            )
            .await;
        }
        Some(false) => {
            return failure(
                db,
                &context,
                "POST",
                "create_membership",
                StatusCode::FORBIDDEN,
                "forbidden",
                "You are not authorized to perform this action.",
            )
            .await;
        }
        Some(true) => {}
    }
    let existing = location_membership::Entity::find()
        .filter(location_membership::Column::LocationId.eq(location.id))
        .filter(location_membership::Column::PersonId.eq(person.id))
        .one(&db)
        .await
        .map_err(database_error)?;
    let (membership, created) = if let Some(existing) = existing {
        (existing, false)
    } else {
        let now = Utc::now().naive_utc();
        (
            location_membership::ActiveModel {
                household_id: Set(household_id),
                location_id: Set(location.id),
                person_id: Set(person.id),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(&db)
            .await
            .map_err(database_error)?,
            true,
        )
    };
    let request_id = Uuid::new_v4().to_string();
    if created {
        record_version(&db, &context, &request_id, "LocationMembership", membership.id, "create", None,
            Some(json!({"id":membership.id,"household_id":membership.household_id,"location_id":membership.location_id,"person_id":membership.person_id,"created_at":membership.created_at,"updated_at":membership.updated_at}))).await?;
        let mut active: person::ActiveModel = person.clone().into();
        active.updated_at = Set(Utc::now().naive_utc());
        active.update(&db).await.map_err(database_error)?;
        record_change(
            &db,
            &context,
            &request_id,
            SyncRecord {
                record_type: "Person",
                record_id: person.id,
                portable_id: &person.portable_id,
                action: "update",
                person_portable_id: Some(&person.portable_id),
            },
        )
        .await?;
    }
    let response_body = membership_body(membership, &location, &person);
    store_keyed(
        &db,
        &context,
        &headers,
        "POST",
        &path,
        &body,
        StatusCode::CREATED,
        &response_body,
        &request_id,
        None,
    )
    .await?;
    finish_with_request_id(
        db,
        &context,
        &request_id,
        "POST",
        "api/v1/location_memberships",
        "LocationMembershipPolicy",
        "create",
        StatusCode::CREATED,
        true,
        response_body,
        None,
    )
    .await
}

pub(super) async fn delete_membership(
    State(state): State<AppState>,
    Path((household_id, location_id, id)): Path<(i64, String, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = match manager_context(
        &state,
        &headers,
        household_id,
        "DELETE",
        "destroy_membership",
    )
    .await?
    {
        Ok(value) => value,
        Err(response) => return Ok(response),
    };
    if !valid_identifier(&location_id) || !valid_numeric_id(&id) || id.parse::<i64>().is_err() {
        return failure(
            db,
            &context,
            "DELETE",
            "destroy_membership",
            StatusCode::BAD_REQUEST,
            "bad_request",
            "Invalid resource ID",
        )
        .await;
    }
    let Some(location) = location(&db, household_id, &location_id, true).await? else {
        return failure(
            db,
            &context,
            "DELETE",
            "destroy_membership",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
        )
        .await;
    };
    let path = format!(
        "/api/v1/households/{household_id}/locations/{location_id}/location_memberships/{id}"
    );
    let request = json!({});
    if let Some(response) = keyed_replay(
        &db,
        &context,
        &headers,
        "DELETE",
        &path,
        &request,
        "destroy",
        "api/v1/location_memberships",
        "LocationMembershipPolicy",
    )
    .await?
    {
        db.commit().await.map_err(database_error)?;
        return Ok(response);
    }
    let member_id = id.parse::<i64>().map_err(|_| ApiError::not_found())?;
    let Some(membership) = location_membership::Entity::find_by_id(member_id)
        .filter(location_membership::Column::HouseholdId.eq(household_id))
        .filter(location_membership::Column::LocationId.eq(location.id))
        .one(&db)
        .await
        .map_err(database_error)?
    else {
        return failure(
            db,
            &context,
            "DELETE",
            "destroy_membership",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
        )
        .await;
    };
    match person_manage_access(&db, &context, membership.person_id).await? {
        None => {
            return failure(
                db,
                &context,
                "DELETE",
                "destroy_membership",
                StatusCode::NOT_FOUND,
                "not_found",
                "Record not found",
            )
            .await;
        }
        Some(false) => {
            return failure(
                db,
                &context,
                "DELETE",
                "destroy_membership",
                StatusCode::FORBIDDEN,
                "forbidden",
                "You are not authorized to perform this action.",
            )
            .await;
        }
        Some(true) => {}
    }
    let person = person::Entity::find_by_id(membership.person_id)
        .one(&db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?;
    let before = json!({"id":membership.id,"household_id":membership.household_id,"location_id":membership.location_id,"person_id":membership.person_id,"created_at":membership.created_at,"updated_at":membership.updated_at});
    let active: location_membership::ActiveModel = membership.into();
    active.delete(&db).await.map_err(database_error)?;
    let request_id = Uuid::new_v4().to_string();
    record_version(
        &db,
        &context,
        &request_id,
        "LocationMembership",
        member_id,
        "destroy",
        Some(before),
        None,
    )
    .await?;
    let mut person_active: person::ActiveModel = person.clone().into();
    person_active.updated_at = Set(Utc::now().naive_utc());
    person_active.update(&db).await.map_err(database_error)?;
    record_change(
        &db,
        &context,
        &request_id,
        SyncRecord {
            record_type: "Person",
            record_id: person.id,
            portable_id: &person.portable_id,
            action: "update",
            person_portable_id: Some(&person.portable_id),
        },
    )
    .await?;
    let response_body = json!({});
    store_keyed(
        &db,
        &context,
        &headers,
        "DELETE",
        &path,
        &request,
        StatusCode::NO_CONTENT,
        &response_body,
        &request_id,
        None,
    )
    .await?;
    finish_with_request_id(
        db,
        &context,
        &request_id,
        "DELETE",
        "api/v1/location_memberships",
        "LocationMembershipPolicy",
        "destroy",
        StatusCode::NO_CONTENT,
        true,
        response_body,
        None,
    )
    .await
}
