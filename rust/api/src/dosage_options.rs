use crate::entities::{dosage, medication};
use crate::medication_management::{
    error_response, finish, finish_with_request_id, household_manager, lock_medication,
    medication_snapshot, record_version, request_context,
};
use crate::sync_events::{lock_household, record_change, SyncRecord};
use crate::{database_error, decimal_string, representation_etag, ApiError, AppState, AuthContext};
use axum::extract::{rejection::JsonRejection, rejection::QueryRejection, Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use chrono::{DateTime, Utc};
use sea_orm::prelude::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, QueryTrait, Set, TransactionTrait,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Deserialize)]
pub(super) struct Pagination {
    page: Option<i64>,
    per_page: Option<i64>,
    updated_since: Option<String>,
}

struct Attributes {
    medication_id: Option<String>,
    amount: Option<Decimal>,
    unit: Option<String>,
    frequency: Option<String>,
    description: Option<String>,
    default_for_adults: Option<bool>,
    default_for_children: Option<bool>,
    default_max_daily_doses: Option<i32>,
    default_min_hours_between_doses: Option<Decimal>,
    default_dose_cycle: Option<i32>,
    current_supply: Option<Option<Decimal>>,
    reorder_threshold: Option<Option<Decimal>>,
}

fn valid_identifier(value: &str) -> bool {
    let numeric = value
        .as_bytes()
        .first()
        .is_some_and(|byte| (b'1'..=b'9').contains(byte))
        && value.bytes().all(|byte| byte.is_ascii_digit());
    numeric
        || (value.len() == 36
            && value.bytes().enumerate().all(|(index, byte)| match index {
                8 | 13 | 18 | 23 => byte == b'-',
                19 => matches!(byte, b'8' | b'9' | b'a' | b'b' | b'A' | b'B'),
                _ => byte.is_ascii_hexdigit(),
            }))
}

fn parse_decimal(value: &Value) -> Option<Decimal> {
    let text = value.as_str()?;
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    let mut parts = unsigned.split('.');
    let whole = parts.next()?;
    if whole.is_empty() || !whole.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    if let Some(fraction) = parts.next() {
        if fraction.is_empty() || !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
    }
    if parts.next().is_some() {
        return None;
    }
    Decimal::from_str_exact(text).ok()
}

fn storage_decimal(value: Decimal, integer_digits: i64, scale: u32) -> Option<Decimal> {
    let bound = Decimal::from(10_i64.pow(integer_digits as u32));
    (value > -bound && value < bound && value.normalize().scale() <= scale).then_some(value)
}

fn required_string(value: &Value) -> Option<String> {
    value
        .as_str()
        .filter(|text| !text.trim().is_empty())
        .map(str::to_owned)
}

fn attributes(body: &Value, create: bool) -> Option<Attributes> {
    let outer = body.as_object()?;
    if outer.len() != 1 {
        return None;
    }
    let inner = outer.get("dosage_option")?.as_object()?;
    let allowed = [
        "amount",
        "unit",
        "frequency",
        "description",
        "default_for_adults",
        "default_for_children",
        "default_max_daily_doses",
        "default_min_hours_between_doses",
        "default_dose_cycle",
        "current_supply",
        "reorder_threshold",
    ];
    if inner.is_empty()
        || inner
            .keys()
            .any(|key| !allowed.contains(&key.as_str()) && !(create && key == "medication_id"))
    {
        return None;
    }
    let medication_id = inner
        .get("medication_id")
        .map(required_string)
        .transpose_option()?;
    if medication_id
        .as_deref()
        .is_some_and(|value| !valid_identifier(value))
    {
        return None;
    }
    let amount = inner.get("amount").map(parse_decimal).transpose_option()?;
    let unit = inner.get("unit").map(required_string).transpose_option()?;
    let frequency = inner
        .get("frequency")
        .map(required_string)
        .transpose_option()?;
    let description = inner
        .get("description")
        .map(|value| value.as_str().map(str::to_owned))
        .transpose_option()?;
    let default_for_adults = inner
        .get("default_for_adults")
        .map(Value::as_bool)
        .transpose_option()?;
    let default_for_children = inner
        .get("default_for_children")
        .map(Value::as_bool)
        .transpose_option()?;
    let default_max_daily_doses = inner
        .get("default_max_daily_doses")
        .map(|value| {
            i32::try_from(value.as_i64()?)
                .ok()
                .filter(|number| *number >= 1)
        })
        .transpose_option()?;
    let default_min_hours_between_doses = inner
        .get("default_min_hours_between_doses")
        .map(|value| storage_decimal(parse_decimal(value)?, 3, 1))
        .transpose_option()?;
    let default_dose_cycle = inner
        .get("default_dose_cycle")
        .map(|value| match value.as_str()? {
            "daily" => Some(0),
            "weekly" => Some(1),
            "monthly" => Some(2),
            _ => None,
        })
        .transpose_option()?;
    let current_supply = inner
        .get("current_supply")
        .map(|value| {
            if value.is_null() {
                Some(None)
            } else {
                storage_decimal(parse_decimal(value)?, 8, 2).map(Some)
            }
        })
        .transpose_option()?;
    let reorder_threshold = inner
        .get("reorder_threshold")
        .map(|value| {
            if value.is_null() {
                Some(None)
            } else {
                storage_decimal(parse_decimal(value)?, 8, 2).map(Some)
            }
        })
        .transpose_option()?;
    if amount.is_some_and(|value| value <= Decimal::ZERO)
        || default_min_hours_between_doses.is_some_and(|value| value < Decimal::ZERO)
        || current_supply
            .flatten()
            .is_some_and(|value| value < Decimal::ZERO)
        || reorder_threshold
            .flatten()
            .is_some_and(|value| value < Decimal::ZERO)
    {
        return None;
    }
    if create
        && (medication_id.is_none()
            || amount.is_none()
            || unit.is_none()
            || frequency.is_none()
            || default_max_daily_doses.is_none()
            || default_min_hours_between_doses.is_none()
            || default_dose_cycle.is_none())
    {
        return None;
    }
    Some(Attributes {
        medication_id,
        amount,
        unit,
        frequency,
        description,
        default_for_adults,
        default_for_children,
        default_max_daily_doses,
        default_min_hours_between_doses,
        default_dose_cycle,
        current_supply,
        reorder_threshold,
    })
}

trait TransposeOption<T> {
    fn transpose_option(self) -> Option<Option<T>>;
}

impl<T> TransposeOption<T> for Option<Option<T>> {
    fn transpose_option(self) -> Option<Option<T>> {
        match self {
            Some(Some(value)) => Some(Some(value)),
            Some(None) => None,
            None => Some(None),
        }
    }
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
        "api/v1/dosage_options",
        "DosageOptionPolicy",
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
        "api/v1/dosage_options",
        "DosageOptionPolicy",
        action,
        StatusCode::UNPROCESSABLE_ENTITY,
        "validation_failed",
        "Validation failed",
        Some(json!({"dosage_option": ["is invalid"]})),
    )
    .await
}

async fn write_context(
    state: &AppState,
    headers: &HeaderMap,
    household_id: i64,
    method: &str,
    action: &str,
) -> Result<Result<(DatabaseTransaction, AuthContext), Response>, ApiError> {
    let (db, context) = request_context(state, headers, household_id).await?;
    if household_manager(&context) {
        Ok(Ok((db, context)))
    } else {
        Ok(Err(failure(
            db,
            &context,
            method,
            action,
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
        )
        .await?))
    }
}

async fn medication_row(
    db: &DatabaseTransaction,
    household_id: i64,
    id: &str,
) -> Result<Option<medication::Model>, ApiError> {
    let query = medication::Entity::find().filter(medication::Column::HouseholdId.eq(household_id));
    let query = if let Ok(id) = id.parse::<i64>() {
        query.filter(medication::Column::Id.eq(id))
    } else {
        query.filter(medication::Column::PortableId.eq(id))
    };
    query.one(db).await.map_err(database_error)
}

async fn dosage_row(
    db: &DatabaseTransaction,
    household_id: i64,
    id: &str,
    lock: bool,
) -> Result<Option<dosage::Model>, ApiError> {
    let query = dosage::Entity::find().filter(dosage::Column::HouseholdId.eq(household_id));
    let query = if let Ok(id) = id.parse::<i64>() {
        query.filter(dosage::Column::Id.eq(id))
    } else {
        query.filter(dosage::Column::PortableId.eq(id))
    };
    let query = if lock { query.lock_exclusive() } else { query };
    query.one(db).await.map_err(database_error)
}

fn dosage_value(record: dosage::Model, medication_portable_id: &str) -> Result<Value, ApiError> {
    let amount = record.amount;
    let unit = record.unit;
    let frequency = record.frequency;
    let max_doses = record.default_max_daily_doses;
    let min_hours = record.default_min_hours_between_doses;
    let cycle = match record.default_dose_cycle {
        0 => "daily",
        1 => "weekly",
        2 => "monthly",
        _ => return Err(ApiError::internal()),
    };
    Ok(json!({
        "id": record.id, "portable_id": record.portable_id,
        "medication_id": record.medication_id, "medication_portable_id": medication_portable_id,
        "amount": decimal_string(amount.to_string()), "unit": unit, "frequency": frequency,
        "description": record.description, "default_for_adults": record.default_for_adults,
        "default_for_children": record.default_for_children,
        "default_max_daily_doses": max_doses,
        "default_min_hours_between_doses": decimal_string(min_hours.to_string()),
        "default_dose_cycle": cycle,
        "current_supply": record.current_supply.map(|value| decimal_string(value.to_string())),
        "reorder_threshold": record.reorder_threshold.map(|value| decimal_string(value.to_string())),
        "updated_at": record.updated_at.and_utc().to_rfc3339()
    }))
}

fn dosage_snapshot(record: &dosage::Model) -> Value {
    json!({
        "id": record.id,
        "portable_id": record.portable_id,
        "medication_id": record.medication_id,
        "amount": record.amount.to_string(),
        "unit": record.unit,
        "frequency": record.frequency,
        "description": record.description,
        "default_for_adults": record.default_for_adults,
        "default_for_children": record.default_for_children,
        "default_max_daily_doses": record.default_max_daily_doses,
        "default_min_hours_between_doses": record.default_min_hours_between_doses.to_string(),
        "default_dose_cycle": record.default_dose_cycle,
        "current_supply": record.current_supply.map(|value| value.to_string()),
        "reorder_threshold": record.reorder_threshold.map(|value| value.to_string())
    })
}

fn valid_persisted_dosage(record: &dosage::Model) -> bool {
    record.amount > Decimal::ZERO
        && !record.unit.trim().is_empty()
        && !record.frequency.trim().is_empty()
        && record.default_max_daily_doses > 0
        && record.default_min_hours_between_doses >= Decimal::ZERO
        && matches!(record.default_dose_cycle, 0..=2)
        && record
            .current_supply
            .is_none_or(|value| value >= Decimal::ZERO)
        && record
            .reorder_threshold
            .is_none_or(|value| value >= Decimal::ZERO)
}

async fn representation(
    db: &DatabaseTransaction,
    record: dosage::Model,
) -> Result<(Value, String), ApiError> {
    let medication = medication::Entity::find_by_id(record.medication_id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::internal)?;
    let body = json!({"data": dosage_value(record, &medication.portable_id)?});
    let etag = representation_etag(&body);
    Ok((body, etag))
}

fn inventory_totals(
    count: i64,
    total: Option<Decimal>,
    threshold: Option<Decimal>,
    existing_baseline: Option<Decimal>,
) -> Option<(Option<Decimal>, Decimal, Option<Decimal>)> {
    if count == 0 {
        return Some((None, Decimal::ZERO, None));
    }
    let total = storage_decimal(total?, 8, 2)?;
    let threshold = storage_decimal(threshold.unwrap_or(Decimal::ZERO), 8, 2)?;
    let baseline = existing_baseline.map_or(total, |baseline| baseline.max(total));
    Some((Some(total), threshold, Some(baseline)))
}

async fn synchronize_inventory(
    db: &DatabaseTransaction,
    parent: medication::Model,
    now: chrono::NaiveDateTime,
    clear_dose: bool,
) -> Result<Option<medication::Model>, ApiError> {
    let aggregated: Option<(i64, Option<Decimal>, Option<Decimal>)> = dosage::Entity::find()
        .select_only()
        .column_as(dosage::Column::Id.count(), "tracked_count")
        .column_as(dosage::Column::CurrentSupply.sum(), "supply_total")
        .column_as(dosage::Column::ReorderThreshold.sum(), "threshold_total")
        .filter(dosage::Column::MedicationId.eq(parent.id))
        .filter(dosage::Column::CurrentSupply.is_not_null())
        .into_tuple()
        .one(db)
        .await
        .map_err(database_error)?;
    let (count, total, threshold) = aggregated.ok_or_else(ApiError::internal)?;
    let Some((current_supply, reorder_threshold, baseline)) =
        inventory_totals(count, total, threshold, parent.supply_at_last_restock)
    else {
        return Ok(None);
    };
    let mut active: medication::ActiveModel = parent.into();
    active.current_supply = Set(current_supply);
    active.reorder_threshold = Set(reorder_threshold);
    active.supply_at_last_restock = Set(baseline);
    if clear_dose {
        active.dose_amount = Set(None);
    }
    active.updated_at = Set(now);
    active.update(db).await.map(Some).map_err(database_error)
}

async fn record_sync(
    db: &DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    record_type: &str,
    id: i64,
    portable_id: &str,
    action: &str,
) -> Result<(), ApiError> {
    record_change(
        db,
        context,
        request_id,
        SyncRecord {
            record_type,
            record_id: id,
            portable_id,
            action,
            person_portable_id: None,
        },
    )
    .await
}

pub(super) async fn index(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    pagination: Result<Query<Pagination>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let Query(pagination) = match pagination {
        Ok(value) => value,
        Err(_) => return validation(db, &context, "GET", "index").await,
    };
    let page = pagination.page.unwrap_or(1);
    let per_page = pagination.per_page.unwrap_or(20);
    if page < 1 || !(1..=100).contains(&per_page) {
        return validation(db, &context, "GET", "index").await;
    }
    let updated_since = match pagination.updated_since {
        Some(value) => match DateTime::parse_from_rfc3339(&value) {
            Ok(value) => Some(value.naive_utc()),
            Err(_) => return validation(db, &context, "GET", "index").await,
        },
        None => None,
    };
    let visible_medication_ids = crate::scope(household_id, &context.membership)
        .select_only()
        .column(medication::Column::Id)
        .into_query();
    let mut query = dosage::Entity::find()
        .filter(dosage::Column::HouseholdId.eq(household_id))
        .filter(dosage::Column::MedicationId.in_subquery(visible_medication_ids));
    if let Some(updated_since) = updated_since {
        query = query.filter(dosage::Column::UpdatedAt.gte(updated_since));
    }
    let total_count = query.clone().count(&db).await.map_err(database_error)?;
    let records = query
        .order_by_asc(dosage::Column::Id)
        .limit(per_page as u64)
        .offset(page.saturating_sub(1).saturating_mul(per_page) as u64)
        .all(&db)
        .await
        .map_err(database_error)?;
    let medication_ids: Vec<i64> = records.iter().map(|record| record.medication_id).collect();
    let medication_ids: HashMap<i64, String> = medication::Entity::find()
        .filter(medication::Column::Id.is_in(medication_ids))
        .all(&db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|record| (record.id, record.portable_id))
        .collect();
    let rows: Vec<Value> = records
        .into_iter()
        .map(|record| {
            let portable = medication_ids
                .get(&record.medication_id)
                .ok_or_else(ApiError::internal)?;
            dosage_value(record, portable)
        })
        .collect::<Result<_, _>>()?;
    finish(db, &context, "GET", "api/v1/dosage_options", "DosageOptionPolicy", "index", StatusCode::OK, true, json!({"data": rows, "meta": {"page": page, "per_page": per_page, "total_count": total_count}}), None).await
}

pub(super) async fn show(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    if !valid_identifier(&id) {
        return failure(
            db,
            &context,
            "GET",
            "show",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
        )
        .await;
    }
    let Some(record) = dosage_row(&db, household_id, &id, false).await? else {
        return failure(
            db,
            &context,
            "GET",
            "show",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
        )
        .await;
    };
    let visible = crate::scope(household_id, &context.membership)
        .filter(medication::Column::Id.eq(record.medication_id))
        .one(&db)
        .await
        .map_err(database_error)?;
    if visible.is_none() {
        return failure(
            db,
            &context,
            "GET",
            "show",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
        )
        .await;
    }
    let (body, etag) = representation(&db, record).await?;
    finish(
        db,
        &context,
        "GET",
        "api/v1/dosage_options",
        "DosageOptionPolicy",
        "show",
        StatusCode::OK,
        true,
        body,
        Some(&etag),
    )
    .await
}

pub(super) async fn create(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    let (db, context) =
        match write_context(&state, &headers, household_id, "POST", "create").await? {
            Ok(value) => value,
            Err(response) => return Ok(response),
        };
    let Json(body) = match payload {
        Ok(value) => value,
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
            .await
        }
    };
    let Some(attrs) = attributes(&body, true) else {
        return validation(db, &context, "POST", "create").await;
    };
    let Some(medication) = medication_row(
        &db,
        household_id,
        attrs
            .medication_id
            .as_deref()
            .expect("create medication ID"),
    )
    .await?
    else {
        return failure(
            db,
            &context,
            "POST",
            "create",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
        )
        .await;
    };
    lock_household(&db, household_id).await?;
    lock_medication(&db, medication.id).await?;
    let medication = medication::Entity::find_by_id(medication.id)
        .one(&db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?;
    let now = Utc::now().naive_utc();
    let active = dosage::ActiveModel {
        household_id: Set(household_id),
        medication_id: Set(medication.id),
        amount: Set(attrs.amount.expect("validated create amount")),
        unit: Set(attrs.unit.expect("validated create unit")),
        frequency: Set(attrs.frequency.expect("validated create frequency")),
        description: Set(attrs.description),
        default_for_adults: Set(attrs.default_for_adults.unwrap_or(false)),
        default_for_children: Set(attrs.default_for_children.unwrap_or(false)),
        default_max_daily_doses: Set(attrs
            .default_max_daily_doses
            .expect("validated create maximum doses")),
        default_min_hours_between_doses: Set(attrs
            .default_min_hours_between_doses
            .expect("validated create minimum hours")),
        default_dose_cycle: Set(attrs
            .default_dose_cycle
            .expect("validated create dose cycle")),
        current_supply: Set(attrs.current_supply.unwrap_or(None)),
        reorder_threshold: Set(attrs.reorder_threshold.unwrap_or(None)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    let savepoint = db.begin().await.map_err(database_error)?;
    let record = match active.insert(&savepoint).await {
        Ok(record) => record,
        Err(error)
            if matches!(
                error.sql_err(),
                Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
            ) =>
        {
            savepoint.rollback().await.map_err(database_error)?;
            return validation(db, &context, "POST", "create").await;
        }
        Err(error) => return Err(database_error(error)),
    };
    let parent_before = medication_snapshot(&medication);
    let tracked_inventory = record.current_supply.is_some();
    let parent = if tracked_inventory {
        match synchronize_inventory(&savepoint, medication.clone(), now, true).await? {
            Some(parent) => parent,
            None => {
                savepoint.rollback().await.map_err(database_error)?;
                return validation(db, &context, "POST", "create").await;
            }
        }
    } else {
        let mut active: medication::ActiveModel = medication.into();
        active.dose_amount = Set(None);
        active.updated_at = Set(now);
        active.update(&savepoint).await.map_err(database_error)?
    };
    savepoint.commit().await.map_err(database_error)?;
    let request_id = Uuid::new_v4().to_string();
    record_version(
        &db,
        &context,
        &request_id,
        "MedicationDosageOption",
        record.id,
        "api_create",
        None,
        Some(dosage_snapshot(&record)),
    )
    .await?;
    if tracked_inventory {
        record_version(
            &db,
            &context,
            &request_id,
            "Medication",
            parent.id,
            "api_update",
            Some(parent_before),
            Some(medication_snapshot(&parent)),
        )
        .await?;
    }
    record_sync(
        &db,
        &context,
        &request_id,
        "MedicationDosageOption",
        record.id,
        &record.portable_id,
        "create",
    )
    .await?;
    record_sync(
        &db,
        &context,
        &request_id,
        "Medication",
        parent.id,
        &parent.portable_id,
        "update",
    )
    .await?;
    let (body, etag) = representation(&db, record).await?;
    finish_with_request_id(
        db,
        &context,
        &request_id,
        "POST",
        "api/v1/dosage_options",
        "DosageOptionPolicy",
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
        match write_context(&state, &headers, household_id, method, "update").await? {
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
    let Some(found) = dosage_row(&db, household_id, &id, false).await? else {
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
    lock_household(&db, household_id).await?;
    lock_medication(&db, found.medication_id).await?;
    let Some(record) = dosage_row(&db, household_id, &id, true).await? else {
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
    if let Some(if_match) = headers.get(header::IF_MATCH) {
        let (_, current) = representation(&db, record.clone()).await?;
        if if_match.to_str().ok() != Some(current.as_str()) {
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
    }
    let Json(body) = match payload {
        Ok(value) => value,
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
    let Some(attrs) = attributes(&body, false) else {
        return validation(db, &context, method, "update").await;
    };
    let new_supply = attrs.current_supply.unwrap_or(record.current_supply);
    let sync_inventory = new_supply.is_some() || new_supply != record.current_supply;
    let before = dosage_snapshot(&record);
    let mut active: dosage::ActiveModel = record.into();
    if let Some(value) = attrs.amount {
        active.amount = Set(value);
    }
    if let Some(value) = attrs.unit {
        active.unit = Set(value);
    }
    if let Some(value) = attrs.frequency {
        active.frequency = Set(value);
    }
    if let Some(value) = attrs.description {
        active.description = Set(Some(value));
    }
    if let Some(value) = attrs.default_for_adults {
        active.default_for_adults = Set(value);
    }
    if let Some(value) = attrs.default_for_children {
        active.default_for_children = Set(value);
    }
    if let Some(value) = attrs.default_max_daily_doses {
        active.default_max_daily_doses = Set(value);
    }
    if let Some(value) = attrs.default_min_hours_between_doses {
        active.default_min_hours_between_doses = Set(value);
    }
    if let Some(value) = attrs.default_dose_cycle {
        active.default_dose_cycle = Set(value);
    }
    if let Some(value) = attrs.current_supply {
        active.current_supply = Set(value);
    }
    if let Some(value) = attrs.reorder_threshold {
        active.reorder_threshold = Set(value);
    }
    let now = Utc::now().naive_utc();
    active.updated_at = Set(now);
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
            return validation(db, &context, method, "update").await;
        }
        Err(error) => return Err(database_error(error)),
    };
    if !valid_persisted_dosage(&record) {
        savepoint.rollback().await.map_err(database_error)?;
        return validation(db, &context, method, "update").await;
    }
    let parent = if sync_inventory {
        let parent = medication::Entity::find_by_id(record.medication_id)
            .one(&savepoint)
            .await
            .map_err(database_error)?
            .ok_or_else(ApiError::not_found)?;
        let before = medication_snapshot(&parent);
        match synchronize_inventory(&savepoint, parent, now, false).await? {
            Some(parent) => Some((before, parent)),
            None => {
                savepoint.rollback().await.map_err(database_error)?;
                return validation(db, &context, method, "update").await;
            }
        }
    } else {
        None
    };
    savepoint.commit().await.map_err(database_error)?;
    let request_id = Uuid::new_v4().to_string();
    record_version(
        &db,
        &context,
        &request_id,
        "MedicationDosageOption",
        record.id,
        "api_update",
        Some(before),
        Some(dosage_snapshot(&record)),
    )
    .await?;
    record_sync(
        &db,
        &context,
        &request_id,
        "MedicationDosageOption",
        record.id,
        &record.portable_id,
        "update",
    )
    .await?;
    if let Some((before, parent)) = parent {
        record_version(
            &db,
            &context,
            &request_id,
            "Medication",
            parent.id,
            "api_update",
            Some(before),
            Some(medication_snapshot(&parent)),
        )
        .await?;
        record_sync(
            &db,
            &context,
            &request_id,
            "Medication",
            parent.id,
            &parent.portable_id,
            "update",
        )
        .await?;
    }
    let (body, etag) = representation(&db, record).await?;
    finish_with_request_id(
        db,
        &context,
        &request_id,
        method,
        "api/v1/dosage_options",
        "DosageOptionPolicy",
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
