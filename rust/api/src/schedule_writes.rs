use crate::audit;
use crate::dosage_options::{parse_decimal, storage_decimal, valid_identifier};
use crate::entities::{dosage, grant, person, schedule};
use crate::medication_management::{
    error_response, finish_with_request_id, record_version, request_context, visible_medication,
};
use crate::mutation_idempotency::{self, Lookup, StoredResponse};
use crate::read_entities::schedule as read_schedule;
use crate::read_resources::serialize_schedules;
use crate::sync_events::{record_change, SyncRecord};
use crate::{database_error, representation_etag, ApiError, AppState, AuthContext};
use axum::extract::{rejection::JsonRejection, Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use chrono::{NaiveDate, Utc};
use sea_orm::prelude::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseTransaction, EntityTrait, QueryFilter, Set,
};
use serde_json::{json, Map, Value};
use uuid::Uuid;

const CONTROLLER: &str = "api/v1/schedules";
const POLICY: &str = "SchedulePolicy";
const UNITS: &[&str] = &[
    "tablet", "capsule", "gummy", "mg", "ml", "g", "mcg", "IU", "spray", "drop", "sachet", "pad",
];
const FIELDS: &[&str] = &[
    "person_id",
    "medication_id",
    "source_dosage_option_id",
    "dose_amount",
    "dose_unit",
    "frequency",
    "start_date",
    "end_date",
    "notes",
    "max_daily_doses",
    "min_hours_between_doses",
    "dose_cycle",
    "schedule_type",
    "schedule_config",
];

enum InputFailure {
    Malformed,
    NotFound,
    Forbidden,
    Invalid(&'static str, &'static str),
    InvalidFields(Value),
}

#[derive(PartialEq)]
struct ScheduleFields {
    person_id: Option<i64>,
    medication_id: Option<i64>,
    source_dosage_option_id: Option<i64>,
    dose_amount: Option<Decimal>,
    dose_unit: Option<String>,
    frequency: Option<String>,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    notes: Option<String>,
    max_daily_doses: Option<i32>,
    min_hours_between_doses: Option<i32>,
    dose_cycle: Option<i32>,
    schedule_type: i32,
    schedule_config: Value,
}

impl ScheduleFields {
    fn new() -> Self {
        Self {
            person_id: None,
            medication_id: None,
            source_dosage_option_id: None,
            dose_amount: None,
            dose_unit: None,
            frequency: None,
            start_date: None,
            end_date: None,
            notes: None,
            max_daily_doses: Some(4),
            min_hours_between_doses: None,
            dose_cycle: None,
            schedule_type: 0,
            schedule_config: json!({}),
        }
    }

    fn from_record(record: &schedule::Model) -> Self {
        Self {
            person_id: Some(record.person_id),
            medication_id: Some(record.medication_id),
            source_dosage_option_id: record.source_dosage_option_id,
            dose_amount: record.dose_amount,
            dose_unit: record.dose_unit.clone(),
            frequency: record.frequency.clone(),
            start_date: record.start_date,
            end_date: record.end_date,
            notes: record.notes.clone(),
            max_daily_doses: record.max_daily_doses,
            min_hours_between_doses: record.min_hours_between_doses,
            dose_cycle: record.dose_cycle,
            schedule_type: record.schedule_type,
            schedule_config: record.schedule_config.clone(),
        }
    }

    fn validate(&self) -> Result<(), InputFailure> {
        let mut errors = Map::new();
        if self.person_id.is_none() {
            errors.insert("person_id".to_owned(), json!(["can't be blank"]));
        }
        if self.medication_id.is_none() {
            errors.insert("medication_id".to_owned(), json!(["can't be blank"]));
        }
        if self.dose_amount.is_none() {
            errors.insert("dose_amount".to_owned(), json!(["can't be blank"]));
        }
        if self.dose_unit.is_none() {
            errors.insert("dose_unit".to_owned(), json!(["can't be blank"]));
        }
        if self.start_date.is_none() {
            errors.insert("start_date".to_owned(), json!(["can't be blank"]));
        }
        if self.end_date.is_none() {
            errors.insert("end_date".to_owned(), json!(["can't be blank"]));
        }
        if let (Some(start), Some(end)) = (self.start_date, self.end_date) {
            if end < start {
                errors.insert(
                    "end_date".to_owned(),
                    json!(["must be after the start date"]),
                );
            }
        }
        if !errors.is_empty() {
            return Err(InputFailure::InvalidFields(Value::Object(errors)));
        }
        Ok(())
    }

    fn assign(self, active: &mut schedule::ActiveModel) {
        active.person_id = Set(self.person_id.unwrap());
        active.medication_id = Set(self.medication_id.unwrap());
        active.source_dosage_option_id = Set(self.source_dosage_option_id);
        active.dose_amount = Set(self.dose_amount);
        active.dose_unit = Set(self.dose_unit);
        active.frequency = Set(self.frequency);
        active.start_date = Set(self.start_date);
        active.end_date = Set(self.end_date);
        active.notes = Set(self.notes);
        active.max_daily_doses = Set(self.max_daily_doses);
        active.min_hours_between_doses = Set(self.min_hours_between_doses);
        active.dose_cycle = Set(self.dose_cycle);
        active.schedule_type = Set(self.schedule_type);
        active.schedule_config = Set(self.schedule_config);
    }
}

fn identifier<'a>(value: &'a Value, field: &'static str) -> Result<&'a str, InputFailure> {
    let id = value
        .as_str()
        .ok_or(InputFailure::Invalid(field, "must be a string"))?;
    if !valid_identifier(id) {
        return Err(InputFailure::Invalid(field, "is invalid"));
    }
    Ok(id)
}

fn attributes(body: &Value) -> Result<&Map<String, Value>, InputFailure> {
    let outer = body.as_object().ok_or(InputFailure::Malformed)?;
    let Some(attributes) = body.get("schedule").and_then(Value::as_object) else {
        return Err(InputFailure::Malformed);
    };
    if outer.len() != 1 {
        return Err(InputFailure::Invalid(
            "schedule",
            "contains an unsupported field",
        ));
    }
    if attributes.is_empty() {
        return Err(InputFailure::Invalid(
            "schedule",
            "must have at least one field",
        ));
    }
    if let Some((field, _)) = attributes
        .iter()
        .find(|(field, _)| !FIELDS.contains(&field.as_str()))
    {
        return Err(InputFailure::Invalid(
            "schedule",
            if field.is_empty() {
                "is invalid"
            } else {
                "contains an unsupported field"
            },
        ));
    }
    Ok(attributes)
}

fn parse_amount(value: &Value) -> Result<Decimal, InputFailure> {
    let amount = parse_decimal(value).ok_or(InputFailure::Invalid(
        "dose_amount",
        "must be a decimal string",
    ))?;
    if amount <= Decimal::ZERO || storage_decimal(amount, 8, 2).is_none() {
        return Err(InputFailure::Invalid(
            "dose_amount",
            "must fit two decimal places",
        ));
    }
    Ok(amount)
}

fn parse_interval(value: &Value) -> Result<Option<i32>, InputFailure> {
    if value.is_null() {
        return Ok(None);
    }
    let decimal = parse_decimal(value).ok_or(InputFailure::Invalid(
        "min_hours_between_doses",
        "must be a decimal string",
    ))?;
    if decimal <= Decimal::ZERO || storage_decimal(decimal, 10, 0).is_none() {
        return Err(InputFailure::Invalid(
            "min_hours_between_doses",
            "must be a whole positive number",
        ));
    }
    decimal
        .normalize()
        .to_string()
        .parse::<i32>()
        .map(Some)
        .map_err(|_| InputFailure::Invalid("min_hours_between_doses", "is out of range"))
}

fn parse_date(value: &Value, field: &'static str) -> Result<NaiveDate, InputFailure> {
    let raw = value
        .as_str()
        .ok_or(InputFailure::Invalid(field, "must be a date"))?;
    if raw.len() != 10 {
        return Err(InputFailure::Invalid(field, "must be a date"));
    }
    NaiveDate::parse_from_str(raw, "%Y-%m-%d")
        .map_err(|_| InputFailure::Invalid(field, "must be a date"))
}

fn valid_time(value: &Value) -> bool {
    let Some(raw) = value.as_str() else {
        return false;
    };
    let bytes = raw.as_bytes();
    bytes.len() == 5
        && bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[2] == b':'
        && bytes[3].is_ascii_digit()
        && bytes[4].is_ascii_digit()
        && raw[0..2].parse::<u8>().is_ok_and(|hour| hour <= 23)
        && raw[3..5].parse::<u8>().is_ok_and(|minute| minute <= 59)
}

fn valid_times(value: &Value) -> bool {
    value
        .as_array()
        .is_some_and(|times| times.iter().all(valid_time))
}

fn valid_config(config: &Value) -> bool {
    let Some(map) = config.as_object() else {
        return false;
    };
    for (key, value) in map {
        let valid = match key.as_str() {
            "times" => valid_times(value),
            "weekdays" => value.as_array().is_some_and(|days| {
                days.iter().all(|day| {
                    day.as_str().is_some_and(|day| {
                        matches!(
                            day.trim().to_ascii_lowercase().as_str(),
                            "sunday"
                                | "sun"
                                | "monday"
                                | "mon"
                                | "tuesday"
                                | "tue"
                                | "wednesday"
                                | "wed"
                                | "thursday"
                                | "thu"
                                | "friday"
                                | "fri"
                                | "saturday"
                                | "sat"
                                | "0"
                                | "1"
                                | "2"
                                | "3"
                                | "4"
                                | "5"
                                | "6"
                                | "7"
                        )
                    })
                })
            }),
            "dates" => value.as_array().is_some_and(|dates| {
                dates
                    .iter()
                    .all(|date| parse_date(date, "schedule_config").is_ok())
            }),
            "as_needed" => value.is_boolean(),
            "taper_steps" => value
                .as_array()
                .is_some_and(|steps| steps.iter().all(valid_taper_step)),
            _ => false,
        };
        if !valid {
            return false;
        }
    }
    true
}

fn valid_taper_step(value: &Value) -> bool {
    let Some(map) = value.as_object() else {
        return false;
    };
    let (Some(start), Some(end)) = (map.get("start_date"), map.get("end_date")) else {
        return false;
    };
    let (Ok(start), Ok(end)) = (
        parse_date(start, "schedule_config"),
        parse_date(end, "schedule_config"),
    ) else {
        return false;
    };
    if end < start {
        return false;
    }
    map.iter().all(|(key, value)| match key.as_str() {
        "start_date" | "end_date" => true,
        "amount" | "dose_amount" => {
            parse_decimal(value).is_some_and(|amount| amount > Decimal::ZERO)
        }
        "unit" | "dose_unit" => value.as_str().is_some_and(|value| !value.is_empty()),
        "max_daily_doses" => value
            .as_i64()
            .is_some_and(|value| value > 0 && value <= i32::MAX as i64),
        "min_hours_between_doses" => {
            parse_decimal(value).is_some_and(|hours| hours > Decimal::ZERO)
        }
        "times" => valid_times(value),
        _ => false,
    })
}

async fn find_person(
    db: &DatabaseTransaction,
    context: &AuthContext,
    id: &str,
) -> Result<Option<person::Model>, ApiError> {
    let mut query = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(context.membership.household_id));
    query = match id.parse::<i64>() {
        Ok(id) => query.filter(person::Column::Id.eq(id)),
        Err(_) => query.filter(person::Column::PortableId.eq(id)),
    };
    query.one(db).await.map_err(database_error)
}

async fn person_access(
    db: &DatabaseTransaction,
    context: &AuthContext,
    person_id: i64,
    manage: bool,
) -> Result<bool, ApiError> {
    let mut query = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(context.membership.household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(context.membership.id))
        .filter(grant::Column::PersonId.eq(person_id))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(
            Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(grant::Column::ExpiresAt.gt(Utc::now().naive_utc())),
        );
    if manage {
        query = query.filter(grant::Column::AccessLevel.eq("manage"));
    } else {
        query = query.filter(grant::Column::AccessLevel.is_in(["view", "record", "manage"]));
    }
    Ok(query.one(db).await.map_err(database_error)?.is_some())
}

async fn find_schedule(
    db: &DatabaseTransaction,
    context: &AuthContext,
    id: &str,
) -> Result<Option<schedule::Model>, ApiError> {
    let mut query = schedule::Entity::find()
        .filter(schedule::Column::HouseholdId.eq(context.membership.household_id))
        .filter(schedule::Column::RetiredAt.is_null());
    query = match id.parse::<i64>() {
        Ok(id) => query.filter(schedule::Column::Id.eq(id)),
        Err(_) => query.filter(schedule::Column::PortableId.eq(id)),
    };
    let found = query.one(db).await.map_err(database_error)?;
    if let Some(found) = found {
        if person_access(db, context, found.person_id, false).await? {
            return Ok(Some(found));
        }
    }
    Ok(None)
}

async fn find_dosage(
    db: &DatabaseTransaction,
    context: &AuthContext,
    id: &str,
) -> Result<Option<dosage::Model>, ApiError> {
    let mut query = dosage::Entity::find()
        .filter(dosage::Column::HouseholdId.eq(context.membership.household_id));
    query = match id.parse::<i64>() {
        Ok(id) => query.filter(dosage::Column::Id.eq(id)),
        Err(_) => query.filter(dosage::Column::PortableId.eq(id)),
    };
    let found = query.one(db).await.map_err(database_error)?;
    if let Some(found) = found {
        if visible_medication(db, context, &found.medication_id.to_string())
            .await?
            .is_some()
        {
            return Ok(Some(found));
        }
    }
    Ok(None)
}

async fn apply_attributes(
    db: &DatabaseTransaction,
    context: &AuthContext,
    attributes: &Map<String, Value>,
    fields: &mut ScheduleFields,
    existing: Option<&schedule::Model>,
) -> Result<Result<(), InputFailure>, ApiError> {
    if let Some(value) = attributes.get("person_id") {
        let id = match identifier(value, "person_id") {
            Ok(id) => id,
            Err(error) => return Ok(Err(error)),
        };
        let Some(person) = find_person(db, context, id).await? else {
            return Ok(Err(InputFailure::NotFound));
        };
        if !person_access(db, context, person.id, false).await? {
            return Ok(Err(InputFailure::NotFound));
        }
        if existing.is_some_and(|existing| existing.person_id != person.id) {
            return Ok(Err(InputFailure::Invalid("person_id", "cannot be changed")));
        }
        fields.person_id = Some(person.id);
    }
    if let Some(value) = attributes.get("medication_id") {
        let id = match identifier(value, "medication_id") {
            Ok(id) => id,
            Err(error) => return Ok(Err(error)),
        };
        let Some(medication) = visible_medication(db, context, id).await? else {
            return Ok(Err(InputFailure::NotFound));
        };
        fields.medication_id = Some(medication.id);
    }
    if let Some(value) = attributes.get("source_dosage_option_id") {
        if value.is_null() {
            return Ok(Err(InputFailure::Invalid(
                "source_dosage_option_id",
                "must be a string",
            )));
        } else {
            let id = match identifier(value, "source_dosage_option_id") {
                Ok(id) => id,
                Err(error) => return Ok(Err(error)),
            };
            let Some(option) = find_dosage(db, context, id).await? else {
                return Ok(Err(InputFailure::NotFound));
            };
            fields.source_dosage_option_id = Some(option.id);
        }
    }
    if let Some(value) = attributes.get("dose_amount") {
        match parse_amount(value) {
            Ok(amount) => fields.dose_amount = Some(amount),
            Err(error) => return Ok(Err(error)),
        }
    }
    if let Some(value) = attributes.get("dose_unit") {
        let Some(unit) = value.as_str() else {
            return Ok(Err(InputFailure::Invalid("dose_unit", "must be a string")));
        };
        if !UNITS.contains(&unit) {
            return Ok(Err(InputFailure::Invalid("dose_unit", "is invalid")));
        }
        fields.dose_unit = Some(unit.to_owned());
    }
    for (key, destination) in [
        ("frequency", &mut fields.frequency),
        ("notes", &mut fields.notes),
    ] {
        if let Some(value) = attributes.get(key) {
            if let Some(value) = value.as_str() {
                *destination = Some(value.to_owned());
            } else {
                return Ok(Err(InputFailure::Invalid(
                    "schedule",
                    "contains an invalid string",
                )));
            }
        }
    }
    if let Some(value) = attributes.get("start_date") {
        match parse_date(value, "start_date") {
            Ok(date) => fields.start_date = Some(date),
            Err(error) => return Ok(Err(error)),
        }
    }
    if let Some(value) = attributes.get("end_date") {
        match parse_date(value, "end_date") {
            Ok(date) => fields.end_date = Some(date),
            Err(error) => return Ok(Err(error)),
        }
    }
    if let Some(value) = attributes.get("max_daily_doses") {
        let Some(amount) = value
            .as_i64()
            .filter(|amount| *amount > 0 && *amount <= i32::MAX as i64)
        else {
            return Ok(Err(InputFailure::Invalid(
                "max_daily_doses",
                "must be positive",
            )));
        };
        fields.max_daily_doses = Some(amount as i32);
    }
    if let Some(value) = attributes.get("min_hours_between_doses") {
        match parse_interval(value) {
            Ok(interval) => fields.min_hours_between_doses = interval,
            Err(error) => return Ok(Err(error)),
        }
    }
    if let Some(value) = attributes.get("dose_cycle") {
        fields.dose_cycle = match value.as_str() {
            Some("daily") => Some(0),
            Some("weekly") => Some(1),
            Some("monthly") => Some(2),
            _ => return Ok(Err(InputFailure::Invalid("dose_cycle", "is invalid"))),
        };
    }
    if let Some(value) = attributes.get("schedule_type") {
        fields.schedule_type = match value.as_str() {
            Some("daily") => 0,
            Some("multiple_daily") => 1,
            Some("weekly") => 2,
            Some("specific_dates") => 3,
            Some("prn") => 4,
            Some("tapering") => 5,
            Some("every_other_day") => 6,
            _ => return Ok(Err(InputFailure::Invalid("schedule_type", "is invalid"))),
        };
    }
    if let Some(value) = attributes.get("schedule_config") {
        if !valid_config(value) {
            return Ok(Err(InputFailure::Invalid("schedule_config", "is invalid")));
        }
        fields.schedule_config = value.clone();
    }
    if let Some(option_id) = fields.source_dosage_option_id {
        let Some(option) = dosage::Entity::find_by_id(option_id)
            .one(db)
            .await
            .map_err(database_error)?
        else {
            return Ok(Err(InputFailure::NotFound));
        };
        if Some(option.medication_id) != fields.medication_id
            || Some(option.amount) != fields.dose_amount
            || fields.dose_unit.as_deref() != Some(option.unit.as_str())
        {
            return Ok(Err(InputFailure::Invalid(
                "source_dosage_option",
                "must match the selected medication and dose",
            )));
        }
    } else if existing.is_none() {
        if let (Some(medication_id), Some(amount), Some(unit)) = (
            fields.medication_id,
            fields.dose_amount,
            fields.dose_unit.as_deref(),
        ) {
            let matches = dosage::Entity::find()
                .filter(dosage::Column::HouseholdId.eq(context.membership.household_id))
                .filter(dosage::Column::MedicationId.eq(medication_id))
                .filter(dosage::Column::Amount.eq(amount))
                .filter(dosage::Column::Unit.eq(unit))
                .all(db)
                .await
                .map_err(database_error)?;
            if matches.len() == 1 {
                fields.source_dosage_option_id = Some(matches[0].id);
            }
        }
    }
    Ok(Ok(()))
}

async fn input_error(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    action: &str,
    failure: InputFailure,
) -> Result<Response, ApiError> {
    match failure {
        InputFailure::Malformed => {
            error_response(
                db,
                context,
                method,
                CONTROLLER,
                POLICY,
                action,
                StatusCode::BAD_REQUEST,
                "bad_request",
                "Invalid request body",
                None,
            )
            .await
        }
        InputFailure::NotFound => {
            error_response(
                db,
                context,
                method,
                CONTROLLER,
                POLICY,
                action,
                StatusCode::NOT_FOUND,
                "not_found",
                "Record not found",
                None,
            )
            .await
        }
        InputFailure::Forbidden => {
            error_response(
                db,
                context,
                method,
                CONTROLLER,
                POLICY,
                action,
                StatusCode::FORBIDDEN,
                "forbidden",
                "You are not authorized to perform this action.",
                None,
            )
            .await
        }
        InputFailure::Invalid(field, message) => {
            error_response(
                db,
                context,
                method,
                CONTROLLER,
                POLICY,
                action,
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_failed",
                "Validation failed",
                Some(json!({field: [message]})),
            )
            .await
        }
        InputFailure::InvalidFields(errors) => {
            error_response(
                db,
                context,
                method,
                CONTROLLER,
                POLICY,
                action,
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_failed",
                "Validation failed",
                Some(errors),
            )
            .await
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn keyed_validation(
    db: DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    method: &str,
    action: &str,
    path: &str,
    request: &Value,
    failure: InputFailure,
) -> Result<Response, ApiError> {
    let errors = match failure {
        InputFailure::Invalid(field, message) => json!({field: [message]}),
        InputFailure::InvalidFields(errors) => errors,
        other => return input_error(db, context, method, action, other).await,
    };
    let response_body = json!({"error": {
        "code": "validation_failed",
        "message": "Validation failed",
        "errors": errors,
    }});
    let request_id = Uuid::new_v4().to_string();
    let mut stored_body = response_body.clone();
    stored_body["error"]["request_id"] = json!(request_id);
    store_key(
        &db,
        context,
        headers,
        method,
        path,
        request,
        StatusCode::UNPROCESSABLE_ENTITY,
        &stored_body,
        &request_id,
        None,
    )
    .await?;
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
        response_body,
        None,
    )
    .await
}

async fn schedule_body(
    db: &DatabaseTransaction,
    context: &AuthContext,
    id: i64,
) -> Result<(Value, String), ApiError> {
    let record = read_schedule::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?;
    let row = serialize_schedules(db, context, vec![record])
        .await?
        .remove(0);
    let body = json!({"data": row});
    let etag = representation_etag(&body);
    Ok((body, etag))
}

fn snapshot(record: &schedule::Model) -> Value {
    json!({
        "household_id": record.household_id,
        "portable_id": record.portable_id,
        "person_id": record.person_id,
        "medication_id": record.medication_id,
        "source_dosage_option_id": record.source_dosage_option_id,
        "dose_amount": record.dose_amount.map(|amount| amount.to_string()),
        "dose_unit": record.dose_unit,
        "frequency": record.frequency,
        "start_date": record.start_date,
        "end_date": record.end_date,
        "notes": record.notes,
        "max_daily_doses": record.max_daily_doses,
        "min_hours_between_doses": record.min_hours_between_doses,
        "dose_cycle": record.dose_cycle,
        "schedule_type": record.schedule_type,
        "schedule_config": record.schedule_config,
    })
}

enum ReplayDecision {
    New,
    Replay(Response),
    Conflict,
}

async fn keyed_replay(
    db: &DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    method: &str,
    action: &str,
    path: &str,
    body: &Value,
) -> Result<ReplayDecision, ApiError> {
    let Some(key) = mutation_idempotency::key(headers) else {
        return Ok(ReplayDecision::New);
    };
    let digest = mutation_idempotency::digest(method, path, body);
    match mutation_idempotency::lookup(db, context, key, method, path, &digest).await? {
        Lookup::New => Ok(ReplayDecision::New),
        Lookup::Replay(saved) => {
            let status = StatusCode::from_u16(saved.response_status as u16)
                .map_err(|_| ApiError::internal())?;
            let replay_id = Uuid::new_v4().to_string();
            audit::record_resource_request_with_id(
                db,
                context,
                &replay_id,
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
                replay_id.parse().map_err(|_| ApiError::internal())?,
            );
            Ok(ReplayDecision::Replay(response))
        }
        Lookup::Conflict => Ok(ReplayDecision::Conflict),
    }
}

#[allow(clippy::too_many_arguments)]
async fn store_key(
    db: &DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    method: &str,
    path: &str,
    request: &Value,
    status: StatusCode,
    body: &Value,
    request_id: &str,
    etag: Option<&str>,
) -> Result<(), ApiError> {
    let Some(key) = mutation_idempotency::key(headers) else {
        return Ok(());
    };
    let digest = mutation_idempotency::digest(method, path, request);
    mutation_idempotency::store(
        db,
        context,
        StoredResponse {
            key,
            method,
            path,
            digest: &digest,
            status,
            body: body.clone(),
            request_id,
            etag,
        },
    )
    .await
}

pub(super) async fn create(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    let (db, _) = request_context(&state, &headers, household_id).await?;
    let (_, context) = mutation_idempotency::lock_household_and_reauthenticate(
        &state,
        &db,
        &headers,
        household_id,
    )
    .await?;
    let Json(body) = match payload {
        Ok(payload) => payload,
        Err(_) => {
            return input_error(db, &context, "POST", "create", InputFailure::Malformed).await
        }
    };
    let attributes = match attributes(&body) {
        Ok(attributes) => attributes,
        Err(failure) => return input_error(db, &context, "POST", "create", failure).await,
    };
    let Some(raw_person_id) = attributes.get("person_id") else {
        return input_error(
            db,
            &context,
            "POST",
            "create",
            InputFailure::Invalid("person_id", "can't be blank"),
        )
        .await;
    };
    let person_id = match identifier(raw_person_id, "person_id") {
        Ok(id) => id,
        Err(failure) => return input_error(db, &context, "POST", "create", failure).await,
    };
    let Some(person) = find_person(&db, &context, person_id).await? else {
        return input_error(db, &context, "POST", "create", InputFailure::NotFound).await;
    };
    if !person_access(&db, &context, person.id, false).await? {
        return input_error(db, &context, "POST", "create", InputFailure::NotFound).await;
    }
    if !person_access(&db, &context, person.id, true).await? {
        return input_error(db, &context, "POST", "create", InputFailure::Forbidden).await;
    }
    let path = format!("/api/v1/households/{household_id}/schedules");
    match keyed_replay(&db, &context, &headers, "POST", "create", &path, &body).await? {
        ReplayDecision::New => {}
        ReplayDecision::Replay(response) => {
            db.commit().await.map_err(database_error)?;
            return Ok(response);
        }
        ReplayDecision::Conflict => {
            return error_response(
                db,
                &context,
                "POST",
                CONTROLLER,
                POLICY,
                "create",
                StatusCode::CONFLICT,
                "idempotency_key_reused",
                "Idempotency key has already been used for a different request",
                None,
            )
            .await;
        }
    }
    let mut fields = ScheduleFields::new();
    if let Err(failure) = apply_attributes(&db, &context, attributes, &mut fields, None).await? {
        return keyed_validation(
            db, &context, &headers, "POST", "create", &path, &body, failure,
        )
        .await;
    }
    if let Err(failure) = fields.validate() {
        return keyed_validation(
            db, &context, &headers, "POST", "create", &path, &body, failure,
        )
        .await;
    }
    let now = Utc::now().naive_utc();
    let mut active = schedule::ActiveModel {
        household_id: Set(household_id),
        portable_id: Set(Uuid::new_v4().to_string()),
        active: Set(true),
        retired_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    fields.assign(&mut active);
    let created = active.insert(&db).await.map_err(database_error)?;
    let request_id = Uuid::new_v4().to_string();
    record_version(
        &db,
        &context,
        &request_id,
        "Schedule",
        created.id,
        "create",
        None,
        Some(snapshot(&created)),
    )
    .await?;
    let person = person::Entity::find_by_id(created.person_id)
        .one(&db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?;
    record_change(
        &db,
        &context,
        &request_id,
        SyncRecord {
            record_type: "Schedule",
            record_id: created.id,
            portable_id: &created.portable_id,
            action: "create",
            person_portable_id: Some(&person.portable_id),
        },
    )
    .await?;
    let (response_body, etag) = schedule_body(&db, &context, created.id).await?;
    store_key(
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
        CONTROLLER,
        POLICY,
        "create",
        StatusCode::CREATED,
        true,
        response_body,
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
    let (db, _) = request_context(&state, &headers, household_id).await?;
    let (_, context) = mutation_idempotency::lock_household_and_reauthenticate(
        &state,
        &db,
        &headers,
        household_id,
    )
    .await?;
    let Some(found) = find_schedule(&db, &context, &id).await? else {
        return input_error(db, &context, method, "update", InputFailure::NotFound).await;
    };
    if !person_access(&db, &context, found.person_id, true).await? {
        return input_error(db, &context, method, "update", InputFailure::Forbidden).await;
    }
    let Json(body) = match payload {
        Ok(payload) => payload,
        Err(_) => {
            return input_error(db, &context, method, "update", InputFailure::Malformed).await
        }
    };
    let path = format!("/api/v1/households/{household_id}/schedules/{id}");
    match keyed_replay(&db, &context, &headers, method, "update", &path, &body).await? {
        ReplayDecision::New => {}
        ReplayDecision::Replay(response) => {
            db.commit().await.map_err(database_error)?;
            return Ok(response);
        }
        ReplayDecision::Conflict => {
            return error_response(
                db,
                &context,
                method,
                CONTROLLER,
                POLICY,
                "update",
                StatusCode::CONFLICT,
                "idempotency_key_reused",
                "Idempotency key has already been used for a different request",
                None,
            )
            .await;
        }
    }
    let (_, current_etag) = schedule_body(&db, &context, found.id).await?;
    if headers
        .get(header::IF_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| !value.is_empty() && value != current_etag)
    {
        return error_response(
            db,
            &context,
            method,
            CONTROLLER,
            POLICY,
            "update",
            StatusCode::CONFLICT,
            "conflict",
            "Record has changed since it was last read",
            None,
        )
        .await;
    }
    let attributes = match attributes(&body) {
        Ok(attributes) => attributes,
        Err(failure) => {
            return keyed_validation(
                db, &context, &headers, method, "update", &path, &body, failure,
            )
            .await
        }
    };
    let mut fields = ScheduleFields::from_record(&found);
    if let Err(failure) =
        apply_attributes(&db, &context, attributes, &mut fields, Some(&found)).await?
    {
        return keyed_validation(
            db, &context, &headers, method, "update", &path, &body, failure,
        )
        .await;
    }
    if let Err(failure) = fields.validate() {
        return keyed_validation(
            db, &context, &headers, method, "update", &path, &body, failure,
        )
        .await;
    }
    if fields == ScheduleFields::from_record(&found) {
        let (response_body, etag) = schedule_body(&db, &context, found.id).await?;
        let request_id = Uuid::new_v4().to_string();
        store_key(
            &db,
            &context,
            &headers,
            method,
            &path,
            &body,
            StatusCode::OK,
            &response_body,
            &request_id,
            Some(&etag),
        )
        .await?;
        return finish_with_request_id(
            db,
            &context,
            &request_id,
            method,
            CONTROLLER,
            POLICY,
            "update",
            StatusCode::OK,
            true,
            response_body,
            Some(&etag),
        )
        .await;
    }
    let before = snapshot(&found);
    let mut active: schedule::ActiveModel = found.into();
    fields.assign(&mut active);
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active.update(&db).await.map_err(database_error)?;
    let request_id = Uuid::new_v4().to_string();
    record_version(
        &db,
        &context,
        &request_id,
        "Schedule",
        updated.id,
        "update",
        Some(before),
        Some(snapshot(&updated)),
    )
    .await?;
    let person = person::Entity::find_by_id(updated.person_id)
        .one(&db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?;
    record_change(
        &db,
        &context,
        &request_id,
        SyncRecord {
            record_type: "Schedule",
            record_id: updated.id,
            portable_id: &updated.portable_id,
            action: "update",
            person_portable_id: Some(&person.portable_id),
        },
    )
    .await?;
    let (response_body, etag) = schedule_body(&db, &context, updated.id).await?;
    store_key(
        &db,
        &context,
        &headers,
        method,
        &path,
        &body,
        StatusCode::OK,
        &response_body,
        &request_id,
        Some(&etag),
    )
    .await?;
    finish_with_request_id(
        db,
        &context,
        &request_id,
        method,
        CONTROLLER,
        POLICY,
        "update",
        StatusCode::OK,
        true,
        response_body,
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
