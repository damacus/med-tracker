use super::*;
use crate::models::{care::doses::CredentialMethod, entities::api_idempotency_key};
use chrono::Duration;

pub enum KeyedResponse {
    New,
    Replay(Box<api_idempotency_key::Model>),
    Conflict,
}

pub async fn lookup(
    tenant: &TenantTransaction,
    key: &str,
    method: &str,
    path: &str,
    body: &Value,
) -> Result<KeyedResponse, OperationError> {
    let saved = api_idempotency_key::Entity::find()
        .filter(api_idempotency_key::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(api_idempotency_key::Column::Key.eq(key))
        .one(tenant.transaction())
        .await
        .map_err(|_| OperationError::Unavailable)?;
    let Some(saved) = saved else {
        return Ok(KeyedResponse::New);
    };
    if saved.expires_at <= Utc::now().naive_utc() {
        api_idempotency_key::Entity::delete_by_id(saved.id)
            .exec(tenant.transaction())
            .await
            .map_err(|_| OperationError::Unavailable)?;
        return Ok(KeyedResponse::New);
    }
    if saved.account_id == tenant.scope().actor.account_id
        && saved.request_method == method
        && saved.request_path == path
        && (saved.request_digest == digest(method, path, body)
            || rails_digest::digest(tenant, method, path, body).as_deref()
                == Some(saved.request_digest.as_str()))
    {
        Ok(KeyedResponse::Replay(Box::new(saved)))
    } else {
        Ok(KeyedResponse::Conflict)
    }
}

fn digest(method: &str, path: &str, body: &Value) -> String {
    let mut request = json!({"method":method,"path":path,"body":body});
    request.sort_all_objects();
    hex::encode(Sha256::digest(request.to_string().as_bytes()))
}

pub struct SavedResponse<'a> {
    pub key: &'a str,
    pub method: &'a str,
    pub path: &'a str,
    pub request: &'a Value,
    pub status: u16,
    pub body: Value,
    pub etag: Option<&'a str>,
}

pub async fn store(
    tenant: &TenantTransaction,
    provenance: &CredentialProvenance,
    response: SavedResponse<'_>,
) -> Result<(), OperationError> {
    let now = Utc::now().naive_utc();
    let reference = provenance.reference.parse::<i64>().ok();
    api_idempotency_key::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        account_id: Set(tenant.scope().actor.account_id),
        api_session_id: Set(
            if matches!(provenance.method, CredentialMethod::ApiSession) {
                reference
            } else {
                None
            },
        ),
        api_app_token_id: Set(
            if matches!(provenance.method, CredentialMethod::ApiAppToken) {
                reference
            } else {
                None
            },
        ),
        key: Set(response.key.into()),
        request_method: Set(response.method.into()),
        request_path: Set(response.path.into()),
        request_digest: Set(digest(response.method, response.path, response.request)),
        response_status: Set(i32::from(response.status)),
        response_body: Set(response.body),
        response_headers: Set(
            json!({"x-request-id":tenant.scope().request_id,"ETag":response.etag}),
        ),
        expires_at: Set(now + Duration::hours(24)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await
    .map_err(|_| OperationError::Unavailable)?;
    Ok(())
}
