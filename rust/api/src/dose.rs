use crate::entities::{
    dosage, grant, location, medication, medication_take, person_medication, schedule,
    security_audit_event,
};
use crate::{authenticate, decimal_string, scope, ApiError, AppState, AuthContext, CredentialKind};
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc};
use sea_orm::prelude::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DatabaseTransaction, DbBackend,
    EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, QueryTrait, Set, Statement,
    TransactionTrait,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct Pagination {
    page: Option<i64>,
    per_page: Option<i64>,
    updated_since: Option<String>,
}

struct Source {
    kind: &'static str,
    id: i64,
    active: bool,
    retired: bool,
    person_id: i64,
    medication_id: i64,
    dose_amount: Option<Decimal>,
    dose_unit: Option<String>,
    max_daily_doses: Option<i32>,
    min_hours_between_doses: Option<Decimal>,
    dose_cycle: Option<i32>,
    source_dosage_option_id: Option<i64>,
    schedule_type: Option<i32>,
    schedule_config: Option<Value>,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
}

fn error(status: StatusCode, message: &'static str) -> ApiError {
    ApiError {
        status,
        code: if status == StatusCode::CONFLICT {
            "conflict"
        } else {
            "unprocessable_content"
        },
        message,
        preserve_activity: false,
    }
}

fn database_error(error: sea_orm::DbErr) -> ApiError {
    let class = match error.sql_err() {
        Some(sea_orm::SqlErr::UniqueConstraintViolation(_)) => "unique_constraint",
        Some(sea_orm::SqlErr::ForeignKeyConstraintViolation(_)) => "foreign_key",
        Some(_) => "sql_other",
        None => match error {
            sea_orm::DbErr::RecordNotInserted => "not_inserted",
            sea_orm::DbErr::RecordNotFound(_) => "not_found",
            sea_orm::DbErr::Query(_) => "query",
            sea_orm::DbErr::Exec(_) => "execution",
            _ => "other",
        },
    };
    eprintln!("dose database operation failed: {class}");
    ApiError::internal()
}

fn source_from_assignment(value: person_medication::Model) -> Source {
    Source {
        kind: "person_medication",
        id: value.id,
        active: value.active,
        retired: value.retired_at.is_some(),
        person_id: value.person_id,
        medication_id: value.medication_id,
        dose_amount: value.dose_amount,
        dose_unit: value.dose_unit,
        max_daily_doses: value.max_daily_doses,
        min_hours_between_doses: value.min_hours_between_doses.map(Decimal::from),
        dose_cycle: value.dose_cycle,
        source_dosage_option_id: value.source_dosage_option_id,
        schedule_type: None,
        schedule_config: None,
        start_date: None,
        end_date: None,
    }
}

fn source_from_schedule(value: schedule::Model) -> Source {
    Source {
        kind: "schedule",
        id: value.id,
        active: value.active,
        retired: value.retired_at.is_some(),
        person_id: value.person_id,
        medication_id: value.medication_id,
        dose_amount: value.dose_amount,
        dose_unit: value.dose_unit,
        max_daily_doses: value.max_daily_doses,
        min_hours_between_doses: value.min_hours_between_doses,
        dose_cycle: value.dose_cycle,
        source_dosage_option_id: value.source_dosage_option_id,
        schedule_type: Some(value.schedule_type),
        schedule_config: Some(value.schedule_config),
        start_date: value.start_date,
        end_date: value.end_date,
    }
}

fn app_zone() -> chrono_tz::Tz {
    std::env::var("TZ")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(chrono_tz::UTC)
}

fn local_date(time: NaiveDateTime) -> NaiveDate {
    local_date_in_zone(time, app_zone())
}

fn local_date_in_zone(time: NaiveDateTime, zone: chrono_tz::Tz) -> NaiveDate {
    Utc.from_utc_datetime(&time)
        .with_timezone(&zone)
        .date_naive()
}

fn config_value<'a>(config: &'a Value, names: &[&str]) -> Option<&'a Value> {
    names.iter().find_map(|name| config.get(*name))
}

fn config_decimal(config: &Value, names: &[&str]) -> Option<Decimal> {
    config_value(config, names)
        .and_then(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .or_else(|| value.as_f64().map(|value| value.to_string()))
        })
        .and_then(|value| Decimal::from_str(&value).ok())
}

fn effective_source(source: &mut Source, date: NaiveDate) {
    let Some(config) = source.schedule_config.as_ref() else {
        return;
    };
    let effective = if source.schedule_type == Some(5) {
        config
            .get("taper_steps")
            .and_then(Value::as_array)
            .and_then(|steps| {
                steps.iter().find(|step| {
                    let start = step
                        .get("start_date")
                        .and_then(Value::as_str)
                        .and_then(|v| NaiveDate::parse_from_str(v, "%Y-%m-%d").ok());
                    let end = step
                        .get("end_date")
                        .and_then(Value::as_str)
                        .and_then(|v| NaiveDate::parse_from_str(v, "%Y-%m-%d").ok());
                    start
                        .zip(end)
                        .is_some_and(|(start, end)| date >= start && date <= end)
                })
            })
            .unwrap_or(config)
    } else {
        config
    };
    source.dose_amount =
        config_decimal(effective, &["amount", "dose_amount"]).or(source.dose_amount);
    source.dose_unit = config_value(effective, &["unit", "dose_unit"])
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| source.dose_unit.clone());
    source.max_daily_doses = config_decimal(effective, &["max_daily_doses", "max_doses", "max"])
        .and_then(|value| value.to_string().parse::<i32>().ok())
        .or(source.max_daily_doses);
    source.min_hours_between_doses = config_decimal(
        effective,
        &["min_hours_between_doses", "min_hours", "minimum_hours"],
    )
    .or(source.min_hours_between_doses);
    if source.schedule_type == Some(0)
        && effective
            .get("times")
            .and_then(Value::as_array)
            .is_some_and(|times| {
                times
                    .iter()
                    .filter(|value| !value.is_null() && value.as_str() != Some(""))
                    .count()
                    == 1
            })
    {
        source.min_hours_between_doses = None;
    }
}

async fn source(
    db: &DatabaseTransaction,
    context: &AuthContext,
    household_id: i64,
    kind: &str,
    id: &str,
) -> Result<Source, ApiError> {
    match kind {
        "person_medication" => {
            let mut query = person_medication::Entity::find()
                .filter(person_medication::Column::HouseholdId.eq(household_id));
            query = match id.parse::<i64>() {
                Ok(id) => query.filter(person_medication::Column::Id.eq(id)),
                Err(_) => query.filter(person_medication::Column::PortableId.eq(id)),
            };
            let value = query
                .one(db)
                .await
                .map_err(database_error)?
                .ok_or_else(ApiError::not_found)?;
            if !allowed_person(db, context, value.person_id, false).await? {
                return Err(ApiError::not_found());
            }
            Ok(source_from_assignment(value))
        }
        "schedule" => {
            let mut query =
                schedule::Entity::find().filter(schedule::Column::HouseholdId.eq(household_id));
            query = match id.parse::<i64>() {
                Ok(id) => query.filter(schedule::Column::Id.eq(id)),
                Err(_) => query.filter(schedule::Column::PortableId.eq(id)),
            };
            let value = query
                .one(db)
                .await
                .map_err(database_error)?
                .ok_or_else(ApiError::not_found)?;
            if !allowed_person(db, context, value.person_id, false).await? {
                return Err(ApiError::not_found());
            }
            Ok(source_from_schedule(value))
        }
        _ => Err(ApiError::not_found()),
    }
}

async fn allowed_person(
    db: &DatabaseTransaction,
    context: &AuthContext,
    person_id: i64,
    write: bool,
) -> Result<bool, ApiError> {
    let levels = if write {
        vec!["record", "manage"]
    } else {
        vec!["view", "record", "manage"]
    };
    Ok(grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(context.membership.household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(context.membership.id))
        .filter(grant::Column::PersonId.eq(person_id))
        .filter(grant::Column::AccessLevel.is_in(levels))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(
            Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(grant::Column::ExpiresAt.gt(Utc::now().naive_utc())),
        )
        .one(db)
        .await
        .map_err(database_error)?
        .is_some())
}

async fn audit(
    db: &DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    action: &str,
    method: &str,
    status: StatusCode,
    authorized: bool,
) -> Result<(), ApiError> {
    let (authentication_method, session_reference) = match context.credential_kind {
        CredentialKind::ApiSession => (
            "api_session",
            format!("api_session:{}", context.credential_reference),
        ),
        CredentialKind::OauthGrant => (
            "oauth",
            format!("oauth_grant:{}", context.credential_reference),
        ),
        CredentialKind::BrowserSession => (
            "browser_session",
            format!("browser_session:{}", context.credential_reference),
        ),
    };
    let mut audit_context = json!({
        "actor_account_id": context.account_id,
        "actor_user_id": context.user_id,
        "actor_membership_id": context.membership.id,
        "active_role": context.membership.role,
        "permissions_version": context.membership.permissions_version,
        "household_id": context.membership.household_id,
        "authentication_method": authentication_method,
        "session_reference": session_reference,
        "request_id": request_id
    });
    if authorized {
        audit_context["policy_class"] = json!("MedicationTakePolicy");
        audit_context["policy_query"] = json!(if action == "create" {
            "create?"
        } else {
            "index?"
        });
    }
    let now = Utc::now().naive_utc();
    security_audit_event::ActiveModel {
        household_id: Set(context.membership.household_id),
        actor_account_id: Set(Some(context.account_id)),
        actor_membership_id: Set(Some(context.membership.id)),
        event_type: Set("api.request".to_owned()),
        request_id: Set(Some(request_id.to_owned())),
        metadata: Set(json!({"http_method": method, "controller": "api/v1/medication_takes", "action": action, "outcome": if status.is_success() { "success" } else { "failure" }, "status": status.as_u16()})),
        audit_context: Set(audit_context),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }.insert(db).await.map_err(database_error)?;
    Ok(())
}

fn request_error_response(error: ApiError, request_id: &str) -> Response {
    let mut response = (error.status, Json(json!({"error": {"code": error.code, "message": error.message, "request_id": request_id}}))).into_response();
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(request_id).unwrap_or_else(|_| HeaderValue::from_static("invalid")),
    );
    response
}

fn success_response(
    status: StatusCode,
    body: Value,
    request_id: &str,
    etag: Option<String>,
) -> Response {
    let mut response = (status, Json(body)).into_response();
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(request_id).unwrap_or_else(|_| HeaderValue::from_static("invalid")),
    );
    if let Some(etag) = etag {
        response.headers_mut().insert(
            header::ETAG,
            HeaderValue::from_str(&etag).unwrap_or_else(|_| HeaderValue::from_static("invalid")),
        );
    }
    response
}

fn take_etag(take: &medication_take::Model) -> String {
    let input = format!(
        "MedicationTake:{}:{}",
        take.id,
        take.updated_at.and_utc().timestamp_micros()
    );
    format!("\"{:x}\"", Sha256::digest(input.as_bytes()))
}

async fn serialize(
    db: &DatabaseTransaction,
    takes: &[medication_take::Model],
) -> Result<Vec<Value>, ApiError> {
    let schedule_ids: Vec<i64> = takes.iter().filter_map(|take| take.schedule_id).collect();
    let assignment_ids: Vec<i64> = takes
        .iter()
        .filter_map(|take| take.person_medication_id)
        .collect();
    let inventory_ids: Vec<i64> = takes
        .iter()
        .filter_map(|take| take.taken_from_medication_id)
        .collect();
    let location_ids: Vec<i64> = takes
        .iter()
        .filter_map(|take| take.taken_from_location_id)
        .collect();
    let schedules: HashMap<_, _> = if schedule_ids.is_empty() {
        HashMap::new()
    } else {
        schedule::Entity::find()
            .filter(schedule::Column::Id.is_in(schedule_ids))
            .all(db)
            .await
            .map_err(database_error)?
            .into_iter()
            .map(|value| (value.id, value))
            .collect()
    };
    let assignments: HashMap<_, _> = if assignment_ids.is_empty() {
        HashMap::new()
    } else {
        person_medication::Entity::find()
            .filter(person_medication::Column::Id.is_in(assignment_ids))
            .all(db)
            .await
            .map_err(database_error)?
            .into_iter()
            .map(|value| (value.id, value))
            .collect()
    };
    let inventory: HashMap<_, _> = if inventory_ids.is_empty() {
        HashMap::new()
    } else {
        medication::Entity::find()
            .filter(medication::Column::Id.is_in(inventory_ids))
            .all(db)
            .await
            .map_err(database_error)?
            .into_iter()
            .map(|value| (value.id, value.portable_id))
            .collect()
    };
    let locations: HashMap<_, _> = if location_ids.is_empty() {
        HashMap::new()
    } else {
        location::Entity::find()
            .filter(location::Column::Id.is_in(location_ids))
            .all(db)
            .await
            .map_err(database_error)?
            .into_iter()
            .map(|value| (value.id, value.portable_id))
            .collect()
    };
    let med_ids: Vec<i64> = schedules
        .values()
        .map(|v| v.medication_id)
        .chain(assignments.values().map(|v| v.medication_id))
        .collect();
    let meds: HashMap<_, _> = if med_ids.is_empty() {
        HashMap::new()
    } else {
        medication::Entity::find()
            .filter(medication::Column::Id.is_in(med_ids))
            .all(db)
            .await
            .map_err(database_error)?
            .into_iter()
            .map(|value| (value.id, value.portable_id))
            .collect()
    };
    let person_ids: Vec<i64> = schedules
        .values()
        .map(|v| v.person_id)
        .chain(assignments.values().map(|v| v.person_id))
        .collect();
    let people: HashMap<_, _> = if person_ids.is_empty() {
        HashMap::new()
    } else {
        crate::entities::person::Entity::find()
            .filter(crate::entities::person::Column::Id.is_in(person_ids))
            .all(db)
            .await
            .map_err(database_error)?
            .into_iter()
            .map(|value| (value.id, value))
            .collect()
    };
    Ok(takes.iter().map(|take| {
        let schedule = take.schedule_id.and_then(|id| schedules.get(&id));
        let assignment = take.person_medication_id.and_then(|id| assignments.get(&id));
        let person_id = schedule.map(|v| v.person_id).or_else(|| assignment.map(|v| v.person_id));
        let medication_id = schedule.map(|v| v.medication_id).or_else(|| assignment.map(|v| v.medication_id));
        json!({
            "id": take.id, "portable_id": take.portable_id, "client_uuid": take.client_uuid,
            "schedule_id": take.schedule_id, "schedule_portable_id": schedule.map(|v| &v.portable_id),
            "person_medication_id": take.person_medication_id, "person_medication_portable_id": assignment.map(|v| &v.portable_id),
            "taken_from_medication_id": take.taken_from_medication_id, "taken_from_medication_portable_id": take.taken_from_medication_id.and_then(|id| inventory.get(&id)),
            "taken_from_location_id": take.taken_from_location_id, "taken_from_location_portable_id": take.taken_from_location_id.and_then(|id| locations.get(&id)),
            "dose_amount": take.dose_amount.map(|amount| decimal_string(amount.to_string())), "dose_unit": take.dose_unit,
            "taken_at": take.taken_at.map(|value| value.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
            "updated_at": take.updated_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            "person_id": person_id, "person_portable_id": person_id.and_then(|id| people.get(&id)).map(|v| &v.portable_id),
            "medication_id": medication_id, "medication_portable_id": medication_id.and_then(|id| meds.get(&id))
        })
    }).collect())
}

pub async fn index(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    Query(pagination): Query<Pagination>,
    headers: HeaderMap,
) -> Response {
    let request_id = Uuid::new_v4().to_string();
    let db = match state.db.begin().await {
        Ok(db) => db,
        Err(error) => return request_error_response(database_error(error), &request_id),
    };
    let context = match authenticate(&state, &db, &headers, household_id).await {
        Ok(context) => context,
        Err(error) => {
            if error.preserve_activity {
                let _ = db.commit().await;
            }
            return request_error_response(error, &request_id);
        }
    };
    let result = index_in_transaction(&db, &context, household_id, pagination).await;
    match result {
        Ok(body) => {
            if let Err(error) = audit(
                &db,
                &context,
                &request_id,
                "index",
                "GET",
                StatusCode::OK,
                true,
            )
            .await
            {
                return request_error_response(error, &request_id);
            }
            if let Err(error) = db.commit().await {
                return request_error_response(database_error(error), &request_id);
            }
            success_response(StatusCode::OK, body, &request_id, None)
        }
        Err(error) => {
            let _ = audit(
                &db,
                &context,
                &request_id,
                "index",
                "GET",
                error.status,
                false,
            )
            .await;
            let _ = db.commit().await;
            request_error_response(error, &request_id)
        }
    }
}

async fn index_in_transaction(
    db: &DatabaseTransaction,
    context: &AuthContext,
    household_id: i64,
    pagination: Pagination,
) -> Result<Value, ApiError> {
    let page = pagination.page.unwrap_or(1).max(1);
    let per_page = pagination.per_page.unwrap_or(20).clamp(1, 100);
    let updated_since = pagination
        .updated_since
        .filter(|v| !v.is_empty())
        .map(|v| {
            DateTime::parse_from_rfc3339(&v)
                .map(|v| v.naive_utc())
                .map_err(|_| ApiError::invalid_filter())
        })
        .transpose()?;
    let mut query = medication_take::Entity::find()
        .filter(medication_take::Column::HouseholdId.eq(household_id));
    if context.membership.role != "owner" && context.membership.role != "administrator" {
        let people = grant::Entity::find()
            .select_only()
            .column(grant::Column::PersonId)
            .filter(grant::Column::HouseholdId.eq(household_id))
            .filter(grant::Column::HouseholdMembershipId.eq(context.membership.id))
            .filter(grant::Column::AccessLevel.is_in(["view", "record", "manage"]))
            .filter(grant::Column::RevokedAt.is_null())
            .filter(
                Condition::any()
                    .add(grant::Column::ExpiresAt.is_null())
                    .add(grant::Column::ExpiresAt.gt(Utc::now().naive_utc())),
            )
            .into_query();
        let schedules = schedule::Entity::find()
            .select_only()
            .column(schedule::Column::Id)
            .filter(schedule::Column::HouseholdId.eq(household_id))
            .filter(schedule::Column::PersonId.in_subquery(people.clone()))
            .into_query();
        let assignments = person_medication::Entity::find()
            .select_only()
            .column(person_medication::Column::Id)
            .filter(person_medication::Column::HouseholdId.eq(household_id))
            .filter(person_medication::Column::PersonId.in_subquery(people))
            .into_query();
        query = query.filter(
            Condition::any()
                .add(medication_take::Column::ScheduleId.in_subquery(schedules))
                .add(medication_take::Column::PersonMedicationId.in_subquery(assignments)),
        );
    }
    if let Some(updated_since) = updated_since {
        query = query.filter(medication_take::Column::UpdatedAt.gte(updated_since));
    }
    let total = query.clone().count(db).await.map_err(database_error)?;
    let rows = query
        .order_by_asc(medication_take::Column::Id)
        .limit(per_page as u64)
        .offset(((page - 1) * per_page) as u64)
        .all(db)
        .await
        .map_err(database_error)?;
    let data = serialize(db, &rows).await?;
    Ok(json!({"data": data, "meta": {"page": page, "per_page": per_page, "total_count": total}}))
}

struct ProposedTake {
    source: Source,
    taken_at: NaiveDateTime,
    amount: Decimal,
    unit: String,
    selected: medication::Model,
}

fn valid_numeric_10_2(amount: Decimal) -> bool {
    amount > Decimal::ZERO && amount.scale() <= 2 && amount < Decimal::from(100_000_000)
}

fn decimal_from_json(value: Option<&Value>) -> Result<Option<Decimal>, ApiError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let Some(raw) = value.as_str() else {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "dose_amount must be a string",
        ));
    };
    let amount = Decimal::from_str(raw)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "Invalid dose configured"))?;
    if !valid_numeric_10_2(amount) {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Invalid dose configured",
        ));
    }
    Ok(Some(amount))
}

fn parse_input_time(value: Option<&Value>) -> Result<NaiveDateTime, ApiError> {
    let raw = value
        .and_then(Value::as_str)
        .ok_or_else(|| error(StatusCode::UNPROCESSABLE_ENTITY, "taken_at is invalid"))?;
    DateTime::parse_from_rfc3339(raw)
        .map(|value| value.naive_utc())
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "taken_at is invalid"))
}

fn string_field<'a>(value: &'a Value, field: &str) -> Result<&'a str, ApiError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(ApiError::not_found)
}

async fn replay_matches(
    db: &DatabaseTransaction,
    context: &AuthContext,
    household_id: i64,
    take: &medication_take::Model,
    attributes: &Value,
) -> Result<bool, ApiError> {
    let stored_source = if let Some(id) = take.schedule_id {
        schedule::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(database_error)?
            .filter(|value| value.household_id == household_id)
            .map(|value| ("schedule", value.id, value.portable_id, value.person_id))
    } else if let Some(id) = take.person_medication_id {
        person_medication::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(database_error)?
            .filter(|value| value.household_id == household_id)
            .map(|value| {
                (
                    "person_medication",
                    value.id,
                    value.portable_id,
                    value.person_id,
                )
            })
    } else {
        None
    };
    let (stored_kind, stored_id, stored_portable_id, stored_person) =
        stored_source.ok_or_else(ApiError::not_found)?;
    if !allowed_person(db, context, stored_person, true).await? {
        return Err(ApiError::forbidden());
    }
    let kind = attributes.get("source_type").and_then(Value::as_str);
    let source_id = attributes.get("source_id").and_then(Value::as_str);
    let source_matches = kind == Some(stored_kind)
        && source_id.is_some_and(|value| {
            value == stored_portable_id || value.parse::<i64>().ok() == Some(stored_id)
        });
    let time_matches = parse_input_time(attributes.get("taken_at")).ok() == take.taken_at;
    let amount_matches = match attributes.get("dose_amount") {
        None => true,
        Some(value) => decimal_from_json(Some(value)).ok().flatten() == take.dose_amount,
    };
    let unit_matches = attributes
        .get("dose_unit")
        .is_none_or(|value| value.is_null() || value.as_str() == take.dose_unit.as_deref());
    let stock_matches = attributes
        .get("taken_from_medication_id")
        .is_none_or(|value| value.as_i64() == take.taken_from_medication_id);
    Ok(source_matches && time_matches && amount_matches && unit_matches && stock_matches)
}

async fn lock_row(db: &DatabaseTransaction, table: &str, id: i64) -> Result<(), ApiError> {
    let sql = match table {
        "households" => "SELECT id FROM households WHERE id = $1 FOR UPDATE",
        "medications" => "SELECT id FROM medications WHERE id = $1 FOR UPDATE",
        "dosages" => "SELECT id FROM dosages WHERE id = $1 FOR UPDATE",
        _ => return Err(ApiError::internal()),
    };
    db.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        [id.into()],
    ))
    .await
    .map_err(database_error)?
    .ok_or_else(ApiError::not_found)?;
    Ok(())
}

async fn lock_client_uuid(db: &DatabaseTransaction, client_uuid: &str) -> Result<(), ApiError> {
    let digest = Sha256::digest(client_uuid.as_bytes());
    let lock_id = i64::from_be_bytes(digest[..8].try_into().map_err(|_| ApiError::internal())?);
    db.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT pg_advisory_xact_lock($1)",
        [lock_id.into()],
    ))
    .await
    .map_err(database_error)?;
    Ok(())
}

async fn prepare(
    db: &DatabaseTransaction,
    context: &AuthContext,
    household_id: i64,
    attributes: &Value,
) -> Result<ProposedTake, ApiError> {
    let kind = string_field(attributes, "source_type")?;
    let source_id = string_field(attributes, "source_id")?;
    let mut source = source(db, context, household_id, kind, source_id).await?;
    if source.retired {
        return Err(ApiError::not_found());
    }
    if !allowed_person(db, context, source.person_id, true).await? {
        return Err(ApiError::forbidden());
    }
    let taken_at = parse_input_time(attributes.get("taken_at"))?;
    if taken_at > Utc::now().naive_utc() + Duration::hours(1) {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Cannot record a dose more than one hour in the future",
        ));
    }
    if !source.active {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Cannot take medication: paused",
        ));
    }
    let effective_date = local_date(taken_at);
    if !applies_on(&source, effective_date) {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Cannot take medication: schedule does not apply on this date",
        ));
    }
    effective_source(&mut source, effective_date);
    let amount = decimal_from_json(attributes.get("dose_amount"))?
        .or(source.dose_amount)
        .ok_or_else(|| error(StatusCode::UNPROCESSABLE_ENTITY, "Invalid dose configured"))?;
    if !valid_numeric_10_2(amount) {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Invalid dose configured",
        ));
    }
    let unit = source
        .dose_unit
        .clone()
        .ok_or_else(|| error(StatusCode::UNPROCESSABLE_ENTITY, "Invalid dose configured"))?;
    let original = medication::Entity::find_by_id(source.medication_id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?;
    let candidates = scope(household_id, &context.membership)
        .all(db)
        .await
        .map_err(database_error)?;
    let candidate_ids: HashSet<i64> =
        if context.membership.role == "owner" || context.membership.role == "administrator" {
            candidates.iter().map(|value| value.id).collect()
        } else {
            let schedule_ids = schedule::Entity::find()
                .filter(schedule::Column::HouseholdId.eq(household_id))
                .filter(schedule::Column::PersonId.eq(source.person_id))
                .all(db)
                .await
                .map_err(database_error)?
                .into_iter()
                .map(|value| value.medication_id);
            let assignment_ids = person_medication::Entity::find()
                .filter(person_medication::Column::HouseholdId.eq(household_id))
                .filter(person_medication::Column::PersonId.eq(source.person_id))
                .all(db)
                .await
                .map_err(database_error)?
                .into_iter()
                .map(|value| value.medication_id);
            schedule_ids
                .chain(assignment_ids)
                .chain(std::iter::once(source.medication_id))
                .collect()
        };
    let matching: Vec<_> = candidates
        .into_iter()
        .filter(|value| {
            candidate_ids.contains(&value.id)
                && value.name == original.name
                && value.dose_amount == original.dose_amount
                && value.dose_unit == original.dose_unit
                && value
                    .current_supply
                    .is_none_or(|supply| supply > Decimal::ZERO)
        })
        .collect();
    let selected_id = attributes
        .get("taken_from_medication_id")
        .and_then(Value::as_i64);
    if attributes.get("taken_from_medication_id").is_some() && selected_id.is_none() {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Selected location is unavailable for this medication.",
        ));
    }
    let selected = if let Some(id) = selected_id {
        matching
            .into_iter()
            .find(|value| value.id == id)
            .ok_or_else(|| {
                error(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Selected location is unavailable for this medication.",
                )
            })?
    } else if matching.len() == 1 {
        matching.into_iter().next().ok_or_else(ApiError::internal)?
    } else if matching.len() > 1 {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Choose a location to record this dose.",
        ));
    } else {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Cannot take medication: out of stock",
        ));
    };
    Ok(ProposedTake {
        source,
        taken_at,
        amount,
        unit,
        selected,
    })
}

fn applies_on(source: &Source, date: NaiveDate) -> bool {
    let Some(kind) = source.schedule_type else {
        return true;
    };
    let (Some(start), Some(end)) = (source.start_date, source.end_date) else {
        return false;
    };
    if date < start || date > end {
        return false;
    }
    let config = source.schedule_config.as_ref();
    match kind {
        2 => config
            .and_then(|v| v.get("weekdays"))
            .and_then(Value::as_array)
            .is_some_and(|days| {
                days.iter().any(|day| {
                    let weekday = date.format("%A").to_string().to_lowercase();
                    let short = &weekday[..3];
                    day.as_str().is_some_and(|value| {
                        value.eq_ignore_ascii_case(&weekday) || value.eq_ignore_ascii_case(short)
                    }) || day.as_u64() == Some(date.format("%w").to_string().parse().unwrap_or(7))
                })
            }),
        3 => config
            .and_then(|v| v.get("dates"))
            .and_then(Value::as_array)
            .is_some_and(|dates| {
                dates
                    .iter()
                    .any(|value| value.as_str() == Some(&date.to_string()))
            }),
        5 => config
            .and_then(|v| v.get("taper_steps"))
            .and_then(Value::as_array)
            .is_some_and(|steps| {
                steps.iter().any(|step| {
                    let start = step
                        .get("start_date")
                        .and_then(Value::as_str)
                        .and_then(|v| NaiveDate::parse_from_str(v, "%Y-%m-%d").ok());
                    let end = step
                        .get("end_date")
                        .and_then(Value::as_str)
                        .and_then(|v| NaiveDate::parse_from_str(v, "%Y-%m-%d").ok());
                    start
                        .zip(end)
                        .is_some_and(|(start, end)| date >= start && date <= end)
                })
            }),
        6 => (date - start).num_days() % 2 == 0,
        _ => true,
    }
}

fn quantity(amount: Decimal, unit: &str) -> Decimal {
    if matches!(
        unit,
        "tablet" | "capsule" | "gummy" | "sachet" | "spray" | "drop" | "pad" | "ml"
    ) {
        amount
    } else {
        Decimal::ONE
    }
}

fn local_midnight_utc(date: NaiveDate, zone: chrono_tz::Tz) -> NaiveDateTime {
    let midnight = date.and_hms_opt(0, 0, 0).expect("midnight is valid");
    zone.from_local_datetime(&midnight)
        .earliest()
        .map(|time| time.with_timezone(&Utc).naive_utc())
        .unwrap_or(midnight)
}

fn cycle_bounds(time: NaiveDateTime, cycle: Option<i32>) -> (NaiveDateTime, NaiveDateTime) {
    cycle_bounds_in_zone(time, cycle, app_zone())
}

fn cycle_bounds_in_zone(
    time: NaiveDateTime,
    cycle: Option<i32>,
    zone: chrono_tz::Tz,
) -> (NaiveDateTime, NaiveDateTime) {
    let date = local_date_in_zone(time, zone);
    let start = match cycle.unwrap_or(0) {
        1 => date - Duration::days(date.weekday().num_days_from_monday() as i64),
        2 => NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap_or(date),
        _ => date,
    };
    let end = match cycle.unwrap_or(0) {
        1 => start + Duration::weeks(1),
        2 => {
            let (year, month) = if start.month() == 12 {
                (start.year() + 1, 1)
            } else {
                (start.year(), start.month() + 1)
            };
            NaiveDate::from_ymd_opt(year, month, 1).unwrap_or(start + Duration::days(31))
        }
        _ => start + Duration::days(1),
    };
    (
        local_midnight_utc(start, zone),
        local_midnight_utc(end, zone),
    )
}

async fn timing_allowed(
    db: &DatabaseTransaction,
    proposed: &ProposedTake,
) -> Result<bool, ApiError> {
    let assignments = person_medication::Entity::find()
        .filter(person_medication::Column::PersonId.eq(proposed.source.person_id))
        .filter(person_medication::Column::MedicationId.eq(proposed.source.medication_id))
        .filter(person_medication::Column::Active.eq(true))
        .filter(person_medication::Column::RetiredAt.is_null())
        .all(db)
        .await
        .map_err(database_error)?;
    let schedules = schedule::Entity::find()
        .filter(schedule::Column::PersonId.eq(proposed.source.person_id))
        .filter(schedule::Column::MedicationId.eq(proposed.source.medication_id))
        .filter(schedule::Column::Active.eq(true))
        .filter(schedule::Column::RetiredAt.is_null())
        .all(db)
        .await
        .map_err(database_error)?;
    let related: Vec<Source> = assignments
        .into_iter()
        .map(source_from_assignment)
        .chain(
            schedules
                .into_iter()
                .map(source_from_schedule)
                .filter(|source| applies_on(source, local_date(proposed.taken_at))),
        )
        .collect();
    if related.is_empty() {
        return Ok(true);
    }
    let schedule_ids: Vec<i64> = related
        .iter()
        .filter(|source| source.kind == "schedule")
        .map(|source| source.id)
        .collect();
    let assignment_ids: Vec<i64> = related
        .iter()
        .filter(|source| source.kind == "person_medication")
        .map(|source| source.id)
        .collect();
    let mut condition = Condition::any();
    if !schedule_ids.is_empty() {
        condition = condition.add(medication_take::Column::ScheduleId.is_in(schedule_ids));
    }
    if !assignment_ids.is_empty() {
        condition =
            condition.add(medication_take::Column::PersonMedicationId.is_in(assignment_ids));
    }
    let takes = medication_take::Entity::find()
        .filter(condition)
        .all(db)
        .await
        .map_err(database_error)?;
    for mut source in related {
        effective_source(&mut source, local_date(proposed.taken_at));
        if let Some(max_doses) = source.max_daily_doses {
            let (start, end) = cycle_bounds(proposed.taken_at, source.dose_cycle);
            let count = takes
                .iter()
                .filter(|take| {
                    take.taken_at
                        .is_some_and(|time| time >= start && time < end)
                })
                .count();
            if count >= max_doses.max(0) as usize {
                return Ok(false);
            }
        }
        if let Some(hours) = source.min_hours_between_doses {
            if hours > Decimal::ZERO {
                let recent = takes
                    .iter()
                    .filter_map(|take| take.taken_at)
                    .filter(|time| *time <= proposed.taken_at)
                    .max();
                if let Some(recent) = recent {
                    let elapsed = Decimal::from((proposed.taken_at - recent).num_seconds());
                    if elapsed < hours * Decimal::from(3600) {
                        return Ok(false);
                    }
                }
            }
        }
    }
    Ok(true)
}

struct StockVersionChange<'a> {
    item_type: &'a str,
    item_id: i64,
    previous: Decimal,
    current: Decimal,
    event: &'a str,
}

fn domain_audit_context(context: &AuthContext, request_id: &str) -> Value {
    let (authentication_method, session_reference) = match context.credential_kind {
        CredentialKind::ApiSession => (
            "api_session",
            format!("api_session:{}", context.credential_reference),
        ),
        CredentialKind::OauthGrant => (
            "oauth",
            format!("oauth_grant:{}", context.credential_reference),
        ),
        CredentialKind::BrowserSession => (
            "browser_session",
            format!("browser_session:{}", context.credential_reference),
        ),
    };
    json!({"actor_account_id": context.account_id, "actor_user_id": context.user_id,
        "actor_membership_id": context.membership.id, "household_id": context.membership.household_id,
        "active_role": context.membership.role, "permissions_version": context.membership.permissions_version,
        "authentication_method": authentication_method, "session_reference": session_reference,
        "policy_class": "MedicationTakePolicy", "policy_query": "create?", "request_id": request_id})
}

async fn stock_version(
    db: &DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    change: StockVersionChange<'_>,
) -> Result<(), ApiError> {
    crate::entities::version::ActiveModel {
        item_type: Set(change.item_type.to_owned()), item_id: Set(change.item_id), event: Set(change.event.to_owned()),
        object: Set(Some(json!({"current_supply": decimal_string(change.previous.to_string())}).to_string())),
        object_changes: Set(Some(json!({"current_supply": [decimal_string(change.previous.to_string()), decimal_string(change.current.to_string())]}).to_string())),
        whodunnit: Set(Some(context.user_id.to_string())), request_id: Set(Some(request_id.to_owned())),
        household_id: Set(Some(context.membership.household_id)), actor_membership_id: Set(Some(context.membership.id)),
        audit_context: Set(domain_audit_context(context, request_id)),
        created_at: Set(Some(Utc::now().naive_utc())), ..Default::default()
    }.insert(db).await.map_err(database_error)?;
    Ok(())
}

fn same_dosage_signature(left: &dosage::Model, right: &dosage::Model) -> bool {
    left.amount == right.amount
        && left.unit == right.unit
        && left.frequency == right.frequency
        && left.description.as_deref().unwrap_or("") == right.description.as_deref().unwrap_or("")
        && left.default_for_adults == right.default_for_adults
        && left.default_for_children == right.default_for_children
        && left.default_max_daily_doses == right.default_max_daily_doses
        && left.default_min_hours_between_doses == right.default_min_hours_between_doses
        && left.default_dose_cycle == right.default_dose_cycle
}

async fn decrement_stock(
    db: &DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    proposed: &ProposedTake,
) -> Result<(), ApiError> {
    lock_row(db, "medications", proposed.selected.id).await?;
    let selected = medication::Entity::find_by_id(proposed.selected.id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?;
    let tracked = dosage::Entity::find()
        .filter(dosage::Column::MedicationId.eq(selected.id))
        .filter(dosage::Column::CurrentSupply.is_not_null())
        .all(db)
        .await
        .map_err(database_error)?;
    if tracked.is_empty() {
        if let Some(supply) = selected.current_supply {
            let needed = quantity(proposed.amount, &proposed.unit);
            if supply < needed {
                return Err(error(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Cannot take medication: out of stock",
                ));
            }
            let selected_id = selected.id;
            let mut update: medication::ActiveModel = selected.into();
            update.current_supply = Set(Some(supply - needed));
            update.updated_at = Set(Utc::now().naive_utc());
            update.update(db).await.map_err(database_error)?;
            stock_version(
                db,
                context,
                request_id,
                StockVersionChange {
                    item_type: "Medication",
                    item_id: selected_id,
                    previous: supply,
                    current: supply - needed,
                    event: "dose_decrement",
                },
            )
            .await?;
        }
        return Ok(());
    }
    let matching: Vec<_> = if let Some(id) = proposed.source.source_dosage_option_id {
        let source_option = dosage::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(database_error)?
            .ok_or_else(ApiError::not_found)?;
        if source_option.medication_id == selected.id {
            tracked
                .iter()
                .filter(|option| option.id == id)
                .cloned()
                .collect()
        } else {
            tracked
                .iter()
                .filter(|option| same_dosage_signature(option, &source_option))
                .cloned()
                .collect()
        }
    } else {
        tracked
            .iter()
            .filter(|option| {
                option.amount == proposed.source.dose_amount
                    && option.unit == proposed.source.dose_unit
            })
            .cloned()
            .collect()
    };
    let selected_option = if matching.len() == 1 {
        matching.into_iter().next()
    } else {
        None
    }
    .ok_or_else(|| {
        error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Selected location is unavailable for this medication.",
        )
    })?;
    lock_row(db, "dosages", selected_option.id).await?;
    let selected_option = dosage::Entity::find_by_id(selected_option.id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?;
    let supply = selected_option
        .current_supply
        .ok_or_else(ApiError::internal)?;
    let needed = quantity(
        proposed.amount,
        selected_option.unit.as_deref().unwrap_or(""),
    );
    if supply < needed {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Cannot take medication: out of stock",
        ));
    }
    let option_id = selected_option.id;
    let mut update: dosage::ActiveModel = selected_option.into();
    update.current_supply = Set(Some(supply - needed));
    update.update(db).await.map_err(database_error)?;
    stock_version(
        db,
        context,
        request_id,
        StockVersionChange {
            item_type: "MedicationDosageOption",
            item_id: option_id,
            previous: supply,
            current: supply - needed,
            event: "update",
        },
    )
    .await?;
    let total = dosage::Entity::find()
        .filter(dosage::Column::MedicationId.eq(proposed.selected.id))
        .filter(dosage::Column::CurrentSupply.is_not_null())
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .filter_map(|v| v.current_supply)
        .sum::<Decimal>();
    if total >= Decimal::from(100_000_000) {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Invalid dose configured",
        ));
    }
    let previous = selected.current_supply;
    let selected_id = selected.id;
    let mut inventory: medication::ActiveModel = selected.into();
    inventory.current_supply = Set(Some(total));
    inventory.updated_at = Set(Utc::now().naive_utc());
    inventory.update(db).await.map_err(database_error)?;
    if let Some(previous) = previous {
        stock_version(
            db,
            context,
            request_id,
            StockVersionChange {
                item_type: "Medication",
                item_id: selected_id,
                previous,
                current: total,
                event: "dose_decrement",
            },
        )
        .await?;
    }
    Ok(())
}

async fn insert_take(
    db: &DatabaseTransaction,
    household_id: i64,
    proposed: &ProposedTake,
    client_uuid: Option<&str>,
) -> Result<Option<medication_take::Model>, ApiError> {
    let now = Utc::now().naive_utc();
    let model = medication_take::ActiveModel {
        household_id: Set(household_id),
        client_uuid: Set(client_uuid.map(str::to_owned)),
        schedule_id: Set((proposed.source.kind == "schedule").then_some(proposed.source.id)),
        person_medication_id: Set(
            (proposed.source.kind == "person_medication").then_some(proposed.source.id)
        ),
        taken_from_medication_id: Set(Some(proposed.selected.id)),
        taken_from_location_id: Set(Some(proposed.selected.location_id)),
        dose_amount: Set(Some(proposed.amount)),
        dose_unit: Set(Some(proposed.unit.clone())),
        taken_at: Set(Some(proposed.taken_at)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    if client_uuid.is_none() {
        return model.insert(db).await.map(Some).map_err(database_error);
    }
    let statement = medication_take::Entity::insert(model)
        .on_conflict(
            sea_orm::sea_query::OnConflict::new()
                .do_nothing()
                .to_owned(),
        )
        .exec_with_returning(db)
        .await;
    match statement {
        Ok(row) => Ok(Some(row)),
        Err(sea_orm::DbErr::RecordNotInserted | sea_orm::DbErr::RecordNotFound(_)) => Ok(None),
        Err(error) => Err(database_error(error)),
    }
}

async fn record_domain_audit(
    db: &DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    take: &medication_take::Model,
    proposed: &ProposedTake,
) -> Result<(), ApiError> {
    let now = Utc::now().naive_utc();
    let mut context_value = domain_audit_context(context, request_id);
    context_value["person_id"] = json!(proposed.source.person_id);
    context_value["medication_id"] = json!(proposed.source.medication_id);
    let snapshot = json!({
        "id": take.id, "portable_id": take.portable_id, "client_uuid": take.client_uuid,
        "household_id": take.household_id, "schedule_id": take.schedule_id,
        "person_medication_id": take.person_medication_id,
        "person_id": proposed.source.person_id, "medication_id": proposed.source.medication_id,
        "taken_from_medication_id": take.taken_from_medication_id,
        "taken_from_location_id": take.taken_from_location_id,
        "dose_amount": take.dose_amount.map(|value| decimal_string(value.to_string())),
        "dose_unit": take.dose_unit,
        "taken_at": take.taken_at.map(|value| value.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
        "created_at": take.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        "updated_at": take.updated_at.format("%Y-%m-%dT%H:%M:%SZ").to_string()
    });
    let changes: serde_json::Map<String, Value> = snapshot
        .as_object()
        .ok_or_else(ApiError::internal)?
        .iter()
        .map(|(key, value)| (key.clone(), json!([null, value])))
        .collect();
    crate::entities::version::ActiveModel {
        item_type: Set("MedicationTake".to_owned()),
        item_id: Set(take.id),
        event: Set("create".to_owned()),
        object: Set(Some(snapshot.to_string())),
        object_changes: Set(Some(Value::Object(changes).to_string())),
        whodunnit: Set(Some(context.user_id.to_string())),
        request_id: Set(Some(request_id.to_owned())),
        household_id: Set(Some(context.membership.household_id)),
        actor_membership_id: Set(Some(context.membership.id)),
        audit_context: Set(context_value),
        created_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(database_error)?;
    crate::entities::api_change_event::ActiveModel {
        household_id: Set(context.membership.household_id),
        household_membership_id: Set(Some(context.membership.id)),
        account_id: Set(Some(context.account_id)),
        action: Set("create".to_owned()),
        record_type: Set("MedicationTake".to_owned()),
        record_id: Set(take.id),
        record_portable_id: Set(Some(take.portable_id.clone())),
        request_id: Set(Some(request_id.to_owned())),
        metadata: Set(json!({})),
        occurred_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(database_error)?;
    Ok(())
}

pub async fn create(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let request_id = Uuid::new_v4().to_string();
    let db = match state.db.begin().await {
        Ok(db) => db,
        Err(error) => return request_error_response(database_error(error), &request_id),
    };
    let context = match authenticate(&state, &db, &headers, household_id).await {
        Ok(context) => context,
        Err(error) => {
            if error.preserve_activity {
                let _ = db.commit().await;
            }
            return request_error_response(error, &request_id);
        }
    };
    let result = create_in_transaction(&db, &context, household_id, &body, &request_id).await;
    match result {
        Ok((status, take)) => {
            let data = match serialize(&db, std::slice::from_ref(&take)).await {
                Ok(mut data) => data.remove(0),
                Err(error) => {
                    let _ = db.rollback().await;
                    return request_error_response(error, &request_id);
                }
            };
            if let Err(error) =
                audit(&db, &context, &request_id, "create", "POST", status, true).await
            {
                let _ = db.rollback().await;
                return request_error_response(error, &request_id);
            }
            if let Err(error) = db.commit().await {
                return request_error_response(database_error(error), &request_id);
            }
            success_response(
                status,
                json!({"data": data}),
                &request_id,
                Some(take_etag(&take)),
            )
        }
        Err(error) => {
            let status = error.status;
            let _ = db.rollback().await;
            if let Ok(audit_db) = state.db.begin().await {
                if let Ok(current_context) =
                    authenticate(&state, &audit_db, &headers, household_id).await
                {
                    let _ = audit(
                        &audit_db,
                        &current_context,
                        &request_id,
                        "create",
                        "POST",
                        status,
                        false,
                    )
                    .await;
                }
                let _ = audit_db.commit().await;
            }
            request_error_response(error, &request_id)
        }
    }
}

async fn create_in_transaction(
    db: &DatabaseTransaction,
    context: &AuthContext,
    household_id: i64,
    body: &Value,
    request_id: &str,
) -> Result<(StatusCode, medication_take::Model), ApiError> {
    let attributes = body
        .get("medication_take")
        .filter(|value| value.is_object())
        .ok_or_else(|| error(StatusCode::BAD_REQUEST, "medication_take is required"))?;
    lock_row(db, "households", household_id).await?;
    let current_membership = crate::entities::membership::Entity::find_by_id(context.membership.id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::forbidden)?;
    if current_membership.status != "active"
        || current_membership.revoked_at.is_some()
        || current_membership.permissions_version != context.membership.permissions_version
        || current_membership.role != context.membership.role
        || current_membership.household_id != household_id
    {
        return Err(ApiError::forbidden());
    }
    let account = crate::entities::account::Entity::find_by_id(context.account_id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    if account.status != 2 {
        return Err(ApiError::unauthorized());
    }
    let lockout = crate::entities::account_lockout::Entity::find_by_id(context.account_id)
        .one(db)
        .await
        .map_err(database_error)?;
    if lockout.is_some_and(|value| value.deadline > Utc::now().naive_utc()) {
        return Err(ApiError::unauthorized());
    }
    let client_uuid = attributes
        .get("client_uuid")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty());
    if let Some(client_uuid) = client_uuid {
        lock_client_uuid(db, client_uuid).await?;
    }
    if let Some(client_uuid) = client_uuid {
        if let Some(existing) = medication_take::Entity::find()
            .filter(medication_take::Column::HouseholdId.eq(household_id))
            .filter(medication_take::Column::ClientUuid.eq(client_uuid))
            .one(db)
            .await
            .map_err(database_error)?
        {
            if !replay_matches(db, context, household_id, &existing, attributes).await? {
                return Err(error(
                    StatusCode::CONFLICT,
                    "client_uuid was already used for a different dose",
                ));
            }
            return Ok((StatusCode::OK, existing));
        }
    }
    let proposed = prepare(db, context, household_id, attributes).await?;
    if attributes
        .get("dose_unit")
        .is_some_and(|unit| !unit.is_null() && unit.as_str() != Some(proposed.unit.as_str()))
    {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "dose_unit does not match the source",
        ));
    }
    if !timing_allowed(db, &proposed).await? {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Cannot take medication: timing restrictions not met",
        ));
    }
    let take = insert_take(db, household_id, &proposed, client_uuid)
        .await?
        .ok_or_else(|| {
            error(
                StatusCode::CONFLICT,
                "client_uuid was already used for a different dose",
            )
        })?;
    decrement_stock(db, context, request_id, &proposed).await?;
    record_domain_audit(db, context, request_id, &take, &proposed).await?;
    Ok((StatusCode::CREATED, take))
}

#[cfg(test)]
mod tests {
    use super::{cycle_bounds_in_zone, valid_numeric_10_2};
    use chrono::NaiveDate;
    use sea_orm::prelude::Decimal;
    use std::str::FromStr;

    #[test]
    fn numeric_10_2_accepts_the_full_column_range_without_rounding() {
        assert!(valid_numeric_10_2(
            Decimal::from_str("99999999.99").unwrap()
        ));
        assert!(!valid_numeric_10_2(
            Decimal::from_str("100000000.00").unwrap()
        ));
        assert!(!valid_numeric_10_2(Decimal::from_str("1.001").unwrap()));
    }

    #[test]
    fn london_monthly_cycle_uses_local_month_boundaries() {
        let time = NaiveDate::from_ymd_opt(2026, 9, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let (start, end) = cycle_bounds_in_zone(time, Some(2), chrono_tz::Europe::London);
        assert_eq!(
            start,
            NaiveDate::from_ymd_opt(2026, 8, 31)
                .unwrap()
                .and_hms_opt(23, 0, 0)
                .unwrap()
        );
        assert_eq!(
            end,
            NaiveDate::from_ymd_opt(2026, 9, 30)
                .unwrap()
                .and_hms_opt(23, 0, 0)
                .unwrap()
        );
    }

    #[test]
    fn london_daily_cycle_observes_spring_dst_shift() {
        let time = NaiveDate::from_ymd_opt(2026, 3, 29)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let (start, end) = cycle_bounds_in_zone(time, Some(0), chrono_tz::Europe::London);
        assert_eq!(
            start,
            NaiveDate::from_ymd_opt(2026, 3, 29)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
        );
        assert_eq!(
            end,
            NaiveDate::from_ymd_opt(2026, 3, 29)
                .unwrap()
                .and_hms_opt(23, 0, 0)
                .unwrap()
        );
    }
}
