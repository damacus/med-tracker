use super::response::{
    audited_error_response, collection_response, detail_response, offset, parse_location_page,
    request_context,
};
use super::source_projection::serialize_assignments;
use crate::read_entities::person_medication;
use crate::{database_error, granted_people, ApiError, AppState, AuthContext, Pagination};
use axum::extract::{rejection::QueryRejection, Path, Query, State};
use axum::http::HeaderMap;
use axum::response::Response;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};

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

pub(crate) async fn person_medications_index(
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

pub(crate) async fn person_medications_show(
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
