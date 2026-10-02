use super::access::manager_context;
use super::idempotency::{keyed_failure, keyed_replay, store_keyed};
use super::persistence::{location_change, representation, snapshot};
use super::responses::failure;
use super::validation::{attribute_errors, attributes};
use crate::database_error;
use crate::medication_management::finish_with_request_id;
use crate::read_entities::stock_location;
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
use sea_orm::Set;
use sea_orm::TransactionTrait;
use serde_json::json;
use serde_json::Value;
use uuid::Uuid;

pub(crate) async fn create(
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
            Some(attribute_errors(&body, true)),
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
