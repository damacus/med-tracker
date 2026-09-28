use crate::entities::{account, membership, person, platform_admin, security_audit_event};
use crate::medication_management::{
    error_response, finish, finish_with_request_id, household_manager, request_context,
};
use crate::mutation_idempotency::{self, Lookup, StoredResponse};
use crate::{audit, database_error, ApiError, AppState, AuthContext};
use axum::extract::{rejection::JsonRejection, Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

const CONTROLLER: &str = "api/v1/admin/memberships";
const POLICY: &str = "HouseholdMembershipPolicy";

struct Attributes {
    role: Option<String>,
    status: Option<String>,
    person_id: Option<Option<i64>>,
}

impl Attributes {
    fn parse(body: &Value) -> Result<Self, StatusCode> {
        let outer = body.as_object().ok_or(StatusCode::BAD_REQUEST)?;
        let inner = outer
            .get("household_membership")
            .and_then(Value::as_object)
            .ok_or(StatusCode::BAD_REQUEST)?;
        if outer.len() != 1
            || inner.is_empty()
            || inner
                .keys()
                .any(|key| !matches!(key.as_str(), "role" | "status" | "person_id"))
        {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let role = match inner.get("role") {
            None => None,
            Some(Value::String(value))
                if matches!(value.as_str(), "owner" | "administrator" | "member") =>
            {
                Some(value.clone())
            }
            Some(_) => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        };
        let status = match inner.get("status") {
            None => None,
            Some(Value::String(value))
                if matches!(value.as_str(), "active" | "suspended" | "revoked") =>
            {
                Some(value.clone())
            }
            Some(_) => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        };
        let person_id = match inner.get("person_id") {
            None => None,
            Some(Value::Null) => Some(None),
            Some(Value::Number(value)) => match value.as_i64().filter(|value| *value > 0) {
                Some(value) => Some(Some(value)),
                None => return Err(StatusCode::UNPROCESSABLE_ENTITY),
            },
            Some(_) => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        };
        Ok(Self {
            role,
            status,
            person_id,
        })
    }

    fn apply(self, row: &mut membership::Model) {
        if let Some(role) = self.role {
            row.role = role;
        }
        if let Some(status) = self.status {
            if status == "active" {
                row.revoked_at = None;
            }
            row.status = status;
        }
        if let Some(person_id) = self.person_id {
            row.person_id = person_id;
        }
    }
}

fn collection_path(household_id: i64) -> String {
    format!("/api/v1/households/{household_id}/admin/memberships")
}

fn item_path(household_id: i64, id: &str) -> String {
    format!("{}/{id}", collection_path(household_id))
}

fn membership_id(value: &str) -> Option<i64> {
    let mut bytes = value.bytes();
    if !matches!(bytes.next(), Some(b'1'..=b'9')) || !bytes.all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

async fn values(
    db: &DatabaseTransaction,
    rows: &[membership::Model],
) -> Result<Vec<Value>, ApiError> {
    let account_ids: Vec<i64> = rows.iter().map(|row| row.account_id).collect();
    let person_ids: Vec<i64> = rows.iter().filter_map(|row| row.person_id).collect();
    let accounts: HashMap<i64, account::Model> = account::Entity::find()
        .filter(account::Column::Id.is_in(account_ids))
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|row| (row.id, row))
        .collect();
    let people: HashMap<i64, person::Model> = person::Entity::find()
        .filter(person::Column::Id.is_in(person_ids))
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|row| (row.id, row))
        .collect();
    rows.iter()
        .map(|row| {
            let email = &accounts
                .get(&row.account_id)
                .ok_or_else(ApiError::internal)?
                .email;
            Ok(json!({
                "id": row.id,
                "account_id": row.account_id,
                "email": email,
                "person_id": row.person_id,
                "person_name": row.person_id.and_then(|id| people.get(&id).map(|person| &person.name)),
                "role": row.role,
                "status": row.status,
                "permissions_version": row.permissions_version,
                "joined_at": row.joined_at.map(|time| time.and_utc().to_rfc3339()),
            }))
        })
        .collect()
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
    let rows = membership::Entity::find()
        .filter(membership::Column::HouseholdId.eq(household_id))
        .order_by_asc(membership::Column::Id)
        .all(&db)
        .await
        .map_err(database_error)?;
    let data = values(&db, &rows).await?;
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

pub(super) async fn patch(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    change(state, household_id, id, headers, Some(payload), "PATCH").await
}

pub(super) async fn put(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    change(state, household_id, id, headers, Some(payload), "PUT").await
}

pub(super) async fn delete(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    change(state, household_id, id, headers, None, "DELETE").await
}

async fn actor_is_platform_admin(
    db: &DatabaseTransaction,
    context: &AuthContext,
) -> Result<bool, ApiError> {
    Ok(platform_admin::Entity::find()
        .filter(platform_admin::Column::AccountId.eq(context.account_id))
        .filter(platform_admin::Column::Status.eq("active"))
        .one(db)
        .await
        .map_err(database_error)?
        .is_some())
}

async fn record_domain_event(
    db: &DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    previous: &membership::Model,
    next: &membership::Model,
    outcome: &str,
) -> Result<(), ApiError> {
    let now = Utc::now().naive_utc();
    let previous_state =
        json!({"role": previous.role, "status": previous.status, "person_id": previous.person_id});
    let new_state = json!({"role": next.role, "status": next.status, "person_id": next.person_id});
    let event_type = if previous.role != next.role {
        "household_membership.role_updated"
    } else {
        "household_access.membership_changed"
    };
    security_audit_event::ActiveModel {
        household_id: Set(previous.household_id),
        actor_account_id: Set(Some(context.account_id)),
        actor_membership_id: Set(Some(context.membership.id)),
        event_type: Set(event_type.to_owned()),
        request_id: Set(Some(request_id.to_owned())),
        metadata: Set(json!({
            "target_account_id": previous.account_id,
            "target_membership_id": previous.id,
            "previous_role": previous.role,
            "new_role": next.role,
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

async fn failure(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    path: &str,
    status: StatusCode,
    key_digest: Option<(&str, &str)>,
) -> Result<Response, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let body = if status == StatusCode::BAD_REQUEST {
        json!({"error": {"code": "bad_request", "message": "household_membership is required or JSON is invalid", "request_id": request_id}})
    } else {
        json!({"error": {"code": "validation_failed", "message": "Validation failed", "request_id": request_id, "errors": {"household_membership": ["is invalid"]}}})
    };
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
                request_id: &request_id,
                etag: None,
            },
        )
        .await?;
    }
    finish_with_request_id(
        db,
        context,
        &request_id,
        method,
        CONTROLLER,
        POLICY,
        if method == "DELETE" {
            "destroy"
        } else {
            "update"
        },
        status,
        false,
        body,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn rejected_transition(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    path: &str,
    key_digest: Option<(&str, &str)>,
    previous: &membership::Model,
    next: &membership::Model,
    message: &str,
) -> Result<Response, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let body = json!({"error": {"code": "validation_failed", "message": message, "request_id": request_id, "errors": {"base": [message]}}});
    record_domain_event(&db, context, &request_id, previous, next, "rejected").await?;
    if let Some((key, digest)) = key_digest {
        mutation_idempotency::store(
            &db,
            context,
            StoredResponse {
                key,
                method,
                path,
                digest,
                status: StatusCode::UNPROCESSABLE_ENTITY,
                body: body.clone(),
                request_id: &request_id,
                etag: None,
            },
        )
        .await?;
    }
    finish_with_request_id(
        db,
        context,
        &request_id,
        method,
        CONTROLLER,
        POLICY,
        if method == "DELETE" {
            "destroy"
        } else {
            "update"
        },
        StatusCode::UNPROCESSABLE_ENTITY,
        false,
        body,
        None,
    )
    .await
}

async fn change(
    state: AppState,
    household_id: i64,
    id: String,
    headers: HeaderMap,
    payload: Option<Result<Json<Value>, JsonRejection>>,
    method: &str,
) -> Result<Response, ApiError> {
    let action = if method == "DELETE" {
        "destroy"
    } else {
        "update"
    };
    let (db, context) = request_context(&state, &headers, household_id).await?;
    if !household_manager(&context) {
        return error_response(
            db,
            &context,
            method,
            CONTROLLER,
            POLICY,
            action,
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
            None,
        )
        .await;
    }
    let (home, context) = mutation_idempotency::lock_household_and_reauthenticate(
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
            method,
            CONTROLLER,
            POLICY,
            action,
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
            None,
        )
        .await;
    }
    let path = item_path(household_id, &id);
    let Some(numeric_id) = membership_id(&id) else {
        return error_response(
            db,
            &context,
            method,
            CONTROLLER,
            POLICY,
            action,
            StatusCode::NOT_FOUND,
            "not_found",
            "Membership not found",
            None,
        )
        .await;
    };
    let current = membership::Entity::find_by_id(numeric_id)
        .filter(membership::Column::HouseholdId.eq(household_id))
        .lock_exclusive()
        .one(&db)
        .await
        .map_err(database_error)?;
    let Some(current) = current else {
        return error_response(
            db,
            &context,
            method,
            CONTROLLER,
            POLICY,
            action,
            StatusCode::NOT_FOUND,
            "not_found",
            "Membership not found",
            None,
        )
        .await;
    };
    let body = match payload {
        Some(Ok(Json(body))) => body,
        Some(Err(_)) => {
            return failure(db, &context, method, &path, StatusCode::BAD_REQUEST, None).await
        }
        None => Value::Null,
    };
    let digest = mutation_idempotency::digest(method, &path, &body);
    let key = mutation_idempotency::key(&headers);
    if let Some(key) = key {
        match mutation_idempotency::lookup(&db, &context, key, method, &path, &digest).await? {
            Lookup::New => {}
            Lookup::Conflict => {
                return error_response(
                    db,
                    &context,
                    method,
                    CONTROLLER,
                    POLICY,
                    action,
                    StatusCode::CONFLICT,
                    "idempotency_key_reused",
                    "Idempotency key has already been used for a different request",
                    None,
                )
                .await
            }
            Lookup::Replay(saved) => {
                let request_id = Uuid::new_v4().to_string();
                let status = StatusCode::from_u16(saved.response_status as u16)
                    .map_err(|_| ApiError::internal())?;
                audit::record_resource_request_with_id(
                    &db,
                    &context,
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
                db.commit().await.map_err(database_error)?;
                let mut response = mutation_idempotency::replay(*saved)?;
                response.headers_mut().insert(
                    "x-request-id",
                    HeaderValue::from_str(&request_id).map_err(|_| ApiError::internal())?,
                );
                return Ok(response);
            }
        }
    }
    let key_digest = key.map(|key| (key, digest.as_str()));
    let mut next = current.clone();
    if method == "DELETE" {
        next.status = "revoked".to_owned();
        if next.revoked_at.is_none() {
            next.revoked_at = Some(Utc::now().naive_utc());
        }
    } else {
        let attrs = match Attributes::parse(&body) {
            Ok(attrs) => attrs,
            Err(status) => return failure(db, &context, method, &path, status, key_digest).await,
        };
        attrs.apply(&mut next);
    }
    if let Some(person_id) = next.person_id {
        let visible = person::Entity::find_by_id(person_id)
            .filter(person::Column::HouseholdId.eq(household_id))
            .one(&db)
            .await
            .map_err(database_error)?;
        if visible.is_none() {
            return rejected_transition(
                db,
                &context,
                method,
                &path,
                key_digest,
                &current,
                &next,
                "Person must belong to the same household",
            )
            .await;
        }
    }
    let previous_owner = current.role == "owner" && current.status == "active";
    let next_owner = next.role == "owner" && next.status == "active";
    if previous_owner != next_owner {
        let platform_admin = actor_is_platform_admin(&db, &context).await?;
        let authorized = if next_owner {
            platform_admin
        } else {
            context.membership.role == "owner" || platform_admin
        };
        if !authorized {
            return rejected_transition(
                db,
                &context,
                method,
                &path,
                key_digest,
                &current,
                &next,
                "Owner change is not authorized",
            )
            .await;
        }
        if previous_owner && home.status == "active" && home.lifecycle_state == "active" {
            let other_owners = membership::Entity::find()
                .filter(membership::Column::HouseholdId.eq(household_id))
                .filter(membership::Column::Role.eq("owner"))
                .filter(membership::Column::Status.eq("active"))
                .filter(membership::Column::Id.ne(current.id))
                .count(&db)
                .await
                .map_err(database_error)?;
            if other_owners == 0 {
                return rejected_transition(
                    db,
                    &context,
                    method,
                    &path,
                    key_digest,
                    &current,
                    &next,
                    "Last active owner cannot be removed",
                )
                .await;
            }
        }
    }
    let changed = current.role != next.role
        || current.status != next.status
        || current.person_id != next.person_id
        || (method != "DELETE" && current.revoked_at != next.revoked_at);
    let request_id = Uuid::new_v4().to_string();
    if changed {
        next.permissions_version += 1;
        next.updated_at = Utc::now().naive_utc();
        let mut active = current.clone().into_active_model();
        active.role = Set(next.role);
        active.status = Set(next.status);
        active.person_id = Set(next.person_id);
        active.revoked_at = Set(next.revoked_at);
        active.permissions_version = Set(next.permissions_version);
        active.updated_at = Set(next.updated_at);
        next = active.update(&db).await.map_err(database_error)?;
    }
    record_domain_event(
        &db,
        &context,
        &request_id,
        &current,
        &next,
        if changed { "success" } else { "no_change" },
    )
    .await?;
    let status = if method == "DELETE" {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::OK
    };
    let response_body = if method == "DELETE" {
        Value::Null
    } else {
        json!({"data": values(&db, &[next]).await?.remove(0)})
    };
    if let Some((key, digest)) = key_digest {
        mutation_idempotency::store(
            &db,
            &context,
            StoredResponse {
                key,
                method,
                path: &path,
                digest,
                status,
                body: response_body.clone(),
                request_id: &request_id,
                etag: None,
            },
        )
        .await?;
    }
    if method == "DELETE" {
        audit::record_resource_request_with_id(
            &db,
            &context,
            &request_id,
            method,
            CONTROLLER,
            POLICY,
            action,
            status,
            true,
        )
        .await
        .map_err(database_error)?;
        db.commit().await.map_err(database_error)?;
        let mut response = StatusCode::NO_CONTENT.into_response();
        response.headers_mut().insert(
            "x-request-id",
            HeaderValue::from_str(&request_id).map_err(|_| ApiError::internal())?,
        );
        return Ok(response);
    }
    finish_with_request_id(
        db,
        &context,
        &request_id,
        method,
        CONTROLLER,
        POLICY,
        action,
        status,
        true,
        response_body,
        None,
    )
    .await
}
