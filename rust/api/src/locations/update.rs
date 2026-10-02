use super::access::manager_context;
use super::idempotency::{keyed_failure, keyed_replay, store_keyed};
use super::persistence::{location, location_change, representation, snapshot};
use super::responses::failure;
use super::validation::{attribute_errors, attributes, valid_identifier};
use crate::database_error;
use crate::medication_management::finish_with_request_id;
use crate::read_entities::stock_location;
use crate::ApiError;
use crate::AppState;
use axum::extract::rejection::JsonRejection;
use axum::extract::Path;
use axum::extract::State;
use axum::http::header;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::Response;
use axum::Json;
use chrono::Utc;
use sea_orm::ActiveModelTrait;
use sea_orm::Set;
use sea_orm::TransactionTrait;
use serde_json::json;
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
            Some(attribute_errors(&request, false)),
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
