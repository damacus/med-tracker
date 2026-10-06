use super::*;
use chrono::DateTime;
use sea_orm::{PaginatorTrait, QueryOrder};

#[derive(serde::Deserialize, Default)]
pub struct Pagination {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub updated_since: Option<String>,
}

pub async fn collection(
    tenant: &TenantTransaction,
    page: Pagination,
) -> Result<Value, OperationError> {
    access::recheck(tenant).await?;
    let number = page.page.unwrap_or(1);
    let size = page.per_page.unwrap_or(20);
    if number < 1 || !(1..=100).contains(&size) {
        return Err(OperationError::Validation {
            details: json!({"error":"page must be positive and per_page must be between 1 and 100"}),
        });
    }
    let updated = page
        .updated_since
        .map(|value| DateTime::parse_from_rfc3339(&value).map(|value| value.naive_utc()))
        .transpose()
        .map_err(|_| OperationError::Validation {
            details: json!({"error":"updated_since must be ISO8601"}),
        })?;
    let mut query = location::Entity::find()
        .filter(location::Column::HouseholdId.eq(tenant.scope().household_id));
    if let Some(updated) = updated {
        query = query.filter(location::Column::UpdatedAt.gte(updated));
    }
    let total = query
        .clone()
        .count(tenant.transaction())
        .await
        .map_err(|_| OperationError::Unavailable)?;
    let rows = query
        .order_by_asc(location::Column::Id)
        .limit(size as u64)
        .offset(number.saturating_sub(1).saturating_mul(size) as u64)
        .all(tenant.transaction())
        .await
        .map_err(|_| OperationError::Unavailable)?;
    let data: Vec<Value> = rows
        .iter()
        .map(|record| representation(record).0["data"].clone())
        .collect();
    Ok(json!({"data":data,"meta":{"page":number,"per_page":size,"total_count":total}}))
}
