use super::context::{lock_medication, request_context, visible_medication};
use super::persistence::{medication_snapshot, record_version};
use super::responses::{
    error_response, finish_with_request_id, medication_body, validation_response,
};
use super::validation::{decimal_field, valid_stock_decimal};
use crate::database_error;
use crate::entities::medication;
use crate::mutation_idempotency;
use crate::sync_events::record_change;
use crate::sync_events::SyncRecord;
use crate::ApiError;
use crate::AppState;
use axum::extract::Path;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::Response;
use chrono::NaiveDate;
use chrono::Utc;
use sea_orm::ActiveModelTrait;
use sea_orm::Set;
use serde_json::json;
use serde_json::Value;
use uuid::Uuid;

async fn reorder(
    state: AppState,
    household_id: i64,
    id: String,
    headers: HeaderMap,
    body: Value,
    status: i32,
) -> Result<Response, ApiError> {
    let action = if status == 1 {
        "mark_as_ordered"
    } else {
        "mark_as_received"
    };
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
            action,
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
            None,
        )
        .await;
    };
    let Some(outer) = body.as_object() else {
        return error_response(
            db,
            &context,
            "PATCH",
            "api/v1/medications",
            "MedicationPolicy",
            action,
            StatusCode::BAD_REQUEST,
            "bad_request",
            "Invalid request body",
            None,
        )
        .await;
    };
    if outer.keys().any(|key| key != "order_details") || (status == 2 && !outer.is_empty()) {
        return validation_response(
            db,
            &context,
            "PATCH",
            action,
            "order_details",
            "contains an unsupported field",
        )
        .await;
    }
    let empty = json!({});
    let details = outer.get("order_details").unwrap_or(&empty);
    let Some(fields) = details.as_object() else {
        return validation_response(
            db,
            &context,
            "PATCH",
            action,
            "order_details",
            "must be an object",
        )
        .await;
    };
    if fields.keys().any(|key| {
        !matches!(
            key.as_str(),
            "supplier" | "quantity" | "expected_arrival_on"
        )
    }) || fields
        .get("supplier")
        .is_some_and(|value| !value.is_string())
        || fields
            .get("expected_arrival_on")
            .is_some_and(|value| !value.is_string())
    {
        return validation_response(
            db,
            &context,
            "PATCH",
            action,
            "order_details",
            "contains an unsupported field",
        )
        .await;
    }
    if let Some(value) = fields.get("expected_arrival_on") {
        let valid = value
            .as_str()
            .and_then(|raw| {
                NaiveDate::parse_from_str(raw, "%Y-%m-%d")
                    .ok()
                    .filter(|date| date.format("%Y-%m-%d").to_string() == raw)
            })
            .is_some();
        if !valid {
            return validation_response(
                db,
                &context,
                "PATCH",
                action,
                "expected_arrival_on",
                "must be an ISO date",
            )
            .await;
        }
    }
    let quantity = match decimal_field(details, "quantity") {
        Ok(Some(quantity)) if !valid_stock_decimal(quantity) => {
            return validation_response(
                db,
                &context,
                "PATCH",
                action,
                "quantity",
                "is outside stock precision",
            )
            .await
        }
        Ok(quantity) => quantity,
        Err(message) => {
            return validation_response(db, &context, "PATCH", action, "quantity", message).await
        }
    };
    lock_medication(&db, found.id).await?;
    let medication = visible_medication(&db, &context, &id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let before = medication_snapshot(&medication);
    let mut active: medication::ActiveModel = medication.into();
    let now = Utc::now().naive_utc();
    active.reorder_status = Set(Some(status));
    if status == 1 {
        active.ordered_at = Set(Some(now));
        active.order_supplier = Set(details
            .get("supplier")
            .and_then(Value::as_str)
            .map(str::to_owned));
        active.order_quantity = Set(quantity);
        active.expected_arrival_on = Set(details
            .get("expected_arrival_on")
            .and_then(Value::as_str)
            .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()));
    } else {
        active.reordered_at = Set(Some(now));
    }
    active.updated_at = Set(now);
    let updated = active.update(&db).await.map_err(database_error)?;
    let request_id = Uuid::new_v4().to_string();
    record_version(
        &db,
        &context,
        &request_id,
        "Medication",
        updated.id,
        action,
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
        action,
        StatusCode::OK,
        true,
        body,
        Some(&etag),
    )
    .await
}

pub(crate) async fn mark_as_ordered(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<Response, ApiError> {
    let body = if body.is_empty() {
        json!({})
    } else {
        match serde_json::from_slice(&body) {
            Ok(body) => body,
            Err(_) => json!(null),
        }
    };
    reorder(state, household_id, id, headers, body, 1).await
}

pub(crate) async fn mark_as_received(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<Response, ApiError> {
    let body = if body.is_empty() {
        json!({})
    } else {
        match serde_json::from_slice(&body) {
            Ok(body) => body,
            Err(_) => json!(null),
        }
    };
    reorder(state, household_id, id, headers, body, 2).await
}
