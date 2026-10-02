use super::access::{manager_context, person_manage_access};
use super::idempotency::{keyed_failure, keyed_replay, store_keyed};
use super::persistence::location;
use super::responses::failure;
use super::validation::{valid_identifier, valid_numeric_id};
use crate::database_error;
use crate::entities::person;
use crate::medication_management::finish_with_request_id;
use crate::medication_management::record_version;
use crate::read_entities::location_membership;
use crate::read_entities::stock_location;
use crate::sync_events::record_change;
use crate::sync_events::SyncRecord;
use crate::ApiError;
use crate::AppState;
use axum::extract::rejection::JsonRejection;
use axum::extract::Path;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::Response;
use axum::Json;
use chrono::Utc;
use sea_orm::ActiveModelTrait;
use sea_orm::ColumnTrait;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::Set;
use serde_json::json;
use serde_json::Value;
use uuid::Uuid;

fn membership_body(
    membership: location_membership::Model,
    location: &stock_location::Model,
    person: &person::Model,
) -> Value {
    json!({"data": {
        "id": membership.id.to_string(),
        "location_id": location.id.to_string(),
        "location_portable_id": location.portable_id,
        "person_id": person.id.to_string(),
        "person_portable_id": person.portable_id,
        "created_at": membership.created_at.and_utc().to_rfc3339()
    }})
}

pub(crate) async fn create_membership(
    State(state): State<AppState>,
    Path((household_id, location_id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    let (db, context) =
        match manager_context(&state, &headers, household_id, "POST", "create_membership").await? {
            Ok(value) => value,
            Err(response) => return Ok(response),
        };
    if !valid_identifier(&location_id) {
        return failure(
            db,
            &context,
            "POST",
            "create_membership",
            StatusCode::BAD_REQUEST,
            "bad_request",
            "Invalid location ID",
        )
        .await;
    }
    let Some(location) = location(&db, household_id, &location_id, true).await? else {
        return failure(
            db,
            &context,
            "POST",
            "create_membership",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
        )
        .await;
    };
    let Json(body) = match payload {
        Ok(body) => body,
        Err(_) => {
            return failure(
                db,
                &context,
                "POST",
                "create_membership",
                StatusCode::BAD_REQUEST,
                "bad_request",
                "Invalid JSON request body",
            )
            .await
        }
    };
    let path =
        format!("/api/v1/households/{household_id}/locations/{location_id}/location_memberships");
    if let Some(response) = keyed_replay(
        &db,
        &context,
        &headers,
        "POST",
        &path,
        &body,
        "create",
        "api/v1/location_memberships",
        "LocationMembershipPolicy",
    )
    .await?
    {
        db.commit().await.map_err(database_error)?;
        return Ok(response);
    }
    let Some(outer) = body.as_object() else {
        return keyed_failure(
            db,
            &context,
            &headers,
            "POST",
            &path,
            &body,
            "api/v1/location_memberships",
            "LocationMembershipPolicy",
            "create",
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            "Validation failed",
            Some(json!({"location_membership": ["is invalid"]})),
        )
        .await;
    };
    let Some(inner) = outer.get("location_membership").and_then(Value::as_object) else {
        return keyed_failure(
            db,
            &context,
            &headers,
            "POST",
            &path,
            &body,
            "api/v1/location_memberships",
            "LocationMembershipPolicy",
            "create",
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            "Validation failed",
            Some(json!({"location_membership": ["is invalid"]})),
        )
        .await;
    };
    if outer.len() != 1 || inner.len() != 1 {
        return keyed_failure(
            db,
            &context,
            &headers,
            "POST",
            &path,
            &body,
            "api/v1/location_memberships",
            "LocationMembershipPolicy",
            "create",
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            "Validation failed",
            Some(json!({"location_membership": ["is invalid"]})),
        )
        .await;
    }
    let Some(person_id) = inner.get("person_id").and_then(Value::as_str) else {
        return keyed_failure(
            db,
            &context,
            &headers,
            "POST",
            &path,
            &body,
            "api/v1/location_memberships",
            "LocationMembershipPolicy",
            "create",
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            "Validation failed",
            Some(json!({"location_membership": ["is invalid"]})),
        )
        .await;
    };
    if !valid_identifier(person_id) {
        return keyed_failure(
            db,
            &context,
            &headers,
            "POST",
            &path,
            &body,
            "api/v1/location_memberships",
            "LocationMembershipPolicy",
            "create",
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            "Validation failed",
            Some(json!({"location_membership": ["is invalid"]})),
        )
        .await;
    }
    let query = person::Entity::find().filter(person::Column::HouseholdId.eq(household_id));
    let query = if let Ok(id) = person_id.parse::<i64>() {
        query.filter(person::Column::Id.eq(id))
    } else {
        query.filter(person::Column::PortableId.eq(person_id))
    };
    let Some(person) = query.one(&db).await.map_err(database_error)? else {
        return failure(
            db,
            &context,
            "POST",
            "create_membership",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
        )
        .await;
    };
    match person_manage_access(&db, &context, person.id).await? {
        None => {
            return failure(
                db,
                &context,
                "POST",
                "create_membership",
                StatusCode::NOT_FOUND,
                "not_found",
                "Record not found",
            )
            .await;
        }
        Some(false) => {
            return failure(
                db,
                &context,
                "POST",
                "create_membership",
                StatusCode::FORBIDDEN,
                "forbidden",
                "You are not authorized to perform this action.",
            )
            .await;
        }
        Some(true) => {}
    }
    let existing = location_membership::Entity::find()
        .filter(location_membership::Column::LocationId.eq(location.id))
        .filter(location_membership::Column::PersonId.eq(person.id))
        .one(&db)
        .await
        .map_err(database_error)?;
    let (membership, created) = if let Some(existing) = existing {
        (existing, false)
    } else {
        let now = Utc::now().naive_utc();
        (
            location_membership::ActiveModel {
                household_id: Set(household_id),
                location_id: Set(location.id),
                person_id: Set(person.id),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(&db)
            .await
            .map_err(database_error)?,
            true,
        )
    };
    let request_id = Uuid::new_v4().to_string();
    if created {
        record_version(&db, &context, &request_id, "LocationMembership", membership.id, "create", None,
            Some(json!({"id":membership.id,"household_id":membership.household_id,"location_id":membership.location_id,"person_id":membership.person_id,"created_at":membership.created_at,"updated_at":membership.updated_at}))).await?;
        let mut active: person::ActiveModel = person.clone().into();
        active.updated_at = Set(Utc::now().naive_utc());
        active.update(&db).await.map_err(database_error)?;
        record_change(
            &db,
            &context,
            &request_id,
            SyncRecord {
                record_type: "Person",
                record_id: person.id,
                portable_id: &person.portable_id,
                action: "update",
                person_portable_id: Some(&person.portable_id),
            },
        )
        .await?;
    }
    let response_body = membership_body(membership, &location, &person);
    store_keyed(
        &db,
        &context,
        &headers,
        "POST",
        &path,
        &body,
        StatusCode::CREATED,
        &response_body,
        &request_id,
        None,
    )
    .await?;
    finish_with_request_id(
        db,
        &context,
        &request_id,
        "POST",
        "api/v1/location_memberships",
        "LocationMembershipPolicy",
        "create",
        StatusCode::CREATED,
        true,
        response_body,
        None,
    )
    .await
}

pub(crate) async fn delete_membership(
    State(state): State<AppState>,
    Path((household_id, location_id, id)): Path<(i64, String, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = match manager_context(
        &state,
        &headers,
        household_id,
        "DELETE",
        "destroy_membership",
    )
    .await?
    {
        Ok(value) => value,
        Err(response) => return Ok(response),
    };
    if !valid_identifier(&location_id) || !valid_numeric_id(&id) || id.parse::<i64>().is_err() {
        return failure(
            db,
            &context,
            "DELETE",
            "destroy_membership",
            StatusCode::BAD_REQUEST,
            "bad_request",
            "Invalid resource ID",
        )
        .await;
    }
    let Some(location) = location(&db, household_id, &location_id, true).await? else {
        return failure(
            db,
            &context,
            "DELETE",
            "destroy_membership",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
        )
        .await;
    };
    let path = format!(
        "/api/v1/households/{household_id}/locations/{location_id}/location_memberships/{id}"
    );
    let request = json!({});
    if let Some(response) = keyed_replay(
        &db,
        &context,
        &headers,
        "DELETE",
        &path,
        &request,
        "destroy",
        "api/v1/location_memberships",
        "LocationMembershipPolicy",
    )
    .await?
    {
        db.commit().await.map_err(database_error)?;
        return Ok(response);
    }
    let member_id = id.parse::<i64>().map_err(|_| ApiError::not_found())?;
    let Some(membership) = location_membership::Entity::find_by_id(member_id)
        .filter(location_membership::Column::HouseholdId.eq(household_id))
        .filter(location_membership::Column::LocationId.eq(location.id))
        .one(&db)
        .await
        .map_err(database_error)?
    else {
        return failure(
            db,
            &context,
            "DELETE",
            "destroy_membership",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
        )
        .await;
    };
    match person_manage_access(&db, &context, membership.person_id).await? {
        None => {
            return failure(
                db,
                &context,
                "DELETE",
                "destroy_membership",
                StatusCode::NOT_FOUND,
                "not_found",
                "Record not found",
            )
            .await;
        }
        Some(false) => {
            return failure(
                db,
                &context,
                "DELETE",
                "destroy_membership",
                StatusCode::FORBIDDEN,
                "forbidden",
                "You are not authorized to perform this action.",
            )
            .await;
        }
        Some(true) => {}
    }
    let person = person::Entity::find_by_id(membership.person_id)
        .one(&db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?;
    let before = json!({"id":membership.id,"household_id":membership.household_id,"location_id":membership.location_id,"person_id":membership.person_id,"created_at":membership.created_at,"updated_at":membership.updated_at});
    let active: location_membership::ActiveModel = membership.into();
    active.delete(&db).await.map_err(database_error)?;
    let request_id = Uuid::new_v4().to_string();
    record_version(
        &db,
        &context,
        &request_id,
        "LocationMembership",
        member_id,
        "destroy",
        Some(before),
        None,
    )
    .await?;
    let mut person_active: person::ActiveModel = person.clone().into();
    person_active.updated_at = Set(Utc::now().naive_utc());
    person_active.update(&db).await.map_err(database_error)?;
    record_change(
        &db,
        &context,
        &request_id,
        SyncRecord {
            record_type: "Person",
            record_id: person.id,
            portable_id: &person.portable_id,
            action: "update",
            person_portable_id: Some(&person.portable_id),
        },
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
        "api/v1/location_memberships",
        "LocationMembershipPolicy",
        "destroy",
        StatusCode::NO_CONTENT,
        true,
        response_body,
        None,
    )
    .await
}
