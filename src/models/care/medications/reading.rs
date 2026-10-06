use super::{database_error, projection, validation};
use crate::models::{
    access::{self, TenantTransaction},
    entities::medication,
    errors::OperationError,
};
use chrono::NaiveDateTime;
use sea_orm::{ColumnTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};
use serde_json::{Value, json};

pub async fn list(
    tenant: &TenantTransaction,
    page: i64,
    per_page: i64,
    updated_since: Option<NaiveDateTime>,
) -> Result<Value, OperationError> {
    access::recheck(tenant).await?;
    if page < 1 || !(1..=100).contains(&per_page) {
        return Err(validation("Invalid pagination"));
    }
    let mut query = access::medication_scope(tenant);
    if let Some(timestamp) = updated_since {
        query = query.filter(medication::Column::UpdatedAt.gte(timestamp));
    }
    let count = query
        .clone()
        .count(tenant.transaction())
        .await
        .map_err(database_error)?;
    let records = query
        .order_by_asc(medication::Column::Id)
        .limit(per_page as u64)
        .offset(page.saturating_sub(1).saturating_mul(per_page) as u64)
        .all(tenant.transaction())
        .await
        .map_err(database_error)?;
    let data = projection::serialize_many(tenant.transaction(), records).await?;
    Ok(json!({"data":data,"meta":{"page":page,"per_page":per_page,"total_count":count}}))
}
