use crate::entities::{household, security_audit_event};
use crate::medication_management::{
    error_response, finish, finish_with_request_id, household_manager, record_version,
    request_context,
};
use crate::mutation_idempotency::{self, Lookup, StoredResponse};
use crate::{database_error, ApiError, AppState, AuthContext};
use axum::extract::{rejection::JsonRejection, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseTransaction, EntityTrait, IntoActiveModel, Set};
use serde_json::{json, Value};
use uuid::Uuid;

const CONTROLLER: &str = "api/v1/admin/settings";
const POLICY: &str = "HouseholdPolicy";

struct Attributes {
    name: Option<String>,
    timezone: Option<String>,
    subscription_plan: Option<String>,
}

impl Attributes {
    fn parse(body: &Value) -> Result<Self, StatusCode> {
        let outer = body.as_object().ok_or(StatusCode::BAD_REQUEST)?;
        let inner = outer.get("household").ok_or(StatusCode::BAD_REQUEST)?;
        let inner = inner.as_object().ok_or(StatusCode::BAD_REQUEST)?;
        if outer.len() != 1
            || inner.is_empty()
            || inner
                .keys()
                .any(|key| !matches!(key.as_str(), "name" | "timezone" | "subscription_plan"))
        {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let text = |name: &str| -> Result<Option<String>, StatusCode> {
            match inner.get(name) {
                None => Ok(None),
                Some(Value::String(value)) if !value.trim().is_empty() => Ok(Some(value.clone())),
                Some(_) => Err(StatusCode::UNPROCESSABLE_ENTITY),
            }
        };
        let subscription_plan = text("subscription_plan")?;
        if subscription_plan
            .as_deref()
            .is_some_and(|value| !matches!(value, "free" | "family_plus"))
        {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        Ok(Self {
            name: text("name")?,
            timezone: text("timezone")?,
            subscription_plan,
        })
    }

    fn apply(self, row: &mut household::ActiveModel) {
        if let Some(name) = self.name {
            row.name = Set(name);
        }
        if let Some(timezone) = self.timezone {
            row.timezone = Set(timezone);
        }
        if let Some(subscription_plan) = self.subscription_plan {
            row.subscription_plan = Set(subscription_plan);
        }
        row.updated_at = Set(Utc::now().naive_utc());
    }
}

fn value(row: &household::Model) -> Value {
    json!({
        "id": row.id,
        "name": row.name,
        "slug": row.slug,
        "timezone": row.timezone,
        "subscription_plan": row.subscription_plan,
        "updated_at": row.updated_at.and_utc().to_rfc3339(),
    })
}

fn path(household_id: i64) -> String {
    format!("/api/v1/households/{household_id}/admin/settings")
}

pub(super) async fn show(
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
            "show",
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
            None,
        )
        .await;
    }
    let home = household::Entity::find_by_id(household_id)
        .one(&db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?;
    finish(
        db,
        &context,
        "GET",
        CONTROLLER,
        POLICY,
        "show",
        StatusCode::OK,
        true,
        json!({"data": value(&home)}),
        None,
    )
    .await
}

pub(super) async fn patch(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    update(state, household_id, headers, payload, "PATCH").await
}

pub(super) async fn put(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    update(state, household_id, headers, payload, "PUT").await
}

async fn failure(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    status: StatusCode,
    key_and_digest: Option<(&str, &str)>,
) -> Result<Response, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let (code, message, errors) = if status == StatusCode::BAD_REQUEST {
        (
            "bad_request",
            "household is required or JSON is invalid",
            None,
        )
    } else {
        (
            "validation_failed",
            "Validation failed",
            Some(json!({"household": ["is invalid"]})),
        )
    };
    let mut body = json!({"error": {"code": code, "message": message, "request_id": request_id}});
    if let Some(errors) = errors {
        body["error"]["errors"] = errors;
    }
    if let Some((key, digest)) = key_and_digest {
        mutation_idempotency::store(
            &db,
            context,
            StoredResponse {
                key,
                method,
                path: &path(context.membership.household_id),
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
        "update",
        status,
        false,
        body,
        None,
    )
    .await
}

async fn update(
    state: AppState,
    household_id: i64,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
    method: &str,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    if !household_manager(&context) {
        return error_response(
            db,
            &context,
            method,
            CONTROLLER,
            POLICY,
            "update",
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
            "update",
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
            None,
        )
        .await;
    }
    let Json(body) = match payload {
        Ok(payload) => payload,
        Err(_) => return failure(db, &context, method, StatusCode::BAD_REQUEST, None).await,
    };
    let request_path = path(household_id);
    let request_digest = mutation_idempotency::digest(method, &request_path, &body);
    let key = mutation_idempotency::key(&headers);
    if let Some(key) = key {
        match mutation_idempotency::lookup(
            &db,
            &context,
            key,
            method,
            &request_path,
            &request_digest,
        )
        .await?
        {
            Lookup::New => {}
            Lookup::Replay(saved) => {
                let replay_id = Uuid::new_v4().to_string();
                let status = StatusCode::from_u16(saved.response_status as u16)
                    .map_err(|_| ApiError::internal())?;
                crate::audit::record_resource_request_with_id(
                    &db,
                    &context,
                    &replay_id,
                    method,
                    CONTROLLER,
                    POLICY,
                    "update",
                    status,
                    status.is_success(),
                )
                .await
                .map_err(database_error)?;
                db.commit().await.map_err(database_error)?;
                let mut response = mutation_idempotency::replay(*saved)?;
                response.headers_mut().insert(
                    "x-request-id",
                    replay_id.parse().map_err(|_| ApiError::internal())?,
                );
                return Ok(response);
            }
            Lookup::Conflict => {
                return error_response(
                    db,
                    &context,
                    method,
                    CONTROLLER,
                    POLICY,
                    "update",
                    StatusCode::CONFLICT,
                    "idempotency_key_reused",
                    "Idempotency key has already been used for a different request",
                    None,
                )
                .await;
            }
        }
    }
    let attrs = match Attributes::parse(&body) {
        Ok(attrs) => attrs,
        Err(status) => {
            return failure(
                db,
                &context,
                method,
                status,
                key.map(|key| (key, request_digest.as_str())),
            )
            .await;
        }
    };
    let before = value(&home);
    let mut active = home.into_active_model();
    attrs.apply(&mut active);
    let updated = active.update(&db).await.map_err(database_error)?;
    let after = value(&updated);
    let request_id = Uuid::new_v4().to_string();
    record_version(
        &db,
        &context,
        &request_id,
        "Household",
        updated.id,
        "update",
        Some(before),
        Some(after.clone()),
    )
    .await?;
    let now = Utc::now().naive_utc();
    security_audit_event::ActiveModel {
        household_id: Set(household_id),
        actor_account_id: Set(Some(context.account_id)),
        actor_membership_id: Set(Some(context.membership.id)),
        event_type: Set("api/admin/household_settings/updated".to_owned()),
        request_id: Set(Some(request_id.clone())),
        metadata: Set(
            json!({"target_type": "Household", "target_id": household_id, "outcome": "success"}),
        ),
        audit_context: Set(json!({
            "actor_account_id": context.account_id,
            "actor_user_id": context.user_id,
            "actor_membership_id": context.membership.id,
            "household_id": household_id,
            "request_id": request_id,
        })),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .map_err(database_error)?;
    let response_body = json!({"data": after});
    if let Some(key) = key {
        mutation_idempotency::store(
            &db,
            &context,
            StoredResponse {
                key,
                method,
                path: &request_path,
                digest: &request_digest,
                status: StatusCode::OK,
                body: response_body.clone(),
                request_id: &request_id,
                etag: None,
            },
        )
        .await?;
    }
    finish_with_request_id(
        db,
        &context,
        &request_id,
        method,
        CONTROLLER,
        POLICY,
        "update",
        StatusCode::OK,
        true,
        response_body,
        None,
    )
    .await
}
