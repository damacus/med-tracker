use super::access::manager_context;
use super::cascade::{delete_dependents, record_cascade_effects};
use super::idempotency::{keyed_failure, keyed_replay, store_keyed};
use super::persistence::{location, location_change, representation, snapshot};
use super::responses::failure;
use super::validation::valid_identifier;
use crate::database_error;
use crate::medication_management::finish_with_request_id;
use crate::ApiError;
use crate::AppState;
use axum::extract::Path;
use axum::extract::State;
use axum::http::header;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::Response;
use sea_orm::TransactionTrait;
use serde_json::json;
use uuid::Uuid;

pub(crate) async fn delete(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) =
        match manager_context(&state, &headers, household_id, "DELETE", "destroy").await? {
            Ok(value) => value,
            Err(response) => return Ok(response),
        };
    let path = format!("/api/v1/households/{household_id}/locations/{id}");
    let request = json!({});
    if let Some(response) = keyed_replay(
        &db,
        &context,
        &headers,
        "DELETE",
        &path,
        &request,
        "destroy",
        "api/v1/locations",
        "LocationPolicy",
    )
    .await?
    {
        db.commit().await.map_err(database_error)?;
        return Ok(response);
    }
    if !valid_identifier(&id) {
        return keyed_failure(
            db,
            &context,
            &headers,
            "DELETE",
            &path,
            &request,
            "api/v1/locations",
            "LocationPolicy",
            "destroy",
            StatusCode::BAD_REQUEST,
            "bad_request",
            "Invalid resource ID",
            None,
        )
        .await;
    }
    let Some(record) = location(&db, household_id, &id, true).await? else {
        return failure(
            db,
            &context,
            "DELETE",
            "destroy",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
        )
        .await;
    };
    let Some(if_match) = headers
        .get(header::IF_MATCH)
        .and_then(|value| value.to_str().ok())
    else {
        return keyed_failure(
            db,
            &context,
            &headers,
            "DELETE",
            &path,
            &request,
            "api/v1/locations",
            "LocationPolicy",
            "destroy",
            StatusCode::PRECONDITION_REQUIRED,
            "precondition_required",
            "If-Match is required",
            None,
        )
        .await;
    };
    if if_match.is_empty() {
        return keyed_failure(
            db,
            &context,
            &headers,
            "DELETE",
            &path,
            &request,
            "api/v1/locations",
            "LocationPolicy",
            "destroy",
            StatusCode::PRECONDITION_REQUIRED,
            "precondition_required",
            "If-Match is required",
            None,
        )
        .await;
    }
    let (_, current_etag) = representation(record.clone());
    if if_match != current_etag {
        return failure(
            db,
            &context,
            "DELETE",
            "destroy",
            StatusCode::CONFLICT,
            "conflict",
            "Record has changed since it was last read",
        )
        .await;
    }
    let savepoint = db.begin().await.map_err(database_error)?;
    let cascade = match delete_dependents(&savepoint, &record).await {
        Ok(Some(cascade)) => {
            savepoint.commit().await.map_err(database_error)?;
            cascade
        }
        Ok(None) => {
            savepoint.rollback().await.map_err(database_error)?;
            return keyed_failure(
                db,
                &context,
                &headers,
                "DELETE",
                &path,
                &request,
                "api/v1/locations",
                "LocationPolicy",
                "destroy",
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_failed",
                "Location cannot be deleted while administration history exists",
                None,
            )
            .await;
        }
        Err(error)
            if matches!(
                error.sql_err(),
                Some(sea_orm::SqlErr::ForeignKeyConstraintViolation(_))
            ) =>
        {
            savepoint.rollback().await.map_err(database_error)?;
            return keyed_failure(
                db,
                &context,
                &headers,
                "DELETE",
                &path,
                &request,
                "api/v1/locations",
                "LocationPolicy",
                "destroy",
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_failed",
                "Location cannot be deleted while retained records exist",
                None,
            )
            .await;
        }
        Err(error) => return Err(database_error(error)),
    };
    let request_id = Uuid::new_v4().to_string();
    record_cascade_effects(&db, &context, &request_id, &cascade).await?;
    location_change(
        &db,
        &context,
        &request_id,
        &record,
        "destroy",
        "delete",
        Some(snapshot(&record)),
        None,
    )
    .await?;
    let response_body = json!({});
    store_keyed(
        &db,
        &context,
        &headers,
        "DELETE",
        &path,
        &request,
        StatusCode::NO_CONTENT,
        &response_body,
        &request_id,
        None,
    )
    .await?;
    finish_with_request_id(
        db,
        &context,
        &request_id,
        "DELETE",
        "api/v1/locations",
        "LocationPolicy",
        "destroy",
        StatusCode::NO_CONTENT,
        true,
        response_body,
        None,
    )
    .await
}
