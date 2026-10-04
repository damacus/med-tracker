use crate::audit;
use crate::auth_sessions;
use crate::entities::{api_app_token, grant, membership, person, security_audit_event, version};
use crate::medication_management::{
    error_response, finish_with_request_id, household_manager, request_context,
};
use crate::mutation_idempotency::{self, Lookup, StoredResponse};
use crate::{database_error, ApiError, AppState, AuthContext, CredentialKind};
use axum::body::Body;
use axum::extract::{rejection::JsonRejection, Path, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::Response;
use axum::Json;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Months, NaiveDateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseTransaction, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, QueryTrait, Set,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const CONTROLLER: &str = "api/v1/admin/app_tokens";
const POLICY: &str = "HouseholdPolicy";

fn path(household_id: i64) -> String {
    format!("/api/v1/households/{household_id}/admin/app_tokens")
}

fn token_id(value: &str) -> Option<i64> {
    let mut bytes = value.bytes();
    if !matches!(bytes.next(), Some(b'1'..=b'9')) || !bytes.all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

fn summary(row: &api_app_token::Model) -> Value {
    json!({
        "id": row.id,
        "name": row.name,
        "last_used_at": row.last_used_at.and_utc().to_rfc3339(),
        "expires_at": row.expires_at.and_utc().to_rfc3339(),
        "revoked_at": row.revoked_at.map(|value| value.and_utc().to_rfc3339()),
        "permissions_version": row.permissions_version,
    })
}

fn household_memberships(household_id: i64) -> sea_orm::sea_query::SelectStatement {
    membership::Entity::find()
        .select_only()
        .column(membership::Column::Id)
        .filter(membership::Column::HouseholdId.eq(household_id))
        .into_query()
}

async fn may_manage_own_tokens(
    db: &DatabaseTransaction,
    context: &AuthContext,
) -> Result<bool, ApiError> {
    if !matches!(context.credential_kind, CredentialKind::BrowserSession) {
        return Ok(false);
    }
    let Some(person_id) = context.membership.person_id else {
        return Ok(false);
    };
    let person = person::Entity::find_by_id(person_id)
        .filter(person::Column::HouseholdId.eq(context.membership.household_id))
        .filter(person::Column::AccountId.eq(context.account_id))
        .one(db)
        .await
        .map_err(database_error)?;
    if person.is_none() {
        return Ok(false);
    }
    let now = Utc::now().naive_utc();
    Ok(grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(context.membership.household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(context.membership.id))
        .filter(grant::Column::PersonId.eq(person_id))
        .filter(grant::Column::AccessLevel.eq("manage"))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(
            Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(grant::Column::ExpiresAt.gt(now)),
        )
        .one(db)
        .await
        .map_err(database_error)?
        .is_some())
}

async fn may_access_tokens(
    db: &DatabaseTransaction,
    context: &AuthContext,
) -> Result<bool, ApiError> {
    if household_manager(context) {
        return Ok(true);
    }
    may_manage_own_tokens(db, context).await
}

async fn token_for_account(
    db: &DatabaseTransaction,
    context: &AuthContext,
    id: i64,
) -> Result<Option<api_app_token::Model>, ApiError> {
    api_app_token::Entity::find_by_id(id)
        .filter(api_app_token::Column::AccountId.eq(context.account_id))
        .filter(
            api_app_token::Column::HouseholdMembershipId
                .in_subquery(household_memberships(context.membership.household_id)),
        )
        .one(db)
        .await
        .map_err(database_error)
}

async fn denied(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    action: &str,
) -> Result<Response, ApiError> {
    error_response(
        db,
        context,
        method,
        CONTROLLER,
        POLICY,
        action,
        StatusCode::FORBIDDEN,
        "forbidden",
        "You are not authorized to perform this action.",
        None,
    )
    .await
}

fn attributes(
    body: &Value,
    now: NaiveDateTime,
) -> Result<(String, NaiveDateTime), (StatusCode, &'static str, &'static str)> {
    let outer =
        body.as_object()
            .ok_or((StatusCode::BAD_REQUEST, "api_app_token", "is required"))?;
    let inner = outer
        .get("api_app_token")
        .and_then(Value::as_object)
        .ok_or((StatusCode::BAD_REQUEST, "api_app_token", "is required"))?;
    if outer.len() != 1
        || inner
            .keys()
            .any(|key| !matches!(key.as_str(), "name" | "expires_at"))
    {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            "api_app_token",
            "contains an unsupported field",
        ));
    }
    let name = inner
        .get("name")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or((StatusCode::UNPROCESSABLE_ENTITY, "name", "can't be blank"))?
        .to_owned();
    let months = auth_sessions::app_age_months().ok_or((
        StatusCode::UNPROCESSABLE_ENTITY,
        "expires_at",
        "configured maximum age is invalid",
    ))?;
    let maximum = now.checked_add_months(Months::new(months)).ok_or((
        StatusCode::UNPROCESSABLE_ENTITY,
        "expires_at",
        "configured maximum age is invalid",
    ))?;
    let expiry = match inner.get("expires_at") {
        None => maximum,
        Some(Value::String(value)) => DateTime::parse_from_rfc3339(value)
            .map(|value| value.with_timezone(&Utc).naive_utc())
            .map_err(|_| {
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "expires_at",
                    "must be an RFC3339 timestamp",
                )
            })?,
        Some(_) => {
            return Err((
                StatusCode::UNPROCESSABLE_ENTITY,
                "expires_at",
                "must be an RFC3339 timestamp",
            ))
        }
    };
    if expiry <= now || expiry > maximum {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            "expires_at",
            "must be after issuance and within the configured maximum age",
        ));
    }
    Ok((name, expiry))
}

fn audit_metadata(context: &AuthContext, action: &str, name: &str) -> Value {
    json!({
        "account_id": context.account_id,
        "token_type": "api_app_token",
        "action": action,
        "device_name_present": !name.trim().is_empty(),
        "device_name_length": name.chars().count(),
    })
}

async fn record_token_event(
    db: &DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    action: &str,
    name: &str,
) -> Result<(), ApiError> {
    let now = Utc::now().naive_utc();
    let event = format!("auth_token/api_app_token/{action}");
    let metadata = audit_metadata(context, action, name);
    let audit_context = json!({
        "actor_account_id": context.account_id,
        "actor_user_id": context.user_id,
        "actor_membership_id": context.membership.id,
        "household_id": context.membership.household_id,
        "request_id": request_id,
    });
    version::ActiveModel {
        item_type: Set("AuthenticationToken".to_owned()),
        item_id: Set(context.account_id),
        event: Set(event.clone()),
        object: Set(Some(metadata.to_string())),
        object_changes: Set(None),
        whodunnit: Set(Some(context.user_id.to_string())),
        request_id: Set(Some(request_id.to_owned())),
        household_id: Set(Some(context.membership.household_id)),
        actor_membership_id: Set(Some(context.membership.id)),
        audit_context: Set(audit_context.clone()),
        created_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(database_error)?;
    security_audit_event::ActiveModel {
        household_id: Set(context.membership.household_id),
        actor_account_id: Set(Some(context.account_id)),
        actor_membership_id: Set(Some(context.membership.id)),
        event_type: Set(event),
        request_id: Set(Some(request_id.to_owned())),
        metadata: Set(metadata),
        audit_context: Set(audit_context),
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
async fn failure(
    db: DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    method: &str,
    action: &str,
    path: &str,
    request: &Value,
    status: StatusCode,
    code: &str,
    message: &str,
    errors: Option<Value>,
) -> Result<Response, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let mut body = json!({"error": {"code": code, "message": message}});
    if let Some(errors) = errors {
        body["error"]["errors"] = errors;
    }
    if let Some(key) = mutation_idempotency::key(headers) {
        let mut stored = body.clone();
        stored["error"]["request_id"] = json!(request_id);
        let digest = mutation_idempotency::digest(method, path, request);
        mutation_idempotency::store(
            &db,
            context,
            StoredResponse {
                key,
                method,
                path,
                digest: &digest,
                status,
                body: stored,
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
        action,
        status,
        false,
        body,
        None,
    )
    .await
}

pub(super) async fn index(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    if !may_access_tokens(&db, &context).await? {
        return denied(db, &context, "GET", "index").await;
    }
    let rows = api_app_token::Entity::find()
        .filter(api_app_token::Column::AccountId.eq(context.account_id))
        .filter(
            api_app_token::Column::HouseholdMembershipId
                .in_subquery(household_memberships(household_id)),
        )
        .order_by_asc(api_app_token::Column::Id)
        .all(&db)
        .await
        .map_err(database_error)?;
    let data: Vec<Value> = rows.iter().map(summary).collect();
    let request_id = Uuid::new_v4().to_string();
    finish_with_request_id(
        db,
        &context,
        &request_id,
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
    let (db, _) = request_context(&state, &headers, household_id).await?;
    let (_, context) = mutation_idempotency::lock_household_and_reauthenticate(
        &state,
        &db,
        &headers,
        household_id,
    )
    .await?;
    if !may_access_tokens(&db, &context).await? {
        return denied(db, &context, "POST", "create").await;
    }
    let Json(body) = match payload {
        Ok(value) => value,
        Err(_) => {
            return error_response(
                db,
                &context,
                "POST",
                CONTROLLER,
                POLICY,
                "create",
                StatusCode::BAD_REQUEST,
                "bad_request",
                "Invalid request body",
                None,
            )
            .await
        }
    };
    let path = path(household_id);
    if let Some(key) = mutation_idempotency::key(&headers) {
        let digest = mutation_idempotency::digest("POST", &path, &body);
        match mutation_idempotency::lookup(&db, &context, key, "POST", &path, &digest).await? {
            Lookup::New => {}
            Lookup::Conflict => {
                return failure(
                    db,
                    &context,
                    &headers,
                    "POST",
                    "create",
                    &path,
                    &body,
                    StatusCode::CONFLICT,
                    "idempotency_key_reused",
                    "Idempotency key has already been used for a different request",
                    None,
                )
                .await
            }
            Lookup::Replay(saved) => {
                let request_id = Uuid::new_v4().to_string();
                let response = if saved.response_status == 201 {
                    json!({"error": {"code": "token_already_issued", "message": "Token was already issued; create with a new idempotency key if its one-time secret was lost"}})
                } else {
                    let mut response = mutation_idempotency::replay(*saved)?;
                    let status = response.status();
                    audit::record_resource_request_with_id(
                        &db,
                        &context,
                        &request_id,
                        "POST",
                        CONTROLLER,
                        POLICY,
                        "create",
                        status,
                        false,
                    )
                    .await
                    .map_err(database_error)?;
                    db.commit().await.map_err(database_error)?;
                    response.headers_mut().insert(
                        "x-request-id",
                        request_id.parse().map_err(|_| ApiError::internal())?,
                    );
                    return Ok(response);
                };
                return finish_with_request_id(
                    db,
                    &context,
                    &request_id,
                    "POST",
                    CONTROLLER,
                    POLICY,
                    "create",
                    StatusCode::CONFLICT,
                    false,
                    response,
                    None,
                )
                .await;
            }
        }
    }
    let now = Utc::now().naive_utc();
    let (name, expires_at) = match attributes(&body, now) {
        Ok(values) => values,
        Err((status, field, message)) => {
            return failure(
                db,
                &context,
                &headers,
                "POST",
                "create",
                &path,
                &body,
                status,
                if status == StatusCode::BAD_REQUEST {
                    "bad_request"
                } else {
                    "validation_failed"
                },
                if status == StatusCode::BAD_REQUEST {
                    "Invalid request body"
                } else {
                    "Validation failed"
                },
                (status == StatusCode::UNPROCESSABLE_ENTITY).then(|| json!({field: [message]})),
            )
            .await
        }
    };
    let mut bytes = [0u8; 48];
    getrandom::fill(&mut bytes).map_err(|_| ApiError::internal())?;
    let raw = format!("mt_app_{}", URL_SAFE_NO_PAD.encode(bytes));
    let digest = hex::encode(Sha256::digest(raw.as_bytes()));
    let row = api_app_token::ActiveModel {
        account_id: Set(context.account_id),
        household_membership_id: Set(context.membership.id),
        token_digest: Set(digest),
        permissions_version: Set(context.membership.permissions_version),
        name: Set(name.clone()),
        expires_at: Set(expires_at),
        revoked_at: Set(None),
        last_used_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .map_err(database_error)?;
    let request_id = Uuid::new_v4().to_string();
    record_token_event(&db, &context, &request_id, "created", &name).await?;
    if let Some(key) = mutation_idempotency::key(&headers) {
        let digest = mutation_idempotency::digest("POST", &path, &body);
        mutation_idempotency::store(
            &db,
            &context,
            StoredResponse {
                key,
                method: "POST",
                path: &path,
                digest: &digest,
                status: StatusCode::CREATED,
                body: json!({"receipt": {"token_id": row.id}}),
                request_id: &request_id,
                etag: None,
            },
        )
        .await?;
    }
    let mut data = summary(&row);
    data["token"] = json!(raw);
    let mut response = finish_with_request_id(
        db,
        &context,
        &request_id,
        "POST",
        CONTROLLER,
        POLICY,
        "create",
        StatusCode::CREATED,
        true,
        json!({"data": data}),
        None,
    )
    .await?;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok(response)
}

pub(super) async fn destroy(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, _) = request_context(&state, &headers, household_id).await?;
    let (_, context) = mutation_idempotency::lock_household_and_reauthenticate(
        &state,
        &db,
        &headers,
        household_id,
    )
    .await?;
    if !may_access_tokens(&db, &context).await? {
        return denied(db, &context, "DELETE", "destroy").await;
    }
    let path = format!("{}/{id}", path(household_id));
    let request = json!({});
    if let Some(key) = mutation_idempotency::key(&headers) {
        let digest = mutation_idempotency::digest("DELETE", &path, &request);
        match mutation_idempotency::lookup(&db, &context, key, "DELETE", &path, &digest).await? {
            Lookup::New => {}
            Lookup::Conflict => {
                return failure(
                    db,
                    &context,
                    &headers,
                    "DELETE",
                    "destroy",
                    &path,
                    &request,
                    StatusCode::CONFLICT,
                    "idempotency_key_reused",
                    "Idempotency key has already been used for a different request",
                    None,
                )
                .await
            }
            Lookup::Replay(saved) => {
                let request_id = Uuid::new_v4().to_string();
                let mut response = mutation_idempotency::replay(*saved)?;
                audit::record_resource_request_with_id(
                    &db,
                    &context,
                    &request_id,
                    "DELETE",
                    CONTROLLER,
                    POLICY,
                    "destroy",
                    response.status(),
                    response.status().is_success(),
                )
                .await
                .map_err(database_error)?;
                db.commit().await.map_err(database_error)?;
                response.headers_mut().insert(
                    "x-request-id",
                    request_id.parse().map_err(|_| ApiError::internal())?,
                );
                return Ok(response);
            }
        }
    }
    let row = match token_id(&id) {
        Some(id) => token_for_account(&db, &context, id).await?,
        None => None,
    };
    let Some(row) = row else {
        return failure(
            db,
            &context,
            &headers,
            "DELETE",
            "destroy",
            &path,
            &request,
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
            None,
        )
        .await;
    };
    let request_id = Uuid::new_v4().to_string();
    if row.revoked_at.is_none() {
        let mut active: api_app_token::ActiveModel = row.clone().into();
        let now = Utc::now().naive_utc();
        active.revoked_at = Set(Some(now));
        active.updated_at = Set(now);
        active.update(&db).await.map_err(database_error)?;
        record_token_event(&db, &context, &request_id, "revoked", &row.name).await?;
    }
    if let Some(key) = mutation_idempotency::key(&headers) {
        let digest = mutation_idempotency::digest("DELETE", &path, &request);
        mutation_idempotency::store(
            &db,
            &context,
            StoredResponse {
                key,
                method: "DELETE",
                path: &path,
                digest: &digest,
                status: StatusCode::NO_CONTENT,
                body: json!({}),
                request_id: &request_id,
                etag: None,
            },
        )
        .await?;
    }
    let mut response = finish_with_request_id(
        db,
        &context,
        &request_id,
        "DELETE",
        CONTROLLER,
        POLICY,
        "destroy",
        StatusCode::NO_CONTENT,
        true,
        json!({}),
        None,
    )
    .await?;
    *response.body_mut() = Body::empty();
    response.headers_mut().remove(header::CONTENT_TYPE);
    Ok(response)
}
