use super::context::{household_manager, lock_medication, may_create, visible_medication};
use super::dose_mode::sync_single_dose_mode;
use super::persistence::{medication_snapshot, record_version};
use super::responses::medication_body;
use super::validation::{
    assign_attributes, barcode_conflict, decimal_field, valid_location, valid_stock_decimal,
    validate_attributes,
};
use crate::database_error;
use crate::entities::api_tombstone;
use crate::entities::location;
use crate::entities::medication;
use crate::entities::schedule;
use crate::sync_events::record_change;
use crate::sync_events::SyncRecord;
use crate::ApiError;
use crate::AuthContext;
use axum::http::StatusCode;
use chrono::NaiveDate;
use chrono::Utc;
use sea_orm::ActiveModelTrait;
use sea_orm::ColumnTrait;
use sea_orm::DatabaseTransaction;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::Set;
use serde_json::json;
use serde_json::Value;

fn sync_batch_error(status: StatusCode, code: &'static str, message: &'static str) -> ApiError {
    ApiError {
        status,
        code,
        message,
        preserve_activity: false,
    }
}

fn sync_batch_invalid() -> ApiError {
    sync_batch_error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "unprocessable_content",
        "Medication attributes are invalid",
    )
}

async fn sync_batch_attributes(
    db: &DatabaseTransaction,
    household_id: i64,
    operation: &crate::sync_batch::SyncOperation,
) -> Result<Value, ApiError> {
    let mut attributes = Value::Object(operation.attributes.clone());
    let Some(value) = attributes.get("location_id") else {
        return Ok(attributes);
    };
    let id = if let Some(id) = value.as_i64() {
        id
    } else if let Some(id) = value.as_str() {
        if let Ok(id) = id.parse::<i64>() {
            id
        } else {
            location::Entity::find()
                .filter(location::Column::HouseholdId.eq(household_id))
                .filter(location::Column::PortableId.eq(id))
                .one(db)
                .await
                .map_err(database_error)?
                .ok_or_else(ApiError::not_found)?
                .id
        }
    } else {
        return Err(sync_batch_invalid());
    };
    attributes["location_id"] = json!(id);
    Ok(attributes)
}

async fn sync_batch_event(
    db: &DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    medication: &medication::Model,
    event: &str,
    action: &str,
    before: Option<Value>,
) -> Result<(), ApiError> {
    record_version(
        db,
        context,
        request_id,
        "Medication",
        medication.id,
        event,
        before,
        Some(medication_snapshot(medication)),
    )
    .await?;
    record_change(
        db,
        context,
        request_id,
        SyncRecord {
            record_type: "Medication",
            record_id: medication.id,
            portable_id: &medication.portable_id,
            action,
            person_portable_id: None,
        },
    )
    .await
}

async fn sync_batch_result(
    db: &DatabaseTransaction,
    medication: medication::Model,
) -> Result<crate::sync_batch::SyncResult, ApiError> {
    let (_, etag) = medication_body(db, medication.clone()).await?;
    Ok(crate::sync_batch::SyncResult {
        record_type: "Medication",
        record_id: Some(medication.id),
        record_portable_id: Some(medication.portable_id),
        etag: Some(etag),
        replayed: None,
    })
}

pub(crate) async fn apply_sync_operation(
    db: &DatabaseTransaction,
    context: &AuthContext,
    operation: &crate::sync_batch::SyncOperation,
    request_id: &str,
) -> Result<crate::sync_batch::SyncResult, ApiError> {
    let household_id = context.membership.household_id;
    if operation.action == "create" {
        if !may_create(db, context).await? {
            return Err(ApiError::forbidden());
        }
        let attributes = sync_batch_attributes(db, household_id, operation).await?;
        validate_attributes(&attributes, None).map_err(|_| sync_batch_invalid())?;
        if !valid_location(db, household_id, &attributes).await? {
            return Err(ApiError::not_found());
        }
        if barcode_conflict(db, &attributes, None).await? {
            return Err(sync_batch_invalid());
        }
        let now = Utc::now().naive_utc();
        let mut active = medication::ActiveModel {
            household_id: Set(household_id),
            created_by_membership_id: Set(Some(context.membership.id)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        assign_attributes(&mut active, &attributes).map_err(|_| sync_batch_invalid())?;
        let medication = active.insert(db).await.map_err(|error| {
            if matches!(
                error.sql_err(),
                Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
            ) {
                sync_batch_invalid()
            } else {
                database_error(error)
            }
        })?;
        sync_batch_event(
            db,
            context,
            request_id,
            &medication,
            "api_create",
            "create",
            None,
        )
        .await?;
        return sync_batch_result(db, medication).await;
    }
    let id = operation.id.as_deref().ok_or_else(sync_batch_invalid)?;
    let found = visible_medication(db, context, id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    if !household_manager(context)
        && !matches!(
            operation.action.as_str(),
            "mark_as_ordered" | "mark_as_received"
        )
    {
        return Err(ApiError::forbidden());
    }
    lock_medication(db, found.id).await?;
    let medication = visible_medication(db, context, id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let if_match = operation
        .if_match
        .as_deref()
        .filter(|etag| !etag.is_empty())
        .ok_or_else(|| {
            sync_batch_error(
                StatusCode::PRECONDITION_REQUIRED,
                "precondition_required",
                "if_match is required",
            )
        })?;
    let (_, current_etag) = medication_body(db, medication.clone()).await?;
    if if_match != current_etag {
        return Err(sync_batch_error(
            StatusCode::CONFLICT,
            "sync_conflict",
            "Record has changed since it was last read",
        ));
    }
    let before = medication_snapshot(&medication);
    if operation.action == "delete" {
        if !operation.attributes.is_empty() {
            return Err(sync_batch_invalid());
        }
        if !crate::locations::delete_medication_tree(db, context, request_id, &medication).await? {
            return Err(sync_batch_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "unprocessable_content",
                "Medication cannot be deleted while retained records exist",
            ));
        }
        record_version(
            db,
            context,
            request_id,
            "Medication",
            medication.id,
            "api_destroy",
            Some(before),
            None,
        )
        .await?;
        return Ok(crate::sync_batch::SyncResult {
            record_type: "Medication",
            record_id: Some(medication.id),
            record_portable_id: Some(medication.portable_id),
            etag: None,
            replayed: None,
        });
    }
    let mut active: medication::ActiveModel = medication.clone().into();
    let mut switching_to_single_dose = false;
    let event = match operation.action.as_str() {
        "update" => {
            let attributes = sync_batch_attributes(db, household_id, operation).await?;
            validate_attributes(&attributes, Some(&medication))
                .map_err(|_| sync_batch_invalid())?;
            if !valid_location(db, household_id, &attributes).await? {
                return Err(ApiError::not_found());
            }
            if barcode_conflict(db, &attributes, Some(medication.id)).await? {
                return Err(sync_batch_invalid());
            }
            switching_to_single_dose = medication.dose_amount.is_none()
                && attributes
                    .get("dose_amount")
                    .is_some_and(|amount| !amount.is_null());
            if switching_to_single_dose
                && schedule::Entity::find()
                    .filter(schedule::Column::MedicationId.eq(medication.id))
                    .one(db)
                    .await
                    .map_err(database_error)?
                    .is_some()
            {
                return Err(sync_batch_invalid());
            }
            assign_attributes(&mut active, &attributes).map_err(|_| sync_batch_invalid())?;
            "api_update"
        }
        "adjust_inventory" => {
            if operation
                .attributes
                .keys()
                .any(|key| !matches!(key.as_str(), "new_quantity" | "reason"))
            {
                return Err(sync_batch_invalid());
            }
            let attributes = Value::Object(operation.attributes.clone());
            let quantity = decimal_field(&attributes, "new_quantity")
                .ok()
                .flatten()
                .filter(|quantity| valid_stock_decimal(*quantity))
                .ok_or_else(sync_batch_invalid)?;
            active.current_supply = Set(Some(quantity));
            "adjust inventory"
        }
        "mark_as_ordered" | "mark_as_received" => {
            let ordered = operation.action == "mark_as_ordered";
            if !ordered && !operation.attributes.is_empty() {
                return Err(sync_batch_invalid());
            }
            if operation.attributes.keys().any(|key| {
                !matches!(
                    key.as_str(),
                    "supplier" | "quantity" | "expected_arrival_on"
                )
            }) {
                return Err(sync_batch_invalid());
            }
            if operation
                .attributes
                .get("supplier")
                .is_some_and(|value| !value.is_string())
            {
                return Err(sync_batch_invalid());
            }
            let details = Value::Object(operation.attributes.clone());
            let quantity = decimal_field(&details, "quantity").map_err(|_| sync_batch_invalid())?;
            if quantity.is_some_and(|quantity| !valid_stock_decimal(quantity)) {
                return Err(sync_batch_invalid());
            }
            let date = match details.get("expected_arrival_on") {
                None => None,
                Some(Value::String(raw)) => {
                    let parsed = NaiveDate::parse_from_str(raw, "%Y-%m-%d")
                        .map_err(|_| sync_batch_invalid())?;
                    if parsed.format("%Y-%m-%d").to_string() != *raw {
                        return Err(sync_batch_invalid());
                    }
                    Some(parsed)
                }
                _ => return Err(sync_batch_invalid()),
            };
            let now = Utc::now().naive_utc();
            active.reorder_status = Set(Some(if ordered { 1 } else { 2 }));
            if ordered {
                active.ordered_at = Set(Some(now));
                active.order_supplier = Set(details
                    .get("supplier")
                    .and_then(Value::as_str)
                    .map(str::to_owned));
                active.order_quantity = Set(quantity);
                active.expected_arrival_on = Set(date);
            } else {
                active.reordered_at = Set(Some(now));
            }
            if ordered {
                "mark_as_ordered"
            } else {
                "mark_as_received"
            }
        }
        _ => {
            return Err(sync_batch_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "sync_operation_unsupported",
                "Operation is not supported offline",
            ))
        }
    };
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active.update(db).await.map_err(|error| {
        if matches!(
            error.sql_err(),
            Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
        ) {
            sync_batch_invalid()
        } else {
            database_error(error)
        }
    })?;
    if switching_to_single_dose {
        sync_single_dose_mode(db, context, updated.id, request_id).await?;
    }
    sync_batch_event(
        db,
        context,
        request_id,
        &updated,
        event,
        "update",
        Some(before),
    )
    .await?;
    sync_batch_result(db, updated).await
}

pub(crate) async fn authorize_sync_replay(
    db: &DatabaseTransaction,
    context: &AuthContext,
    operation: &crate::sync_batch::SyncOperation,
    saved: &Value,
) -> Result<(), ApiError> {
    let portable_id = saved
        .get("record_portable_id")
        .and_then(Value::as_str)
        .ok_or_else(ApiError::forbidden)?;
    if operation.action == "delete" {
        if !household_manager(context) {
            return Err(ApiError::forbidden());
        }
        let found = api_tombstone::Entity::find()
            .filter(api_tombstone::Column::HouseholdId.eq(context.membership.household_id))
            .filter(api_tombstone::Column::RecordType.eq("Medication"))
            .filter(api_tombstone::Column::RecordPortableId.eq(portable_id))
            .one(db)
            .await
            .map_err(database_error)?;
        return if found.is_some() {
            Ok(())
        } else {
            Err(ApiError::forbidden())
        };
    }
    if operation.action == "create" {
        if !may_create(db, context).await? {
            return Err(ApiError::forbidden());
        }
    } else if !matches!(
        operation.action.as_str(),
        "mark_as_ordered" | "mark_as_received"
    ) && !household_manager(context)
    {
        return Err(ApiError::forbidden());
    }
    if visible_medication(db, context, portable_id)
        .await?
        .is_none()
    {
        return Err(ApiError::forbidden());
    }
    Ok(())
}
