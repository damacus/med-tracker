use super::response::{
    audited_error_response, collection_response, detail_response, offset, parse_location_page,
    request_context,
};
use crate::read_entities::stock_location;
use crate::{database_error, ApiError, AppState, Pagination};
use axum::extract::{rejection::QueryRejection, Path, Query, State};
use axum::http::HeaderMap;
use axum::response::Response;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};
use serde_json::{json, Value};

pub(crate) fn location_value(location: stock_location::Model) -> Value {
    json!({
        "id": location.id,
        "portable_id": location.portable_id,
        "name": location.name,
        "description": location.description,
        "updated_at": location.updated_at.and_utc().to_rfc3339()
    })
}

pub(crate) async fn locations_index(
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

pub(crate) async fn locations_show(
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
