mod creation;
mod persistence;
mod projection;
mod validation;
mod writing;

pub use creation::create;
pub(crate) use projection::values;
pub use projection::{Pagination, list, read, representation};
pub use writing::update;

use crate::models::{
    access::{self, PersonAccess, TenantTransaction},
    care::doses::{CredentialMethod, CredentialProvenance},
    entities::{
        api_change_event, carer_relationship, grant, household, location, location_membership,
        membership, notification_preference, person, security_audit_event, version,
    },
    errors::OperationError,
};
use chrono::{Datelike, NaiveDate, Utc};
use chrono_tz::Tz;
use sea_orm::sea_query::{Expr, ExprTrait};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, QueryTrait, Set,
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
        OperationError::Unavailable
    }
}
pub(crate) fn age(date: NaiveDate, today: NaiveDate) -> i32 {
    today.year()
        - date.year()
        - i32::from((today.month(), today.day()) < (date.month(), date.day()))
}
fn active_grants(tenant: &TenantTransaction) -> sea_orm::Select<grant::Entity> {
    grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(tenant.membership().id))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(
            Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(
                    Expr::col(grant::Column::ExpiresAt)
                        .gt(Expr::cust("timezone('UTC', clock_timestamp())")),
                ),
        )
}
pub async fn can_create(tenant: &TenantTransaction) -> Result<bool, OperationError> {
    access::recheck(tenant).await?;
    if access::can_manage_household(tenant) {
        return Ok(true);
    }
    access::has_person_access(tenant, PersonAccess::Manage).await
}
pub async fn can_manage(tenant: &TenantTransaction, id: i64) -> Result<bool, OperationError> {
    access::can_access_person(tenant, id, PersonAccess::Manage).await
}
pub async fn authorize_create(tenant: &TenantTransaction) -> Result<(), OperationError> {
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
