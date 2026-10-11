mod inventory;
mod persistence;
mod reading;
pub(crate) use reading::value;
mod removal;
mod validation;
mod writing;

use crate::models::care::{administration, doses::CredentialProvenance};
use crate::models::{
    access::{self, TenantTransaction},
    entities::{api_change_event, dosage, medication},
    errors::OperationError,
};
use chrono::Utc;
use sea_orm::prelude::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub use reading::{Pagination, for_medication, list, read};
pub use removal::destroy;
use validation::{attributes, storage_decimal, valid_persisted_dosage};
pub(crate) use writing::{add_stock, create_with_parent};
pub use writing::{create, update};

fn invalid() -> OperationError {
    OperationError::Validation {
        details: json!({"errors":{"dosage_option":["is invalid"]}}),
    }
}
fn database_error(_: sea_orm::DbErr) -> OperationError {
    OperationError::Unavailable
}

pub async fn can_manage(tenant: &TenantTransaction) -> Result<bool, OperationError> {
    access::recheck(tenant).await?;
    Ok(matches!(
        tenant.membership().role.as_str(),
        "owner" | "administrator"
    ))
}
async fn authorize(tenant: &TenantTransaction) -> Result<(), OperationError> {
    administration::authorize(tenant).await
}

async fn parent(
    tenant: &TenantTransaction,
    id: &str,
    lock: bool,
) -> Result<medication::Model, OperationError> {
    let query = medication::Entity::find()
        .filter(medication::Column::HouseholdId.eq(tenant.scope().household_id));
    let query = if let Ok(id) = id.parse::<i64>() {
        query.filter(medication::Column::Id.eq(id))
    } else {
        query.filter(medication::Column::PortableId.eq(id))
    };
    let query = if lock { query.lock_exclusive() } else { query };
    query
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)
}

async fn record(
    tenant: &TenantTransaction,
    id: &str,
    lock: bool,
) -> Result<dosage::Model, OperationError> {
    let query =
        dosage::Entity::find().filter(dosage::Column::HouseholdId.eq(tenant.scope().household_id));
    let query = if let Ok(id) = id.parse::<i64>() {
        query.filter(dosage::Column::Id.eq(id))
    } else {
        query.filter(dosage::Column::PortableId.eq(id))
    };
    let query = if lock { query.lock_exclusive() } else { query };
    query
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)
}
