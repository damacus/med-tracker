use crate::{
    audit, authenticate, database_error, if_none_match_matches, representation_etag, ApiError,
    AppState, AuthContext, Pagination,
};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::DateTime;
use sea_orm::{DatabaseTransaction, TransactionTrait};
use serde_json::{json, Value};

pub(super) struct Page {
    pub(super) number: i64,
    pub(super) size: i64,
    pub(super) updated_since: Option<chrono::NaiveDateTime>,
}

pub(super) async fn request_context(
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

pub(super) fn parse_page(
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

pub(super) fn parse_location_page(
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

pub(super) async fn audited_error_response(
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

pub(super) fn offset(page: &Page) -> u64 {
    page.number.saturating_sub(1).saturating_mul(page.size) as u64
}

pub(super) async fn collection_response(
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

pub(super) async fn detail_response(
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

pub(super) fn timestamp(value: chrono::NaiveDateTime) -> String {
    value.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}
