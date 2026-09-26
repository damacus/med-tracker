use crate::entities::{dosage, medication, version};
use crate::medication_management::{
    error_response, finish, finish_with_request_id, household_manager, lock_medication,
    record_version, request_context, valid_stock_decimal, visible_medication,
};
use crate::sync_events::{lock_household, record_change, SyncRecord};
use crate::{database_error, ApiError, AppState, AuthContext};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use chrono::Utc;
use sea_orm::prelude::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, DbBackend, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set, Statement,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::str::FromStr;
use uuid::Uuid;

#[derive(Deserialize)]
pub(super) struct Pagination {
    page: Option<i64>,
    per_page: Option<i64>,
}

fn history_query(household_id: i64, medication_id: i64) -> sea_orm::Select<version::Entity> {
    version::Entity::find()
        .filter(version::Column::HouseholdId.eq(household_id))
        .filter(version::Column::ItemType.eq("MedicationStockRemoval"))
        .filter(version::Column::ItemId.eq(medication_id))
        .filter(version::Column::Event.eq("stock_removal"))
}

fn removal_row(event: &version::Model) -> Value {
    let values: Value = event
        .object
        .as_deref()
        .and_then(|value| serde_json::from_str(value).ok())
        .unwrap_or(Value::Null);
    json!({
        "id": event.id.to_string(),
        "medication_id": event.item_id.to_string(),
        "dosage_id": values.get("dosage_id").and_then(Value::as_str).filter(|value| !value.is_empty()),
        "quantity": values.get("quantity"),
        "reason": values.get("reason"),
        "note": values.get("note"),
        "submission_id": values.get("submission_id"),
        "previous_quantity": values.get("previous_quantity"),
        "remaining_quantity": values.get("remaining_quantity"),
        "unit": values.get("unit"),
        "created_at": event.created_at.map(|value| value.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
        "actor_membership_id": event.actor_membership_id.map(|id| id.to_string())
    })
}

async fn forbidden_or_missing(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    action: &str,
    status: StatusCode,
) -> Result<Response, ApiError> {
    let (code, message) = if status == StatusCode::NOT_FOUND {
        ("not_found", "Record not found")
    } else {
        (
            "forbidden",
            "You are not authorized to perform this action.",
        )
    };
    error_response(
        db,
        context,
        method,
        "api/v1/stock_removals",
        "MedicationPolicy",
        action,
        status,
        code,
        message,
        None,
    )
    .await
}

async fn invalid(db: DatabaseTransaction, context: &AuthContext) -> Result<Response, ApiError> {
    error_response(
        db,
        context,
        "POST",
        "api/v1/stock_removals",
        "MedicationPolicy",
        "create",
        StatusCode::UNPROCESSABLE_ENTITY,
        "unprocessable_content",
        "Stock removal could not be recorded",
        None,
    )
    .await
}

pub(super) async fn index(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    Query(pagination): Query<Pagination>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let Some(medication) = visible_medication(&db, &context, &id).await? else {
        return forbidden_or_missing(db, &context, "GET", "index", StatusCode::NOT_FOUND).await;
    };
    if !household_manager(&context) {
        return forbidden_or_missing(db, &context, "GET", "index", StatusCode::FORBIDDEN).await;
    }
    let page = pagination.page.unwrap_or(1).max(1);
    let per_page = pagination.per_page.unwrap_or(20).clamp(1, 100);
    let query = history_query(household_id, medication.id);
    let total_count = query.clone().count(&db).await.map_err(database_error)?;
    let rows: Vec<Value> = query
        .order_by_desc(version::Column::Id)
        .limit(per_page as u64)
        .offset(page.saturating_sub(1).saturating_mul(per_page) as u64)
        .all(&db)
        .await
        .map_err(database_error)?
        .iter()
        .map(removal_row)
        .collect();
    finish(
        db,
        &context,
        "GET",
        "api/v1/stock_removals",
        "MedicationPolicy",
        "index",
        StatusCode::OK,
        true,
        json!({"data": rows, "meta": {"page": page, "per_page": per_page, "total_count": total_count}}),
        None,
    )
    .await
}

fn parse_quantity(attributes: &Value) -> Option<Decimal> {
    let raw = attributes.get("quantity")?.as_str()?;
    let mut segments = raw.split('.');
    let whole = segments.next()?;
    if whole.is_empty() || !whole.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    if let Some(fraction) = segments.next() {
        if fraction.is_empty()
            || fraction.len() > 2
            || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        {
            return None;
        }
    }
    if segments.next().is_some() {
        return None;
    }
    Decimal::from_str(raw)
        .ok()
        .filter(|value| *value > Decimal::ZERO && valid_stock_decimal(*value))
}

fn format_quantity(value: Decimal) -> String {
    let raw = value.normalize().to_string();
    raw.trim_end_matches(".0").to_owned()
}

fn payload(attributes: &Value, quantity: Decimal) -> Option<Value> {
    let reason = attributes.get("reason")?.as_str()?;
    if ![
        "dropped",
        "damaged",
        "expired",
        "discarded",
        "lost",
        "transferred_out",
        "other",
    ]
    .contains(&reason)
    {
        return None;
    }
    let note = attributes
        .get("note")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if note.len() > 1000 {
        return None;
    }
    let submission_id = attributes.get("submission_id")?.as_str()?;
    if submission_id.len() != 36
        || !submission_id.bytes().enumerate().all(|(index, byte)| {
            if [8, 13, 18, 23].contains(&index) {
                byte == b'-'
            } else {
                byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
            }
        })
        || Uuid::parse_str(submission_id).is_err()
    {
        return None;
    }
    let dosage_id = match attributes.get("dosage_id") {
        None | Some(Value::Null) => "".to_owned(),
        Some(Value::String(id)) if id.is_empty() || id.parse::<i64>().is_ok_and(|id| id > 0) => {
            id.clone()
        }
        _ => return None,
    };
    Some(json!({
        "quantity": format_quantity(quantity),
        "reason": reason,
        "note": note,
        "submission_id": submission_id,
        "dosage_id": dosage_id
    }))
}

async fn lock_dosage(db: &DatabaseTransaction, dosage_id: i64) -> Result<(), ApiError> {
    db.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT id FROM dosages WHERE id = $1 FOR UPDATE",
        [dosage_id.into()],
    ))
    .await
    .map_err(database_error)?
    .ok_or_else(ApiError::not_found)?;
    Ok(())
}

async fn replay_event(
    db: &DatabaseTransaction,
    household_id: i64,
    medication_id: i64,
    submission_id: &str,
) -> Result<Option<version::Model>, ApiError> {
    let events = history_query(household_id, medication_id)
        .all(db)
        .await
        .map_err(database_error)?;
    Ok(events.into_iter().find(|event| {
        event
            .object
            .as_deref()
            .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
            .is_some_and(|object| object["submission_id"] == submission_id)
    }))
}

pub(super) async fn create(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let Some(found) = visible_medication(&db, &context, &id).await? else {
        return forbidden_or_missing(db, &context, "POST", "create", StatusCode::NOT_FOUND).await;
    };
    if !household_manager(&context) {
        return forbidden_or_missing(db, &context, "POST", "create", StatusCode::FORBIDDEN).await;
    }
    let attributes = body.get("stock_removal").unwrap_or(&Value::Null);
    let Some(quantity) = parse_quantity(attributes) else {
        return invalid(db, &context).await;
    };
    let Some(payload) = payload(attributes, quantity) else {
        return invalid(db, &context).await;
    };
    let dosage_id = payload["dosage_id"]
        .as_str()
        .and_then(|id| id.parse::<i64>().ok());
    if let Some(dosage_id) = dosage_id {
        let owned = dosage::Entity::find_by_id(dosage_id)
            .one(&db)
            .await
            .map_err(database_error)?
            .is_some_and(|dosage| {
                dosage.medication_id == found.id && dosage.household_id == household_id
            });
        if !owned {
            return invalid(db, &context).await;
        }
    }
    lock_household(&db, household_id).await?;
    lock_medication(&db, found.id).await?;
    if let Some(dosage_id) = dosage_id {
        lock_dosage(&db, dosage_id).await?;
    }
    let medication = visible_medication(&db, &context, &id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let submission_id = payload["submission_id"].as_str().unwrap_or("");
    if let Some(event) = replay_event(&db, household_id, medication.id, submission_id).await? {
        let prior: Value = event
            .object
            .as_deref()
            .and_then(|raw| serde_json::from_str(raw).ok())
            .unwrap_or(Value::Null);
        let same = ["quantity", "reason", "note", "submission_id", "dosage_id"]
            .iter()
            .all(|field| prior[*field] == payload[*field]);
        if !same {
            return invalid(db, &context).await;
        }
        return finish(
            db,
            &context,
            "POST",
            "api/v1/stock_removals",
            "MedicationPolicy",
            "create",
            StatusCode::CREATED,
            true,
            json!({"data": removal_row(&event)}),
            None,
        )
        .await;
    }
    let request_id = Uuid::new_v4().to_string();
    let (previous, remaining, unit) = if let Some(dosage_id) = dosage_id {
        let dosage = dosage::Entity::find_by_id(dosage_id)
            .one(&db)
            .await
            .map_err(database_error)?
            .ok_or_else(ApiError::not_found)?;
        let Some(previous) = dosage.current_supply else {
            return invalid(db, &context).await;
        };
        if previous < quantity {
            return invalid(db, &context).await;
        }
        let remaining = previous - quantity;
        let unit = dosage.unit.clone();
        let dosage_portable_id = dosage.portable_id.clone();
        let mut active: dosage::ActiveModel = dosage.into();
        active.current_supply = Set(Some(remaining));
        active.updated_at = Set(Utc::now().naive_utc());
        active.update(&db).await.map_err(database_error)?;
        record_version(
            &db,
            &context,
            &request_id,
            "MedicationDosageOption",
            dosage_id,
            "update",
            Some(json!({"current_supply": format_quantity(previous)})),
            Some(json!({"current_supply": format_quantity(remaining)})),
        )
        .await?;
        record_change(
            &db,
            &context,
            &request_id,
            SyncRecord {
                record_type: "MedicationDosageOption",
                record_id: dosage_id,
                portable_id: &dosage_portable_id,
                action: "update",
                person_portable_id: None,
            },
        )
        .await?;
        let tracked = dosage::Entity::find()
            .filter(dosage::Column::MedicationId.eq(medication.id))
            .filter(dosage::Column::CurrentSupply.is_not_null())
            .all(&db)
            .await
            .map_err(database_error)?;
        let total: Decimal = tracked.iter().filter_map(|row| row.current_supply).sum();
        let threshold: Decimal = tracked.iter().filter_map(|row| row.reorder_threshold).sum();
        let last_restock = medication
            .supply_at_last_restock
            .map_or(total, |previous| previous.max(total));
        let mut active: medication::ActiveModel = medication.clone().into();
        active.current_supply = Set(Some(total));
        active.reorder_threshold = Set(threshold);
        active.supply_at_last_restock = Set(Some(last_restock));
        active.updated_at = Set(Utc::now().naive_utc());
        active.update(&db).await.map_err(database_error)?;
        record_version(
            &db,
            &context,
            &request_id,
            "Medication",
            medication.id,
            "update",
            Some(json!({
                "current_supply": medication.current_supply.map(format_quantity),
                "reorder_threshold": format_quantity(medication.reorder_threshold),
                "supply_at_last_restock": medication.supply_at_last_restock.map(format_quantity)
            })),
            Some(json!({
                "current_supply": format_quantity(total),
                "reorder_threshold": format_quantity(threshold),
                "supply_at_last_restock": format_quantity(last_restock)
            })),
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
                action: "update",
                person_portable_id: None,
            },
        )
        .await?;
        (previous, remaining, unit)
    } else {
        let has_tracked = dosage::Entity::find()
            .filter(dosage::Column::MedicationId.eq(medication.id))
            .filter(dosage::Column::CurrentSupply.is_not_null())
            .one(&db)
            .await
            .map_err(database_error)?
            .is_some();
        if has_tracked {
            return invalid(db, &context).await;
        }
        let Some(previous) = medication.current_supply else {
            return invalid(db, &context).await;
        };
        if previous < quantity {
            return invalid(db, &context).await;
        }
        let remaining = previous - quantity;
        let unit = medication.dose_unit.clone().unwrap_or_default();
        let mut active: medication::ActiveModel = medication.clone().into();
        active.current_supply = Set(Some(remaining));
        active.updated_at = Set(Utc::now().naive_utc());
        active.update(&db).await.map_err(database_error)?;
        record_version(
            &db,
            &context,
            &request_id,
            "Medication",
            medication.id,
            "update",
            Some(json!({"current_supply": format_quantity(previous)})),
            Some(json!({"current_supply": format_quantity(remaining)})),
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
                action: "update",
                person_portable_id: None,
            },
        )
        .await?;
        (previous, remaining, unit)
    };
    let unit = if [
        "tablet", "capsule", "gummy", "sachet", "spray", "drop", "pad", "ml",
    ]
    .contains(&unit.as_str())
    {
        unit
    } else {
        "units".to_owned()
    };
    let mut object = payload;
    object["previous_quantity"] = json!(format_quantity(previous));
    object["remaining_quantity"] = json!(format_quantity(remaining));
    object["unit"] = json!(unit);
    let event = version::ActiveModel {
        item_type: Set("MedicationStockRemoval".to_owned()),
        item_id: Set(medication.id),
        event: Set("stock_removal".to_owned()),
        object: Set(Some(object.to_string())),
        request_id: Set(Some(request_id.clone())),
        household_id: Set(Some(household_id)),
        actor_membership_id: Set(Some(context.membership.id)),
        whodunnit: Set(Some(context.user_id.to_string())),
        audit_context: Set(json!({"actor_account_id": context.account_id, "actor_user_id": context.user_id, "actor_membership_id": context.membership.id, "household_id": household_id})),
        created_at: Set(Some(Utc::now().naive_utc())),
        ..Default::default()
    }
    .insert(&db)
    .await
    .map_err(database_error)?;
    finish_with_request_id(
        db,
        &context,
        &request_id,
        "POST",
        "api/v1/stock_removals",
        "MedicationPolicy",
        "create",
        StatusCode::CREATED,
        true,
        json!({"data": removal_row(&event)}),
        None,
    )
    .await
}
