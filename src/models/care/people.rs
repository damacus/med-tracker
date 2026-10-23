pub(crate) mod browser;
mod creation;
mod me;
mod persistence;
mod projection;
mod update_input;
mod validation;
mod writing;

pub(crate) use creation::create_imported;
pub use creation::{CreateDependent, create};
pub(crate) use creation::{DependentInput, DependentKind};
pub(crate) use me::me_read;
pub(crate) use projection::values;
pub use projection::{Pagination, list, read, representation};
pub(crate) use update_input::{PersonKind, PersonUpdateInput, UpdateField};
pub(crate) use writing::update;

use crate::models::{
    access::{self, PersonAccess, TenantTransaction},
    care::doses::CredentialProvenance,
    entities::{
        api_change_event, carer_relationship, household, location, location_membership, membership,
        notification_preference, person, version,
    },
    errors::OperationError,
};
use chrono::{Datelike, NaiveDate, Utc};
use chrono_tz::Tz;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

fn invalid(field: &str, message: &str) -> OperationError {
    OperationError::Validation {
        details: json!({"errors":{field:[message]}}),
    }
}
fn write_error(error: sea_orm::DbErr) -> OperationError {
    if matches!(
        error.sql_err(),
        Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
    ) {
        invalid("email", "has already been taken")
    } else {
        error.into()
    }
}

pub(crate) fn age(date: NaiveDate, today: NaiveDate) -> i32 {
    today.year()
        - date.year()
        - i32::from((today.month(), today.day()) < (date.month(), date.day()))
}
pub async fn can_create(tenant: &TenantTransaction) -> Result<bool, OperationError> {
    access::recheck(tenant).await?;
    Ok(access::can_manage_household(tenant))
}
pub async fn can_manage(tenant: &TenantTransaction, id: i64) -> Result<bool, OperationError> {
    access::can_access_person(tenant, id, PersonAccess::Manage).await
}
pub async fn authorize_create(tenant: &TenantTransaction) -> Result<(), OperationError> {
    crate::models::care::guardian_declarations::lock_creator_account(tenant).await?;
    lock(tenant).await?;
    if !can_create(tenant).await? {
        return Err(OperationError::Forbidden);
    }
    Ok(())
}
pub async fn authorize_update(
    tenant: &TenantTransaction,
    id: &str,
) -> Result<person::Model, OperationError> {
    lock(tenant).await?;
    let found = projection::selected(tenant, id).await?;
    access::require_person_access(tenant, found.id, PersonAccess::Manage).await?;
    Ok(found)
}
async fn lock(tenant: &TenantTransaction) -> Result<(), OperationError> {
    household::Entity::find_by_id(tenant.scope().household_id)
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    access::recheck(tenant).await
}

#[cfg(test)]
mod tests {
    #[test]
    fn creator_write_failure_keeps_safe_database_diagnostics() -> Result<(), String> {
        let sentinel = "synthetic-clinical-and-secret-database-detail";
        let (error, events) = crate::models::errors::tests::capture_operation(|| {
            super::write_error(sea_orm::DbErr::Exec(sea_orm::RuntimeErr::Internal(
                sentinel.to_owned(),
            )))
        })?;
        assert_eq!(error, crate::models::errors::OperationError::Unavailable);
        assert_eq!(events.len(), 1);
        let fields = events
            .first()
            .ok_or("Expected creator database diagnostic")?;
        assert!(
            fields
                .iter()
                .any(|(name, value)| name == "category" && value == "execution")
        );
        let trace = fields
            .iter()
            .find(|(name, _)| name == "error_trace_id")
            .map(|(_, value)| value.as_str())
            .ok_or("Expected creator error trace ID")?;
        assert!(uuid::Uuid::parse_str(trace).is_ok());
        assert!(!format!("{fields:?}").contains(sentinel));
        assert!(!error.to_string().contains(sentinel));
        Ok(())
    }
}
