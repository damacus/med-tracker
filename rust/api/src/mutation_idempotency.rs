use crate::entities::{api_idempotency_key, household};
use crate::{authenticate, database_error, ApiError, AppState, AuthContext, CredentialKind};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, QuerySelect, Set,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub(super) enum Lookup {
    New,
    Replay(Box<api_idempotency_key::Model>),
    Conflict,
}

pub(super) struct StoredResponse<'a> {
    pub key: &'a str,
    pub method: &'a str,
    pub path: &'a str,
    pub digest: &'a str,
    pub status: StatusCode,
    pub body: Value,
    pub request_id: &'a str,
    pub etag: Option<&'a str>,
}

pub(super) fn key(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
}

pub(super) fn digest(method: &str, path: &str, body: &Value) -> String {
    let request = json!({"method": method, "path": path, "body": body});
    format!("{:x}", Sha256::digest(request.to_string().as_bytes()))
}

pub(super) async fn lock_household_and_reauthenticate(
    state: &AppState,
    db: &DatabaseTransaction,
    headers: &HeaderMap,
    household_id: i64,
) -> Result<(household::Model, AuthContext), ApiError> {
    let home = household::Entity::find_by_id(household_id)
        .lock_exclusive()
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?;
    let context = authenticate(state, db, headers, household_id).await?;
    Ok((home, context))
}

pub(super) async fn lookup(
    db: &DatabaseTransaction,
    context: &AuthContext,
    key: &str,
    method: &str,
    path: &str,
    digest: &str,
) -> Result<Lookup, ApiError> {
    let found = api_idempotency_key::Entity::find()
        .filter(api_idempotency_key::Column::HouseholdId.eq(context.membership.household_id))
        .filter(api_idempotency_key::Column::Key.eq(key))
        .one(db)
        .await
        .map_err(database_error)?;
    let Some(found) = found else {
        return Ok(Lookup::New);
    };
    if found.expires_at <= Utc::now().naive_utc() {
        api_idempotency_key::Entity::delete_by_id(found.id)
            .exec(db)
            .await
            .map_err(database_error)?;
        return Ok(Lookup::New);
    }
    if found.account_id == context.account_id
        && found.request_method == method
        && found.request_path == path
        && found.request_digest == digest
    {
        Ok(Lookup::Replay(Box::new(found)))
    } else {
        Ok(Lookup::Conflict)
    }
}

pub(super) fn replay(saved: api_idempotency_key::Model) -> Result<Response, ApiError> {
    let status =
        StatusCode::from_u16(saved.response_status as u16).map_err(|_| ApiError::internal())?;
    let mut response = if status == StatusCode::NO_CONTENT {
        status.into_response()
    } else {
        (status, Json(saved.response_body)).into_response()
    };
    response
        .headers_mut()
        .insert("idempotency-replayed", HeaderValue::from_static("true"));
    if let Some(headers) = saved.response_headers.as_object() {
        for (name, value) in headers {
            if !matches!(name.as_str(), "ETag" | "x-request-id") {
                continue;
            }
            if let Some(value) = value.as_str() {
                let name = axum::http::HeaderName::try_from(name.as_str())
                    .map_err(|_| ApiError::internal())?;
                let value = HeaderValue::from_str(value).map_err(|_| ApiError::internal())?;
                response.headers_mut().insert(name, value);
            }
        }
    }
    Ok(response)
}

pub(super) async fn store(
    db: &DatabaseTransaction,
    context: &AuthContext,
    response: StoredResponse<'_>,
) -> Result<(), ApiError> {
    if response.status.is_server_error() || response.status == StatusCode::CONFLICT {
        return Ok(());
    }
    let now = Utc::now().naive_utc();
    let reference = context.credential_reference.parse::<i64>().ok();
    let (api_session_id, api_app_token_id) = match context.credential_kind {
        CredentialKind::ApiSession => (reference, None),
        CredentialKind::ApiAppToken => (None, reference),
        CredentialKind::OauthGrant | CredentialKind::BrowserSession => (None, None),
    };
    api_idempotency_key::ActiveModel {
        household_id: Set(context.membership.household_id),
        account_id: Set(context.account_id),
        api_session_id: Set(api_session_id),
        api_app_token_id: Set(api_app_token_id),
        key: Set(response.key.to_owned()),
        request_method: Set(response.method.to_owned()),
        request_path: Set(response.path.to_owned()),
        request_digest: Set(response.digest.to_owned()),
        response_status: Set(i32::from(response.status.as_u16())),
        response_body: Set(response.body),
        response_headers: Set(json!({"x-request-id": response.request_id, "ETag": response.etag})),
        expires_at: Set(now + Duration::hours(24)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(database_error)?;
    Ok(())
}
