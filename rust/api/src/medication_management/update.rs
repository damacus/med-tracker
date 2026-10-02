use super::context::{household_manager, lock_medication, request_context, visible_medication};
use super::dose_mode::sync_single_dose_mode;
use super::persistence::{medication_snapshot, record_version};
use super::responses::{
    error_response, finish, finish_with_request_id, medication_body, validation_response,
};
use super::validation::{assign_attributes, barcode_conflict, valid_location, validate_attributes};
use crate::database_error;
use crate::entities::medication;
use crate::entities::schedule;
use crate::sync_events::lock_household;
use crate::sync_events::record_change;
use crate::sync_events::SyncRecord;
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
use sea_orm::ColumnTrait;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::Set;
use sea_orm::TransactionTrait;
use serde_json::Value;
use uuid::Uuid;

async fn update_medication(
    state: AppState,
    household_id: i64,
    id: String,
    headers: HeaderMap,
    body: Result<Json<Value>, JsonRejection>,
    method: &str,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let body = match body {
        Ok(Json(body)) => body,
        Err(_) => {
            return error_response(
                db,
                &context,
                method,
                "api/v1/medications",
                "MedicationPolicy",
                "update",
                StatusCode::BAD_REQUEST,
                "bad_request",
                "Invalid JSON request body",
                None,
            )
            .await;
        }
    };
    let Some(found) = visible_medication(&db, &context, &id).await? else {
        return error_response(
            db,
            &context,
            method,
            "api/v1/medications",
            "MedicationPolicy",
            "update",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
            None,
        )
        .await;
    };
    if !household_manager(&context) {
        return error_response(
            db,
            &context,
            method,
            "api/v1/medications",
            "MedicationPolicy",
            "update",
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
            None,
        )
        .await;
    }
    lock_household(&db, household_id).await?;
    lock_medication(&db, found.id).await?;
    let medication = visible_medication(&db, &context, &id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let (_, current_etag) = medication_body(&db, medication.clone()).await?;
    if headers
        .get(header::IF_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| !value.is_empty() && value != current_etag)
    {
        return error_response(
            db,
            &context,
            method,
            "api/v1/medications",
            "MedicationPolicy",
            "update",
            StatusCode::CONFLICT,
            "conflict",
            "Record has changed since it was last read",
            None,
        )
        .await;
    }
    let Some(attributes) = body.get("medication").filter(|value| value.is_object()) else {
        return error_response(
            db,
            &context,
            method,
            "api/v1/medications",
            "MedicationPolicy",
            "update",
            StatusCode::BAD_REQUEST,
            "bad_request",
            "medication is required",
            None,
        )
        .await;
    };
    if body.as_object().is_none_or(|object| object.len() != 1) {
        return validation_response(
            db,
            &context,
            method,
            "update",
            "medication",
            "contains an unknown root field",
        )
        .await;
    }
    if !valid_location(&db, household_id, attributes).await? {
        return error_response(
            db,
            &context,
            method,
            "api/v1/medications",
            "MedicationPolicy",
            "update",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
            None,
        )
        .await;
    }
    if let Err((field, message)) = validate_attributes(attributes, Some(&medication)) {
        return validation_response(db, &context, method, "update", field, message).await;
    }
    if barcode_conflict(&db, attributes, Some(medication.id)).await? {
        return validation_response(
            db,
            &context,
            method,
            "update",
            "barcode",
            "has already been taken",
        )
        .await;
    }
    let switching_to_single_dose = medication.dose_amount.is_none()
        && attributes
            .get("dose_amount")
            .is_some_and(|amount| !amount.is_null());
    if switching_to_single_dose {
        let has_schedule = schedule::Entity::find()
            .filter(schedule::Column::MedicationId.eq(medication.id))
            .one(&db)
            .await
            .map_err(database_error)?
            .is_some();
        if has_schedule {
            return validation_response(
                db,
                &context,
                method,
                "update",
                "dose_amount",
                "cannot switch dose mode while schedules exist",
            )
            .await;
        }
    }
    if medication.dose_amount.is_none()
        && attributes.as_object().is_some_and(|attributes| {
            attributes.len() == 1 && attributes.get("dose_amount") == Some(&Value::Null)
        })
    {
        let (body, etag) = medication_body(&db, medication).await?;
        return finish(
            db,
            &context,
            method,
            "api/v1/medications",
            "MedicationPolicy",
            "update",
            StatusCode::OK,
            true,
            body,
            Some(&etag),
        )
        .await;
    }
    let before = medication_snapshot(&medication);
    let mut active: medication::ActiveModel = medication.into();
    if let Err((field, message)) = assign_attributes(&mut active, attributes) {
        return validation_response(db, &context, method, "update", field, message).await;
    }
    active.updated_at = Set(Utc::now().naive_utc());
    let savepoint = db.begin().await.map_err(database_error)?;
    let updated = match active.update(&savepoint).await {
        Ok(updated) => {
            savepoint.commit().await.map_err(database_error)?;
            updated
        }
        Err(error)
            if matches!(
                error.sql_err(),
                Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
            ) =>
        {
            savepoint.rollback().await.map_err(database_error)?;
            return validation_response(
                db,
                &context,
                method,
                "update",
                "barcode",
                "has already been taken",
            )
            .await;
        }
        Err(error) => return Err(database_error(error)),
    };
    let request_id = Uuid::new_v4().to_string();
    if switching_to_single_dose {
        sync_single_dose_mode(&db, &context, updated.id, &request_id).await?;
    }
    record_version(
        &db,
        &context,
        &request_id,
        "Medication",
        updated.id,
        "api_update",
        Some(before),
        Some(medication_snapshot(&updated)),
    )
    .await?;
    record_change(
        &db,
        &context,
        &request_id,
        SyncRecord {
            record_type: "Medication",
            record_id: updated.id,
            portable_id: &updated.portable_id,
            action: "update",
            person_portable_id: None,
        },
    )
    .await?;
    let (body, etag) = medication_body(&db, updated).await?;
    finish_with_request_id(
        db,
        &context,
        &request_id,
        method,
        "api/v1/medications",
        "MedicationPolicy",
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
    body: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    update_medication(state, household_id, id, headers, body, "PATCH").await
}

pub(crate) async fn put(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    body: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    update_medication(state, household_id, id, headers, body, "PUT").await
}
