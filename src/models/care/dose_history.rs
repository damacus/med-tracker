use crate::models::{
    access::{self, TenantTransaction},
    entities::medication_take,
    errors::OperationError,
};
use chrono::NaiveDateTime;
use loco_rs::model::TenantQueryExt;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};

pub struct History {
    pub records: Vec<medication_take::Model>,
    pub total_count: u64,
}

pub async fn list(
    tenant: &TenantTransaction,
    page: i64,
    per_page: i64,
    updated_since: Option<NaiveDateTime>,
) -> Result<History, OperationError> {
    access::recheck(tenant).await?;
    if page < 1 || !(1..=100).contains(&per_page) {
        return Err(OperationError::Validation {
            details: serde_json::json!({"error":"Invalid pagination"}),
        });
    }
    let household_id = tenant.scope().household_id;
    let mut query = medication_take::Entity::find().in_tenant(household_id);
    if let Some(timestamp) = updated_since {
        query = query.filter(medication_take::Column::UpdatedAt.gte(timestamp));
    }
    let total_count = query.clone().count(tenant.transaction()).await?;
    let records = query
        .order_by_asc(medication_take::Column::Id)
        .limit(per_page as u64)
        .offset(page.saturating_sub(1).saturating_mul(per_page) as u64)
        .all(tenant.transaction())
        .await?;
    Ok(History {
        records,
        total_count,
    })
}
