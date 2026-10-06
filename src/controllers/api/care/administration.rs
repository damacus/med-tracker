use super::*;
use crate::models::care::{
    administration,
    locations::{self, SavedResponse},
};
mod grants;
mod memberships;
mod settings;
pub(super) use grants::{
    create as grants_create, destroy as grants_destroy, index as grants_index,
};
pub(super) use memberships::{
    destroy as memberships_destroy, index as memberships_index, update as memberships_update,
};
pub(super) use settings::{show as settings_show, update as settings_update};

fn request_id(request: Option<Extension<LocoRequestId>>) -> String {
    request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    )
}
fn key(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
}
fn attributes(body: &Value, envelope: &str) -> std::result::Result<Value, response::Failure> {
    let allowed: &[&str] = match envelope {
        "household" => &["name", "timezone", "subscription_plan"],
        "household_membership" => &["role", "status", "person_id"],
        "person_access_grant" => &[
            "household_membership_id",
            "person_id",
            "access_level",
            "relationship_type",
            "expires_at",
        ],
        _ => return Err(response::Failure::bad_request("Invalid request body")),
    };
    let fields = body
        .get(envelope)
        .and_then(Value::as_object)
        .ok_or_else(|| response::Failure::bad_request("Invalid request body"))?;
    let attributes: serde_json::Map<String, Value> = fields
        .iter()
        .filter(|(name, value)| {
            allowed.contains(&name.as_str()) && !value.is_object() && !value.is_array()
        })
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();
    if attributes.is_empty() {
        return Err(response::Failure::bad_request("Invalid request body"));
    }
    Ok(Value::Object(attributes))
}
fn numeric_id(value: &str) -> std::result::Result<i64, response::Failure> {
    value
        .parse::<i64>()
        .ok()
        .filter(|id| {
            *id > 0 && value.bytes().all(|byte| byte.is_ascii_digit()) && !value.starts_with('0')
        })
        .ok_or_else(|| response::operation(OperationError::NotFound))
}
async fn complete(
    tenant: TenantTransaction,
    principal: &ValidatedPrincipal,
    audit: audit::RequestAudit,
    result: std::result::Result<(StatusCode, Value, Option<String>, bool), response::Failure>,
    request_id: &str,
) -> Response {
    let replay = result.as_ref().is_ok_and(|(_, _, _, replay)| *replay);
    let mut response = finish(
        tenant,
        principal.provenance(),
        audit,
        result.map(|(status, body, etag, _)| (status, body, etag)),
        request_id,
    )
    .await;
    if replay && response.status().as_u16() < 500 && response.status() != StatusCode::CONFLICT {
        response.headers_mut().insert(
            "idempotency-replayed",
            axum::http::HeaderValue::from_static("true"),
        );
    }
    response
}
async fn store(
    tenant: &TenantTransaction,
    principal: &ValidatedPrincipal,
    saved: SavedResponse<'_>,
) -> std::result::Result<(StatusCode, Value, Option<String>, bool), response::Failure> {
    let status = StatusCode::from_u16(saved.status).map_err(|_| response::unavailable())?;
    let body = saved.body.clone();
    locations::store(tenant, principal.provenance(), saved)
        .await
        .map_err(response::operation)?;
    Ok((status, body, None, false))
}
async fn reply(
    savepoint: DatabaseTransaction,
    tenant: &TenantTransaction,
    result: Result<Value, OperationError>,
    success: StatusCode,
) -> std::result::Result<(StatusCode, Value), response::Failure> {
    match result {
        Ok(body) => {
            savepoint
                .commit()
                .await
                .map_err(|_| response::unavailable())?;
            Ok((success, body))
        }
        Err(OperationError::Validation { details }) => {
            savepoint
                .commit()
                .await
                .map_err(|_| response::unavailable())?;
            Ok((
                StatusCode::UNPROCESSABLE_ENTITY,
                json!({"error":{"code":"validation_failed","message":"Validation failed","request_id":tenant.scope().request_id,"errors":details["errors"]}}),
            ))
        }
        Err(error) => {
            savepoint
                .rollback()
                .await
                .map_err(|_| response::unavailable())?;
            Err(response::operation(error))
        }
    }
}
