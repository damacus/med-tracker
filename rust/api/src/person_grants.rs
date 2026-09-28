use crate::entities::{grant, membership, person, security_audit_event};
use crate::medication_management::{
    error_response, finish, finish_with_request_id, household_manager, request_context,
};
use crate::mutation_idempotency::{self, Lookup, StoredResponse};
use crate::{audit, database_error, ApiError, AppState, AuthContext};
use axum::body::Body;
use axum::extract::{rejection::JsonRejection, Path, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

const CONTROLLER: &str = "api/v1/admin/person_access_grants";
const POLICY: &str = "PersonAccessGrantPolicy";

struct Attributes {
    membership_id: i64,
    person_id: i64,
    access_level: String,
    relationship_type: String,
    expires_at: Option<chrono::NaiveDateTime>,
}

impl Attributes {
    fn attempted_state(&self) -> Value {
        json!({
            "household_membership_id": self.membership_id,
            "person_id": self.person_id,
            "access_level": self.access_level,
            "relationship_type": self.relationship_type,
            "expires_at": self.expires_at.map(|time| time.and_utc().to_rfc3339()),
            "revoked_at": Value::Null,
            "carer_relationship_id": Value::Null,
        })
    }

    fn parse(body: &Value) -> Result<Self, StatusCode> {
        let outer = body.as_object().ok_or(StatusCode::BAD_REQUEST)?;
        let inner = outer
            .get("person_access_grant")
            .and_then(Value::as_object)
            .ok_or(StatusCode::BAD_REQUEST)?;
        if outer.len() != 1
            || inner.keys().any(|key| {
                !matches!(
                    key.as_str(),
                    "household_membership_id"
                        | "person_id"
                        | "access_level"
                        | "relationship_type"
                        | "expires_at"
                )
            })
        {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let numeric = |name: &str| {
            inner
                .get(name)
                .and_then(Value::as_i64)
                .filter(|value| *value > 0)
                .ok_or(StatusCode::UNPROCESSABLE_ENTITY)
        };
        let access_level = inner
            .get("access_level")
            .and_then(Value::as_str)
            .filter(|value| matches!(*value, "view" | "record" | "manage"))
            .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?
            .to_owned();
        let relationship_type = inner
            .get("relationship_type")
            .and_then(Value::as_str)
            .filter(|value| {
                matches!(
                    *value,
                    "self" | "parent" | "family_member" | "carer" | "professional"
                )
            })
            .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?
            .to_owned();
        let expires_at = match inner.get("expires_at") {
            None | Some(Value::Null) => None,
            Some(Value::String(value)) => Some(
                DateTime::parse_from_rfc3339(value)
                    .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?
                    .naive_utc(),
            ),
            Some(_) => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        };
        Ok(Self {
            membership_id: numeric("household_membership_id")?,
            person_id: numeric("person_id")?,
            access_level,
            relationship_type,
            expires_at,
        })
    }
}

fn collection_path(household_id: i64) -> String {
    format!("/api/v1/households/{household_id}/admin/person_access_grants")
}

fn item_path(household_id: i64, id: &str) -> String {
    format!("{}/{id}", collection_path(household_id))
}

fn numeric_id(value: &str) -> Option<i64> {
    let mut bytes = value.bytes();
    if !matches!(bytes.next(), Some(b'1'..=b'9')) || !bytes.all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

fn grant_state(row: &grant::Model) -> Value {
    json!({
        "household_membership_id": row.household_membership_id,
        "person_id": row.person_id,
        "access_level": row.access_level,
        "relationship_type": row.relationship_type,
        "expires_at": row.expires_at.map(|time| time.and_utc().to_rfc3339()),
        "revoked_at": row.revoked_at.map(|time| time.and_utc().to_rfc3339()),
        "carer_relationship_id": row.carer_relationship_id,
    })
}

async fn representation(
    db: &DatabaseTransaction,
    rows: &[grant::Model],
) -> Result<Vec<Value>, ApiError> {
    let people: HashMap<i64, person::Model> = person::Entity::find()
        .filter(person::Column::Id.is_in(rows.iter().map(|row| row.person_id).collect::<Vec<_>>()))
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|row| (row.id, row))
        .collect();
    rows.iter()
        .map(|row| {
            let person_name = &people
                .get(&row.person_id)
                .ok_or_else(ApiError::internal)?
                .name;
            Ok(json!({
                "id": row.id,
                "household_membership_id": row.household_membership_id,
                "person_id": row.person_id,
                "person_name": person_name,
                "access_level": row.access_level,
                "relationship_type": row.relationship_type,
                "expires_at": row.expires_at.map(|time| time.and_utc().to_rfc3339()),
                "revoked_at": row.revoked_at.map(|time| time.and_utc().to_rfc3339()),
            }))
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
async fn record_domain_event(
    db: &DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    target_membership_id: Option<i64>,
    target_grant_id: Option<i64>,
    previous_state: Option<Value>,
    new_state: Value,
    outcome: &str,
) -> Result<(), ApiError> {
    let now = Utc::now().naive_utc();
    security_audit_event::ActiveModel {
        household_id: Set(context.membership.household_id),
        actor_account_id: Set(Some(context.account_id)),
        actor_membership_id: Set(Some(context.membership.id)),
        event_type: Set("household_access.person_grant_changed".to_owned()),
        request_id: Set(Some(request_id.to_owned())),
        metadata: Set(json!({
            "target_membership_id": target_membership_id,
            "target_grant_id": target_grant_id,
            "previous_state": previous_state,
            "new_state": new_state,
            "outcome": outcome,
        })),
        audit_context: Set(json!({})),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(database_error)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn response(
    db: DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    method: &str,
    action: &str,
    path: &str,
    key_digest: Option<(&str, &str)>,
    status: StatusCode,
    body: Value,
) -> Result<Response, ApiError> {
    if let Some((key, digest)) = key_digest {
        mutation_idempotency::store(
            &db,
            context,
            StoredResponse {
                key,
                method,
                path,
                digest,
                status,
                body: body.clone(),
                request_id,
                etag: None,
            },
        )
        .await?;
    }
    let mut result = finish_with_request_id(
        db,
        context,
        request_id,
        method,
        CONTROLLER,
        POLICY,
        action,
        status,
        status.is_success(),
        body,
        None,
    )
    .await?;
    if status == StatusCode::NO_CONTENT {
        *result.body_mut() = Body::empty();
        result.headers_mut().remove(header::CONTENT_TYPE);
    }
    Ok(result)
}

#[allow(clippy::too_many_arguments)]
async fn failure(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    action: &str,
    path: &str,
    key_digest: Option<(&str, &str)>,
    status: StatusCode,
    message: &str,
    target_membership_id: Option<i64>,
    attempted_state: Option<Value>,
    field_errors: Option<Value>,
) -> Result<Response, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    if status == StatusCode::UNPROCESSABLE_ENTITY {
        record_domain_event(
            &db,
            context,
            &request_id,
            target_membership_id,
            None,
            None,
            attempted_state.unwrap_or_else(|| json!({})),
            "rejected",
        )
        .await?;
    }
    let body = if status == StatusCode::BAD_REQUEST {
        json!({"error": {"code": "bad_request", "message": message, "request_id": request_id}})
    } else {
        json!({"error": {"code": "validation_failed", "message": message, "request_id": request_id, "errors": field_errors.unwrap_or_else(|| json!({"base": [message]}))}})
    };
    response(
        db,
        context,
        &request_id,
        method,
        action,
        path,
        key_digest,
        status,
        body,
    )
    .await
}

async fn key_lookup(
    db: &DatabaseTransaction,
    context: &AuthContext,
    key: Option<&str>,
    method: &str,
    path: &str,
    digest: &str,
    action: &str,
) -> Result<Option<Response>, ApiError> {
    let Some(key) = key else {
        return Ok(None);
    };
    match mutation_idempotency::lookup(db, context, key, method, path, digest).await? {
        Lookup::New => Ok(None),
        Lookup::Conflict => {
            let request_id = Uuid::new_v4().to_string();
            let body = json!({"error": {"code": "idempotency_key_reused", "message": "Idempotency key has already been used for a different request", "request_id": request_id}});
            audit::record_resource_request_with_id(
                db,
                context,
                &request_id,
                method,
                CONTROLLER,
                POLICY,
                action,
                StatusCode::CONFLICT,
                false,
            )
            .await
            .map_err(database_error)?;
            let mut result = (StatusCode::CONFLICT, Json(body)).into_response();
            result.headers_mut().insert(
                "x-request-id",
                HeaderValue::from_str(&request_id).map_err(|_| ApiError::internal())?,
            );
            Ok(Some(result))
        }
        Lookup::Replay(saved) => {
            let request_id = Uuid::new_v4().to_string();
            let status = StatusCode::from_u16(saved.response_status as u16)
                .map_err(|_| ApiError::internal())?;
            audit::record_resource_request_with_id(
                db,
                context,
                &request_id,
                method,
                CONTROLLER,
                POLICY,
                action,
                status,
                status.is_success(),
            )
            .await
            .map_err(database_error)?;
            let mut result = mutation_idempotency::replay(*saved)?;
            result.headers_mut().insert(
                "x-request-id",
                HeaderValue::from_str(&request_id).map_err(|_| ApiError::internal())?,
            );
            Ok(Some(result))
        }
    }
}

pub(super) async fn index(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    if !household_manager(&context) {
        return error_response(
            db,
            &context,
            "GET",
            CONTROLLER,
            POLICY,
            "index",
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
            None,
        )
        .await;
    }
    let rows = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(household_id))
        .order_by_asc(grant::Column::Id)
        .all(&db)
        .await
        .map_err(database_error)?;
    let data = representation(&db, &rows).await?;
    finish(
        db,
        &context,
        "GET",
        CONTROLLER,
        POLICY,
        "index",
        StatusCode::OK,
        true,
        json!({"data": data}),
        None,
    )
    .await
}

pub(super) async fn create(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    if !household_manager(&context) {
        return error_response(
            db,
            &context,
            "POST",
            CONTROLLER,
            POLICY,
            "create",
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
            None,
        )
        .await;
    }
    let (_, context) = mutation_idempotency::lock_household_and_reauthenticate(
        &state,
        &db,
        &headers,
        household_id,
    )
    .await?;
    if !household_manager(&context) {
        return error_response(
            db,
            &context,
            "POST",
            CONTROLLER,
            POLICY,
            "create",
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
            None,
        )
        .await;
    }
    let path = collection_path(household_id);
    let body = match payload {
        Ok(Json(body)) => body,
        Err(_) => {
            return failure(
                db,
                &context,
                "POST",
                "create",
                &path,
                None,
                StatusCode::BAD_REQUEST,
                "person_access_grant is required or JSON is invalid",
                None,
                None,
                None,
            )
            .await
        }
    };
    let digest = mutation_idempotency::digest("POST", &path, &body);
    let key = mutation_idempotency::key(&headers);
    if let Some(result) = key_lookup(&db, &context, key, "POST", &path, &digest, "create").await? {
        db.commit().await.map_err(database_error)?;
        return Ok(result);
    }
    let key_digest = key.map(|key| (key, digest.as_str()));
    let attrs = match Attributes::parse(&body) {
        Ok(attrs) => attrs,
        Err(status) => {
            return failure(
                db,
                &context,
                "POST",
                "create",
                &path,
                key_digest,
                status,
                "Person access grant is invalid",
                None,
                None,
                body["person_access_grant"]["access_level"]
                    .as_str()
                    .filter(|value| !matches!(*value, "view" | "record" | "manage"))
                    .map(|_| json!({"access_level": ["is invalid"]})),
            )
            .await
        }
    };
    let target = membership::Entity::find_by_id(attrs.membership_id)
        .filter(membership::Column::HouseholdId.eq(household_id))
        .lock_exclusive()
        .one(&db)
        .await
        .map_err(database_error)?;
    let selected_person = person::Entity::find_by_id(attrs.person_id)
        .filter(person::Column::HouseholdId.eq(household_id))
        .one(&db)
        .await
        .map_err(database_error)?;
    if target.is_none() || selected_person.is_none() {
        return failure(
            db,
            &context,
            "POST",
            "create",
            &path,
            key_digest,
            StatusCode::UNPROCESSABLE_ENTITY,
            "Grant records must belong to the household",
            Some(attrs.membership_id),
            Some(attrs.attempted_state()),
            None,
        )
        .await;
    }
    let target = target.ok_or_else(ApiError::internal)?;
    let duplicate = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(attrs.membership_id))
        .filter(grant::Column::PersonId.eq(attrs.person_id))
        .filter(grant::Column::RevokedAt.is_null())
        .one(&db)
        .await
        .map_err(database_error)?;
    if duplicate.is_some() {
        return failure(
            db,
            &context,
            "POST",
            "create",
            &path,
            key_digest,
            StatusCode::UNPROCESSABLE_ENTITY,
            "Grant already exists for membership and person",
            Some(attrs.membership_id),
            Some(attrs.attempted_state()),
            Some(json!({"household_membership_id": ["has already been taken"]})),
        )
        .await;
    }
    let now = Utc::now().naive_utc();
    let row = grant::ActiveModel {
        household_id: Set(household_id),
        household_membership_id: Set(attrs.membership_id),
        person_id: Set(attrs.person_id),
        access_level: Set(attrs.access_level),
        relationship_type: Set(attrs.relationship_type),
        expires_at: Set(attrs.expires_at),
        revoked_at: Set(None),
        granted_by_membership_id: Set(Some(context.membership.id)),
        carer_relationship_id: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .map_err(database_error)?;
    let mut active: membership::ActiveModel = target.clone().into();
    active.permissions_version = Set(target.permissions_version + 1);
    active.updated_at = Set(now);
    active.update(&db).await.map_err(database_error)?;
    let request_id = Uuid::new_v4().to_string();
    record_domain_event(
        &db,
        &context,
        &request_id,
        Some(target.id),
        Some(row.id),
        None,
        grant_state(&row),
        "success",
    )
    .await?;
    let data = representation(&db, &[row]).await?.remove(0);
    response(
        db,
        &context,
        &request_id,
        "POST",
        "create",
        &path,
        key_digest,
        StatusCode::CREATED,
        json!({"data": data}),
    )
    .await
}

pub(super) async fn destroy(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    if !household_manager(&context) {
        return error_response(
            db,
            &context,
            "DELETE",
            CONTROLLER,
            POLICY,
            "destroy",
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
            None,
        )
        .await;
    }
    let (_, context) = mutation_idempotency::lock_household_and_reauthenticate(
        &state,
        &db,
        &headers,
        household_id,
    )
    .await?;
    if !household_manager(&context) {
        return error_response(
            db,
            &context,
            "DELETE",
            CONTROLLER,
            POLICY,
            "destroy",
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
            None,
        )
        .await;
    }
    let Some(id_value) = numeric_id(&id) else {
        return error_response(
            db,
            &context,
            "DELETE",
            CONTROLLER,
            POLICY,
            "destroy",
            StatusCode::NOT_FOUND,
            "not_found",
            "Person access grant not found",
            None,
        )
        .await;
    };
    let row = grant::Entity::find_by_id(id_value)
        .filter(grant::Column::HouseholdId.eq(household_id))
        .lock_exclusive()
        .one(&db)
        .await
        .map_err(database_error)?;
    let Some(row) = row else {
        return error_response(
            db,
            &context,
            "DELETE",
            CONTROLLER,
            POLICY,
            "destroy",
            StatusCode::NOT_FOUND,
            "not_found",
            "Person access grant not found",
            None,
        )
        .await;
    };
    let path = item_path(household_id, &id);
    let digest = mutation_idempotency::digest("DELETE", &path, &Value::Null);
    let key = mutation_idempotency::key(&headers);
    if let Some(result) =
        key_lookup(&db, &context, key, "DELETE", &path, &digest, "destroy").await?
    {
        db.commit().await.map_err(database_error)?;
        return Ok(result);
    }
    let key_digest = key.map(|key| (key, digest.as_str()));
    if row.carer_relationship_id.is_some() {
        let request_id = Uuid::new_v4().to_string();
        record_domain_event(
            &db,
            &context,
            &request_id,
            Some(row.household_membership_id),
            Some(row.id),
            Some(grant_state(&row)),
            grant_state(&row),
            "rejected",
        )
        .await?;
        return response(
            db,
            &context,
            &request_id,
            "DELETE",
            "destroy",
            &path,
            key_digest,
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({"error": {"code": "validation_failed", "message": "Relationship-owned grants must be revoked through their carer relationship", "request_id": request_id, "errors": {"base": ["Relationship-owned grants must be revoked through their carer relationship"]}}}),
        )
        .await;
    }
    let request_id = Uuid::new_v4().to_string();
    let previous = grant_state(&row);
    let updated = if row.revoked_at.is_none() {
        let now = Utc::now().naive_utc();
        let mut active: grant::ActiveModel = row.clone().into();
        active.revoked_at = Set(Some(now));
        active.updated_at = Set(now);
        let updated = active.update(&db).await.map_err(database_error)?;
        let target = membership::Entity::find_by_id(row.household_membership_id)
            .filter(membership::Column::HouseholdId.eq(household_id))
            .lock_exclusive()
            .one(&db)
            .await
            .map_err(database_error)?
            .ok_or_else(ApiError::internal)?;
        let mut target_active: membership::ActiveModel = target.clone().into();
        target_active.permissions_version = Set(target.permissions_version + 1);
        target_active.updated_at = Set(now);
        target_active.update(&db).await.map_err(database_error)?;
        updated
    } else {
        row.clone()
    };
    let outcome = if row.revoked_at.is_none() {
        "success"
    } else {
        "no_change"
    };
    record_domain_event(
        &db,
        &context,
        &request_id,
        Some(row.household_membership_id),
        Some(row.id),
        Some(previous),
        grant_state(&updated),
        outcome,
    )
    .await?;
    response(
        db,
        &context,
        &request_id,
        "DELETE",
        "destroy",
        &path,
        key_digest,
        StatusCode::NO_CONTENT,
        Value::Null,
    )
    .await
}
