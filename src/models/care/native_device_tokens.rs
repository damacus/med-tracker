use crate::models::{
    access::TenantTransaction, care::push_subscriptions, entities::native_device_token,
    errors::OperationError,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, Set};
use serde_json::{Value, json};

struct Attributes {
    device_token: String,
    platform: String,
    apns_environment: Option<String>,
}

fn invalid() -> OperationError {
    OperationError::Validation {
        details: json!({"errors":{"native_device_token":["is invalid"]}}),
    }
}

impl Attributes {
    fn parse(body: &Value) -> Result<Self, OperationError> {
        let inner = body
            .as_object()
            .filter(|outer| outer.len() == 1)
            .and_then(|outer| outer.get("native_device_token"))
            .and_then(Value::as_object)
            .ok_or_else(invalid)?;
        if inner.keys().any(|key| {
            !matches!(
                key.as_str(),
                "device_token" | "platform" | "apns_environment"
            )
        }) {
            return Err(invalid());
        }
        let device_token = inner
            .get("device_token")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(invalid)?;
        let platform = inner
            .get("platform")
            .and_then(Value::as_str)
            .filter(|value| matches!(*value, "ios" | "android"))
            .ok_or_else(invalid)?;
        let apns_environment = match inner.get("apns_environment") {
            None => None,
            Some(Value::String(value)) if matches!(value.as_str(), "sandbox" | "production") => {
                Some(value.clone())
            }
            Some(_) => return Err(invalid()),
        };
        Ok(Self {
            device_token: device_token.into(),
            platform: platform.into(),
            apns_environment,
        })
    }
}

pub async fn register(
    tenant: &TenantTransaction,
    body: &Value,
    user_agent: Option<String>,
) -> Result<(), OperationError> {
    push_subscriptions::lock_account(tenant).await?;
    let attributes = Attributes::parse(body)?;
    let existing = native_device_token::Entity::find()
        .filter(native_device_token::Column::AccountId.eq(tenant.scope().actor.account_id))
        .filter(native_device_token::Column::DeviceToken.eq(&attributes.device_token))
        .one(tenant.transaction())
        .await?;
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
            active.update(tenant.transaction()).await?;
        }
        return Ok(());
    }
    let mut active = native_device_token::ActiveModel {
        account_id: Set(tenant.scope().actor.account_id),
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
    match active.insert(tenant.transaction()).await {
        Ok(_) => Ok(()),
        Err(error)
            if matches!(
                error.sql_err(),
                Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
            ) =>
        {
            Err(invalid())
        }
        Err(error) => Err(error.into()),
    }
}

pub async fn revoke(tenant: &TenantTransaction, device_token: &str) -> Result<(), OperationError> {
    push_subscriptions::lock_account(tenant).await?;
    native_device_token::Entity::delete_many()
        .filter(native_device_token::Column::AccountId.eq(tenant.scope().actor.account_id))
        .filter(native_device_token::Column::DeviceToken.eq(device_token))
        .exec(tenant.transaction())
        .await?;
    Ok(())
}
