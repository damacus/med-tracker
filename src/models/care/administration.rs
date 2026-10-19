pub mod app_tokens;
pub mod audit_logs;
pub mod delegation;
pub mod grants;
pub mod memberships;
pub(crate) mod persistence;
pub mod settings;

use crate::models::{
    access::{self, TenantTransaction},
    care::{doses::CredentialProvenance, locations},
    entities::{
        account, carer_relationship, grant, household, membership, person, security_audit_event,
        user, version,
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
    Ok(access::can_manage_household(tenant))
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
