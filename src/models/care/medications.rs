mod audit;
pub(crate) use audit::medication_snapshot;
pub(crate) use projection::{decimal_string, serialize_many};
pub mod crud;
mod forecast;
mod inventory;
mod projection;
pub mod reading;
pub mod stock_removals;

use crate::models::{
    access::{self, TenantTransaction},
    care::doses::{CredentialMethod, CredentialProvenance},
    entities::{api_change_event, dosage, medication, version},
    errors::OperationError,
};
use chrono::Utc;
use sea_orm::prelude::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, DbBackend, EntityTrait,
    QueryFilter, Set, Statement,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::str::FromStr;

type ApiError = OperationError;

#[derive(Clone, Debug)]
pub struct AdjustStock {
    pub medication_id: String,
    pub new_quantity: String,
    pub reason: Option<String>,
}

#[derive(Clone, Debug)]
pub enum Command {
    AdjustStock(AdjustStock),
}

#[derive(Clone, Debug)]
pub struct ScalarPrecondition {
    pub original_etag: String,
}

#[derive(Debug)]
pub struct StockSnapshot {
    pub medication: medication::Model,
    pub representation: Value,
    pub etag: String,
}

struct StockContext<'a> {
    tenant: &'a TenantTransaction,
    provenance: Option<&'a CredentialProvenance>,
}

pub async fn execute(
    tenant: &TenantTransaction,
    command: Command,
) -> Result<medication::Model, OperationError> {
    execute_with_options(tenant, command, None, None).await
}

pub async fn execute_with_options(
    tenant: &TenantTransaction,
    command: Command,
    scalar: Option<&ScalarPrecondition>,
    provenance: Option<&CredentialProvenance>,
) -> Result<medication::Model, OperationError> {
    let Command::AdjustStock(input) = command;
    inventory::adjust(&StockContext { tenant, provenance }, input, scalar).await
}

pub async fn read_stock_snapshot(
    tenant: &TenantTransaction,
    id: &str,
) -> Result<StockSnapshot, OperationError> {
    access::recheck(tenant).await?;
    let medication = visible_medication(tenant, id)
        .await?
        .ok_or(OperationError::NotFound)?;
    snapshot(tenant.transaction(), medication).await
}

async fn snapshot(
    db: &DatabaseTransaction,
    medication: medication::Model,
) -> Result<StockSnapshot, OperationError> {
    let row = projection::serialize_many(db, vec![medication.clone()])
        .await?
        .remove(0);
    let mut representation = json!({"data": row});
    representation.sort_all_objects();
    let etag = format!(
        "\"{}\"",
        hex::encode(Sha256::digest(
            serde_json::to_vec(&representation).expect("JSON value must serialize")
        ))
    );
    Ok(StockSnapshot {
        medication,
        representation,
        etag,
    })
}

async fn visible_medication(
    tenant: &TenantTransaction,
    id: &str,
) -> Result<Option<medication::Model>, OperationError> {
    let query = access::medication_scope(tenant);
    let query = match id.parse::<i64>() {
        Ok(id) => query.filter(medication::Column::Id.eq(id)),
        Err(_) => query.filter(medication::Column::PortableId.eq(id)),
    };
    query
        .one(tenant.transaction())
        .await
        .map_err(database_error)
}

async fn lock_row(db: &DatabaseTransaction, table: &str, id: i64) -> Result<(), OperationError> {
    let sql = match table {
        "households" => "SELECT id FROM households WHERE id=$1 FOR UPDATE",
        "medications" => "SELECT id FROM medications WHERE id=$1 FOR UPDATE",
        _ => return Err(OperationError::Unavailable),
    };
    db.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        [id.into()],
    ))
    .await
    .map_err(database_error)?
    .ok_or(OperationError::NotFound)?;
    Ok(())
}

fn database_error(_: sea_orm::DbErr) -> OperationError {
    OperationError::Unavailable
}

fn validation(message: &str) -> OperationError {
    OperationError::Validation {
        details: json!({"error": message}),
    }
}

fn conflict() -> OperationError {
    OperationError::Conflict {
        code: "conflict".into(),
        details: json!({"error": "Record has changed since it was last read"}),
    }
}
