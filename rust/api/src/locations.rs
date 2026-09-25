use crate::entities::person;
use crate::medication_management::{error_response, finish, request_context};
use crate::read_entities::{location_membership, stock_location};
use crate::read_resources::location_value;
use crate::{database_error, representation_etag, ApiError, AppState, AuthContext};
use axum::extract::{rejection::JsonRejection, Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, QuerySelect, Set,
    TransactionTrait,
};
use serde_json::{json, Value};

fn owner(context: &AuthContext) -> bool {
    context.membership.role == "owner"
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

async fn validation(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    action: &str,
) -> Result<Response, ApiError> {
    error_response(
        db,
        context,
        method,
        "api/v1/locations",
        "LocationPolicy",
        action,
        StatusCode::UNPROCESSABLE_ENTITY,
        "validation_failed",
        "Validation failed",
        Some(json!({"location": ["is invalid"]})),
    )
    .await
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

async fn owner_context(
    state: &AppState,
    headers: &HeaderMap,
    household_id: i64,
    method: &str,
    action: &str,
) -> Result<Result<(DatabaseTransaction, AuthContext), Response>, ApiError> {
    let (db, context) = request_context(state, headers, household_id).await?;
    if owner(&context) {
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
        match owner_context(&state, &headers, household_id, "POST", "create").await? {
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
    let Some((Some(name), description)) = attributes(&body, true) else {
        return validation(db, &context, "POST", "create").await;
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
            return failure(
                db,
                &context,
                "POST",
                "create",
                StatusCode::CONFLICT,
                "conflict",
                "Location already exists",
            )
            .await;
        }
        Err(error) => return Err(database_error(error)),
    };
    let (body, etag) = representation(record);
    finish(
        db,
        &context,
        "POST",
        "api/v1/locations",
        "LocationPolicy",
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
    let (db, context) =
        match owner_context(&state, &headers, household_id, method, "update").await? {
            Ok(value) => value,
            Err(response) => return Ok(response),
        };
    if !valid_identifier(&id) {
        return failure(
            db,
            &context,
            method,
            "update",
            StatusCode::BAD_REQUEST,
            "bad_request",
            "Invalid resource ID",
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
        return failure(
            db,
            &context,
            method,
            "update",
            StatusCode::PRECONDITION_REQUIRED,
            "precondition_required",
            "If-Match is required",
        )
        .await;
    };
    if if_match.is_empty() {
        return failure(
            db,
            &context,
            method,
            "update",
            StatusCode::PRECONDITION_REQUIRED,
            "precondition_required",
            "If-Match is required",
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
    let Json(body) = match payload {
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
            .await
        }
    };
    let Some((name, description)) = attributes(&body, false) else {
        return validation(db, &context, method, "update").await;
    };
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
            return failure(
                db,
                &context,
                method,
                "update",
                StatusCode::CONFLICT,
                "conflict",
                "Location already exists",
            )
            .await;
        }
        Err(error) => return Err(database_error(error)),
    };
    let (body, etag) = representation(record);
    finish(
        db,
        &context,
        method,
        "api/v1/locations",
        "LocationPolicy",
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

pub(super) async fn delete(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) =
        match owner_context(&state, &headers, household_id, "DELETE", "destroy").await? {
            Ok(value) => value,
            Err(response) => return Ok(response),
        };
    if !valid_identifier(&id) {
        return failure(
            db,
            &context,
            "DELETE",
            "destroy",
            StatusCode::BAD_REQUEST,
            "bad_request",
            "Invalid resource ID",
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
        return failure(
            db,
            &context,
            "DELETE",
            "destroy",
            StatusCode::PRECONDITION_REQUIRED,
            "precondition_required",
            "If-Match is required",
        )
        .await;
    };
    if if_match.is_empty() {
        return failure(
            db,
            &context,
            "DELETE",
            "destroy",
            StatusCode::PRECONDITION_REQUIRED,
            "precondition_required",
            "If-Match is required",
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
    match stock_location::Entity::delete_by_id(record.id)
        .exec(&savepoint)
        .await
    {
        Ok(_) => savepoint.commit().await.map_err(database_error)?,
        Err(error)
            if matches!(
                error.sql_err(),
                Some(sea_orm::SqlErr::ForeignKeyConstraintViolation(_))
            ) =>
        {
            savepoint.rollback().await.map_err(database_error)?;
            return failure(
                db,
                &context,
                "DELETE",
                "destroy",
                StatusCode::CONFLICT,
                "conflict",
                "Location is still in use",
            )
            .await;
        }
        Err(error) => return Err(database_error(error)),
    }
    finish(
        db,
        &context,
        "DELETE",
        "api/v1/locations",
        "LocationPolicy",
        "destroy",
        StatusCode::NO_CONTENT,
        true,
        json!({}),
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
        match owner_context(&state, &headers, household_id, "POST", "create_membership").await? {
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
    let Some(outer) = body.as_object() else {
        return validation(db, &context, "POST", "create_membership").await;
    };
    let Some(inner) = outer.get("location_membership").and_then(Value::as_object) else {
        return validation(db, &context, "POST", "create_membership").await;
    };
    if outer.len() != 1 || inner.len() != 1 {
        return validation(db, &context, "POST", "create_membership").await;
    }
    let Some(person_id) = inner.get("person_id").and_then(Value::as_str) else {
        return validation(db, &context, "POST", "create_membership").await;
    };
    if !valid_identifier(person_id) {
        return validation(db, &context, "POST", "create_membership").await;
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
    let existing = location_membership::Entity::find()
        .filter(location_membership::Column::LocationId.eq(location.id))
        .filter(location_membership::Column::PersonId.eq(person.id))
        .one(&db)
        .await
        .map_err(database_error)?;
    let membership = if let Some(existing) = existing {
        existing
    } else {
        let now = Utc::now().naive_utc();
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
        .map_err(database_error)?
    };
    let body = membership_body(membership, &location, &person);
    finish(
        db,
        &context,
        "POST",
        "api/v1/location_memberships",
        "LocationMembershipPolicy",
        "create",
        StatusCode::CREATED,
        true,
        body,
        None,
    )
    .await
}

pub(super) async fn delete_membership(
    State(state): State<AppState>,
    Path((household_id, location_id, id)): Path<(i64, String, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = match owner_context(
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
    let active: location_membership::ActiveModel = membership.into();
    active.delete(&db).await.map_err(database_error)?;
    finish(
        db,
        &context,
        "DELETE",
        "api/v1/location_memberships",
        "LocationMembershipPolicy",
        "destroy",
        StatusCode::NO_CONTENT,
        true,
        json!({}),
        None,
    )
    .await
}
