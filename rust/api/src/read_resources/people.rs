use super::response::{
    audited_error_response, collection_response, detail_response, offset, parse_location_page,
    request_context, timestamp,
};
use crate::read_entities::{location_membership, notification_preference, person};
use crate::{database_error, granted_people, ApiError, AppState, AuthContext, Pagination};
use axum::extract::{rejection::QueryRejection, Path, Query, State};
use axum::http::HeaderMap;
use axum::response::Response;
use chrono::{Datelike, NaiveDate, Utc};
use sea_orm::{
    ColumnTrait, DatabaseTransaction, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect,
};
use serde_json::{json, Value};
use std::collections::HashMap;

fn person_scope(household_id: i64, context: &AuthContext) -> sea_orm::Select<person::Entity> {
    person::Entity::find()
        .filter(person::Column::HouseholdId.eq(household_id))
        .filter(person::Column::Id.in_subquery(granted_people(&context.membership)))
}

pub(crate) fn today() -> NaiveDate {
    let timezone = std::env::var("TZ")
        .ok()
        .and_then(|value| value.parse::<chrono_tz::Tz>().ok())
        .unwrap_or(chrono_tz::UTC);
    Utc::now().with_timezone(&timezone).date_naive()
}

pub(crate) fn age(birth_date: Option<NaiveDate>, reference: NaiveDate) -> Option<i32> {
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

pub(crate) async fn people_index(
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

pub(crate) async fn people_show(
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

pub(crate) async fn serialize_people(
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
