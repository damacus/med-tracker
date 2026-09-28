use crate::dosage_options::{parse_decimal, storage_decimal, valid_identifier};
use crate::entities::{api_tombstone, dosage, grant, medication, person_medication};
use crate::medication_management::{
    error_response, finish_with_request_id, record_version, request_context,
};
use crate::mutation_idempotency::{self, Lookup, StoredResponse};
use crate::read_entities::{person, person_medication as read_assignment};
use crate::read_resources::serialize_assignments;
use crate::sync_batch::{SyncOperation, SyncResult};
use crate::sync_events::{record_change, SyncRecord};
use crate::{
    audit, database_error, granted_people, representation_etag, scope, ApiError, AppState,
    AuthContext,
};
use axum::extract::{rejection::JsonRejection, Path, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;
use sea_orm::prelude::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseTransaction, EntityTrait, IntoActiveModel,
    QueryFilter, QueryOrder, Set,
};
use serde_json::{json, Map, Value};
use uuid::Uuid;

const CONTROLLER: &str = "api/v1/person_medications";
const POLICY: &str = "PersonMedicationPolicy";
const DOSE_UNITS: &[&str] = &[
    "tablet", "capsule", "gummy", "mg", "ml", "g", "mcg", "IU", "spray", "drop", "sachet", "pad",
];

#[derive(Default)]
struct Attributes {
    person_id: Option<String>,
    medication_id: Option<String>,
    source_dosage_option_id: Option<String>,
    dose_amount: Option<Decimal>,
    dose_unit: Option<String>,
    administration_kind: Option<i32>,
    notes: Option<String>,
    max_daily_doses: Option<i32>,
    min_hours_between_doses: Option<Option<i32>>,
    dose_cycle: Option<i32>,
}

fn required_string(attributes: &Map<String, Value>, name: &str) -> Option<Option<String>> {
    attributes
        .get(name)
        .map(|value| {
            value
                .as_str()
                .filter(|text| !text.trim().is_empty())
                .map(str::to_owned)
        })
        .map_or(Some(None), |value| value.map(Some))
}

fn identifier(attributes: &Map<String, Value>, name: &str) -> Option<Option<String>> {
    let value = required_string(attributes, name)?;
    if value
        .as_deref()
        .is_some_and(|value| !valid_identifier(value))
    {
        return None;
    }
    Some(value)
}

fn optional_string(attributes: &Map<String, Value>, name: &str) -> Option<Option<String>> {
    attributes
        .get(name)
        .map(|value| value.as_str().map(str::to_owned))
        .map_or(Some(None), |value| value.map(Some))
}

fn numeric_10_2(value: Decimal) -> bool {
    value > Decimal::ZERO && storage_decimal(value, 8, 2).is_some()
}

fn decimal(attributes: &Map<String, Value>, name: &str) -> Option<Option<Decimal>> {
    attributes
        .get(name)
        .map(|value| {
            let amount = parse_decimal(value)?;
            numeric_10_2(amount).then_some(amount)
        })
        .map_or(Some(None), |value| value.map(Some))
}

fn positive_integer(attributes: &Map<String, Value>, name: &str) -> Option<Option<i32>> {
    attributes
        .get(name)
        .map(|value| {
            i32::try_from(value.as_i64()?)
                .ok()
                .filter(|number| *number >= 1)
        })
        .map_or(Some(None), |value| value.map(Some))
}

fn whole_hours(attributes: &Map<String, Value>) -> Option<Option<Option<i32>>> {
    let Some(value) = attributes.get("min_hours_between_doses") else {
        return Some(None);
    };
    if value.is_null() {
        return Some(Some(None));
    }
    let amount = parse_decimal(value)?;
    if amount.fract() != Decimal::ZERO || amount <= Decimal::ZERO {
        return None;
    }
    let hours = amount.trunc().to_string().parse::<i32>().ok()?;
    Some(Some(Some(hours)))
}

impl Attributes {
    fn parse(body: &Value, create: bool) -> Result<Self, StatusCode> {
        let outer = body.as_object().ok_or(StatusCode::BAD_REQUEST)?;
        let inner = outer
            .get("person_medication")
            .ok_or(StatusCode::BAD_REQUEST)?
            .as_object()
            .ok_or(StatusCode::BAD_REQUEST)?;
        if outer.len() != 1
            || inner.is_empty()
            || inner.keys().any(|key| {
                !matches!(
                    key.as_str(),
                    "person_id"
                        | "medication_id"
                        | "source_dosage_option_id"
                        | "dose_amount"
                        | "dose_unit"
                        | "administration_kind"
                        | "notes"
                        | "max_daily_doses"
                        | "min_hours_between_doses"
                        | "dose_cycle"
                )
            })
        {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let person_id = identifier(inner, "person_id").ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
        let medication_id =
            identifier(inner, "medication_id").ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
        if create && (person_id.is_none() || medication_id.is_none()) {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let dose_unit =
            required_string(inner, "dose_unit").ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
        if dose_unit
            .as_deref()
            .is_some_and(|unit| !DOSE_UNITS.contains(&unit))
        {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let administration_kind = inner
            .get("administration_kind")
            .map(|value| match value.as_str()? {
                "routine" => Some(0),
                "as_needed" => Some(1),
                _ => None,
            })
            .map_or(Some(None), |value| value.map(Some))
            .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
        let dose_cycle = inner
            .get("dose_cycle")
            .map(|value| match value.as_str()? {
                "daily" => Some(0),
                "weekly" => Some(1),
                "monthly" => Some(2),
                _ => None,
            })
            .map_or(Some(None), |value| value.map(Some))
            .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
        Ok(Self {
            person_id,
            medication_id,
            source_dosage_option_id: identifier(inner, "source_dosage_option_id")
                .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?,
            dose_amount: decimal(inner, "dose_amount").ok_or(StatusCode::UNPROCESSABLE_ENTITY)?,
            dose_unit,
            administration_kind,
            notes: optional_string(inner, "notes").ok_or(StatusCode::UNPROCESSABLE_ENTITY)?,
            max_daily_doses: positive_integer(inner, "max_daily_doses")
                .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?,
            min_hours_between_doses: whole_hours(inner).ok_or(StatusCode::UNPROCESSABLE_ENTITY)?,
            dose_cycle,
        })
    }
}

fn path(household_id: i64, id: Option<&str>) -> String {
    let base = format!("/api/v1/households/{household_id}/person_medications");
    id.map_or(base.clone(), |id| format!("{base}/{id}"))
}

async fn failure(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    action: &str,
    status: StatusCode,
) -> Result<Response, ApiError> {
    let (code, message, errors) = match status {
        StatusCode::BAD_REQUEST => ("bad_request", "Invalid request", None),
        StatusCode::FORBIDDEN => (
            "forbidden",
            "You are not authorized to perform this action.",
            None,
        ),
        StatusCode::NOT_FOUND => ("not_found", "Record not found", None),
        StatusCode::CONFLICT => (
            "conflict",
            "Record has changed since it was last read",
            None,
        ),
        _ => (
            "validation_failed",
            "Validation failed",
            Some(json!({"person_medication": ["is invalid"]})),
        ),
    };
    error_response(
        db, context, method, CONTROLLER, POLICY, action, status, code, message, errors,
    )
    .await
}

async fn validation_failure(
    db: DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    method: &str,
    action: &str,
    request_path: &str,
    request_digest: &str,
) -> Result<Response, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let body = json!({"error": {
        "code": "validation_failed",
        "message": "Validation failed",
        "errors": {"person_medication": ["is invalid"]},
        "request_id": request_id
    }});
    if let Some(key) = mutation_idempotency::key(headers) {
        mutation_idempotency::store(
            &db,
            context,
            StoredResponse {
                key,
                method,
                path: request_path,
                digest: request_digest,
                status: StatusCode::UNPROCESSABLE_ENTITY,
                body: body.clone(),
                request_id: &request_id,
                etag: None,
            },
        )
        .await?;
    }
    finish_with_request_id(
        db,
        context,
        &request_id,
        method,
        CONTROLLER,
        POLICY,
        action,
        StatusCode::UNPROCESSABLE_ENTITY,
        false,
        body,
        None,
    )
    .await
}

async fn find_person(
    db: &DatabaseTransaction,
    context: &AuthContext,
    identifier: &str,
) -> Result<Option<person::Model>, ApiError> {
    let query = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(context.membership.household_id))
        .filter(person::Column::Id.in_subquery(granted_people(&context.membership)));
    let query = match identifier.parse::<i64>() {
        Ok(id) => query.filter(person::Column::Id.eq(id)),
        Err(_) => query.filter(person::Column::PortableId.eq(identifier)),
    };
    query.one(db).await.map_err(database_error)
}

async fn can_manage_person(
    db: &DatabaseTransaction,
    context: &AuthContext,
    person_id: i64,
) -> Result<bool, ApiError> {
    let grant = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(context.membership.household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(context.membership.id))
        .filter(grant::Column::PersonId.eq(person_id))
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

async fn find_medication(
    db: &DatabaseTransaction,
    context: &AuthContext,
    identifier: &str,
) -> Result<Option<medication::Model>, ApiError> {
    let query = scope(context.membership.household_id, &context.membership);
    let query = match identifier.parse::<i64>() {
        Ok(id) => query.filter(medication::Column::Id.eq(id)),
        Err(_) => query.filter(medication::Column::PortableId.eq(identifier)),
    };
    query.one(db).await.map_err(database_error)
}

async fn find_visible_option(
    db: &DatabaseTransaction,
    context: &AuthContext,
    identifier: &str,
) -> Result<Option<dosage::Model>, ApiError> {
    let query = dosage::Entity::find()
        .filter(dosage::Column::HouseholdId.eq(context.membership.household_id));
    let query = match identifier.parse::<i64>() {
        Ok(id) => query.filter(dosage::Column::Id.eq(id)),
        Err(_) => query.filter(dosage::Column::PortableId.eq(identifier)),
    };
    let Some(option) = query.one(db).await.map_err(database_error)? else {
        return Ok(None);
    };
    if find_medication(db, context, &option.medication_id.to_string())
        .await?
        .is_none()
    {
        return Ok(None);
    }
    Ok(Some(option))
}

async fn find_assignment(
    db: &DatabaseTransaction,
    context: &AuthContext,
    identifier: &str,
) -> Result<Option<person_medication::Model>, ApiError> {
    let query = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(context.membership.household_id))
        .filter(person_medication::Column::RetiredAt.is_null())
        .filter(
            person_medication::Column::PersonId.in_subquery(granted_people(&context.membership)),
        );
    let query = match identifier.parse::<i64>() {
        Ok(id) => query.filter(person_medication::Column::Id.eq(id)),
        Err(_) => query.filter(person_medication::Column::PortableId.eq(identifier)),
    };
    query.one(db).await.map_err(database_error)
}

async fn representation(
    db: &DatabaseTransaction,
    context: &AuthContext,
    id: i64,
) -> Result<(Value, String), ApiError> {
    let row = read_assignment::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?;
    let data = serialize_assignments(db, context, vec![row])
        .await?
        .pop()
        .ok_or_else(ApiError::internal)?;
    let body = json!({"data": data});
    let etag = representation_etag(&body);
    Ok((body, etag))
}

struct WriteCompletion<'a> {
    context: &'a AuthContext,
    headers: &'a HeaderMap,
    method: &'a str,
    action: &'a str,
    request_path: &'a str,
    request_digest: &'a str,
    request_id: &'a str,
    status: StatusCode,
    body: Value,
    etag: &'a str,
}

async fn finish_write(
    db: DatabaseTransaction,
    write: WriteCompletion<'_>,
) -> Result<Response, ApiError> {
    let WriteCompletion {
        context,
        headers,
        method,
        action,
        request_path,
        request_digest,
        request_id,
        status,
        body,
        etag,
    } = write;
    if let Some(key) = mutation_idempotency::key(headers) {
        mutation_idempotency::store(
            &db,
            context,
            StoredResponse {
                key,
                method,
                path: request_path,
                digest: request_digest,
                status,
                body: body.clone(),
                request_id,
                etag: Some(etag),
            },
        )
        .await?;
    }
    finish_with_request_id(
        db,
        context,
        request_id,
        method,
        CONTROLLER,
        POLICY,
        action,
        status,
        true,
        body,
        Some(etag),
    )
    .await
}

async fn replay_or_conflict(
    db: &DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    method: &str,
    action: &str,
    request_path: &str,
    request_digest: &str,
) -> Result<Option<Response>, ApiError> {
    let Some(key) = mutation_idempotency::key(headers) else {
        return Ok(None);
    };
    match mutation_idempotency::lookup(db, context, key, method, request_path, request_digest)
        .await?
    {
        Lookup::New => Ok(None),
        Lookup::Conflict => {
            let request_id = Uuid::new_v4().to_string();
            audit::record_resource_request_with_id(
                db,
                context,
                &request_id,
                method,
                CONTROLLER,
                POLICY,
                action,
                StatusCode::CONFLICT,
                false,
            )
            .await
            .map_err(database_error)?;
            let mut response = (
                StatusCode::CONFLICT,
                Json(json!({"error": {"code": "idempotency_key_reused", "message": "Idempotency key has already been used for a different request", "request_id": request_id}})),
            )
                .into_response();
            response.headers_mut().insert(
                "x-request-id",
                HeaderValue::from_str(&request_id).map_err(|_| ApiError::internal())?,
            );
            Ok(Some(response))
        }
        Lookup::Replay(saved) => {
            let request_id = Uuid::new_v4().to_string();
            let status = StatusCode::from_u16(saved.response_status as u16)
                .map_err(|_| ApiError::internal())?;
            audit::record_resource_request_with_id(
                db,
                context,
                &request_id,
                method,
                CONTROLLER,
                POLICY,
                action,
                status,
                status.is_success(),
            )
            .await
            .map_err(database_error)?;
            let mut response = mutation_idempotency::replay(*saved)?;
            response.headers_mut().insert(
                "x-request-id",
                HeaderValue::from_str(&request_id).map_err(|_| ApiError::internal())?,
            );
            Ok(Some(response))
        }
    }
}

fn option_matches(option: &dosage::Model, amount: Decimal, unit: &str) -> bool {
    option.amount == amount && option.unit == unit
}

async fn resolved_dose(
    db: &DatabaseTransaction,
    context: &AuthContext,
    attrs: &Attributes,
    person: &person::Model,
    medication: &medication::Model,
    previous: Option<&person_medication::Model>,
    selected_option: Option<&dosage::Model>,
) -> Result<Option<(Decimal, String, Option<i64>)>, ApiError> {
    let options = dosage::Entity::find()
        .filter(dosage::Column::HouseholdId.eq(context.membership.household_id))
        .filter(dosage::Column::MedicationId.eq(medication.id))
        .order_by_asc(dosage::Column::Amount)
        .order_by_asc(dosage::Column::Id)
        .all(db)
        .await
        .map_err(database_error)?;
    let explicit_option = selected_option;
    if explicit_option.is_some_and(|option| option.medication_id != medication.id) {
        return Ok(None);
    }
    let amount = attrs
        .dose_amount
        .or_else(|| previous.and_then(|record| record.dose_amount));
    let unit = attrs
        .dose_unit
        .clone()
        .or_else(|| previous.and_then(|record| record.dose_unit.clone()));
    let default_option = if explicit_option.is_none() && (amount.is_none() || unit.is_none()) {
        if person.person_type != 0 {
            options.iter().find(|option| option.default_for_children)
        } else {
            None
        }
        .or_else(|| options.iter().find(|option| option.default_for_adults))
        .or_else(|| options.first())
    } else {
        None
    };
    let chosen = explicit_option.or(default_option);
    let amount = amount
        .or_else(|| chosen.map(|option| option.amount))
        .or_else(|| {
            medication
                .dose_amount
                .and_then(|amount| Decimal::from_str_exact(&amount.to_string()).ok())
        });
    let unit = unit
        .or_else(|| chosen.map(|option| option.unit.clone()))
        .or_else(|| medication.dose_unit.clone());
    let (Some(amount), Some(unit)) = (amount, unit) else {
        return Ok(None);
    };
    if !numeric_10_2(amount) || !DOSE_UNITS.contains(&unit.as_str()) {
        return Ok(None);
    }
    if explicit_option.is_some_and(|option| !option_matches(option, amount, &unit)) {
        return Ok(None);
    }
    let matching: Vec<_> = options
        .iter()
        .filter(|option| option_matches(option, amount, &unit))
        .collect();
    let option_id = if let Some(option) = explicit_option {
        Some(option.id)
    } else if matching.len() == 1 {
        Some(matching[0].id)
    } else {
        chosen
            .filter(|option| option_matches(option, amount, &unit))
            .map(|option| option.id)
    };
    Ok(Some((amount, unit, option_id)))
}

pub(super) async fn create(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let Json(body) = match payload {
        Ok(value) => value,
        Err(_) => return failure(db, &context, "POST", "create", StatusCode::BAD_REQUEST).await,
    };
    let parsed = Attributes::parse(&body, true);
    let person_identifier = body
        .get("person_medication")
        .and_then(Value::as_object)
        .and_then(|inner| inner.get("person_id"))
        .and_then(Value::as_str)
        .filter(|value| valid_identifier(value));
    let medication_identifier = body
        .get("person_medication")
        .and_then(Value::as_object)
        .and_then(|inner| inner.get("medication_id"))
        .and_then(Value::as_str)
        .filter(|value| valid_identifier(value));
    let (Some(person_identifier), Some(medication_identifier)) =
        (person_identifier, medication_identifier)
    else {
        let status = parsed.err().unwrap_or(StatusCode::UNPROCESSABLE_ENTITY);
        return failure(db, &context, "POST", "create", status).await;
    };
    let (_, context) = mutation_idempotency::lock_household_and_reauthenticate(
        &state,
        &db,
        &headers,
        household_id,
    )
    .await?;
    let person = match find_person(&db, &context, person_identifier).await? {
        Some(value) => value,
        None => return failure(db, &context, "POST", "create", StatusCode::NOT_FOUND).await,
    };
    if !can_manage_person(&db, &context, person.id).await? {
        return failure(db, &context, "POST", "create", StatusCode::FORBIDDEN).await;
    }
    let medication = match find_medication(&db, &context, medication_identifier).await? {
        Some(value) => value,
        None => return failure(db, &context, "POST", "create", StatusCode::NOT_FOUND).await,
    };
    let request_path = path(household_id, None);
    let request_digest = mutation_idempotency::digest("POST", &request_path, &body);
    if let Some(response) = replay_or_conflict(
        &db,
        &context,
        &headers,
        "POST",
        "create",
        &request_path,
        &request_digest,
    )
    .await?
    {
        db.commit().await.map_err(database_error)?;
        return Ok(response);
    }
    let attrs = match parsed {
        Ok(value) => value,
        Err(StatusCode::UNPROCESSABLE_ENTITY) => {
            return validation_failure(
                db,
                &context,
                &headers,
                "POST",
                "create",
                &request_path,
                &request_digest,
            )
            .await;
        }
        Err(status) => return failure(db, &context, "POST", "create", status).await,
    };
    if person_medication::Entity::find()
        .filter(person_medication::Column::PersonId.eq(person.id))
        .filter(person_medication::Column::MedicationId.eq(medication.id))
        .filter(person_medication::Column::RetiredAt.is_null())
        .one(&db)
        .await
        .map_err(database_error)?
        .is_some()
    {
        return validation_failure(
            db,
            &context,
            &headers,
            "POST",
            "create",
            &request_path,
            &request_digest,
        )
        .await;
    }
    let selected_option = if let Some(identifier) = attrs.source_dosage_option_id.as_deref() {
        match find_visible_option(&db, &context, identifier).await? {
            Some(option) => Some(option),
            None => return failure(db, &context, "POST", "create", StatusCode::NOT_FOUND).await,
        }
    } else {
        None
    };
    let Some((amount, unit, source_option)) = resolved_dose(
        &db,
        &context,
        &attrs,
        &person,
        &medication,
        None,
        selected_option.as_ref(),
    )
    .await?
    else {
        return validation_failure(
            db,
            &context,
            &headers,
            "POST",
            "create",
            &request_path,
            &request_digest,
        )
        .await;
    };
    let position = person_medication::Entity::find()
        .filter(person_medication::Column::PersonId.eq(person.id))
        .order_by_desc(person_medication::Column::Position)
        .one(&db)
        .await
        .map_err(database_error)?
        .map_or(1, |record| record.position + 1);
    let now = Utc::now().naive_utc();
    let kind = attrs.administration_kind.unwrap_or(1);
    let min_hours = attrs.min_hours_between_doses.unwrap_or(None);
    let min_hours = if kind == 0 && attrs.max_daily_doses == Some(1) && min_hours == Some(24) {
        None
    } else {
        min_hours
    };
    let record = person_medication::ActiveModel {
        household_id: Set(household_id),
        person_id: Set(person.id),
        medication_id: Set(medication.id),
        active: Set(true),
        administration_kind: Set(kind),
        dose_amount: Set(Some(amount)),
        dose_unit: Set(Some(unit)),
        source_dosage_option_id: Set(source_option),
        notes: Set(attrs.notes),
        max_daily_doses: Set(attrs.max_daily_doses),
        min_hours_between_doses: Set(min_hours),
        dose_cycle: Set(attrs.dose_cycle),
        position: Set(position),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .map_err(database_error)?;
    let (response_body, etag) = representation(&db, &context, record.id).await?;
    let request_id = Uuid::new_v4().to_string();
    record_version(
        &db,
        &context,
        &request_id,
        "PersonMedication",
        record.id,
        "create",
        None,
        Some(response_body["data"].clone()),
    )
    .await?;
    record_change(
        &db,
        &context,
        &request_id,
        SyncRecord {
            record_type: "PersonMedication",
            record_id: record.id,
            portable_id: &record.portable_id,
            action: "create",
            person_portable_id: Some(&person.portable_id),
        },
    )
    .await?;
    finish_write(
        db,
        WriteCompletion {
            context: &context,
            headers: &headers,
            method: "POST",
            action: "create",
            request_path: &request_path,
            request_digest: &request_digest,
            request_id: &request_id,
            status: StatusCode::CREATED,
            body: response_body,
            etag: &etag,
        },
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

async fn update(
    state: AppState,
    household_id: i64,
    id: String,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
    method: &str,
) -> Result<Response, ApiError> {
    let (db, _) = request_context(&state, &headers, household_id).await?;
    let (_, context) = mutation_idempotency::lock_household_and_reauthenticate(
        &state,
        &db,
        &headers,
        household_id,
    )
    .await?;
    let record = match find_assignment(&db, &context, &id).await? {
        Some(value) => value,
        None => return failure(db, &context, method, "update", StatusCode::NOT_FOUND).await,
    };
    if !can_manage_person(&db, &context, record.person_id).await? {
        return failure(db, &context, method, "update", StatusCode::FORBIDDEN).await;
    }
    let Json(body) = match payload {
        Ok(value) => value,
        Err(_) => return failure(db, &context, method, "update", StatusCode::BAD_REQUEST).await,
    };
    let request_path = path(household_id, Some(&id));
    let request_digest = mutation_idempotency::digest(method, &request_path, &body);
    if let Some(response) = replay_or_conflict(
        &db,
        &context,
        &headers,
        method,
        "update",
        &request_path,
        &request_digest,
    )
    .await?
    {
        db.commit().await.map_err(database_error)?;
        return Ok(response);
    }
    let attrs = match Attributes::parse(&body, false) {
        Ok(value) => value,
        Err(StatusCode::UNPROCESSABLE_ENTITY) => {
            return validation_failure(
                db,
                &context,
                &headers,
                method,
                "update",
                &request_path,
                &request_digest,
            )
            .await;
        }
        Err(status) => return failure(db, &context, method, "update", status).await,
    };
    let (before_body, current_etag) = representation(&db, &context, record.id).await?;
    if headers
        .get(header::IF_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| !value.is_empty() && value != current_etag)
    {
        return failure(db, &context, method, "update", StatusCode::CONFLICT).await;
    }
    let person = person::Entity::find_by_id(record.person_id)
        .one(&db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?;
    if let Some(identifier) = attrs.person_id.as_deref() {
        let Some(requested) = find_person(&db, &context, identifier).await? else {
            return failure(db, &context, method, "update", StatusCode::NOT_FOUND).await;
        };
        if requested.id != record.person_id {
            return validation_failure(
                db,
                &context,
                &headers,
                method,
                "update",
                &request_path,
                &request_digest,
            )
            .await;
        }
    }
    let medication = if let Some(identifier) = attrs.medication_id.as_deref() {
        match find_medication(&db, &context, identifier).await? {
            Some(value) => value,
            None => return failure(db, &context, method, "update", StatusCode::NOT_FOUND).await,
        }
    } else {
        medication::Entity::find_by_id(record.medication_id)
            .one(&db)
            .await
            .map_err(database_error)?
            .ok_or_else(ApiError::not_found)?
    };
    if medication.id != record.medication_id
        && person_medication::Entity::find()
            .filter(person_medication::Column::PersonId.eq(record.person_id))
            .filter(person_medication::Column::MedicationId.eq(medication.id))
            .filter(person_medication::Column::RetiredAt.is_null())
            .one(&db)
            .await
            .map_err(database_error)?
            .is_some()
    {
        return validation_failure(
            db,
            &context,
            &headers,
            method,
            "update",
            &request_path,
            &request_digest,
        )
        .await;
    }
    let selected_option = if let Some(identifier) = attrs.source_dosage_option_id.as_deref() {
        match find_visible_option(&db, &context, identifier).await? {
            Some(option) => Some(option),
            None => return failure(db, &context, method, "update", StatusCode::NOT_FOUND).await,
        }
    } else if let Some(option_id) = record.source_dosage_option_id {
        dosage::Entity::find_by_id(option_id)
            .one(&db)
            .await
            .map_err(database_error)?
    } else {
        None
    };
    let Some((amount, unit, source_option)) = resolved_dose(
        &db,
        &context,
        &attrs,
        &person,
        &medication,
        Some(&record),
        selected_option.as_ref(),
    )
    .await?
    else {
        return validation_failure(
            db,
            &context,
            &headers,
            method,
            "update",
            &request_path,
            &request_digest,
        )
        .await;
    };
    let kind = attrs
        .administration_kind
        .unwrap_or(record.administration_kind);
    let maximum = attrs.max_daily_doses.or(record.max_daily_doses);
    let mut minimum = attrs
        .min_hours_between_doses
        .unwrap_or(record.min_hours_between_doses);
    if kind == 0 && maximum == Some(1) && minimum == Some(24) {
        minimum = None;
    }
    let unchanged = medication.id == record.medication_id
        && record.dose_amount == Some(amount)
        && record.dose_unit.as_deref() == Some(unit.as_str())
        && record.source_dosage_option_id == source_option
        && record.administration_kind == kind
        && record.notes == attrs.notes.clone().or(record.notes.clone())
        && record.max_daily_doses == maximum
        && record.min_hours_between_doses == minimum
        && record.dose_cycle == attrs.dose_cycle.or(record.dose_cycle);
    if unchanged {
        let request_id = Uuid::new_v4().to_string();
        return finish_write(
            db,
            WriteCompletion {
                context: &context,
                headers: &headers,
                method,
                action: "update",
                request_path: &request_path,
                request_digest: &request_digest,
                request_id: &request_id,
                status: StatusCode::OK,
                body: before_body,
                etag: &current_etag,
            },
        )
        .await;
    }
    let mut active = record.clone().into_active_model();
    active.medication_id = Set(medication.id);
    active.dose_amount = Set(Some(amount));
    active.dose_unit = Set(Some(unit));
    active.source_dosage_option_id = Set(source_option);
    if let Some(value) = attrs.administration_kind {
        active.administration_kind = Set(value);
    }
    if let Some(value) = attrs.notes {
        active.notes = Set(Some(value));
    }
    if let Some(value) = attrs.max_daily_doses {
        active.max_daily_doses = Set(Some(value));
    }
    if let Some(value) = attrs.min_hours_between_doses {
        active.min_hours_between_doses = Set(value);
    }
    if let Some(value) = attrs.dose_cycle {
        active.dose_cycle = Set(Some(value));
    }
    active.min_hours_between_doses = Set(minimum);
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active.update(&db).await.map_err(database_error)?;
    let (response_body, etag) = representation(&db, &context, updated.id).await?;
    let request_id = Uuid::new_v4().to_string();
    record_version(
        &db,
        &context,
        &request_id,
        "PersonMedication",
        updated.id,
        "update",
        Some(before_body["data"].clone()),
        Some(response_body["data"].clone()),
    )
    .await?;
    record_change(
        &db,
        &context,
        &request_id,
        SyncRecord {
            record_type: "PersonMedication",
            record_id: updated.id,
            portable_id: &updated.portable_id,
            action: "update",
            person_portable_id: Some(&person.portable_id),
        },
    )
    .await?;
    finish_write(
        db,
        WriteCompletion {
            context: &context,
            headers: &headers,
            method,
            action: "update",
            request_path: &request_path,
            request_digest: &request_digest,
            request_id: &request_id,
            status: StatusCode::OK,
            body: response_body,
            etag: &etag,
        },
    )
    .await
}

fn sync_error(status: StatusCode) -> ApiError {
    let (code, message) = match status {
        StatusCode::NOT_FOUND => ("not_found", "Record not found"),
        StatusCode::FORBIDDEN => (
            "forbidden",
            "You are not authorized to perform this action.",
        ),
        StatusCode::PRECONDITION_REQUIRED => (
            "precondition_required",
            "A current resource version is required",
        ),
        StatusCode::CONFLICT => ("sync_conflict", "Record has changed since it was last read"),
        StatusCode::BAD_REQUEST => ("bad_request", "Invalid request body"),
        _ => ("unprocessable_content", "Person medication is invalid"),
    };
    ApiError {
        status,
        code,
        message,
        preserve_activity: false,
    }
}

fn sync_precondition(actual: &str, expected: Option<&str>) -> Result<(), ApiError> {
    let expected = expected.ok_or_else(|| sync_error(StatusCode::PRECONDITION_REQUIRED))?;
    if actual != expected {
        return Err(sync_error(StatusCode::CONFLICT));
    }
    Ok(())
}

pub(super) async fn apply_sync_operation(
    db: &DatabaseTransaction,
    context: &AuthContext,
    operation: &SyncOperation,
    request_id: &str,
) -> Result<SyncResult, ApiError> {
    let household_id = context.membership.household_id;
    let (record, changed) = if operation.action == "create" {
        let body = json!({"person_medication": operation.attributes});
        let attrs = Attributes::parse(&body, true).map_err(sync_error)?;
        let person = find_person(db, context, attrs.person_id.as_deref().unwrap())
            .await?
            .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?;
        if !can_manage_person(db, context, person.id).await? {
            return Err(sync_error(StatusCode::FORBIDDEN));
        }
        let medication = find_medication(db, context, attrs.medication_id.as_deref().unwrap())
            .await?
            .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?;
        if person_medication::Entity::find()
            .filter(person_medication::Column::PersonId.eq(person.id))
            .filter(person_medication::Column::MedicationId.eq(medication.id))
            .filter(person_medication::Column::RetiredAt.is_null())
            .one(db)
            .await
            .map_err(database_error)?
            .is_some()
        {
            return Err(sync_error(StatusCode::UNPROCESSABLE_ENTITY));
        }
        let selected_option = if let Some(id) = attrs.source_dosage_option_id.as_deref() {
            Some(
                find_visible_option(db, context, id)
                    .await?
                    .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?,
            )
        } else {
            None
        };
        let (amount, unit, source_option) = resolved_dose(
            db,
            context,
            &attrs,
            &person,
            &medication,
            None,
            selected_option.as_ref(),
        )
        .await?
        .ok_or_else(|| sync_error(StatusCode::UNPROCESSABLE_ENTITY))?;
        let position = person_medication::Entity::find()
            .filter(person_medication::Column::PersonId.eq(person.id))
            .order_by_desc(person_medication::Column::Position)
            .one(db)
            .await
            .map_err(database_error)?
            .map_or(1, |row| row.position + 1);
        let now = Utc::now().naive_utc();
        let kind = attrs.administration_kind.unwrap_or(1);
        let mut minimum = attrs.min_hours_between_doses.unwrap_or(None);
        if kind == 0 && attrs.max_daily_doses == Some(1) && minimum == Some(24) {
            minimum = None;
        }
        let record = person_medication::ActiveModel {
            household_id: Set(household_id),
            person_id: Set(person.id),
            medication_id: Set(medication.id),
            active: Set(true),
            administration_kind: Set(kind),
            dose_amount: Set(Some(amount)),
            dose_unit: Set(Some(unit)),
            source_dosage_option_id: Set(source_option),
            notes: Set(attrs.notes),
            max_daily_doses: Set(attrs.max_daily_doses),
            min_hours_between_doses: Set(minimum),
            dose_cycle: Set(attrs.dose_cycle),
            position: Set(position),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(database_error)?;
        let (body, _) = representation(db, context, record.id).await?;
        record_version(
            db,
            context,
            request_id,
            "PersonMedication",
            record.id,
            "create",
            None,
            Some(body["data"].clone()),
        )
        .await?;
        record_change(
            db,
            context,
            request_id,
            SyncRecord {
                record_type: "PersonMedication",
                record_id: record.id,
                portable_id: &record.portable_id,
                action: "create",
                person_portable_id: Some(&person.portable_id),
            },
        )
        .await?;
        (record, true)
    } else {
        let id = operation
            .id
            .as_deref()
            .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?;
        let found = find_assignment(db, context, id)
            .await?
            .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?;
        if !can_manage_person(db, context, found.person_id).await? {
            return Err(sync_error(StatusCode::FORBIDDEN));
        }
        let (before, current_etag) = representation(db, context, found.id).await?;
        sync_precondition(&current_etag, operation.if_match.as_deref())?;
        match operation.action.as_str() {
            "update" => {
                let body = json!({"person_medication": operation.attributes});
                let attrs = Attributes::parse(&body, false).map_err(sync_error)?;
                let person = person::Entity::find_by_id(found.person_id)
                    .one(db)
                    .await
                    .map_err(database_error)?
                    .ok_or_else(ApiError::not_found)?;
                if let Some(id) = attrs.person_id.as_deref() {
                    let requested = find_person(db, context, id)
                        .await?
                        .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?;
                    if requested.id != found.person_id {
                        return Err(sync_error(StatusCode::UNPROCESSABLE_ENTITY));
                    }
                }
                let medication = if let Some(id) = attrs.medication_id.as_deref() {
                    find_medication(db, context, id)
                        .await?
                        .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?
                } else {
                    medication::Entity::find_by_id(found.medication_id)
                        .one(db)
                        .await
                        .map_err(database_error)?
                        .ok_or_else(ApiError::not_found)?
                };
                if medication.id != found.medication_id
                    && person_medication::Entity::find()
                        .filter(person_medication::Column::PersonId.eq(found.person_id))
                        .filter(person_medication::Column::MedicationId.eq(medication.id))
                        .filter(person_medication::Column::RetiredAt.is_null())
                        .one(db)
                        .await
                        .map_err(database_error)?
                        .is_some()
                {
                    return Err(sync_error(StatusCode::UNPROCESSABLE_ENTITY));
                }
                let selected_option = if let Some(id) = attrs.source_dosage_option_id.as_deref() {
                    Some(
                        find_visible_option(db, context, id)
                            .await?
                            .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?,
                    )
                } else if let Some(id) = found.source_dosage_option_id {
                    dosage::Entity::find_by_id(id)
                        .one(db)
                        .await
                        .map_err(database_error)?
                } else {
                    None
                };
                let (amount, unit, source_option) = resolved_dose(
                    db,
                    context,
                    &attrs,
                    &person,
                    &medication,
                    Some(&found),
                    selected_option.as_ref(),
                )
                .await?
                .ok_or_else(|| sync_error(StatusCode::UNPROCESSABLE_ENTITY))?;
                let kind = attrs
                    .administration_kind
                    .unwrap_or(found.administration_kind);
                let maximum = attrs.max_daily_doses.or(found.max_daily_doses);
                let mut minimum = attrs
                    .min_hours_between_doses
                    .unwrap_or(found.min_hours_between_doses);
                if kind == 0 && maximum == Some(1) && minimum == Some(24) {
                    minimum = None;
                }
                let unchanged = medication.id == found.medication_id
                    && found.dose_amount == Some(amount)
                    && found.dose_unit.as_deref() == Some(unit.as_str())
                    && found.source_dosage_option_id == source_option
                    && found.administration_kind == kind
                    && found.notes == attrs.notes.clone().or(found.notes.clone())
                    && found.max_daily_doses == maximum
                    && found.min_hours_between_doses == minimum
                    && found.dose_cycle == attrs.dose_cycle.or(found.dose_cycle);
                if unchanged {
                    (found, false)
                } else {
                    let mut active = found.clone().into_active_model();
                    active.medication_id = Set(medication.id);
                    active.dose_amount = Set(Some(amount));
                    active.dose_unit = Set(Some(unit));
                    active.source_dosage_option_id = Set(source_option);
                    if let Some(value) = attrs.administration_kind {
                        active.administration_kind = Set(value);
                    }
                    if let Some(value) = attrs.notes {
                        active.notes = Set(Some(value));
                    }
                    if let Some(value) = attrs.max_daily_doses {
                        active.max_daily_doses = Set(Some(value));
                    }
                    if let Some(value) = attrs.dose_cycle {
                        active.dose_cycle = Set(Some(value));
                    }
                    active.min_hours_between_doses = Set(minimum);
                    active.updated_at = Set(Utc::now().naive_utc());
                    let updated = active.update(db).await.map_err(database_error)?;
                    let (after, _) = representation(db, context, updated.id).await?;
                    record_version(
                        db,
                        context,
                        request_id,
                        "PersonMedication",
                        updated.id,
                        "update",
                        Some(before["data"].clone()),
                        Some(after["data"].clone()),
                    )
                    .await?;
                    record_change(
                        db,
                        context,
                        request_id,
                        SyncRecord {
                            record_type: "PersonMedication",
                            record_id: updated.id,
                            portable_id: &updated.portable_id,
                            action: "update",
                            person_portable_id: Some(&person.portable_id),
                        },
                    )
                    .await?;
                    (updated, true)
                }
            }
            "delete" => {
                if !operation.attributes.is_empty() {
                    return Err(sync_error(StatusCode::UNPROCESSABLE_ENTITY));
                }
                let mut active = found.clone().into_active_model();
                let now = Utc::now().naive_utc();
                active.retired_at = Set(Some(now));
                active.active = Set(false);
                active.updated_at = Set(now);
                let retired = active.update(db).await.map_err(database_error)?;
                record_version(
                    db,
                    context,
                    request_id,
                    "PersonMedication",
                    retired.id,
                    "destroy",
                    Some(before["data"].clone()),
                    None,
                )
                .await?;
                let person = person::Entity::find_by_id(retired.person_id)
                    .one(db)
                    .await
                    .map_err(database_error)?
                    .ok_or_else(ApiError::not_found)?;
                record_change(
                    db,
                    context,
                    request_id,
                    SyncRecord {
                        record_type: "PersonMedication",
                        record_id: retired.id,
                        portable_id: &retired.portable_id,
                        action: "update",
                        person_portable_id: Some(&person.portable_id),
                    },
                )
                .await?;
                api_tombstone::ActiveModel {
                    household_id: Set(household_id),
                    household_membership_id: Set(Some(context.membership.id)),
                    account_id: Set(Some(context.account_id)),
                    action: Set("delete".to_owned()),
                    record_type: Set("PersonMedication".to_owned()),
                    record_portable_id: Set(retired.portable_id.clone()),
                    metadata: Set(json!({"record_type": "PersonMedication", "record_id": retired.id,
                        "portable_id": retired.portable_id, "person_portable_id": person.portable_id})),
                    deleted_at: Set(now), created_at: Set(now), updated_at: Set(now),
                    ..Default::default()
                }.insert(db).await.map_err(database_error)?;
                return Ok(SyncResult {
                    record_type: "PersonMedication",
                    record_id: Some(retired.id),
                    record_portable_id: Some(retired.portable_id),
                    etag: None,
                    replayed: None,
                });
            }
            _ => return Err(sync_error(StatusCode::UNPROCESSABLE_ENTITY)),
        }
    };
    let (_, etag) = representation(db, context, record.id).await?;
    Ok(SyncResult {
        record_type: "PersonMedication",
        record_id: Some(record.id),
        record_portable_id: Some(record.portable_id),
        etag: Some(etag),
        replayed: Some(!changed),
    })
}

pub(super) async fn authorize_sync_replay(
    db: &DatabaseTransaction,
    context: &AuthContext,
    operation: &SyncOperation,
    saved: &Value,
) -> Result<(), ApiError> {
    if saved.get("action").and_then(Value::as_str) != Some(operation.action.as_str())
        || saved.get("record_type").and_then(Value::as_str) != Some("PersonMedication")
    {
        return Err(ApiError::forbidden());
    }
    let portable_id = saved
        .get("record_portable_id")
        .and_then(Value::as_str)
        .ok_or_else(ApiError::forbidden)?;
    let record = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(context.membership.household_id))
        .filter(person_medication::Column::PortableId.eq(portable_id))
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::forbidden)?;
    if saved.get("record_id").and_then(Value::as_str) != Some(record.id.to_string().as_str())
        || operation
            .id
            .as_deref()
            .is_some_and(|id| id != record.portable_id && id != record.id.to_string())
        || !can_manage_person(db, context, record.person_id).await?
    {
        return Err(ApiError::forbidden());
    }
    if let Some(id) = operation
        .attributes
        .get("person_id")
        .and_then(Value::as_str)
    {
        if find_person(db, context, id)
            .await?
            .is_none_or(|person| person.id != record.person_id)
        {
            return Err(ApiError::forbidden());
        }
    }
    if let Some(id) = operation
        .attributes
        .get("medication_id")
        .and_then(Value::as_str)
    {
        if find_medication(db, context, id).await?.is_none() {
            return Err(ApiError::forbidden());
        }
    }
    if operation.action == "delete"
        && (record.retired_at.is_none()
            || api_tombstone::Entity::find()
                .filter(api_tombstone::Column::HouseholdId.eq(context.membership.household_id))
                .filter(api_tombstone::Column::RecordType.eq("PersonMedication"))
                .filter(api_tombstone::Column::RecordPortableId.eq(&record.portable_id))
                .one(db)
                .await
                .map_err(database_error)?
                .is_none())
    {
        return Err(ApiError::forbidden());
    }
    Ok(())
}
