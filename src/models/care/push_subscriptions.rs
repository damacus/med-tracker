use crate::models::{
    access::{self, TenantTransaction},
    entities::{account, household, push_subscription},
    errors::OperationError,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, QuerySelect, Set,
};
use serde_json::{Value, json};
use url::{Host, Url};

struct Attributes {
    endpoint: String,
    p256dh: String,
    auth: String,
}

fn invalid(message: &str) -> OperationError {
    OperationError::Validation {
        details: json!({"errors":{"push_subscription":[message]}}),
    }
}

pub(crate) fn allowed_endpoint(endpoint: &str) -> bool {
    if endpoint.trim() != endpoint
        || endpoint.chars().any(char::is_whitespace)
        || endpoint.contains('\\')
        || !endpoint.starts_with("https://")
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
    if authority.contains(['@', '%', '\\']) {
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

impl Attributes {
    fn parse(body: &Value) -> Result<Self, OperationError> {
        let outer = body
            .as_object()
            .ok_or_else(|| invalid("must be an object"))?;
        if outer.len() != 1 {
            return Err(invalid("contains unsupported fields"));
        }
        let fields = outer
            .get("push_subscription")
            .and_then(Value::as_object)
            .ok_or_else(|| invalid("is required"))?;
        if fields
            .keys()
            .any(|key| !matches!(key.as_str(), "endpoint" | "keys"))
        {
            return Err(invalid("contains unsupported fields"));
        }
        let endpoint = fields
            .get("endpoint")
            .and_then(Value::as_str)
            .filter(|value| allowed_endpoint(value))
            .ok_or_else(|| invalid("endpoint is invalid"))?;
        let keys = fields
            .get("keys")
            .and_then(Value::as_object)
            .ok_or_else(|| invalid("keys are required"))?;
        if keys
            .keys()
            .any(|key| !matches!(key.as_str(), "p256dh" | "auth"))
        {
            return Err(invalid("keys contain unsupported fields"));
        }
        let p256dh = keys
            .get("p256dh")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| invalid("p256dh is required"))?;
        let auth = keys
            .get("auth")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| invalid("auth is required"))?;
        Ok(Self {
            endpoint: endpoint.into(),
            p256dh: p256dh.into(),
            auth: auth.into(),
        })
    }
}

pub(crate) async fn lock_account(tenant: &TenantTransaction) -> Result<(), OperationError> {
    household::Entity::find_by_id(tenant.scope().household_id)
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    access::recheck(tenant).await?;
    account::Entity::find_by_id(tenant.scope().actor.account_id)
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Forbidden)?;
    Ok(())
}

pub async fn register(
    tenant: &TenantTransaction,
    body: &Value,
    user_agent: Option<String>,
) -> Result<bool, OperationError> {
    lock_account(tenant).await?;
    let attributes = Attributes::parse(body)?;
    let existing = push_subscription::Entity::find()
        .filter(push_subscription::Column::AccountId.eq(tenant.scope().actor.account_id))
        .filter(push_subscription::Column::Endpoint.eq(&attributes.endpoint))
        .one(tenant.transaction())
        .await?;
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
            active.update(tenant.transaction()).await?;
        }
        return Ok(false);
    }
    let inserted = push_subscription::ActiveModel {
        account_id: Set(tenant.scope().actor.account_id),
        endpoint: Set(attributes.endpoint),
        p256dh: Set(attributes.p256dh),
        auth: Set(attributes.auth),
        user_agent: Set(user_agent),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await;
    match inserted {
        Ok(_) => Ok(true),
        Err(error)
            if matches!(
                error.sql_err(),
                Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
            ) =>
        {
            Err(invalid("endpoint is already registered"))
        }
        Err(error) => Err(error.into()),
    }
}

pub async fn revoke(tenant: &TenantTransaction, endpoint: &str) -> Result<u64, OperationError> {
    lock_account(tenant).await?;
    if endpoint.trim().is_empty() {
        return Err(invalid("endpoint is required"));
    }
    Ok(push_subscription::Entity::delete_many()
        .filter(push_subscription::Column::AccountId.eq(tenant.scope().actor.account_id))
        .filter(push_subscription::Column::Endpoint.eq(endpoint))
        .exec(tenant.transaction())
        .await?
        .rows_affected)
}
