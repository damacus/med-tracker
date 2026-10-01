use crate::entities::{medication, membership};
use crate::read_entities::{
    location_membership, notification_preference, pause_period, person, person_medication,
    schedule, stock_location,
};
use crate::{
    audit, authenticate, database_error, decimal_string, granted_people, if_none_match_matches,
    representation_etag, ApiError, AppState, AuthContext, Pagination,
};
use axum::extract::{rejection::QueryRejection, Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Datelike, NaiveDate, Utc};
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

fn parse_location_page(
    db: DatabaseTransaction,
    pagination: Pagination,
) -> Result<(DatabaseTransaction, Page), (DatabaseTransaction, ApiError)> {
    if pagination.page.is_some_and(|page| page < 1)
        || pagination
            .per_page
            .is_some_and(|per_page| !(1..=100).contains(&per_page))
    {
        return Err((db, ApiError::invalid_pagination()));
    }
    if pagination.updated_since.as_deref() == Some("") {
        return Err((db, ApiError::invalid_filter()));
    }
    parse_page(db, pagination)
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

pub(super) fn location_value(location: stock_location::Model) -> Value {
    json!({
        "id": location.id,
        "portable_id": location.portable_id,
        "name": location.name,
        "description": location.description,
        "updated_at": location.updated_at.and_utc().to_rfc3339()
    })
}

pub(super) async fn locations_index(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    pagination: Result<Query<Pagination>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let pagination = match pagination {
        Ok(Query(pagination)) => pagination,
        Err(_) => {
            return audited_error_response(
                db,
                &context,
                "api/v1/locations",
                "LocationPolicy",
                "index",
                ApiError::invalid_pagination(),
                true,
            )
            .await;
        }
    };
    let (db, page) = match parse_location_page(db, pagination) {
        Ok(value) => value,
        Err((db, error)) => {
            return audited_error_response(
                db,
                &context,
                "api/v1/locations",
                "LocationPolicy",
                "index",
                error,
                true,
            )
            .await;
        }
    };
    let mut query =
        stock_location::Entity::find().filter(stock_location::Column::HouseholdId.eq(household_id));
    if let Some(updated_since) = page.updated_since {
        query = query.filter(stock_location::Column::UpdatedAt.gte(updated_since));
    }
    let total = query.clone().count(&db).await.map_err(database_error)?;
    let rows = query
        .order_by_asc(stock_location::Column::Id)
        .limit(page.size as u64)
        .offset(offset(&page))
        .all(&db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(location_value)
        .collect();
    collection_response(
        db,
        &context,
        "api/v1/locations",
        "LocationPolicy",
        rows,
        page,
        total,
    )
    .await
}

pub(super) async fn locations_show(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let mut query =
        stock_location::Entity::find().filter(stock_location::Column::HouseholdId.eq(household_id));
    query = if let Ok(numeric_id) = id.parse::<i64>() {
        query.filter(stock_location::Column::Id.eq(numeric_id))
    } else {
        query.filter(stock_location::Column::PortableId.eq(id))
    };
    let row = query
        .one(&db)
        .await
        .map_err(database_error)?
        .map(location_value);
    detail_response(
        db,
        &context,
        "api/v1/locations",
        "LocationPolicy",
        row,
        &headers,
    )
    .await
}

pub(super) fn today() -> NaiveDate {
    let timezone = std::env::var("TZ")
        .ok()
        .and_then(|value| value.parse::<chrono_tz::Tz>().ok())
        .unwrap_or(chrono_tz::UTC);
    Utc::now().with_timezone(&timezone).date_naive()
}

pub(super) fn age(birth_date: Option<NaiveDate>, reference: NaiveDate) -> Option<i32> {
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
    pagination: Result<Query<Pagination>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let pagination = match pagination {
        Ok(Query(pagination)) => pagination,
        Err(_) => {
            return audited_error_response(
                db,
                &context,
                "api/v1/people",
                "PersonPolicy",
                "index",
                ApiError::invalid_pagination(),
                true,
            )
            .await;
        }
    };
    let (db, page) = match parse_location_page(db, pagination) {
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
        Err(_) => {
            person_scope(household_id, &context)
                .filter(person::Column::PortableId.eq(&id))
                .one(&db)
                .await
        }
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
    pagination: Result<Query<Pagination>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let pagination = match pagination {
        Ok(Query(value)) => value,
        Err(_) => {
            return audited_error_response(
                db,
                &context,
                "api/v1/person_medications",
                "PersonMedicationPolicy",
                "index",
                ApiError::invalid_pagination(),
                true,
            )
            .await;
        }
    };
    let (db, page) = match parse_location_page(db, pagination) {
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

pub(super) async fn serialize_people(
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
    let source_medications: HashMap<i64, medication::Model> = medication::Entity::find()
        .filter(medication::Column::Id.is_in(medication_ids))
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|medication| (medication.id, medication))
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
    let (recordable_people, manageable_people) = source_permissions(&permission_grants);
    let eligible_stock = source_stock(
        db,
        context,
        sources,
        &source_medications,
        &recordable_people,
    )
    .await?;
    let medications = source_medications
        .into_iter()
        .map(|(id, medication)| (id, medication.portable_id))
        .collect();
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

fn source_permissions(grants: &[crate::entities::grant::Model]) -> (HashSet<i64>, HashSet<i64>) {
    let recordable = grants
        .iter()
        .filter(|grant| matches!(grant.access_level.as_str(), "record" | "manage"))
        .map(|grant| grant.person_id)
        .collect();
    let manageable = grants
        .iter()
        .filter(|grant| grant.access_level == "manage")
        .map(|grant| grant.person_id)
        .collect();
    (recordable, manageable)
}

async fn source_stock(
    db: &DatabaseTransaction,
    context: &AuthContext,
    sources: &[SourceRef],
    medications: &HashMap<i64, medication::Model>,
    recordable_people: &HashSet<i64>,
) -> Result<HashMap<i64, Vec<i64>>, ApiError> {
    let recordable_sources: Vec<&SourceRef> = sources
        .iter()
        .filter(|source| recordable_people.contains(&source.person_id))
        .collect();
    if recordable_sources.is_empty() {
        return Ok(HashMap::new());
    }
    let household_id = context.membership.household_id;
    let all_stock = matches!(context.membership.role.as_str(), "owner" | "administrator");
    let mut linked_stock: HashMap<i64, HashSet<i64>> = HashMap::new();
    if !all_stock {
        let person_ids: Vec<i64> = recordable_sources
            .iter()
            .map(|source| source.person_id)
            .collect();
        let assignments: Vec<(i64, i64)> = person_medication::Entity::find()
            .select_only()
            .columns([
                person_medication::Column::PersonId,
                person_medication::Column::MedicationId,
            ])
            .filter(person_medication::Column::HouseholdId.eq(household_id))
            .filter(person_medication::Column::PersonId.is_in(person_ids.clone()))
            .into_tuple()
            .all(db)
            .await
            .map_err(database_error)?;
        let schedules: Vec<(i64, i64)> = schedule::Entity::find()
            .select_only()
            .columns([schedule::Column::PersonId, schedule::Column::MedicationId])
            .filter(schedule::Column::HouseholdId.eq(household_id))
            .filter(schedule::Column::PersonId.is_in(person_ids))
            .into_tuple()
            .all(db)
            .await
            .map_err(database_error)?;
        for (person_id, medication_id) in assignments.into_iter().chain(schedules) {
            linked_stock
                .entry(person_id)
                .or_default()
                .insert(medication_id);
        }
    }
    let mut names = Condition::any();
    for source in &recordable_sources {
        if let Some(original) = medications.get(&source.medication_id) {
            names = names.add(match &original.name {
                Some(name) => medication::Column::Name.eq(name),
                None => medication::Column::Name.is_null(),
            });
        }
    }
    let mut query = crate::scope(household_id, &context.membership).filter(names);
    if !all_stock {
        let candidate_ids: HashSet<i64> = linked_stock
            .values()
            .flat_map(|ids| ids.iter().copied())
            .chain(recordable_sources.iter().map(|source| source.medication_id))
            .collect();
        query = query.filter(medication::Column::Id.is_in(candidate_ids));
    }
    let candidates = query.all(db).await.map_err(database_error)?;
    Ok(recordable_sources
        .into_iter()
        .filter_map(|source| {
            let original = medications.get(&source.medication_id)?;
            let ids = eligible_stock_ids(
                household_id,
                all_stock,
                original,
                &candidates,
                linked_stock.get(&source.person_id),
            );
            Some((source.id, ids))
        })
        .collect())
}

fn eligible_stock_ids(
    household_id: i64,
    all_stock: bool,
    original: &medication::Model,
    candidates: &[medication::Model],
    linked_stock: Option<&HashSet<i64>>,
) -> Vec<i64> {
    let mut ids: Vec<i64> = candidates
        .iter()
        .filter(|candidate| {
            candidate.household_id == household_id
                && (all_stock
                    || candidate.id == original.id
                    || linked_stock.is_some_and(|ids| ids.contains(&candidate.id)))
                && crate::dose::same_stock_signature(candidate, original)
                && candidate
                    .current_supply
                    .is_none_or(|supply| supply > sea_orm::prelude::Decimal::ZERO)
        })
        .map(|candidate| candidate.id)
        .collect();
    ids.sort_unstable();
    ids
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

pub(super) async fn serialize_schedules(
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
        })
        .collect();
    let associations = source_context(db, context, &sources, SourceKind::Schedule).await?;
    Ok(records
        .into_iter()
        .map(|record| schedule_row(record, &associations, reference_date))
        .collect())
}

fn schedule_row(
    record: schedule::Model,
    associations: &SourceContext,
    reference_date: NaiveDate,
) -> Value {
    let active = record.active
        && record
            .start_date
            .is_some_and(|start| start <= reference_date)
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
}

pub(super) async fn serialize_assignments(
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
        })
        .collect();
    let associations = source_context(db, context, &sources, SourceKind::Assignment).await?;
    Ok(records
        .into_iter()
        .map(|record| assignment_row(record, &associations))
        .collect())
}

fn assignment_row(record: person_medication::Model, associations: &SourceContext) -> Value {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::prelude::Decimal;

    fn assignment() -> person_medication::Model {
        person_medication::Model {
            id: 1,
            household_id: 2,
            portable_id: "assignment".into(),
            person_id: 3,
            medication_id: 10,
            source_dosage_option_id: None,
            dose_amount: Some(Decimal::ONE),
            dose_unit: Some("tablet".into()),
            active: true,
            notes: None,
            updated_at: Utc::now().naive_utc(),
            dose_cycle: Some(0),
            administration_kind: 0,
            position: 0,
            max_daily_doses: None,
            min_hours_between_doses: None,
            retired_at: None,
        }
    }

    fn scheduled() -> schedule::Model {
        schedule::Model {
            id: 1,
            household_id: 2,
            portable_id: "schedule".into(),
            person_id: 3,
            medication_id: 10,
            source_dosage_option_id: None,
            dose_amount: Some(Decimal::ONE),
            dose_unit: Some("tablet".into()),
            frequency: Some("daily".into()),
            dose_cycle: Some(0),
            start_date: Some(today()),
            end_date: Some(today()),
            active: true,
            notes: None,
            updated_at: Utc::now().naive_utc(),
            schedule_type: 0,
            schedule_config: json!({}),
            max_daily_doses: None,
            min_hours_between_doses: None,
            retired_at: None,
        }
    }

    fn associations() -> SourceContext {
        SourceContext {
            people: HashMap::from([(3, "person".into())]),
            medications: HashMap::from([(10, "medication".into())]),
            manageable_people: HashSet::from([3]),
            pauses: HashMap::new(),
            recordable_people: HashSet::from([3]),
            eligible_stock: HashMap::from([(1, vec![10])]),
        }
    }

    #[test]
    fn assignment_projection_supplies_dose_recording_permission() {
        let row = assignment_row(assignment(), &associations());
        assert_eq!(row["can_record"], true);
    }

    #[test]
    fn schedule_projection_supplies_dose_recording_permission() {
        let row = schedule_row(scheduled(), &associations(), today());
        assert_eq!(row["can_record"], true);
    }

    #[test]
    fn source_projections_supply_eligible_stock_inventory() {
        let associations = associations();
        for row in [
            assignment_row(assignment(), &associations),
            schedule_row(scheduled(), &associations, today()),
        ] {
            assert_eq!(row["eligible_stock_medication_ids"], json!([10]));
        }
    }

    fn grant(person_id: i64, access_level: &str) -> crate::entities::grant::Model {
        crate::entities::grant::Model {
            id: person_id,
            household_id: 2,
            household_membership_id: 4,
            person_id,
            access_level: access_level.into(),
            expires_at: None,
            revoked_at: None,
            relationship_type: "carer".into(),
            granted_by_membership_id: None,
            carer_relationship_id: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        }
    }

    #[test]
    fn source_permissions_keep_record_only_manage_and_view_grants_distinct() {
        let (recordable, manageable) =
            source_permissions(&[grant(3, "record"), grant(5, "manage"), grant(6, "view")]);
        assert_eq!(recordable, HashSet::from([3, 5]));
        assert_eq!(manageable, HashSet::from([5]));
        for (person_id, can_record, can_manage) in [
            (3, true, false),
            (5, true, true),
            (6, false, false),
            (7, false, false),
        ] {
            let mut associations = associations();
            associations.recordable_people = recordable.clone();
            associations.manageable_people = manageable.clone();
            let mut assignment = assignment();
            assignment.person_id = person_id;
            let mut schedule = scheduled();
            schedule.person_id = person_id;
            for row in [
                assignment_row(assignment, &associations),
                schedule_row(schedule, &associations, today()),
            ] {
                assert_eq!(row["can_record"], can_record);
                assert_eq!(row["can_manage"], can_manage);
            }
        }
    }

    fn medication(id: i64) -> medication::Model {
        medication::Model {
            id,
            household_id: 2,
            created_at: Utc::now().naive_utc(),
            created_by_membership_id: Some(4),
            portable_id: format!("medication-{id}"),
            name: Some("Paracetamol".into()),
            friendly_name: None,
            category: None,
            description: None,
            barcode: None,
            dmd_code: None,
            dmd_system: None,
            dmd_concept_class: None,
            dose_amount: Some(500.0),
            dose_unit: Some("mg".into()),
            current_supply: Some(Decimal::ONE),
            supply_at_last_restock: None,
            reorder_threshold: Decimal::ZERO,
            reorder_status: None,
            order_supplier: None,
            order_quantity: None,
            expected_arrival_on: None,
            ordered_at: None,
            reordered_at: None,
            warnings: None,
            default_schedule_type: 0,
            location_id: 8,
            updated_at: Utc::now().naive_utc(),
        }
    }

    fn stock_candidates() -> Vec<medication::Model> {
        let mut candidates: Vec<_> = (10..=19).map(medication).collect();
        candidates[2].household_id = 9;
        candidates[3].name = Some("Ibuprofen".into());
        candidates[4].dose_amount = Some(250.0);
        candidates[5].dose_unit = Some("ml".into());
        candidates[6].current_supply = Some(Decimal::ZERO);
        candidates[7].current_supply = Some(-Decimal::ONE);
        candidates[8].current_supply = None;
        candidates
    }

    #[test]
    fn owner_stock_excludes_foreign_mismatched_and_exhausted_inventory() {
        let candidates = stock_candidates();
        assert_eq!(
            eligible_stock_ids(2, true, &candidates[0], &candidates, None),
            vec![10, 11, 18, 19]
        );
    }

    #[test]
    fn delegated_stock_uses_only_source_person_links_and_original_inventory() {
        let candidates = stock_candidates();
        let links = HashSet::from([11, 12, 13, 14, 15, 16, 17, 18]);
        assert_eq!(
            eligible_stock_ids(2, false, &candidates[0], &candidates, Some(&links)),
            vec![10, 11, 18]
        );
        assert_eq!(
            eligible_stock_ids(2, false, &candidates[0], &candidates, None),
            vec![10]
        );
    }

    #[test]
    fn missing_stock_projection_serializes_an_empty_inventory_list() {
        let mut associations = associations();
        associations.eligible_stock.clear();
        for row in [
            assignment_row(assignment(), &associations),
            schedule_row(scheduled(), &associations, today()),
        ] {
            assert_eq!(row["eligible_stock_medication_ids"], json!([]));
        }
    }
}
