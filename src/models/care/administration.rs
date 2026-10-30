pub mod app_tokens;
pub mod audit_logs;
pub mod delegation;
pub mod grants;
pub mod member_browser;
pub mod member_history;
pub mod memberships;
pub mod overview;
pub(crate) mod persistence;
pub mod recovery;
pub mod settings;

use crate::models::{
    access::{self, TenantTransaction},
    care::locations,
    entities::{
        account, carer_relationship, grant, household, household_invitation, membership, person,
        security_audit_event, version,
    },
    errors::OperationError,
    identity::credentials::CredentialProvenance,
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
