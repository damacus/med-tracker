use async_trait::async_trait;
pub(crate) mod low_stock;
pub mod reminders;
pub mod scheduler;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use std::{sync::Arc, time::Duration};
use web_push::{
    ContentEncoding, SubscriptionInfo, VapidSignatureBuilder, WebPushMessageBuilder,
    request_builder,
};

use crate::models::entities::push_subscription;
use crate::models::{
    access::TenantTransaction,
    entities::{household, security_audit_event},
    errors::OperationError,
};
use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::{Value, json};

pub(crate) async fn audit(
    tenant: &TenantTransaction,
    action: &str,
    metadata: Value,
) -> Result<(), OperationError> {
    let now = Utc::now().naive_utc();
    security_audit_event::ActiveModel {
        actor_membership_id: Set(Some(tenant.membership().id)),
        actor_account_id: Set(Some(tenant.scope().actor.account_id)),
        household_id: Set(tenant.scope().household_id),
        event_type: Set(format!("browser_push.{action}")),
        request_id: Set(Some(tenant.scope().request_id.clone())),
        metadata: Set(metadata),
        audit_context: Set(json!({"actor_membership_id":tenant.membership().id,"household_id":tenant.scope().household_id})),
        created_at: Set(now), updated_at: Set(now), ..Default::default()
    }.insert(tenant.transaction()).await?;
    Ok(())
}

pub async fn send_test(
    tenant: &TenantTransaction,
    service: &Service,
    endpoint: &str,
) -> Result<Delivery, OperationError> {
    super::push_subscriptions::lock_account(tenant).await?;
    let subscription = push_subscription::Entity::find()
        .filter(push_subscription::Column::AccountId.eq(tenant.scope().actor.account_id))
        .filter(push_subscription::Column::Endpoint.eq(endpoint))
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    let household = household::Entity::find_by_id(tenant.scope().household_id)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    audit(
        tenant,
        "test.requested",
        json!({"subscription_id":subscription.id}),
    )
    .await?;
    let payload = serde_json::to_vec(&json!({"title":"MedTracker","body":"Test notification","path":format!("/households/{}/profile#notifications",household.slug)})).map_err(|_| OperationError::Unavailable)?;
    let outcome = service.transport.send(&subscription, &payload).await;
    if outcome == Delivery::Expired {
        push_subscription::Entity::delete_by_id(subscription.id)
            .exec(tenant.transaction())
            .await?;
    }
    audit(
        tenant,
        "test.completed",
        json!({"subscription_id":subscription.id,"status":outcome.status()}),
    )
    .await?;
    Ok(outcome)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Delivery {
    Accepted,
    Expired,
    Failed,
    Permanent,
    Temporary(Option<Duration>),
    Uncertain,
}

impl Delivery {
    pub fn status(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Expired => "expired",
            Self::Failed => "failed",
            Self::Permanent => "failed",
            Self::Temporary(_) => "failed",
            Self::Uncertain => "failed",
        }
    }
}

#[async_trait]
pub trait Transport: Send + Sync {
    async fn send(&self, subscription: &push_subscription::Model, payload: &[u8]) -> Delivery;

    async fn send_with_ttl(
        &self,
        subscription: &push_subscription::Model,
        payload: &[u8],
        _ttl: u32,
    ) -> Delivery {
        self.send(subscription, payload).await
    }
}

#[derive(Clone)]
pub struct Service {
    pub public_key: String,
    pub transport: Arc<dyn Transport>,
}

struct WebPush {
    private_key: String,
    subject: String,
    client: reqwest::Client,
}

fn classify_response(status: u16, retry_after: Option<&str>, now: DateTime<Utc>) -> Delivery {
    match status {
        200..=299 => Delivery::Accepted,
        404 | 410 => Delivery::Expired,
        429 | 500..=599 => {
            let Some(header) = retry_after else {
                return Delivery::Temporary(None);
            };
            let delay = header
                .trim()
                .parse::<u64>()
                .ok()
                .map(Duration::from_secs)
                .or_else(|| {
                    DateTime::parse_from_rfc2822(header).ok().map(|date| {
                        (date.with_timezone(&Utc) - now)
                            .to_std()
                            .unwrap_or(Duration::ZERO)
                    })
                });
            delay.map_or(Delivery::Uncertain, |delay| {
                Delivery::Temporary(Some(delay))
            })
        }
        400..=499 => Delivery::Permanent,
        _ => Delivery::Uncertain,
    }
}

impl Service {
    pub fn from_context(ctx: &loco_rs::app::AppContext) -> Result<Option<Self>, &'static str> {
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Configuration {
            private_key: String,
            subject: String,
        }
        if let Some(value) = ctx
            .config
            .settings
            .as_ref()
            .and_then(|settings| settings.get("browser_push"))
        {
            let configuration: Configuration = serde_json::from_value(value.clone())
                .map_err(|_| "Web Push configuration is invalid")?;
            return Self::configured(configuration.private_key, configuration.subject).map(Some);
        }
        Self::from_environment()
    }

    pub fn from_environment() -> Result<Option<Self>, &'static str> {
        match (
            std::env::var("MEDTRACKER_WEB_PUSH_PRIVATE_KEY").ok(),
            std::env::var("MEDTRACKER_WEB_PUSH_SUBJECT").ok(),
        ) {
            (None, None) => Ok(None),
            (Some(key), Some(subject)) => Self::configured(key, subject).map(Some),
            _ => Err("Web Push requires both a private key and contact subject"),
        }
    }

    pub fn configured(private_key: String, subject: String) -> Result<Self, &'static str> {
        if !URL_SAFE_NO_PAD
            .decode(&private_key)
            .is_ok_and(|bytes| bytes.len() == 32)
        {
            return Err("Web Push private key is invalid");
        }
        let contact =
            url::Url::parse(&subject).map_err(|_| "Web Push contact subject is invalid")?;
        if !matches!(contact.scheme(), "https" | "mailto") || contact.path().is_empty() {
            return Err("Web Push contact subject is invalid");
        }
        let signature = VapidSignatureBuilder::from_base64_no_sub(&private_key)
            .map_err(|_| "Web Push private key is invalid")?;
        let public_key = URL_SAFE_NO_PAD.encode(signature.get_public_key());
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .build()
            .map_err(|_| "Web Push HTTP client is unavailable")?;
        Ok(Self {
            public_key,
            transport: Arc::new(WebPush {
                private_key,
                subject,
                client,
            }),
        })
    }
}

#[async_trait]
impl Transport for WebPush {
    async fn send(&self, subscription: &push_subscription::Model, payload: &[u8]) -> Delivery {
        self.send_with_ttl(subscription, payload, 300).await
    }

    async fn send_with_ttl(
        &self,
        subscription: &push_subscription::Model,
        payload: &[u8],
        ttl: u32,
    ) -> Delivery {
        if !super::push_subscriptions::allowed_endpoint(&subscription.endpoint) {
            return Delivery::Permanent;
        }
        let info = SubscriptionInfo::new(
            &subscription.endpoint,
            &subscription.p256dh,
            &subscription.auth,
        );
        let Ok(mut signature) = VapidSignatureBuilder::from_base64(&self.private_key, &info) else {
            return Delivery::Permanent;
        };
        signature.add_claim("sub", self.subject.as_str());
        let Ok(signature) = signature.build() else {
            return Delivery::Permanent;
        };
        let mut message = WebPushMessageBuilder::new(&info);
        message.set_payload(ContentEncoding::Aes128Gcm, payload);
        message.set_vapid_signature(signature);
        message.set_ttl(ttl);
        let Ok(message) = message.build() else {
            return Delivery::Permanent;
        };
        let built = request_builder::build_request::<Vec<u8>>(message);
        let (parts, body) = built.into_parts();
        let mut request = self.client.post(parts.uri.to_string());
        for (name, value) in &parts.headers {
            request = request.header(name.as_str(), value.as_bytes().to_vec());
        }
        let Ok(response) = request.body(body).send().await else {
            return Delivery::Uncertain;
        };
        let retry_after = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok());
        classify_response(response.status().as_u16(), retry_after, Utc::now())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_rate_limits_respect_delta_and_http_date_retry_after() {
        let now = DateTime::parse_from_rfc2822("Wed, 21 Oct 2015 07:18:00 GMT")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(
            classify_response(429, Some("600"), now),
            Delivery::Temporary(Some(Duration::from_secs(600)))
        );
        assert_eq!(
            classify_response(429, Some("Wed, 21 Oct 2015 07:28:00 GMT"), now),
            Delivery::Temporary(Some(Duration::from_secs(600)))
        );
        assert_eq!(
            classify_response(429, Some("invalid"), now),
            Delivery::Uncertain
        );
    }

    #[test]
    fn malformed_push_private_keys_return_errors_without_panicking() {
        for length in [31, 0, 1, 33, 64] {
            assert!(
                Service::configured(
                    URL_SAFE_NO_PAD.encode(vec![7_u8; length]),
                    "mailto:fixture@example.test".into(),
                )
                .is_err()
            );
        }
        for bytes in [[0_u8; 32], [255_u8; 32]] {
            assert!(
                Service::configured(
                    URL_SAFE_NO_PAD.encode(bytes),
                    "mailto:fixture@example.test".into(),
                )
                .is_err()
            );
        }
    }
}
