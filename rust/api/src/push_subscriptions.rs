use crate::entities::{account, push_subscription};
use crate::medication_management::{error_response, finish, request_context};
use crate::{database_error, ApiError, AppState, AuthContext};
use axum::body::Body;
use axum::extract::{rejection::JsonRejection, Path, RawQuery, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, IntoActiveModel, QueryFilter,
    QuerySelect, Set, TransactionTrait,
};
use serde_json::{json, Value};
use url::{form_urlencoded, Host, Url};

const CONTROLLER: &str = "api/v1/push_subscriptions";
const POLICY: &str = "PushSubscriptionPolicy";

struct Attributes {
    endpoint: String,
    p256dh: String,
    auth: String,
}

impl Attributes {
    fn parse(body: &Value) -> Option<Self> {
        let outer = body.as_object()?;
        if outer.len() != 1 {
            return None;
        }
        let inner = outer.get("push_subscription")?.as_object()?;
        if inner
            .keys()
            .any(|key| !matches!(key.as_str(), "endpoint" | "keys"))
        {
            return None;
        }
        let endpoint = inner.get("endpoint")?.as_str()?;
        if !allowed_endpoint(endpoint) {
            return None;
        }
        let keys = inner.get("keys")?.as_object()?;
        if keys
            .keys()
            .any(|key| !matches!(key.as_str(), "p256dh" | "auth"))
        {
            return None;
        }
        let p256dh = keys.get("p256dh")?.as_str()?;
        let auth = keys.get("auth")?.as_str()?;
        if p256dh.trim().is_empty() || auth.trim().is_empty() {
            return None;
        }
        Some(Self {
            endpoint: endpoint.to_owned(),
            p256dh: p256dh.to_owned(),
            auth: auth.to_owned(),
        })
    }
}

fn allowed_endpoint(endpoint: &str) -> bool {
    if !endpoint.starts_with("https://")
        || endpoint.trim() != endpoint
        || endpoint.chars().any(char::is_whitespace)
    {
        return false;
    }
    let Some((_, authority_and_path)) = endpoint.split_once("://") else {
        return false;
    };
    let authority = authority_and_path
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("");
    if authority.contains(['@', '%', '\\']) || endpoint.contains('\\') {
        return false;
    }
    let Ok(url) = Url::parse(endpoint) else {
        return false;
    };
    if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    let Some(Host::Domain(host)) = url.host() else {
        return false;
    };
    matches!(
        host,
        "fcm.googleapis.com" | "updates.push.services.mozilla.com" | "web.push.apple.com"
    ) || host.ends_with(".notify.windows.com")
        || host.ends_with(".push.apple.com")
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
        Some(json!({"push_subscription": ["is invalid"]})),
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
        if body.get("push_subscription").is_none() {
            return failure(
                db,
                &context,
                "POST",
                "create",
                StatusCode::BAD_REQUEST,
                "bad_request",
                "push_subscription is required",
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
    let existing = push_subscription::Entity::find()
        .filter(push_subscription::Column::AccountId.eq(context.account_id))
        .filter(push_subscription::Column::Endpoint.eq(&attributes.endpoint))
        .one(&db)
        .await
        .map_err(database_error)?;
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let now = Utc::now().naive_utc();
    if let Some(existing) = existing {
        if existing.p256dh != attributes.p256dh
            || existing.auth != attributes.auth
            || existing.user_agent != user_agent
        {
            let mut active = existing.into_active_model();
            active.p256dh = Set(attributes.p256dh);
            active.auth = Set(attributes.auth);
            active.user_agent = Set(user_agent);
            active.updated_at = Set(now);
            active.update(&db).await.map_err(database_error)?;
        }
    } else {
        let active = push_subscription::ActiveModel {
            account_id: Set(context.account_id),
            endpoint: Set(attributes.endpoint),
            p256dh: Set(attributes.p256dh),
            auth: Set(attributes.auth),
            user_agent: Set(user_agent),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
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
    Path(household_id): Path<i64>,
    RawQuery(raw_query): RawQuery,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let endpoint = raw_query.as_deref().and_then(|raw| {
        let mut pairs = form_urlencoded::parse(raw.as_bytes())
            .filter(|(name, _)| name == "endpoint")
            .map(|(_, value)| value.into_owned());
        let first = pairs.next()?;
        if first.trim().is_empty() || pairs.next().is_some() {
            return None;
        }
        Some(first)
    });
    let Some(endpoint) = endpoint else {
        return failure(
            db,
            &context,
            "DELETE",
            "destroy",
            StatusCode::BAD_REQUEST,
            "bad_request",
            "endpoint is required",
        )
        .await;
    };
    account::Entity::find_by_id(context.account_id)
        .lock_exclusive()
        .one(&db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    push_subscription::Entity::delete_many()
        .filter(push_subscription::Column::AccountId.eq(context.account_id))
        .filter(push_subscription::Column::Endpoint.eq(endpoint))
        .exec(&db)
        .await
        .map_err(database_error)?;
    empty_success(db, &context, "DELETE", "destroy", StatusCode::NO_CONTENT).await
}
