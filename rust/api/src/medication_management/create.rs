use super::context::{may_create, request_context};
use super::persistence::{medication_snapshot, record_version};
use super::responses::{
    error_response, finish_with_request_id, medication_body, validation_response,
};
use super::validation::{assign_attributes, barcode_conflict, valid_location, validate_attributes};
use crate::database_error;
use crate::entities::medication;
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
use sea_orm::Set;
use sea_orm::TransactionTrait;
use serde_json::Value;
use uuid::Uuid;

pub(crate) async fn create(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    body: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let body = match body {
        Ok(Json(body)) => body,
        Err(_) => {
            return error_response(
                db,
                &context,
                "POST",
                "api/v1/medications",
                "MedicationPolicy",
                "create",
                StatusCode::BAD_REQUEST,
                "bad_request",
                "Invalid JSON request body",
                None,
            )
            .await;
        }
    };
    if !may_create(&db, &context).await? {
        return error_response(
            db,
            &context,
            "POST",
            "api/v1/medications",
            "MedicationPolicy",
            "create",
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
            None,
        )
        .await;
    }
    let Some(attributes) = body.get("medication").filter(|value| value.is_object()) else {
        return error_response(
            db,
            &context,
            "POST",
            "api/v1/medications",
            "MedicationPolicy",
            "create",
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
            "POST",
            "create",
            "medication",
            "contains an unknown root field",
        )
        .await;
    }
    if !valid_location(&db, household_id, attributes).await? {
        return error_response(
            db,
            &context,
            "POST",
            "api/v1/medications",
            "MedicationPolicy",
            "create",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
            None,
        )
        .await;
    }
    if let Err((field, message)) = validate_attributes(attributes, None) {
        return validation_response(db, &context, "POST", "create", field, message).await;
    }
    if barcode_conflict(&db, attributes, None).await? {
        return validation_response(
            db,
            &context,
            "POST",
            "create",
            "barcode",
            "has already been taken",
        )
        .await;
    }
    let now = Utc::now().naive_utc();
    let mut active = medication::ActiveModel {
        household_id: Set(household_id),
        created_by_membership_id: Set(Some(context.membership.id)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    if let Err((field, message)) = assign_attributes(&mut active, attributes) {
        return validation_response(db, &context, "POST", "create", field, message).await;
    }
    lock_household(&db, household_id).await?;
    let savepoint = db.begin().await.map_err(database_error)?;
    let medication = match active.insert(&savepoint).await {
        Ok(medication) => {
            savepoint.commit().await.map_err(database_error)?;
            medication
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
                "POST",
                "create",
                "barcode",
                "has already been taken",
            )
            .await;
        }
        Err(error) => return Err(database_error(error)),
    };
    let request_id = Uuid::new_v4().to_string();
    record_version(
        &db,
        &context,
        &request_id,
        "Medication",
        medication.id,
        "api_create",
        None,
        Some(medication_snapshot(&medication)),
    )
    .await?;
    record_change(
        &db,
        &context,
        &request_id,
        SyncRecord {
            record_type: "Medication",
            record_id: medication.id,
            portable_id: &medication.portable_id,
            action: "create",
            person_portable_id: None,
        },
    )
    .await?;
    let (body, etag) = medication_body(&db, medication).await?;
    finish_with_request_id(
        db,
        &context,
        &request_id,
        "POST",
        "api/v1/medications",
        "MedicationPolicy",
        "create",
        StatusCode::CREATED,
        true,
        body,
        Some(&etag),
    )
    .await
}
