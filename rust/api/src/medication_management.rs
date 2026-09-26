use crate::entities::{
    api_change_event, api_tombstone, dosage, grant, location, medication, person,
    person_medication, schedule, version,
};
use crate::sync_events::{lock_household, record_change, SyncRecord};
use crate::{
    audit, authenticate, database_error, representation_etag, scope, serialize_many, ApiError,
    AppState, AuthContext,
};
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{NaiveDate, Utc};
use sea_orm::prelude::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DatabaseTransaction, DbBackend,
    EntityTrait, QueryFilter, Set, Statement, TransactionTrait,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::str::FromStr;
use uuid::Uuid;

pub(super) async fn request_context(
    state: &AppState,
    headers: &HeaderMap,
    household_id: i64,
) -> Result<(DatabaseTransaction, AuthContext), ApiError> {
    let db = state.db.begin().await.map_err(database_error)?;
    let context = match authenticate(state, &db, headers, household_id).await {
        Ok(context) => context,
        Err(error) => {
            if error.preserve_activity {
                db.commit().await.map_err(database_error)?;
            }
            return Err(error);
        }
    };
    Ok((db, context))
}

pub(super) fn household_manager(context: &AuthContext) -> bool {
    matches!(context.membership.role.as_str(), "owner" | "administrator")
}

pub(super) async fn visible_medication(
    db: &DatabaseTransaction,
    context: &AuthContext,
    id: &str,
) -> Result<Option<medication::Model>, ApiError> {
    let mut query = scope(context.membership.household_id, &context.membership);
    query = match id.parse::<i64>() {
        Ok(id) => query.filter(medication::Column::Id.eq(id)),
        Err(_) => query.filter(medication::Column::PortableId.eq(id)),
    };
    query.one(db).await.map_err(database_error)
}

pub(super) async fn lock_medication(db: &DatabaseTransaction, id: i64) -> Result<(), ApiError> {
    db.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT id FROM medications WHERE id = $1 FOR UPDATE",
        [id.into()],
    ))
    .await
    .map_err(database_error)?
    .ok_or_else(ApiError::not_found)?;
    Ok(())
}

pub(super) async fn medication_body(
    db: &DatabaseTransaction,
    medication: medication::Model,
) -> Result<(Value, String), ApiError> {
    let row = serialize_many(db, vec![medication]).await?.remove(0);
    let body = json!({"data": row});
    let etag = representation_etag(&body);
    Ok((body, etag))
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn finish(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    controller: &str,
    policy: &str,
    action: &str,
    status: StatusCode,
    authorized: bool,
    body: Value,
    etag: Option<&str>,
) -> Result<Response, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    finish_with_request_id(
        db,
        context,
        &request_id,
        method,
        controller,
        policy,
        action,
        status,
        authorized,
        body,
        etag,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn finish_with_request_id(
    db: DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    method: &str,
    controller: &str,
    policy: &str,
    action: &str,
    status: StatusCode,
    authorized: bool,
    mut body: Value,
    etag: Option<&str>,
) -> Result<Response, ApiError> {
    audit::record_resource_request_with_id(
        &db, context, request_id, method, controller, policy, action, status, authorized,
    )
    .await
    .map_err(database_error)?;
    if status.as_u16() >= 400 && body.get("error").is_some() {
        body["error"]["request_id"] = json!(request_id);
    }
    db.commit().await.map_err(database_error)?;
    let mut response = (status, Json(body)).into_response();
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(request_id).map_err(|_| ApiError::internal())?,
    );
    if let Some(etag) = etag {
        response.headers_mut().insert(
            header::ETAG,
            HeaderValue::from_str(etag).map_err(|_| ApiError::internal())?,
        );
    }
    Ok(response)
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn error_response(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    controller: &str,
    policy: &str,
    action: &str,
    status: StatusCode,
    code: &str,
    message: &str,
    errors: Option<Value>,
) -> Result<Response, ApiError> {
    let mut body = json!({"error": {"code": code, "message": message}});
    if let Some(errors) = errors {
        body["error"]["errors"] = errors;
    }
    finish(
        db, context, method, controller, policy, action, status, false, body, None,
    )
    .await
}

async fn validation_response(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    action: &str,
    field: &str,
    message: &str,
) -> Result<Response, ApiError> {
    error_response(
        db,
        context,
        method,
        "api/v1/medications",
        "MedicationPolicy",
        action,
        StatusCode::UNPROCESSABLE_ENTITY,
        "validation_failed",
        "Validation failed",
        Some(json!({field: [message]})),
    )
    .await
}

async fn may_create(db: &DatabaseTransaction, context: &AuthContext) -> Result<bool, ApiError> {
    if household_manager(context) {
        return Ok(true);
    }
    let grant = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(context.membership.household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(context.membership.id))
        .filter(grant::Column::AccessLevel.eq("manage"))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(
            Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(grant::Column::ExpiresAt.gt(Utc::now().naive_utc())),
        )
        .one(db)
        .await
        .map_err(database_error)?;
    Ok(grant.is_some())
}

async fn valid_location(
    db: &DatabaseTransaction,
    household_id: i64,
    attributes: &Value,
) -> Result<bool, ApiError> {
    let Some(value) = attributes.get("location_id") else {
        return Ok(true);
    };
    let id = value
        .as_i64()
        .or_else(|| value.as_str().and_then(|value| value.parse::<i64>().ok()));
    let Some(id) = id else {
        return Ok(false);
    };
    Ok(location::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(database_error)?
        .is_some_and(|location| location.household_id == household_id))
}

fn scalar_string(value: &Value, field: &str) -> Result<Option<String>, &'static str> {
    match value.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(Value::Number(_)) => Err("must be a string"),
        _ => Err("is invalid"),
    }
}

fn decimal_field(value: &Value, field: &str) -> Result<Option<Decimal>, &'static str> {
    let Some(raw) = scalar_string(value, field)? else {
        return Ok(None);
    };
    Decimal::from_str(&raw)
        .map(Some)
        .map_err(|_| "is not a number")
}

pub(super) fn valid_stock_decimal(value: Decimal) -> bool {
    value >= Decimal::ZERO && value.scale() <= 2 && value < Decimal::from(100_000_000)
}

pub(super) fn medication_snapshot(record: &medication::Model) -> Value {
    json!({
        "id": record.id,
        "portable_id": record.portable_id,
        "name": record.name,
        "friendly_name": record.friendly_name,
        "dose_amount": record.dose_amount,
        "dose_unit": record.dose_unit,
        "location_id": record.location_id,
        "current_supply": record.current_supply.map(|value| value.to_string()),
        "reorder_threshold": record.reorder_threshold.to_string(),
        "reorder_status": record.reorder_status
    })
}

async fn sync_single_dose_mode(
    db: &DatabaseTransaction,
    context: &AuthContext,
    medication_id: i64,
    request_id: &str,
) -> Result<(), ApiError> {
    let sources = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(context.membership.household_id))
        .filter(person_medication::Column::MedicationId.eq(medication_id))
        .filter(person_medication::Column::SourceDosageOptionId.is_not_null())
        .all(db)
        .await
        .map_err(database_error)?;
    let options = dosage::Entity::find()
        .filter(dosage::Column::HouseholdId.eq(context.membership.household_id))
        .filter(dosage::Column::MedicationId.eq(medication_id))
        .all(db)
        .await
        .map_err(database_error)?;
    let person_ids: Vec<i64> = sources.iter().map(|source| source.person_id).collect();
    let people = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(context.membership.household_id))
        .filter(person::Column::Id.is_in(person_ids))
        .all(db)
        .await
        .map_err(database_error)?;
    let person_portable_ids: HashMap<i64, String> = people
        .into_iter()
        .map(|record| (record.id, record.portable_id))
        .collect();
    let now = Utc::now().naive_utc();
    for source in &sources {
        let mut active: person_medication::ActiveModel = source.clone().into();
        active.source_dosage_option_id = Set(None);
        active.updated_at = Set(now);
        active.update(db).await.map_err(database_error)?;
    }
    dosage::Entity::delete_many()
        .filter(dosage::Column::HouseholdId.eq(context.membership.household_id))
        .filter(dosage::Column::MedicationId.eq(medication_id))
        .exec(db)
        .await
        .map_err(database_error)?;
    for option in options {
        api_tombstone::ActiveModel {
            household_id: Set(context.membership.household_id),
            household_membership_id: Set(Some(context.membership.id)),
            account_id: Set(Some(context.account_id)),
            action: Set("delete".to_owned()),
            record_type: Set("MedicationDosageOption".to_owned()),
            record_portable_id: Set(option.portable_id.clone()),
            metadata: Set(json!({
                "record_type": "MedicationDosageOption",
                "record_id": option.id,
                "portable_id": option.portable_id
            })),
            deleted_at: Set(now),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(database_error)?;
    }
    for source in sources {
        api_change_event::ActiveModel {
            household_id: Set(context.membership.household_id),
            household_membership_id: Set(Some(context.membership.id)),
            account_id: Set(Some(context.account_id)),
            action: Set("update".to_owned()),
            record_type: Set("PersonMedication".to_owned()),
            record_id: Set(source.id),
            record_portable_id: Set(Some(source.portable_id.clone())),
            request_id: Set(Some(request_id.to_owned())),
            metadata: Set(json!({
                "record_type": "PersonMedication",
                "record_id": source.id,
                "portable_id": source.portable_id,
                "person_portable_id": person_portable_ids.get(&source.person_id)
            })),
            occurred_at: Set(now),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(database_error)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn record_version(
    db: &DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    item_type: &str,
    item_id: i64,
    event: &str,
    before: Option<Value>,
    after: Option<Value>,
) -> Result<(), ApiError> {
    let mut changes = serde_json::Map::new();
    let mut fields = std::collections::HashSet::new();
    if let Some(before) = before.as_ref().and_then(Value::as_object) {
        fields.extend(before.keys().cloned());
    }
    if let Some(after) = after.as_ref().and_then(Value::as_object) {
        fields.extend(after.keys().cloned());
    }
    for field in fields {
        let old = before
            .as_ref()
            .and_then(|row| row.get(&field))
            .cloned()
            .unwrap_or(Value::Null);
        let new = after
            .as_ref()
            .and_then(|row| row.get(&field))
            .cloned()
            .unwrap_or(Value::Null);
        if old != new {
            changes.insert(field, json!([old, new]));
        }
    }
    let audit_context = json!({
        "actor_account_id": context.account_id,
        "actor_user_id": context.user_id,
        "actor_membership_id": context.membership.id,
        "household_id": context.membership.household_id,
        "request_id": request_id
    });
    version::ActiveModel {
        item_type: Set(item_type.to_owned()),
        item_id: Set(item_id),
        event: Set(event.to_owned()),
        object: Set(before.as_ref().map(Value::to_string)),
        object_changes: Set(Some(Value::Object(changes).to_string())),
        whodunnit: Set(Some(context.user_id.to_string())),
        request_id: Set(Some(request_id.to_owned())),
        household_id: Set(Some(context.membership.household_id)),
        actor_membership_id: Set(Some(context.membership.id)),
        audit_context: Set(audit_context),
        created_at: Set(Some(Utc::now().naive_utc())),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(database_error)?;
    Ok(())
}

fn location_id(value: &Value) -> Option<i64> {
    value.get("location_id").and_then(|id| {
        id.as_i64()
            .or_else(|| id.as_str().and_then(|id| id.parse::<i64>().ok()))
    })
}

fn validate_attributes(
    attributes: &Value,
    existing: Option<&medication::Model>,
) -> Result<(), (&'static str, &'static str)> {
    if existing.is_none() || attributes.get("name").is_some() {
        let name = scalar_string(attributes, "name").map_err(|error| ("name", error))?;
        if name.as_deref().is_none_or(|name| name.trim().is_empty()) {
            return Err(("name", "can't be blank"));
        }
    }
    if existing.is_none() && location_id(attributes).is_none() {
        return Err(("location_id", "can't be blank"));
    }
    for field in ["dose_amount", "current_supply", "reorder_threshold"] {
        if attributes.get(field).is_none() {
            continue;
        }
        let value = decimal_field(attributes, field).map_err(|error| (field, error))?;
        if let Some(value) = value {
            if value < Decimal::ZERO || (field == "dose_amount" && value == Decimal::ZERO) {
                return Err((field, "must be greater than 0"));
            }
            if field != "dose_amount" && !valid_stock_decimal(value) {
                return Err((field, "is outside stock precision"));
            }
        } else if field == "reorder_threshold" {
            return Err((field, "can't be blank"));
        }
    }
    if let Some(unit) =
        scalar_string(attributes, "dose_unit").map_err(|error| ("dose_unit", error))?
    {
        if ![
            "tablet", "capsule", "gummy", "mg", "ml", "g", "mcg", "IU", "spray", "drop", "sachet",
            "pad",
        ]
        .contains(&unit.as_str())
        {
            return Err(("dose_unit", "is not included in the list"));
        }
    }
    if let Some(barcode) =
        scalar_string(attributes, "barcode").map_err(|error| ("barcode", error))?
    {
        if !barcode.is_empty()
            && (!matches!(barcode.len(), 13 | 14)
                || !barcode.bytes().all(|byte| byte.is_ascii_digit()))
        {
            return Err(("barcode", "is invalid"));
        }
    }
    if let Some(category) =
        scalar_string(attributes, "category").map_err(|error| ("category", error))?
    {
        if !category.is_empty()
            && ![
                "Analgesic",
                "Antibiotic",
                "Anticoagulant",
                "Anticonvulsant",
                "Antidepressant",
                "Antidiabetic",
                "Antiemetic",
                "Antifungal",
                "Antihistamine",
                "Antihypertensive",
                "Anti-Inflammatory",
                "Antiparasitic",
                "Antipsychotic",
                "Antiviral",
                "Anxiolytic",
                "Cardiovascular",
                "Cholesterol",
                "Contraceptive",
                "Dermatological",
                "Gastrointestinal",
                "Hormonal",
                "Immunosuppressant",
                "Migraine",
                "Mineral",
                "Muscle Relaxant",
                "Neurological",
                "Oncology",
                "Ophthalmic",
                "Osmotic Laxative",
                "Opioid",
                "Osteoporosis",
                "Respiratory",
                "Sleep Aid",
                "Smoking Cessation",
                "Supplement",
                "Thyroid",
                "Urological",
                "Vitamin",
                "Weight Management",
            ]
            .contains(&category.as_str())
        {
            return Err(("category", "is not included in the list"));
        }
    }
    let code = if attributes.get("dmd_code").is_some() {
        scalar_string(attributes, "dmd_code").map_err(|error| ("dmd_code", error))?
    } else {
        existing.and_then(|record| record.dmd_code.clone())
    };
    if code.as_deref().is_some_and(|value| !value.is_empty()) {
        let system = if attributes.get("dmd_system").is_some() {
            scalar_string(attributes, "dmd_system").map_err(|error| ("dmd_system", error))?
        } else {
            existing.and_then(|record| record.dmd_system.clone())
        };
        if system.as_deref().is_none_or(str::is_empty) {
            return Err(("dmd_system", "can't be blank"));
        }
    }
    if let Some(schedule_type) = attributes.get("default_schedule_type") {
        let valid = schedule_type
            .as_i64()
            .is_some_and(|value| (0..=6).contains(&value))
            || schedule_type.as_str().is_some_and(|value| {
                [
                    "daily",
                    "multiple_daily",
                    "weekly",
                    "specific_dates",
                    "prn",
                    "tapering",
                    "every_other_day",
                ]
                .contains(&value)
            });
        if !valid {
            return Err(("default_schedule_type", "is invalid"));
        }
    }
    Ok(())
}

async fn barcode_conflict(
    db: &DatabaseTransaction,
    attributes: &Value,
    existing_id: Option<i64>,
) -> Result<bool, ApiError> {
    let Some(barcode) = attributes.get("barcode").and_then(Value::as_str) else {
        return Ok(false);
    };
    if barcode.is_empty() {
        return Ok(false);
    }
    let row = medication::Entity::find()
        .filter(medication::Column::Barcode.eq(barcode))
        .one(db)
        .await
        .map_err(database_error)?;
    Ok(row.is_some_and(|row| Some(row.id) != existing_id))
}

fn assign_attributes(
    active: &mut medication::ActiveModel,
    attributes: &Value,
) -> Result<(), (&'static str, &'static str)> {
    for field in [
        "name",
        "friendly_name",
        "category",
        "description",
        "dose_unit",
        "barcode",
        "dmd_code",
        "dmd_system",
        "dmd_concept_class",
        "warnings",
    ] {
        if attributes.get(field).is_none() {
            continue;
        }
        let value = scalar_string(attributes, field).map_err(|error| (field, error))?;
        match field {
            "name" => active.name = Set(value),
            "friendly_name" => active.friendly_name = Set(value),
            "category" => active.category = Set(value),
            "description" => active.description = Set(value),
            "dose_unit" => active.dose_unit = Set(value),
            "barcode" => active.barcode = Set(value),
            "dmd_code" => active.dmd_code = Set(value),
            "dmd_system" => active.dmd_system = Set(value),
            "dmd_concept_class" => active.dmd_concept_class = Set(value),
            "warnings" => active.warnings = Set(value),
            _ => {}
        }
    }
    if let Some(id) = location_id(attributes) {
        active.location_id = Set(id);
    }
    if attributes.get("dose_amount").is_some() {
        let value =
            decimal_field(attributes, "dose_amount").map_err(|error| ("dose_amount", error))?;
        active.dose_amount =
            Set(value.map(|value| value.to_string().parse::<f64>().unwrap_or(0.0)));
    }
    if attributes.get("current_supply").is_some() {
        active.current_supply = Set(decimal_field(attributes, "current_supply")
            .map_err(|error| ("current_supply", error))?);
    }
    if attributes.get("reorder_threshold").is_some() {
        active.reorder_threshold = Set(decimal_field(attributes, "reorder_threshold")
            .map_err(|error| ("reorder_threshold", error))?
            .unwrap_or(Decimal::ZERO));
    }
    if let Some(value) = attributes.get("default_schedule_type") {
        let names = [
            "daily",
            "multiple_daily",
            "weekly",
            "specific_dates",
            "prn",
            "tapering",
            "every_other_day",
        ];
        let number = value
            .as_i64()
            .or_else(|| {
                value.as_str().and_then(|value| {
                    names
                        .iter()
                        .position(|name| *name == value)
                        .map(|index| index as i64)
                })
            })
            .ok_or(("default_schedule_type", "is invalid"))?;
        active.default_schedule_type = Set(number as i32);
    }
    Ok(())
}

pub(super) async fn create(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
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
    let attributes = body.get("medication").unwrap_or(&Value::Null);
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

async fn update_medication(
    state: AppState,
    household_id: i64,
    id: String,
    headers: HeaderMap,
    body: Value,
    method: &str,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
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
    let attributes = body.get("medication").unwrap_or(&Value::Null);
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

pub(super) async fn patch(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Response, ApiError> {
    update_medication(state, household_id, id, headers, body, "PATCH").await
}

pub(super) async fn put(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Response, ApiError> {
    update_medication(state, household_id, id, headers, body, "PUT").await
}

pub(super) async fn adjust_inventory(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
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
    let attributes = body.get("adjustment").unwrap_or(&Value::Null);
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
    lock_household(&db, household_id).await?;
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
    record_version(
        &db,
        &context,
        &request_id,
        "Medication",
        updated.id,
        "adjust inventory",
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
    let (db, context) = request_context(&state, &headers, household_id).await?;
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
    let details = body.get("order_details").unwrap_or(&Value::Null);
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
    lock_household(&db, household_id).await?;
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

pub(super) async fn mark_as_ordered(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Response, ApiError> {
    reorder(state, household_id, id, headers, body, 1).await
}

pub(super) async fn mark_as_received(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Response, ApiError> {
    reorder(state, household_id, id, headers, body, 2).await
}
