use crate::entities::{
    dosage, medication, membership, person_medication as write_assignment,
    schedule as write_schedule,
};
use crate::read_entities::{
    location_membership, notification_preference, pause_period, person, person_medication,
    schedule, stock_location,
};
use crate::{
    audit, authenticate, database_error, decimal_string, granted_people, if_none_match_matches,
    representation_etag, ApiError, AppState, AuthContext, Pagination,
};
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use sea_orm::prelude::Decimal;
use sea_orm::{
    ColumnTrait, Condition, DatabaseTransaction, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, TransactionTrait,
};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

struct Page {
    number: i64,
    size: i64,
    updated_since: Option<chrono::NaiveDateTime>,
}

async fn request_context(
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

fn parse_page(
    db: DatabaseTransaction,
    pagination: Pagination,
) -> Result<(DatabaseTransaction, Page), (DatabaseTransaction, ApiError)> {
    let updated_since = pagination
        .updated_since
        .filter(|value| !value.is_empty())
        .map(|value| DateTime::parse_from_rfc3339(&value).map_err(|_| ApiError::invalid_filter()))
        .transpose();
    let updated_since = match updated_since {
        Ok(value) => value.map(|value| value.naive_utc()),
        Err(error) => return Err((db, error)),
    };
    Ok((
        db,
        Page {
            number: pagination.page.unwrap_or(1).max(1),
            size: pagination.per_page.unwrap_or(20).clamp(1, 100),
            updated_since,
        },
    ))
}

async fn audited_error_response(
    db: DatabaseTransaction,
    context: &AuthContext,
    controller: &str,
    policy: &str,
    action: &str,
    error: ApiError,
    authorized: bool,
) -> Result<Response, ApiError> {
    let request_id = audit::record_resource_read(
        &db,
        context,
        controller,
        policy,
        action,
        error.status,
        authorized,
    )
    .await
    .map_err(database_error)?;
    db.commit().await.map_err(database_error)?;
    let mut response = (
        error.status,
        Json(json!({"error": {
            "code": error.code,
            "message": error.message,
            "request_id": request_id
        }})),
    )
        .into_response();
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&request_id).map_err(|_| ApiError::internal())?,
    );
    Ok(response)
}

fn offset(page: &Page) -> u64 {
    page.number.saturating_sub(1).saturating_mul(page.size) as u64
}

async fn collection_response(
    db: DatabaseTransaction,
    context: &AuthContext,
    controller: &str,
    policy: &str,
    rows: Vec<Value>,
    page: Page,
    total_count: u64,
) -> Result<Response, ApiError> {
    let request_id = audit::record_resource_read(
        &db,
        context,
        controller,
        policy,
        "index",
        StatusCode::OK,
        true,
    )
    .await
    .map_err(database_error)?;
    db.commit().await.map_err(database_error)?;
    let mut response = Json(json!({
        "data": rows,
        "meta": {"page": page.number, "per_page": page.size, "total_count": total_count}
    }))
    .into_response();
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&request_id).map_err(|_| ApiError::internal())?,
    );
    Ok(response)
}

async fn detail_response(
    db: DatabaseTransaction,
    context: &AuthContext,
    controller: &str,
    policy: &str,
    record: Option<Value>,
    headers: &HeaderMap,
) -> Result<Response, ApiError> {
    let Some(record) = record else {
        audit::record_resource_read(
            &db,
            context,
            controller,
            policy,
            "show",
            StatusCode::NOT_FOUND,
            false,
        )
        .await
        .map_err(database_error)?;
        db.commit().await.map_err(database_error)?;
        return Err(ApiError::not_found());
    };
    let body = json!({"data": record});
    let etag = representation_etag(&body);
    let not_modified = if_none_match_matches(headers, &etag);
    audit::record_resource_read(
        &db,
        context,
        controller,
        policy,
        "show",
        if not_modified {
            StatusCode::NOT_MODIFIED
        } else {
            StatusCode::OK
        },
        true,
    )
    .await
    .map_err(database_error)?;
    db.commit().await.map_err(database_error)?;
    let mut response = if not_modified {
        StatusCode::NOT_MODIFIED.into_response()
    } else {
        Json(body).into_response()
    };
    response.headers_mut().insert(
        header::ETAG,
        HeaderValue::from_str(&etag).map_err(|_| ApiError::internal())?,
    );
    Ok(response)
}

fn person_scope(household_id: i64, context: &AuthContext) -> sea_orm::Select<person::Entity> {
    person::Entity::find()
        .filter(person::Column::HouseholdId.eq(household_id))
        .filter(person::Column::Id.in_subquery(granted_people(&context.membership)))
}

fn schedule_scope(household_id: i64, context: &AuthContext) -> sea_orm::Select<schedule::Entity> {
    schedule::Entity::find()
        .filter(schedule::Column::HouseholdId.eq(household_id))
        .filter(schedule::Column::RetiredAt.is_null())
        .filter(schedule::Column::PersonId.in_subquery(granted_people(&context.membership)))
}

fn assignment_scope(
    household_id: i64,
    context: &AuthContext,
) -> sea_orm::Select<person_medication::Entity> {
    person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(household_id))
        .filter(person_medication::Column::RetiredAt.is_null())
        .filter(
            person_medication::Column::PersonId.in_subquery(granted_people(&context.membership)),
        )
}

fn today() -> NaiveDate {
    let timezone = std::env::var("TZ")
        .ok()
        .and_then(|value| value.parse::<chrono_tz::Tz>().ok())
        .unwrap_or(chrono_tz::UTC);
    Utc::now().with_timezone(&timezone).date_naive()
}

fn age(birth_date: Option<NaiveDate>, reference: NaiveDate) -> Option<i32> {
    birth_date.map(|birth_date| {
        let birthday_passed =
            (reference.month(), reference.day()) >= (birth_date.month(), birth_date.day());
        reference.year() - birth_date.year() - i32::from(!birthday_passed)
    })
}

fn person_type(value: i32) -> &'static str {
    match value {
        1 => "minor",
        2 => "dependent_adult",
        _ => "adult",
    }
}

async fn require_adult_schedule_index(
    db: &DatabaseTransaction,
    context: &AuthContext,
) -> Result<(), ApiError> {
    let person_id = context
        .membership
        .person_id
        .ok_or_else(ApiError::forbidden)?;
    let person = person::Entity::find_by_id(person_id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::forbidden)?;
    let adult =
        age(person.date_of_birth, today()).is_some_and(|age| age >= 18) || person.person_type == 0;
    if adult {
        Ok(())
    } else {
        Err(ApiError::forbidden())
    }
}

pub(super) async fn people_index(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    Query(pagination): Query<Pagination>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let (db, page) = match parse_page(db, pagination) {
        Ok(value) => value,
        Err((db, error)) => {
            return audited_error_response(
                db,
                &context,
                "api/v1/people",
                "PersonPolicy",
                "index",
                error,
                true,
            )
            .await;
        }
    };
    let mut query = person_scope(household_id, &context);
    if let Some(updated_since) = page.updated_since {
        query = query.filter(person::Column::UpdatedAt.gte(updated_since));
    }
    let total = query.clone().count(&db).await.map_err(database_error)?;
    let records = query
        .order_by_asc(person::Column::Id)
        .limit(page.size as u64)
        .offset(offset(&page))
        .all(&db)
        .await
        .map_err(database_error)?;
    let rows = serialize_people(&db, records).await?;
    collection_response(
        db,
        &context,
        "api/v1/people",
        "PersonPolicy",
        rows,
        page,
        total,
    )
    .await
}

pub(super) async fn people_show(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let record = match id.parse::<i64>() {
        Ok(id) => {
            person_scope(household_id, &context)
                .filter(person::Column::Id.eq(id))
                .one(&db)
                .await
        }
        Err(_) => Ok(None),
    }
    .map_err(database_error)?;
    let row = match record {
        Some(record) => serialize_people(&db, vec![record]).await?.pop(),
        None => None,
    };
    detail_response(db, &context, "api/v1/people", "PersonPolicy", row, &headers).await
}

pub(super) async fn schedules_index(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    Query(pagination): Query<Pagination>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    if let Err(error) = require_adult_schedule_index(&db, &context).await {
        if error.status == StatusCode::FORBIDDEN {
            return audited_error_response(
                db,
                &context,
                "api/v1/schedules",
                "SchedulePolicy",
                "index",
                error,
                false,
            )
            .await;
        }
        return Err(error);
    }
    let (db, page) = match parse_page(db, pagination) {
        Ok(value) => value,
        Err((db, error)) => {
            return audited_error_response(
                db,
                &context,
                "api/v1/schedules",
                "SchedulePolicy",
                "index",
                error,
                true,
            )
            .await;
        }
    };
    let mut query = schedule_scope(household_id, &context);
    if let Some(updated_since) = page.updated_since {
        query = query.filter(schedule::Column::UpdatedAt.gte(updated_since));
    }
    let total = query.clone().count(&db).await.map_err(database_error)?;
    let records = query
        .order_by_asc(schedule::Column::Id)
        .limit(page.size as u64)
        .offset(offset(&page))
        .all(&db)
        .await
        .map_err(database_error)?;
    let rows = serialize_schedules(&db, &context, records).await?;
    collection_response(
        db,
        &context,
        "api/v1/schedules",
        "SchedulePolicy",
        rows,
        page,
        total,
    )
    .await
}

pub(super) async fn schedules_show(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let query = schedule_scope(household_id, &context);
    let query = match id.parse::<i64>() {
        Ok(id) => query.filter(schedule::Column::Id.eq(id)),
        Err(_) => query.filter(schedule::Column::PortableId.eq(id)),
    };
    let row = match query.one(&db).await.map_err(database_error)? {
        Some(record) => serialize_schedules(&db, &context, vec![record])
            .await?
            .pop(),
        None => None,
    };
    detail_response(
        db,
        &context,
        "api/v1/schedules",
        "SchedulePolicy",
        row,
        &headers,
    )
    .await
}

pub(super) async fn person_medications_index(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    Query(pagination): Query<Pagination>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let (db, page) = match parse_page(db, pagination) {
        Ok(value) => value,
        Err((db, error)) => {
            return audited_error_response(
                db,
                &context,
                "api/v1/person_medications",
                "PersonMedicationPolicy",
                "index",
                error,
                true,
            )
            .await;
        }
    };
    let mut query = assignment_scope(household_id, &context);
    if let Some(updated_since) = page.updated_since {
        query = query.filter(person_medication::Column::UpdatedAt.gte(updated_since));
    }
    let total = query.clone().count(&db).await.map_err(database_error)?;
    let records = query
        .order_by_asc(person_medication::Column::Id)
        .limit(page.size as u64)
        .offset(offset(&page))
        .all(&db)
        .await
        .map_err(database_error)?;
    let rows = serialize_assignments(&db, &context, records).await?;
    collection_response(
        db,
        &context,
        "api/v1/person_medications",
        "PersonMedicationPolicy",
        rows,
        page,
        total,
    )
    .await
}

pub(super) async fn person_medications_show(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let query = assignment_scope(household_id, &context);
    let query = match id.parse::<i64>() {
        Ok(id) => query.filter(person_medication::Column::Id.eq(id)),
        Err(_) => query.filter(person_medication::Column::PortableId.eq(id)),
    };
    let row = match query.one(&db).await.map_err(database_error)? {
        Some(record) => serialize_assignments(&db, &context, vec![record])
            .await?
            .pop(),
        None => None,
    };
    detail_response(
        db,
        &context,
        "api/v1/person_medications",
        "PersonMedicationPolicy",
        row,
        &headers,
    )
    .await
}

async fn serialize_people(
    db: &DatabaseTransaction,
    records: Vec<person::Model>,
) -> Result<Vec<Value>, ApiError> {
    if records.is_empty() {
        return Ok(Vec::new());
    }
    let person_ids: Vec<i64> = records.iter().map(|record| record.id).collect();
    let memberships = location_membership::Entity::find()
        .filter(location_membership::Column::PersonId.is_in(person_ids.clone()))
        .order_by_asc(location_membership::Column::Id)
        .all(db)
        .await
        .map_err(database_error)?;
    let location_ids: Vec<i64> = memberships.iter().map(|row| row.location_id).collect();
    let locations: HashMap<i64, String> = if location_ids.is_empty() {
        HashMap::new()
    } else {
        crate::entities::location::Entity::find()
            .filter(crate::entities::location::Column::Id.is_in(location_ids))
            .all(db)
            .await
            .map_err(database_error)?
            .into_iter()
            .map(|row| (row.id, row.portable_id))
            .collect()
    };
    let preferences: HashMap<i64, notification_preference::Model> =
        notification_preference::Entity::find()
            .filter(notification_preference::Column::PersonId.is_in(person_ids))
            .all(db)
            .await
            .map_err(database_error)?
            .into_iter()
            .map(|row| (row.person_id, row))
            .collect();
    let mut memberships_by_person: HashMap<i64, Vec<i64>> = HashMap::new();
    for row in memberships {
        memberships_by_person
            .entry(row.person_id)
            .or_default()
            .push(row.location_id);
    }
    let reference_date = today();
    Ok(records
        .into_iter()
        .map(|record| {
            let location_ids = memberships_by_person.remove(&record.id).unwrap_or_default();
            let location_portable_ids: Vec<&str> = location_ids
                .iter()
                .filter_map(|id| locations.get(id).map(String::as_str))
                .collect();
            let preference = preferences.get(&record.id);
            json!({
                "id": record.id,
                "portable_id": record.portable_id,
                "updated_at": timestamp(record.updated_at),
                "name": record.name,
                "email": record.email,
                "date_of_birth": record.date_of_birth.map(|date| date.to_string()),
                "person_type": person_type(record.person_type),
                "has_capacity": record.has_capacity,
                "age": age(record.date_of_birth, reference_date),
                "location_ids": location_ids,
                "location_portable_ids": location_portable_ids,
                "notification_preference_id": preference.map(|row| row.id),
                "notification_preference_portable_id": preference.map(|row| &row.portable_id)
            })
        })
        .collect())
}

struct SourceRef {
    id: i64,
    person_id: i64,
    medication_id: i64,
    portable_id: String,
    source_dosage_option_id: Option<i64>,
    stock_dose: Option<(Decimal, String)>,
}

struct SourceContext {
    people: HashMap<i64, String>,
    medications: HashMap<i64, String>,
    manageable_people: HashSet<i64>,
    recordable_people: HashSet<i64>,
    eligible_stock: HashMap<i64, Vec<i64>>,
    pauses: HashMap<i64, Value>,
}

enum SourceKind {
    Schedule,
    Assignment,
}

async fn source_context(
    db: &DatabaseTransaction,
    context: &AuthContext,
    sources: &[SourceRef],
    kind: SourceKind,
) -> Result<SourceContext, ApiError> {
    if sources.is_empty() {
        return Ok(SourceContext {
            people: HashMap::new(),
            medications: HashMap::new(),
            manageable_people: HashSet::new(),
            recordable_people: HashSet::new(),
            eligible_stock: HashMap::new(),
            pauses: HashMap::new(),
        });
    }
    let person_ids: Vec<i64> = sources.iter().map(|source| source.person_id).collect();
    let medication_ids: Vec<i64> = sources.iter().map(|source| source.medication_id).collect();
    let people = person::Entity::find()
        .filter(person::Column::Id.is_in(person_ids.clone()))
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|person| (person.id, person.portable_id))
        .collect();
    let medications = medication::Entity::find()
        .filter(medication::Column::Id.is_in(medication_ids))
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|medication| (medication.id, medication.portable_id))
        .collect();
    let permission_grants = crate::entities::grant::Entity::find()
        .filter(crate::entities::grant::Column::HouseholdId.eq(context.membership.household_id))
        .filter(crate::entities::grant::Column::HouseholdMembershipId.eq(context.membership.id))
        .filter(crate::entities::grant::Column::PersonId.is_in(person_ids))
        .filter(crate::entities::grant::Column::AccessLevel.is_in(["record", "manage"]))
        .filter(crate::entities::grant::Column::RevokedAt.is_null())
        .filter(
            Condition::any()
                .add(crate::entities::grant::Column::ExpiresAt.is_null())
                .add(crate::entities::grant::Column::ExpiresAt.gt(Utc::now().naive_utc())),
        )
        .all(db)
        .await
        .map_err(database_error)?;
    let manageable_people: HashSet<i64> = permission_grants
        .iter()
        .filter(|grant| grant.access_level == "manage")
        .map(|grant| grant.person_id)
        .collect();
    let recordable_people: HashSet<i64> = permission_grants
        .iter()
        .map(|grant| grant.person_id)
        .collect();
    let eligible_stock = eligible_stock_by_source(db, context, sources, &recordable_people).await?;
    let source_ids: Vec<i64> = sources.iter().map(|source| source.id).collect();
    let pauses = match kind {
        SourceKind::Schedule => {
            pause_period::Entity::find()
                .filter(pause_period::Column::ScheduleId.is_in(source_ids))
                .filter(pause_period::Column::EndedAt.is_null())
                .all(db)
                .await
        }
        SourceKind::Assignment => {
            pause_period::Entity::find()
                .filter(pause_period::Column::PersonMedicationId.is_in(source_ids))
                .filter(pause_period::Column::EndedAt.is_null())
                .all(db)
                .await
        }
    }
    .map_err(database_error)?;
    let actor_ids: Vec<i64> = pauses
        .iter()
        .flat_map(|pause| {
            [
                pause.recorded_by_membership_id,
                pause.resumed_by_membership_id,
            ]
        })
        .flatten()
        .collect();
    let actor_memberships = if actor_ids.is_empty() {
        Vec::new()
    } else {
        membership::Entity::find()
            .filter(membership::Column::Id.is_in(actor_ids))
            .all(db)
            .await
            .map_err(database_error)?
    };
    let actor_person_ids: Vec<i64> = actor_memberships
        .iter()
        .filter_map(|membership| membership.person_id)
        .collect();
    let actor_people: HashMap<i64, String> = if actor_person_ids.is_empty() {
        HashMap::new()
    } else {
        person::Entity::find()
            .filter(person::Column::Id.is_in(actor_person_ids))
            .all(db)
            .await
            .map_err(database_error)?
            .into_iter()
            .map(|person| (person.id, person.name))
            .collect()
    };
    let actor_names: HashMap<i64, String> = actor_memberships
        .into_iter()
        .filter_map(|membership| {
            membership
                .person_id
                .and_then(|person_id| actor_people.get(&person_id).cloned())
                .map(|name| (membership.id, name))
        })
        .collect();
    let source_portable_ids: HashMap<i64, &str> = sources
        .iter()
        .map(|source| (source.id, source.portable_id.as_str()))
        .collect();
    let pauses = pauses
        .into_iter()
        .filter_map(|pause| {
            let source_id = match kind {
                SourceKind::Schedule => pause.schedule_id?,
                SourceKind::Assignment => pause.person_medication_id?,
            };
            let source_portable_id = source_portable_ids.get(&source_id)?;
            let source_type = match kind {
                SourceKind::Schedule => "schedule",
                SourceKind::Assignment => "person_medication",
            };
            Some((
                source_id,
                json!({
                    "id": pause.portable_id,
                    "portable_id": pause.portable_id,
                    "source_type": source_type,
                    "source_id": source_portable_id,
                    "reason": pause.reason,
                    "note": pause.note,
                    "legacy_context": pause.legacy_context,
                    "recorded_by_membership_id": pause.recorded_by_membership_id.map(|id| id.to_string()),
                    "resumed_by_membership_id": pause.resumed_by_membership_id.map(|id| id.to_string()),
                    "recorded_by_name": pause.recorded_by_membership_id.and_then(|id| actor_names.get(&id)),
                    "resumed_by_name": pause.resumed_by_membership_id.and_then(|id| actor_names.get(&id)),
                    "started_at": pause.started_at.map(timestamp),
                    "ended_at": pause.ended_at.map(timestamp),
                    "created_at": timestamp(pause.created_at),
                    "updated_at": timestamp(pause.updated_at)
                }),
            ))
        })
        .collect();
    Ok(SourceContext {
        people,
        medications,
        manageable_people,
        recordable_people,
        eligible_stock,
        pauses,
    })
}

async fn eligible_stock_by_source(
    db: &DatabaseTransaction,
    context: &AuthContext,
    sources: &[SourceRef],
    recordable_people: &HashSet<i64>,
) -> Result<HashMap<i64, Vec<i64>>, ApiError> {
    let actionable: Vec<&SourceRef> = sources
        .iter()
        .filter(|source| {
            recordable_people.contains(&source.person_id) && source.stock_dose.is_some()
        })
        .collect();
    if actionable.is_empty() {
        return Ok(HashMap::new());
    }
    let household_id = context.membership.household_id;
    let original_ids: Vec<i64> = actionable
        .iter()
        .map(|source| source.medication_id)
        .collect();
    let originals: HashMap<i64, medication::Model> = medication::Entity::find()
        .filter(medication::Column::HouseholdId.eq(household_id))
        .filter(medication::Column::Id.is_in(original_ids))
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|row| (row.id, row))
        .collect();
    let names: Vec<String> = originals
        .values()
        .filter_map(|row| row.name.clone())
        .collect();
    if names.is_empty() {
        return Ok(HashMap::new());
    }
    let candidates = crate::scope(household_id, &context.membership)
        .filter(medication::Column::Name.is_in(names))
        .all(db)
        .await
        .map_err(database_error)?;
    if candidates.is_empty() {
        return Ok(HashMap::new());
    }
    let candidate_ids: Vec<i64> = candidates.iter().map(|row| row.id).collect();
    let location_ids: Vec<i64> = candidates.iter().map(|row| row.location_id).collect();
    let locations: HashMap<i64, String> = stock_location::Entity::find()
        .filter(stock_location::Column::HouseholdId.eq(household_id))
        .filter(stock_location::Column::Id.is_in(location_ids))
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|row| (row.id, row.name))
        .collect();
    let option_ids: Vec<i64> = actionable
        .iter()
        .filter_map(|source| source.source_dosage_option_id)
        .collect();
    let options = dosage::Entity::find()
        .filter(
            Condition::any()
                .add(dosage::Column::MedicationId.is_in(candidate_ids))
                .add(dosage::Column::Id.is_in(option_ids)),
        )
        .all(db)
        .await
        .map_err(database_error)?;
    let options_by_id: HashMap<i64, dosage::Model> =
        options.iter().cloned().map(|row| (row.id, row)).collect();
    let mut tracked_by_medication = HashMap::<i64, Vec<dosage::Model>>::new();
    for option in options {
        if option.current_supply.is_some() {
            tracked_by_medication
                .entry(option.medication_id)
                .or_default()
                .push(option);
        }
    }
    let manager = matches!(context.membership.role.as_str(), "owner" | "administrator");
    let mut assigned_by_person = HashMap::<i64, HashSet<i64>>::new();
    if !manager {
        let person_ids: Vec<i64> = actionable.iter().map(|source| source.person_id).collect();
        for row in write_schedule::Entity::find()
            .filter(write_schedule::Column::HouseholdId.eq(household_id))
            .filter(write_schedule::Column::PersonId.is_in(person_ids.clone()))
            .all(db)
            .await
            .map_err(database_error)?
        {
            assigned_by_person
                .entry(row.person_id)
                .or_default()
                .insert(row.medication_id);
        }
        for row in write_assignment::Entity::find()
            .filter(write_assignment::Column::HouseholdId.eq(household_id))
            .filter(write_assignment::Column::PersonId.is_in(person_ids))
            .all(db)
            .await
            .map_err(database_error)?
        {
            assigned_by_person
                .entry(row.person_id)
                .or_default()
                .insert(row.medication_id);
        }
    }
    let mut result = HashMap::new();
    for source in actionable {
        let Some(original) = originals.get(&source.medication_id) else {
            continue;
        };
        let Some((amount, unit)) = source.stock_dose.as_ref() else {
            continue;
        };
        let source_option = source
            .source_dosage_option_id
            .and_then(|id| options_by_id.get(&id));
        if source.source_dosage_option_id.is_some() && source_option.is_none() {
            continue;
        }
        let mut matched: Vec<&medication::Model> = candidates
            .iter()
            .filter(|candidate| {
                if !manager
                    && candidate.id != source.medication_id
                    && !assigned_by_person
                        .get(&source.person_id)
                        .is_some_and(|ids| ids.contains(&candidate.id))
                {
                    return false;
                }
                if !crate::dose::same_stock_signature(candidate, original)
                    || !locations.contains_key(&candidate.location_id)
                {
                    return false;
                }
                let tracked = tracked_by_medication.get(&candidate.id);
                if let Some(tracked) = tracked.filter(|options| !options.is_empty()) {
                    let selected = crate::dose::selected_tracked_dosage(
                        tracked,
                        candidate.id,
                        source_option,
                        Some(*amount),
                        Some(unit),
                    );
                    selected.is_some_and(|option| {
                        option
                            .current_supply
                            .is_some_and(|supply| supply > Decimal::ZERO)
                            && crate::dose::sufficient_stock(
                                option.current_supply,
                                *amount,
                                option.unit.as_deref().unwrap_or(""),
                            )
                    })
                } else {
                    candidate
                        .current_supply
                        .is_none_or(|supply| supply > Decimal::ZERO)
                        && crate::dose::sufficient_stock(candidate.current_supply, *amount, unit)
                }
            })
            .collect();
        matched.sort_by(|left, right| {
            locations
                .get(&left.location_id)
                .cmp(&locations.get(&right.location_id))
                .then(left.id.cmp(&right.id))
        });
        result.insert(source.id, matched.into_iter().map(|row| row.id).collect());
    }
    Ok(result)
}

fn dose_cycle(value: Option<i32>) -> Option<&'static str> {
    match value {
        Some(0) => Some("daily"),
        Some(1) => Some("weekly"),
        Some(2) => Some("monthly"),
        _ => None,
    }
}

fn schedule_type(value: i32) -> &'static str {
    match value {
        1 => "multiple_daily",
        2 => "weekly",
        3 => "specific_dates",
        4 => "prn",
        5 => "tapering",
        6 => "every_other_day",
        _ => "daily",
    }
}

fn timestamp(value: chrono::NaiveDateTime) -> String {
    value.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

async fn serialize_schedules(
    db: &DatabaseTransaction,
    context: &AuthContext,
    records: Vec<schedule::Model>,
) -> Result<Vec<Value>, ApiError> {
    let reference_date = today();
    let sources: Vec<SourceRef> = records
        .iter()
        .map(|record| SourceRef {
            id: record.id,
            person_id: record.person_id,
            medication_id: record.medication_id,
            portable_id: record.portable_id.clone(),
            source_dosage_option_id: record.source_dosage_option_id,
            stock_dose: crate::dose::schedule_stock_dose(record, reference_date),
        })
        .collect();
    let associations = source_context(db, context, &sources, SourceKind::Schedule).await?;
    Ok(records
        .into_iter()
        .map(|record| {
            let active = record.active
                && record.start_date.is_some_and(|start| start <= reference_date)
                && record.end_date.is_some_and(|end| end >= reference_date);
            json!({
                "id": record.id,
                "portable_id": record.portable_id,
                "person_id": record.person_id,
                "person_portable_id": associations.people.get(&record.person_id),
                "medication_id": record.medication_id,
                "medication_portable_id": associations.medications.get(&record.medication_id),
                "dose_amount": record.dose_amount.map(|value| decimal_string(value.to_string())),
                "dose_unit": record.dose_unit,
                "frequency": record.frequency,
                "dose_cycle": dose_cycle(record.dose_cycle),
                "start_date": record.start_date.map(|date| date.to_string()),
                "end_date": record.end_date.map(|date| date.to_string()),
                "active": active,
                "paused": !record.active,
                "can_manage": associations.manageable_people.contains(&record.person_id),
                "can_record": associations.recordable_people.contains(&record.person_id),
                "eligible_stock_medication_ids": associations.eligible_stock.get(&record.id).cloned().unwrap_or_default(),
                "notes": record.notes,
                "updated_at": timestamp(record.updated_at),
                "schedule_type": schedule_type(record.schedule_type),
                "schedule_config": record.schedule_config,
                "max_daily_doses": record.max_daily_doses,
                "min_hours_between_doses": record.min_hours_between_doses.map(|value| decimal_string(value.to_string())),
                "current_pause_period": associations.pauses.get(&record.id)
            })
        })
        .collect())
}

async fn serialize_assignments(
    db: &DatabaseTransaction,
    context: &AuthContext,
    records: Vec<person_medication::Model>,
) -> Result<Vec<Value>, ApiError> {
    let sources: Vec<SourceRef> = records
        .iter()
        .map(|record| SourceRef {
            id: record.id,
            person_id: record.person_id,
            medication_id: record.medication_id,
            portable_id: record.portable_id.clone(),
            source_dosage_option_id: record.source_dosage_option_id,
            stock_dose: if record.active {
                record.dose_amount.zip(record.dose_unit.clone())
            } else {
                None
            },
        })
        .collect();
    let associations = source_context(db, context, &sources, SourceKind::Assignment).await?;
    Ok(records
        .into_iter()
        .map(|record| {
            json!({
                "id": record.id,
                "portable_id": record.portable_id,
                "person_id": record.person_id,
                "person_portable_id": associations.people.get(&record.person_id),
                "medication_id": record.medication_id,
                "medication_portable_id": associations.medications.get(&record.medication_id),
                "dose_amount": record.dose_amount.map(|value| decimal_string(value.to_string())),
                "dose_unit": record.dose_unit,
                "active": record.active,
                "paused": !record.active,
                "can_manage": associations.manageable_people.contains(&record.person_id),
                "can_record": associations.recordable_people.contains(&record.person_id),
                "eligible_stock_medication_ids": associations.eligible_stock.get(&record.id).cloned().unwrap_or_default(),
                "dose_cycle": dose_cycle(record.dose_cycle),
                "administration_kind": if record.administration_kind == 0 { "routine" } else { "as_needed" },
                "notes": record.notes,
                "position": record.position,
                "updated_at": timestamp(record.updated_at),
                "max_daily_doses": record.max_daily_doses,
                "min_hours_between_doses": record.min_hours_between_doses.map(|value| decimal_string(value.to_string())),
                "current_pause_period": associations.pauses.get(&record.id)
            })
        })
        .collect())
}
