use crate::entities::{account, native_device_token};
use crate::medication_management::{error_response, finish, request_context};
use crate::{database_error, ApiError, AppState, AuthContext};
use axum::body::Body;
use axum::extract::{rejection::JsonRejection, Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, IntoActiveModel, QueryFilter,
    QuerySelect, Set, TransactionTrait,
};
use serde_json::{json, Value};

const CONTROLLER: &str = "api/v1/native_device_tokens";
const POLICY: &str = "NativeDeviceTokenPolicy";

struct Attributes {
    device_token: String,
    platform: String,
    apns_environment: Option<String>,
}

impl Attributes {
    fn parse(body: &Value) -> Option<Self> {
        let outer = body.as_object()?;
        if outer.len() != 1 {
            return None;
        }
        let inner = outer.get("native_device_token")?.as_object()?;
        if inner.keys().any(|key| {
            !matches!(
                key.as_str(),
                "device_token" | "platform" | "apns_environment"
            )
        }) {
            return None;
        }
        let device_token = inner.get("device_token")?.as_str()?;
        if device_token.trim().is_empty() {
            return None;
        }
        let platform = inner.get("platform")?.as_str()?;
        if !matches!(platform, "ios" | "android") {
            return None;
        }
        let apns_environment = match inner.get("apns_environment") {
            None => None,
            Some(Value::String(value)) if matches!(value.as_str(), "sandbox" | "production") => {
                Some(value.clone())
            }
            Some(_) => return None,
        };
        Some(Self {
            device_token: device_token.to_owned(),
            platform: platform.to_owned(),
            apns_environment,
        })
    }
}

async fn failure(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    action: &str,
    status: StatusCode,
    code: &str,
    message: &str,
) -> Result<Response, ApiError> {
    error_response(
        db, context, method, CONTROLLER, POLICY, action, status, code, message, None,
    )
    .await
}

async fn validation(db: DatabaseTransaction, context: &AuthContext) -> Result<Response, ApiError> {
    error_response(
        db,
        context,
        "POST",
        CONTROLLER,
        POLICY,
        "create",
        StatusCode::UNPROCESSABLE_ENTITY,
        "validation_failed",
        "Validation failed",
        Some(json!({"native_device_token": ["is invalid"]})),
    )
    .await
}

async fn empty_success(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    action: &str,
    status: StatusCode,
) -> Result<Response, ApiError> {
    let mut response = finish(
        db,
        context,
        method,
        CONTROLLER,
        POLICY,
        action,
        status,
        true,
        json!({}),
        None,
    )
    .await?;
    *response.body_mut() = Body::empty();
    response.headers_mut().remove(header::CONTENT_TYPE);
    Ok(response)
}

pub(super) async fn create(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let Json(body) = match payload {
        Ok(value) => value,
        Err(_) => {
            return failure(
                db,
                &context,
                "POST",
                "create",
                StatusCode::BAD_REQUEST,
                "bad_request",
                "Invalid JSON request",
            )
            .await;
        }
    };
    let Some(attributes) = Attributes::parse(&body) else {
        if body.get("native_device_token").is_none() {
            return failure(
                db,
                &context,
                "POST",
                "create",
                StatusCode::BAD_REQUEST,
                "bad_request",
                "native_device_token is required",
            )
            .await;
        }
        return validation(db, &context).await;
    };
    account::Entity::find_by_id(context.account_id)
        .lock_exclusive()
        .one(&db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    let existing = native_device_token::Entity::find()
        .filter(native_device_token::Column::AccountId.eq(context.account_id))
        .filter(native_device_token::Column::DeviceToken.eq(&attributes.device_token))
        .one(&db)
        .await
        .map_err(database_error)?;
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let now = Utc::now().naive_utc();
    if let Some(existing) = existing {
        let apns_matches = attributes
            .apns_environment
            .as_ref()
            .is_none_or(|value| existing.apns_environment.as_ref() == Some(value));
        if existing.platform != attributes.platform
            || !apns_matches
            || existing.user_agent != user_agent
        {
            let mut active = existing.into_active_model();
            active.platform = Set(attributes.platform);
            if let Some(value) = attributes.apns_environment {
                active.apns_environment = Set(Some(value));
            }
            active.user_agent = Set(user_agent);
            active.updated_at = Set(now);
            active.update(&db).await.map_err(database_error)?;
        }
    } else {
        let mut active = native_device_token::ActiveModel {
            account_id: Set(context.account_id),
            device_token: Set(attributes.device_token),
            platform: Set(attributes.platform),
            user_agent: Set(user_agent),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        if let Some(value) = attributes.apns_environment {
            active.apns_environment = Set(Some(value));
        }
        let savepoint = db.begin().await.map_err(database_error)?;
        match active.insert(&savepoint).await {
            Ok(_) => savepoint.commit().await.map_err(database_error)?,
            Err(error)
                if matches!(
                    error.sql_err(),
                    Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
                ) =>
            {
                savepoint.rollback().await.map_err(database_error)?;
                return validation(db, &context).await;
            }
            Err(error) => return Err(database_error(error)),
        }
    }
    empty_success(db, &context, "POST", "create", StatusCode::CREATED).await
}

pub(super) async fn delete(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    account::Entity::find_by_id(context.account_id)
        .lock_exclusive()
        .one(&db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    native_device_token::Entity::delete_many()
        .filter(native_device_token::Column::AccountId.eq(context.account_id))
        .filter(native_device_token::Column::DeviceToken.eq(id))
        .exec(&db)
        .await
        .map_err(database_error)?;
    empty_success(db, &context, "DELETE", "destroy", StatusCode::NO_CONTENT).await
}
