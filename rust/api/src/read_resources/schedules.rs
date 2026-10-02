use super::people::{age, today};
use super::response::{
    audited_error_response, collection_response, detail_response, offset, parse_page,
    request_context,
};
use super::source_projection::serialize_schedules;
use crate::read_entities::{person, schedule};
use crate::{database_error, granted_people, ApiError, AppState, AuthContext, Pagination};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use sea_orm::{
    ColumnTrait, DatabaseTransaction, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect,
};

fn schedule_scope(household_id: i64, context: &AuthContext) -> sea_orm::Select<schedule::Entity> {
    schedule::Entity::find()
        .filter(schedule::Column::HouseholdId.eq(household_id))
        .filter(schedule::Column::RetiredAt.is_null())
        .filter(schedule::Column::PersonId.in_subquery(granted_people(&context.membership)))
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

pub(crate) async fn schedules_index(
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

pub(crate) async fn schedules_show(
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
