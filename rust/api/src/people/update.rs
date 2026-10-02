use super::access::{carer_exists, manageable};
use super::persistence::{representation, snapshot};
use super::responses::{failure, failure_with_errors};
use super::validation::{name_errors, parse_attributes, person_valid, valid_identifier};
use crate::database_error;
use crate::granted_people;
use crate::medication_management::finish;
use crate::medication_management::finish_with_request_id;
use crate::medication_management::record_version;
use crate::medication_management::request_context;
use crate::read_entities::person;
use crate::read_resources::age;
use crate::read_resources::today;
use crate::sync_events::lock_household;
use crate::sync_events::record_change;
use crate::sync_events::SyncRecord;
use crate::ApiError;
use crate::AppState;
use axum::extract::rejection::JsonRejection;
use axum::extract::Path;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::Response;
use axum::Json;
use chrono::Utc;
use sea_orm::ActiveModelTrait;
use sea_orm::ColumnTrait;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::QuerySelect;
use sea_orm::Set;
use sea_orm::TransactionTrait;
use serde_json::Value;
use uuid::Uuid;

async fn update(
    state: AppState,
    household_id: i64,
    id: String,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
    method: &str,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    if !valid_identifier(&id) {
        return failure(db, &context, method, "update", StatusCode::BAD_REQUEST).await;
    }
    let query = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(household_id))
        .filter(person::Column::Id.in_subquery(granted_people(&context.membership)));
    let query = if let Ok(id) = id.parse::<i64>() {
        query.filter(person::Column::Id.eq(id))
    } else {
        query.filter(person::Column::PortableId.eq(&id))
    };
    let Some(found) = query.one(&db).await.map_err(database_error)? else {
        return failure(db, &context, method, "update", StatusCode::NOT_FOUND).await;
    };
    if !manageable(&db, &context, found.id).await? {
        return failure(db, &context, method, "update", StatusCode::FORBIDDEN).await;
    }
    let Json(body) = match payload {
        Ok(body) => body,
        Err(_) => return failure(db, &context, method, "update", StatusCode::BAD_REQUEST).await,
    };
    let attrs = match parse_attributes(&body, false) {
        Ok(attrs) => attrs,
        Err(status) => {
            return failure_with_errors(
                db,
                &context,
                method,
                "update",
                status,
                name_errors(&body, false),
            )
            .await;
        }
    };
    lock_household(&db, household_id).await?;
    let Some(record) = person::Entity::find_by_id(found.id)
        .lock_exclusive()
        .one(&db)
        .await
        .map_err(database_error)?
    else {
        return failure(db, &context, method, "update", StatusCode::NOT_FOUND).await;
    };
    let before = snapshot(&record);
    let mut active: person::ActiveModel = record.clone().into();
    if let Some(value) = attrs.name {
        active.name = Set(value)
    }
    if let Some(value) = attrs.email {
        active.email = Set(value)
    }
    if let Some(value) = attrs.date_of_birth {
        active.date_of_birth = Set(Some(value))
    }
    if let Some(value) = attrs.person_type {
        active.person_type = Set(value)
    }
    if let Some(value) = attrs.has_capacity {
        active.has_capacity = Set(value)
    }
    let Some(years) = age(active.date_of_birth.clone().take().flatten(), today()) else {
        return failure(
            db,
            &context,
            method,
            "update",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await;
    };
    let person_type = active
        .person_type
        .clone()
        .take()
        .ok_or_else(ApiError::internal)?;
    if (years < 18 && person_type == 1) || (years >= 18 && person_type == 2) {
        active.has_capacity = Set(false)
    }
    let unchanged = active
        .name
        .clone()
        .take()
        .unwrap_or_else(|| record.name.clone())
        == record.name
        && active
            .email
            .clone()
            .take()
            .unwrap_or_else(|| record.email.clone())
            == record.email
        && active
            .date_of_birth
            .clone()
            .take()
            .unwrap_or(record.date_of_birth)
            == record.date_of_birth
        && active
            .person_type
            .clone()
            .take()
            .unwrap_or(record.person_type)
            == record.person_type
        && active
            .has_capacity
            .clone()
            .take()
            .unwrap_or(record.has_capacity)
            == record.has_capacity;
    if unchanged {
        if !person_valid(&record, carer_exists(&db, record.id).await?) {
            return failure(
                db,
                &context,
                method,
                "update",
                StatusCode::UNPROCESSABLE_ENTITY,
            )
            .await;
        }
        let (body, etag) = representation(&db, record).await?;
        return finish(
            db,
            &context,
            method,
            "api/v1/people",
            "PersonPolicy",
            "update",
            StatusCode::OK,
            true,
            body,
            Some(&etag),
        )
        .await;
    }
    active.updated_at = Set(Utc::now().naive_utc());
    let savepoint = db.begin().await.map_err(database_error)?;
    let record = match active.update(&savepoint).await {
        Ok(record) => record,
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
                StatusCode::UNPROCESSABLE_ENTITY,
            )
            .await;
        }
        Err(error) => return Err(database_error(error)),
    };
    if !person_valid(&record, carer_exists(&savepoint, record.id).await?) {
        savepoint.rollback().await.map_err(database_error)?;
        return failure(
            db,
            &context,
            method,
            "update",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await;
    }
    savepoint.commit().await.map_err(database_error)?;
    let request_id = Uuid::new_v4().to_string();
    record_version(
        &db,
        &context,
        &request_id,
        "Person",
        record.id,
        "update",
        Some(before),
        Some(snapshot(&record)),
    )
    .await?;
    record_change(
        &db,
        &context,
        &request_id,
        SyncRecord {
            record_type: "Person",
            record_id: record.id,
            portable_id: &record.portable_id,
            action: "update",
            person_portable_id: Some(&record.portable_id),
        },
    )
    .await?;
    let (body, etag) = representation(&db, record).await?;
    finish_with_request_id(
        db,
        &context,
        &request_id,
        method,
        "api/v1/people",
        "PersonPolicy",
        "update",
        StatusCode::OK,
        true,
        body,
        Some(&etag),
    )
    .await
}

pub(crate) async fn patch(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    update(state, household_id, id, headers, payload, "PATCH").await
}

pub(crate) async fn put(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    update(state, household_id, id, headers, payload, "PUT").await
}
