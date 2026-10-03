use super::{bounded_json, configured_url, http_client};
use crate::entities::{household, native_device_token, push_subscription};
use crate::medication_management::{error_response, finish, request_context};
use crate::push_subscriptions::allowed_endpoint;
use crate::{database_error, ApiError, AppState};
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::Response;
use chrono::Utc;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::{json, Value};
use url::Url;
use web_push::{
    request_builder, ContentEncoding, SubscriptionInfo, VapidSignatureBuilder,
    WebPushMessageBuilder,
};

const CONTROLLER: &str = "api/v1/push_subscriptions";
const POLICY: &str = "PushSubscriptionPolicy";

enum DeliveryOutcome {
    Accepted,
    Permanent,
    Transient,
}

async fn web_push_delivery(
    client: &reqwest::Client,
    subscription: &push_subscription::Model,
    path: &str,
) -> DeliveryOutcome {
    if !allowed_endpoint(&subscription.endpoint) {
        return DeliveryOutcome::Transient;
    }
    let Ok(private_key) = std::env::var("VAPID_PRIVATE_KEY") else {
        return DeliveryOutcome::Transient;
    };
    let subject =
        std::env::var("VAPID_SUBJECT").unwrap_or_else(|_| "notifications@example.com".to_owned());
    let subject = if subject.starts_with("mailto:") {
        subject
    } else {
        format!("mailto:{subject}")
    };
    let info = SubscriptionInfo::new(
        &subscription.endpoint,
        &subscription.p256dh,
        &subscription.auth,
    );
    let Ok(mut signature) = VapidSignatureBuilder::from_base64(&private_key, &info) else {
        return DeliveryOutcome::Transient;
    };
    signature.add_claim("sub", subject);
    let Ok(signature) = signature.build() else {
        return DeliveryOutcome::Transient;
    };
    let payload = json!({"title": "MedTracker Test", "options": {"body": "Push notifications are working correctly from the server.", "data": {"path": path}}}).to_string();
    let mut message = WebPushMessageBuilder::new(&info);
    message.set_payload(ContentEncoding::Aes128Gcm, payload.as_bytes());
    message.set_vapid_signature(signature);
    let Ok(message) = message.build() else {
        return DeliveryOutcome::Transient;
    };
    let built = request_builder::build_request::<bytes::Bytes>(message);
    let (parts, body) = built.into_parts();
    let mut request = client.post(parts.uri.to_string());
    for (name, value) in &parts.headers {
        request = request.header(name.as_str().to_owned(), value.as_bytes().to_vec());
    }
    let response = request.body(body).send().await;
    let Ok(response) = response else {
        return DeliveryOutcome::Transient;
    };
    match response.status().as_u16() {
        200..=299 => DeliveryOutcome::Accepted,
        404 | 410 => DeliveryOutcome::Permanent,
        _ => DeliveryOutcome::Transient,
    }
}

fn provider_base(name: &str, default: &str) -> Option<Url> {
    let configured = std::env::var(name).unwrap_or_else(|_| default.to_owned());
    let url = configured_url(&configured, true)?;
    if url.scheme() == "http" && url.host_str() != Some("127.0.0.1") {
        return None;
    }
    Some(url)
}

async fn fcm_delivery(
    client: &reqwest::Client,
    token: &native_device_token::Model,
    path: &str,
) -> DeliveryOutcome {
    let Ok(project) = std::env::var("FCM_PROJECT_ID") else {
        return DeliveryOutcome::Transient;
    };
    let Ok(bearer) = std::env::var("FCM_BEARER_TOKEN") else {
        return DeliveryOutcome::Transient;
    };
    if project.is_empty() || bearer.is_empty() {
        return DeliveryOutcome::Transient;
    }
    let Some(base) = provider_base("MEDTRACKER_FCM_BASE_URL", "https://fcm.googleapis.com/") else {
        return DeliveryOutcome::Transient;
    };
    let Ok(url) = base.join(&format!("v1/projects/{project}/messages:send")) else {
        return DeliveryOutcome::Transient;
    };
    let body = json!({"message": {"token": token.device_token, "notification": {"title":"MedTracker Test","body":"Push notifications are working correctly from the server."}, "data": {"path":path}}});
    let response = client
        .post(url)
        .bearer_auth(bearer)
        .json(&body)
        .send()
        .await;
    let Ok(response) = response else {
        return DeliveryOutcome::Transient;
    };
    if response.status() == StatusCode::OK {
        return DeliveryOutcome::Accepted;
    }
    let body = bounded_json(response).await.unwrap_or(Value::Null);
    let code = body
        .pointer("/error/details")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|detail| {
            detail
                .get("@type")
                .and_then(Value::as_str)
                .is_some_and(|kind| kind.contains("google.firebase.fcm.v1.FcmError"))
        })
        .and_then(|detail| detail.get("errorCode"))
        .and_then(Value::as_str)
        .or_else(|| body.pointer("/error/status").and_then(Value::as_str));
    if matches!(code, Some("UNREGISTERED" | "INVALID_ARGUMENT")) {
        DeliveryOutcome::Permanent
    } else {
        DeliveryOutcome::Transient
    }
}

async fn apns_delivery(
    client: &reqwest::Client,
    token: &native_device_token::Model,
    path: &str,
) -> DeliveryOutcome {
    let (Ok(bundle), Ok(team), Ok(key_id), Ok(private_key)) = (
        std::env::var("APNS_BUNDLE_ID"),
        std::env::var("APNS_TEAM_ID"),
        std::env::var("APNS_KEY_ID"),
        std::env::var("APNS_PRIVATE_KEY"),
    ) else {
        return DeliveryOutcome::Transient;
    };
    if bundle.is_empty() || team.is_empty() || key_id.is_empty() || private_key.is_empty() {
        return DeliveryOutcome::Transient;
    }
    let private_key = private_key.replace("\\n", "\n");
    let Ok(key) = EncodingKey::from_ec_pem(private_key.as_bytes()) else {
        return DeliveryOutcome::Transient;
    };
    let mut header = Header::new(Algorithm::ES256);
    header.kid = Some(key_id);
    let claims = json!({"iss": team, "iat": Utc::now().timestamp()});
    let Ok(jwt) = encode(&header, &claims, &key) else {
        return DeliveryOutcome::Transient;
    };
    let default = match token.apns_environment.as_deref() {
        Some("sandbox") => "https://api.sandbox.push.apple.com/".to_owned(),
        Some("production") => "https://api.push.apple.com/".to_owned(),
        _ => std::env::var("APNS_HOST").unwrap_or_else(|_| {
            if ["1", "true", "yes", "on"].contains(
                &std::env::var("APNS_SANDBOX")
                    .unwrap_or_default()
                    .to_ascii_lowercase()
                    .as_str(),
            ) {
                "https://api.sandbox.push.apple.com/".to_owned()
            } else {
                "https://api.push.apple.com/".to_owned()
            }
        }),
    };
    let Some(base) = provider_base("MEDTRACKER_APNS_BASE_URL", &default) else {
        return DeliveryOutcome::Transient;
    };
    let Ok(url) = base.join(&format!("3/device/{}", token.device_token)) else {
        return DeliveryOutcome::Transient;
    };
    let payload = json!({"aps": {"alert": {"title": "MedTracker", "body": "Open MedTracker to view your notification."}, "sound": "default"}, "path": path, "kind": "test"});
    let response = client
        .post(url)
        .header("authorization", format!("bearer {jwt}"))
        .header("apns-topic", bundle)
        .header("apns-push-type", "alert")
        .header("apns-priority", "10")
        .json(&payload)
        .send()
        .await;
    let Ok(response) = response else {
        return DeliveryOutcome::Transient;
    };
    if response.status() == StatusCode::OK {
        return DeliveryOutcome::Accepted;
    }
    let status = response.status();
    let body = bounded_json(response).await.unwrap_or(Value::Null);
    if status == StatusCode::GONE
        || body.get("reason").and_then(Value::as_str) == Some("BadDeviceToken")
    {
        DeliveryOutcome::Permanent
    } else {
        DeliveryOutcome::Transient
    }
}

pub(crate) async fn test_push_subscription(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let household = household::Entity::find_by_id(household_id)
        .one(&db)
        .await
        .map_err(database_error)?;
    let Some(household) = household else {
        return Err(ApiError::not_found());
    };
    if household.status != "active" || household.lifecycle_state != "active" {
        let mut response = finish(
            db,
            &context,
            "POST",
            CONTROLLER,
            POLICY,
            "test",
            StatusCode::NO_CONTENT,
            true,
            json!({}),
            None,
        )
        .await?;
        *response.body_mut() = Body::empty();
        response.headers_mut().remove(header::CONTENT_TYPE);
        return Ok(response);
    }
    let vapid_configured = ["VAPID_PUBLIC_KEY", "VAPID_PRIVATE_KEY"]
        .iter()
        .all(|name| std::env::var(name).is_ok_and(|value| !value.trim().is_empty()));
    let subscriptions = push_subscription::Entity::find()
        .filter(push_subscription::Column::AccountId.eq(context.account_id))
        .all(&db)
        .await;
    let native_tokens = native_device_token::Entity::find()
        .filter(native_device_token::Column::AccountId.eq(context.account_id))
        .all(&db)
        .await;
    let client = http_client(10);
    let (Ok(subscriptions), Ok(native_tokens), Ok(client)) = (subscriptions, native_tokens, client)
    else {
        return error_response(
            db,
            &context,
            "POST",
            CONTROLLER,
            POLICY,
            "test",
            StatusCode::SERVICE_UNAVAILABLE,
            "push_test_failed",
            "Unable to send test notification.",
            None,
        )
        .await;
    };
    let path = format!("/households/{}/dashboard", household.slug);
    let web_configuration_failed = !subscriptions.is_empty() && !vapid_configured;
    for subscription in subscriptions {
        if !vapid_configured {
            continue;
        }
        if matches!(
            web_push_delivery(&client, &subscription, &path).await,
            DeliveryOutcome::Permanent
        ) {
            push_subscription::Entity::delete_by_id(subscription.id)
                .exec(&db)
                .await
                .map_err(database_error)?;
        }
    }
    for token in native_tokens {
        let outcome = match token.platform.as_str() {
            "android" => fcm_delivery(&client, &token, &path).await,
            "ios" => apns_delivery(&client, &token, &path).await,
            _ => DeliveryOutcome::Transient,
        };
        if matches!(outcome, DeliveryOutcome::Permanent) {
            native_device_token::Entity::delete_by_id(token.id)
                .exec(&db)
                .await
                .map_err(database_error)?;
        }
    }
    if web_configuration_failed {
        return error_response(
            db,
            &context,
            "POST",
            CONTROLLER,
            POLICY,
            "test",
            StatusCode::SERVICE_UNAVAILABLE,
            "push_test_failed",
            "Unable to send test notification.",
            None,
        )
        .await;
    }
    let mut response = finish(
        db,
        &context,
        "POST",
        CONTROLLER,
        POLICY,
        "test",
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

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use p256::ecdsa::{signature::Verifier, Signature, VerifyingKey};

    #[test]
    fn api_dependencies_exclude_the_vulnerable_rsa_crate() {
        assert!(!include_str!("../../Cargo.lock").contains("\nname = \"rsa\"\n"));
    }

    #[test]
    fn web_push_signs_for_the_endpoint_origin_and_encrypts_for_the_subscriber() {
        let (recipient, auth) = ece::generate_keypair_and_auth_secret().unwrap();
        let public = URL_SAFE_NO_PAD.encode(recipient.pub_as_raw().unwrap());
        let private = URL_SAFE_NO_PAD.encode([7_u8; 32]);
        let subscription = SubscriptionInfo::new(
            "https://push.example/messages/123",
            &public,
            &URL_SAFE_NO_PAD.encode(auth),
        );
        let mut signature = VapidSignatureBuilder::from_base64(&private, &subscription).unwrap();
        signature.add_claim("sub", "mailto:notifications@example.com");
        let mut message = WebPushMessageBuilder::new(&subscription);
        message.set_payload(ContentEncoding::Aes128Gcm, b"push payload");
        message.set_vapid_signature(signature.build().unwrap());
        let request = request_builder::build_request::<Vec<u8>>(message.build().unwrap());
        assert_eq!(request.method().as_str(), "POST");
        assert_eq!(request.headers()["content-encoding"], "aes128gcm");
        assert_eq!(request.headers()["ttl"], "2419200");
        let authorization = request.headers()["authorization"].to_str().unwrap();
        let (token, public) = authorization
            .strip_prefix("vapid t=")
            .unwrap()
            .split_once(", k=")
            .unwrap();
        let segments = token.split('.').collect::<Vec<_>>();
        let claims: Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(segments[1]).unwrap()).unwrap();
        assert_eq!(claims["aud"], "https://push.example");
        assert_eq!(claims["sub"], "mailto:notifications@example.com");
        let expires = claims["exp"].as_i64().unwrap();
        assert!(
            (Utc::now().timestamp() + 43190..=Utc::now().timestamp() + 43200).contains(&expires)
        );
        let verifying =
            VerifyingKey::from_sec1_bytes(&URL_SAFE_NO_PAD.decode(public).unwrap()).unwrap();
        let signature =
            Signature::from_slice(&URL_SAFE_NO_PAD.decode(segments[2]).unwrap()).unwrap();
        let signed = format!("{}.{}", segments[0], segments[1]);
        verifying.verify(signed.as_bytes(), &signature).unwrap();
        assert!(verifying.verify(b"tampered", &signature).is_err());
        let encrypted = request.body();
        assert_eq!(
            ece::decrypt(&recipient.raw_components().unwrap(), &auth, encrypted).unwrap(),
            b"push payload"
        );
    }

    #[test]
    fn web_push_rejects_invalid_signing_keys() {
        let subscription =
            SubscriptionInfo::new("https://push.example/messages", "invalid", "invalid");
        for private in ["invalid", "invalid!"] {
            assert!(VapidSignatureBuilder::from_base64(private, &subscription).is_err());
        }
    }
}
