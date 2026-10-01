use super::context::{household_manager, lock_medication, request_context, visible_medication};
use super::persistence::{medication_snapshot, record_version};
use super::responses::{
    error_response, finish_with_request_id, medication_body, validation_response,
};
use super::validation::{decimal_field, valid_stock_decimal};
use crate::database_error;
use crate::entities::{dosage, medication};
use crate::mutation_idempotency;
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
use axum::Extension;
use axum::Json;
use chrono::Utc;
use sea_orm::ActiveModelTrait;
use sea_orm::Set;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct ScalarAdjustment {
    pub(crate) original_etag: String,
}

pub(crate) async fn adjust_inventory(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    scalar: Option<Extension<ScalarAdjustment>>,
    body: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    let (db, _) = request_context(&state, &headers, household_id).await?;
    let (_, context) = mutation_idempotency::lock_household_and_reauthenticate(
        &state,
        &db,
        &headers,
        household_id,
    )
    .await?;
    let Some(found) = visible_medication(&db, &context, &id).await? else {
        return error_response(
            db,
            &context,
            "PATCH",
            "api/v1/medications",
            "MedicationPolicy",
            "adjust_inventory",
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
            "PATCH",
            "api/v1/medications",
            "MedicationPolicy",
            "adjust_inventory",
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
            None,
        )
        .await;
    }
    let Json(body) = match body {
        Ok(value) => value,
        Err(_) => {
            return error_response(
                db,
                &context,
                "PATCH",
                "api/v1/medications",
                "MedicationPolicy",
                "adjust_inventory",
                StatusCode::BAD_REQUEST,
                "bad_request",
                "Invalid request body",
                None,
            )
            .await
        }
    };
    let Some(outer) = body.as_object() else {
        return error_response(
            db,
            &context,
            "PATCH",
            "api/v1/medications",
            "MedicationPolicy",
            "adjust_inventory",
            StatusCode::BAD_REQUEST,
            "bad_request",
            "Invalid request body",
            None,
        )
        .await;
    };
    let Some(attributes) = outer.get("adjustment").and_then(Value::as_object) else {
        return error_response(
            db,
            &context,
            "PATCH",
            "api/v1/medications",
            "MedicationPolicy",
            "adjust_inventory",
            StatusCode::BAD_REQUEST,
            "bad_request",
            "Invalid request body",
            None,
        )
        .await;
    };
    if outer.len() != 1
        || attributes
            .keys()
            .any(|key| !matches!(key.as_str(), "new_quantity" | "reason"))
        || attributes
            .get("reason")
            .is_some_and(|value| !value.is_string())
    {
        return validation_response(
            db,
            &context,
            "PATCH",
            "adjust_inventory",
            "adjustment",
            "contains an unsupported field",
        )
        .await;
    }
    let attributes = &body["adjustment"];
    if let Some(Extension(scalar)) = scalar {
        lock_medication(&db, found.id).await?;
        let current = visible_medication(&db, &context, &id)
            .await?
            .ok_or_else(ApiError::not_found)?;
        let (_, current_etag) = medication_body(&db, current).await?;
        let has_options = dosage::Entity::find()
            .filter(dosage::Column::HouseholdId.eq(household_id))
            .filter(dosage::Column::MedicationId.eq(found.id))
            .one(&db)
            .await
            .map_err(database_error)?
            .is_some();
        if scalar.original_etag != current_etag || has_options {
            return error_response(
                db,
                &context,
                "PATCH",
                "api/v1/medications",
                "MedicationPolicy",
                "adjust_inventory",
                StatusCode::CONFLICT,
                "conflict",
                "Record has changed since it was last read",
                None,
            )
            .await;
        }
    }
    let quantity = match decimal_field(attributes, "new_quantity") {
        Ok(Some(value)) if valid_stock_decimal(value) => value,
        Err("must be a string") => {
            return validation_response(
                db,
                &context,
                "PATCH",
                "adjust_inventory",
                "new_quantity",
                "must be a string",
            )
            .await
        }
        _ => {
            return error_response(
                db,
                &context,
                "PATCH",
                "api/v1/medications",
                "MedicationPolicy",
                "adjust_inventory",
                StatusCode::UNPROCESSABLE_ENTITY,
                "unprocessable_content",
                "Quantity must be a valid nonnegative number",
                None,
            )
            .await
        }
    };
    lock_medication(&db, found.id).await?;
    let medication = visible_medication(&db, &context, &id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let before = medication_snapshot(&medication);
    let mut active: medication::ActiveModel = medication.into();
    active.current_supply = Set(Some(quantity));
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active.update(&db).await.map_err(database_error)?;
    let request_id = Uuid::new_v4().to_string();
    let mut event = format!("adjust inventory (qty: {}", quantity.normalize());
    if let Some(reason) = attributes
        .get("reason")
        .and_then(Value::as_str)
        .filter(|reason| !reason.trim().is_empty())
    {
        event.push_str(&format!(", reason: {reason}"));
    }
    event.push(')');
    record_version(
        &db,
        &context,
        &request_id,
        "Medication",
        updated.id,
        &event,
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
        "PATCH",
        "api/v1/medications",
        "MedicationPolicy",
        "adjust_inventory",
        StatusCode::OK,
        true,
        body,
        Some(&etag),
    )
    .await
}
