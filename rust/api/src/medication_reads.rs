use crate::entities::{grant, medication, membership, person_medication, schedule};
use crate::{
    audit, authenticate, database_error, if_none_match_matches, representation_etag,
    serialize_many, ApiError, AppState, CredentialKind, Pagination,
};
use axum::extract::{rejection::QueryRejection, Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Utc};
use sea_orm::{
    ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
    QueryTrait, TransactionTrait,
};
use serde_json::json;

pub(super) fn granted_people(
    membership: &membership::Model,
) -> sea_orm::sea_query::SelectStatement {
    grant::Entity::find()
        .select_only()
        .column(grant::Column::PersonId)
        .filter(grant::Column::HouseholdId.eq(membership.household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(membership.id))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(grant::Column::AccessLevel.is_in(["view", "record", "manage"]))
        .filter(
            Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(grant::Column::ExpiresAt.gt(Utc::now().naive_utc())),
        )
        .into_query()
}

pub(super) fn scope(
    household_id: i64,
    membership: &membership::Model,
) -> sea_orm::Select<medication::Entity> {
    let query = medication::Entity::find().filter(medication::Column::HouseholdId.eq(household_id));
    if membership.role == "owner" || membership.role == "administrator" {
        return query;
    }
    let granted_schedules = schedule::Entity::find()
        .select_only()
        .column(schedule::Column::MedicationId)
        .filter(schedule::Column::HouseholdId.eq(household_id))
        .filter(schedule::Column::PersonId.in_subquery(granted_people(membership)))
        .into_query();
    let granted_assignments = person_medication::Entity::find()
        .select_only()
        .column(person_medication::Column::MedicationId)
        .filter(person_medication::Column::HouseholdId.eq(household_id))
        .filter(person_medication::Column::PersonId.in_subquery(granted_people(membership)))
        .into_query();
    let linked_schedules = schedule::Entity::find()
        .select_only()
        .column(schedule::Column::MedicationId)
        .filter(schedule::Column::HouseholdId.eq(household_id))
        .into_query();
    let linked_assignments = person_medication::Entity::find()
        .select_only()
        .column(person_medication::Column::MedicationId)
        .filter(person_medication::Column::HouseholdId.eq(household_id))
        .into_query();
    query.filter(
        Condition::any()
            .add(medication::Column::Id.in_subquery(granted_schedules))
            .add(medication::Column::Id.in_subquery(granted_assignments))
            .add(
                Condition::all()
                    .add(medication::Column::CreatedByMembershipId.eq(membership.id))
                    .add(medication::Column::Id.not_in_subquery(linked_schedules))
                    .add(medication::Column::Id.not_in_subquery(linked_assignments)),
            ),
    )
}

pub(super) async fn index(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    pagination: Result<Query<Pagination>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let db = state.db.begin().await.map_err(database_error)?;
    let context = match authenticate(&state, &db, &headers, household_id).await {
        Ok(context) => context,
        Err(error) => {
            if error.preserve_activity {
                db.commit().await.map_err(database_error)?;
            }
            return Err(error);
        }
    };
    let pagination = match pagination {
        Ok(Query(pagination)) => pagination,
        Err(_) => {
            if matches!(context.credential_kind, CredentialKind::OauthGrant) {
                db.commit().await.map_err(database_error)?;
            }
            return Err(ApiError::invalid_pagination());
        }
    };
    if pagination.page.is_some_and(|page| page < 1)
        || pagination
            .per_page
            .is_some_and(|per_page| !(1..=100).contains(&per_page))
    {
        if matches!(context.credential_kind, CredentialKind::OauthGrant) {
            db.commit().await.map_err(database_error)?;
        }
        return Err(ApiError::invalid_pagination());
    }
    if pagination.updated_since.as_deref() == Some("") {
        if matches!(context.credential_kind, CredentialKind::OauthGrant) {
            db.commit().await.map_err(database_error)?;
        }
        return Err(ApiError::invalid_filter());
    }
    let page = pagination.page.unwrap_or(1);
    let per_page = pagination.per_page.unwrap_or(20);
    let updated_since = pagination
        .updated_since
        .filter(|value| !value.is_empty())
        .map(|value| DateTime::parse_from_rfc3339(&value).map_err(|_| ApiError::invalid_filter()))
        .transpose();
    let updated_since = match updated_since {
        Ok(value) => value,
        Err(error) => {
            if matches!(context.credential_kind, CredentialKind::OauthGrant) {
                db.commit().await.map_err(database_error)?;
            }
            return Err(error);
        }
    }
    .map(|value| value.naive_utc());
    let mut query = scope(household_id, &context.membership);
    if let Some(updated_since) = updated_since {
        query = query.filter(medication::Column::UpdatedAt.gte(updated_since));
    }
    let total = query.clone().count(&db).await.map_err(database_error)?;
    let records = query
        .order_by_asc(medication::Column::Id)
        .limit(per_page as u64)
        .offset(((page - 1) * per_page) as u64)
        .all(&db)
        .await
        .map_err(database_error)?;
    let data = serialize_many(&db, records).await?;
    let request_id = audit::record_medication_read(&db, &context, "index", StatusCode::OK, true)
        .await
        .map_err(database_error)?;
    db.commit().await.map_err(database_error)?;
    let mut response = Json(
        json!({"data": data, "meta": {"page": page, "per_page": per_page, "total_count": total}}),
    )
    .into_response();
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&request_id).map_err(|_| ApiError::internal())?,
    );
    Ok(response)
}

pub(super) async fn show(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let db = state.db.begin().await.map_err(database_error)?;
    let context = match authenticate(&state, &db, &headers, household_id).await {
        Ok(context) => context,
        Err(error) => {
            if error.preserve_activity {
                db.commit().await.map_err(database_error)?;
            }
            return Err(error);
        }
    };
    let query = scope(household_id, &context.membership);
    let query = match id.parse::<i64>() {
        Ok(id) => query.filter(medication::Column::Id.eq(id)),
        Err(_) => query.filter(medication::Column::PortableId.eq(id)),
    };
    let record = query.one(&db).await.map_err(database_error)?;
    let Some(record) = record else {
        audit::record_medication_read(&db, &context, "show", StatusCode::NOT_FOUND, false)
            .await
            .map_err(database_error)?;
        db.commit().await.map_err(database_error)?;
        return Err(ApiError::not_found());
    };
    let data = serialize_many(&db, vec![record]).await?.remove(0);
    let body = json!({"data": data});
    let etag = representation_etag(&body);
    if if_none_match_matches(&headers, &etag) {
        audit::record_medication_read(&db, &context, "show", StatusCode::NOT_MODIFIED, true)
            .await
            .map_err(database_error)?;
        db.commit().await.map_err(database_error)?;
        let mut response = StatusCode::NOT_MODIFIED.into_response();
        response.headers_mut().insert(
            header::ETAG,
            HeaderValue::from_str(&etag).map_err(|_| ApiError::internal())?,
        );
        return Ok(response);
    }
    audit::record_medication_read(&db, &context, "show", StatusCode::OK, true)
        .await
        .map_err(database_error)?;
    db.commit().await.map_err(database_error)?;
    let mut response = Json(body).into_response();
    response.headers_mut().insert(
        header::ETAG,
        HeaderValue::from_str(&etag).map_err(|_| ApiError::internal())?,
    );
    Ok(response)
}
