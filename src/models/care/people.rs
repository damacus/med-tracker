mod creation;
mod me;
mod persistence;
mod projection;
mod validation;
mod writing;

pub use creation::create;
pub(crate) use me::me_read;
pub(crate) use projection::values;
pub use projection::{Pagination, list, read, representation};
pub use writing::update;

use crate::models::{
    access::{self, TenantTransaction},
    care::doses::CredentialProvenance,
    entities::{api_change_event, notification_preference, person, version},
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
        OperationError::Unavailable
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
    if !access::participates(tenant, id).await? {
        return Ok(false);
    }
    if tenant.person_id().get() == id {
        return Ok(true);
    }
    if !access::can_manage_household(tenant) {
        return Ok(false);
    }
    use sea_orm::{ConnectionTrait, DbBackend, Statement};
    let row = tenant
        .transaction()
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT med_tracker.can_edit_clinical_person($1) AS permitted",
            [id.into()],
        ))
        .await?
        .ok_or(OperationError::Unavailable)?;
    Ok(row.try_get("", "permitted")?)
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
    if !can_manage(tenant, found.id).await? {
        return Err(OperationError::Forbidden);
    }
    Ok(found)
}
async fn lock(tenant: &TenantTransaction) -> Result<(), OperationError> {
    access::lock_household(tenant).await
}
