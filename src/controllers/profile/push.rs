use super::*;
use crate::models::care::browser_push::{self, Service, audit};
use crate::models::{care::push_subscriptions, entities::push_subscription};
use axum::Json;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sea_orm::{ColumnTrait, QueryFilter};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Registration {
    authenticity_token: String,
    subscription: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Endpoint {
    authenticity_token: String,
    endpoint: String,
}

fn json_response(value: Value) -> Response {
    ([(header::CACHE_CONTROL, "no-store")], Json(value)).into_response()
}

fn valid_keys(subscription: &Value) -> bool {
    let Some(endpoint) = subscription["endpoint"]
        .as_str()
        .filter(|value| value.len() <= 4096)
    else {
        return false;
    };
    let Some(key) = subscription["keys"]["p256dh"]
        .as_str()
        .filter(|value| value.len() <= 128)
    else {
        return false;
    };
    let Some(auth) = subscription["keys"]["auth"]
        .as_str()
        .filter(|value| value.len() <= 32)
    else {
        return false;
    };
    if !URL_SAFE_NO_PAD
        .decode(key)
        .is_ok_and(|bytes| bytes.len() == 65 && bytes[0] == 4)
        || !URL_SAFE_NO_PAD
            .decode(auth)
            .is_ok_and(|bytes| bytes.len() == 16)
    {
        return false;
    }
    let info = web_push::SubscriptionInfo::new(endpoint, key, auth);
    let mut message = web_push::WebPushMessageBuilder::new(&info);
    message.set_payload(web_push::ContentEncoding::Aes128Gcm, b"validation");
    message.build().is_ok()
}

pub(super) async fn register(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    (headers, request): (HeaderMap, Option<Extension<LocoRequestId>>),
    token: CsrfToken,
    Json(body): Json<Registration>,
) -> Response {
    if token.verify(&body.authenticity_token).is_err() {
        return operation_error(OperationError::Forbidden);
    }
    let (_, tenant) = match begin(&ctx, &session, &slug, &request_id(request)).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    if ctx.shared_store.get::<Service>().is_none() {
        return unavailable();
    }
    if !valid_keys(&body.subscription) {
        return operation_error(profile::validation("push_subscription", "keys are invalid"));
    }
    let result = async {
        let created = push_subscriptions::register(
            &tenant,
            &json!({"push_subscription":body.subscription}),
            headers
                .get(header::USER_AGENT)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned),
        )
        .await?;
        audit(&tenant, "registered", json!({"created":created})).await
    }
    .await;
    if let Err(error) = result {
        return operation_error(error);
    }
    if tenant.commit().await.is_err() {
        return unavailable();
    }
    json_response(json!({"subscribed":true}))
}

pub(super) async fn remove(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    Json(body): Json<Endpoint>,
) -> Response {
    if token.verify(&body.authenticity_token).is_err() {
        return operation_error(OperationError::Forbidden);
    }
    let (_, tenant) = match begin(&ctx, &session, &slug, &request_id(request)).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let result = async {
        let removed = push_subscriptions::revoke(&tenant, &body.endpoint).await?;
        if removed > 0 {
            audit(&tenant, "removed", json!({"removed":removed})).await?;
        }
        Ok::<_, OperationError>(())
    }
    .await;
    if let Err(error) = result {
        return operation_error(error);
    }
    if tenant.commit().await.is_err() {
        return unavailable();
    }
    json_response(json!({"subscribed":false}))
}

pub(super) async fn subscription_status(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    Json(body): Json<Endpoint>,
) -> Response {
    if token.verify(&body.authenticity_token).is_err() {
        return operation_error(OperationError::Forbidden);
    }
    let (principal, tenant) = match begin(&ctx, &session, &slug, &request_id(request)).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let subscribed = match push_subscription::Entity::find()
        .filter(push_subscription::Column::AccountId.eq(principal.account_id()))
        .filter(push_subscription::Column::Endpoint.eq(body.endpoint))
        .one(tenant.transaction())
        .await
    {
        Ok(value) => value.is_some(),
        Err(_) => return unavailable(),
    };
    if tenant.commit().await.is_err() {
        return unavailable();
    }
    json_response(json!({"subscribed":subscribed}))
}

pub(super) async fn status(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
) -> Response {
    let (_, tenant) = match begin(&ctx, &session, &slug, &request_id(request)).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    if tenant.commit().await.is_err() {
        return unavailable();
    }
    let service = ctx.shared_store.get::<Service>();
    ([(header::CACHE_CONTROL, "no-store")], axum::Json(json!({"configured":service.is_some(),"public_key":service.map(|service| service.public_key.clone())}))).into_response()
}

pub(super) async fn send_test(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    Json(body): Json<Endpoint>,
) -> Response {
    if token.verify(&body.authenticity_token).is_err() {
        return operation_error(OperationError::Forbidden);
    }
    let (_, tenant) = match begin(&ctx, &session, &slug, &request_id(request)).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let Some(service) = ctx.shared_store.get::<Service>() else {
        return unavailable();
    };
    let result = match browser_push::send_test(&tenant, &service, &body.endpoint).await {
        Ok(value) => value,
        Err(error) => return operation_error(error),
    };
    if tenant.commit().await.is_err() {
        return unavailable();
    }
    json_response(json!({"status":result.status()}))
}
