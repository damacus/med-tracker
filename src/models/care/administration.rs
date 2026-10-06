pub mod delegation;
pub mod grants;
pub mod memberships;
pub(crate) mod persistence;
pub mod settings;

use crate::models::{
    access::{self, TenantTransaction},
    care::{
        doses::{CredentialMethod, CredentialProvenance},
        locations,
    },
    entities::{
        account, carer_relationship, grant, household, membership, person, platform_admin,
        security_audit_event, user, version,
    },
    errors::OperationError,
};
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde_json::{Value, json};
use std::collections::HashMap;

pub async fn can_manage(tenant: &TenantTransaction) -> Result<bool, OperationError> {
    access::recheck(tenant).await?;
    Ok(matches!(
        tenant.membership().role.as_str(),
        "owner" | "administrator"
    ))
}
pub async fn authorize(tenant: &TenantTransaction) -> Result<(), OperationError> {
    locations::authorize(tenant).await
}
fn invalid(field: &str, message: &str) -> OperationError {
    OperationError::Validation {
        details: json!({"errors": {field: [message]}}),
    }
}
fn fields<'a>(
    attributes: &'a Value,
    allowed: &[&str],
    name: &str,
) -> Result<&'a serde_json::Map<String, Value>, OperationError> {
    let fields = attributes
        .as_object()
        .ok_or_else(|| invalid(name, "is invalid"))?;
    if fields.is_empty() || fields.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err(invalid(name, "is invalid"));
    }
    Ok(fields)
}
fn positive_id(attributes: &Value, name: &str) -> Result<i64, OperationError> {
    attributes
        .get(name)
        .and_then(Value::as_i64)
        .filter(|id| *id > 0)
        .ok_or_else(|| invalid(name, "is invalid"))
}
