use loco_rs::model::TenantQueryExt;
pub mod browser;

use crate::models::{
    access::{self, PersonAccess, TenantTransaction},
    care::{people::Pagination, sync},
    entities::health_event,
    errors::OperationError,
    identity::credentials::CredentialProvenance,
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};
use serde_json::{Value, json};
pub(crate) use sync::health_events::HealthInput;

fn scope(tenant: &TenantTransaction) -> sea_orm::Select<health_event::Entity> {
    health_event::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .filter(
            health_event::Column::PersonId
                .in_subquery(access::participating_people(tenant.membership())),
        )
}

pub async fn selected(
    tenant: &TenantTransaction,
    id: &str,
) -> Result<health_event::Model, OperationError> {
    access::recheck(tenant).await?;
    if !super::doses::valid_identifier(id) {
        return Err(OperationError::NotFound);
    }
    let query = scope(tenant);
    let query = if let Ok(id) = id.parse::<i64>() {
        query.filter(health_event::Column::Id.eq(id))
    } else {
        query.filter(health_event::Column::PortableId.eq(id))
    };
    let row = query
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    access::require_person_access(tenant, row.person_id, PersonAccess::View).await?;
    Ok(row)
}

pub async fn read(tenant: &TenantTransaction, id: &str) -> Result<(Value, String), OperationError> {
    let row = selected(tenant, id).await?;
    sync::health_events::representation(tenant.transaction(), &row).await
}

pub async fn list(tenant: &TenantTransaction, page: Pagination) -> Result<Value, OperationError> {
    access::recheck(tenant).await?;
    let number = page.page.unwrap_or(1);
    let size = page.per_page.unwrap_or(20);
    if number < 1 || !(1..=100).contains(&size) {
        return Err(OperationError::Validation {
            details: json!({"errors":{"page":["page must be positive and per_page must be between 1 and 100"]}}),
        });
    }
    let mut query = scope(tenant);
    if let Some(value) = page.updated_since {
        let date = chrono::DateTime::parse_from_rfc3339(&value)
            .map_err(|_| OperationError::Validation {
                details: json!({"errors":{"updated_since":["is invalid"]}}),
            })?
            .naive_utc();
        query = query.filter(health_event::Column::UpdatedAt.gte(date));
    }
    let total = query.clone().count(tenant.transaction()).await?;
    let rows = query
        .order_by_asc(health_event::Column::Id)
        .limit(size as u64)
        .offset(((number - 1) as u64).saturating_mul(size as u64))
        .all(tenant.transaction())
        .await?;
    Ok(
        json!({"data":sync::health_events::values(tenant.transaction(),&rows).await?,"meta":{"page":number,"per_page":size,"total_count":total}}),
    )
}

pub(crate) async fn mutate(
    tenant: &TenantTransaction,
    id: Option<&str>,
    input: Result<HealthInput, OperationError>,
    provenance: &CredentialProvenance,
) -> Result<(Value, String), OperationError> {
    sync::lock(tenant).await?;
    if let Some(id) = id {
        let row = selected(tenant, id).await?;
        access::require_person_access(tenant, row.person_id, PersonAccess::Manage).await?;
    }
    let input = input?;
    let record = sync::health_events::apply_typed(tenant, &input, provenance).await?;
    sync::health_events::representation(tenant.transaction(), &record).await
}
