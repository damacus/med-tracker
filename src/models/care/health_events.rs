pub mod browser;

use crate::models::{
    access::{self, PersonAccess, TenantTransaction},
    care::{doses::CredentialProvenance, people::Pagination, sync},
    entities::health_event,
    errors::OperationError,
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};
use serde_json::{Value, json};

fn scope(tenant: &TenantTransaction) -> sea_orm::Select<health_event::Entity> {
    health_event::Entity::find()
        .filter(health_event::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(
            health_event::Column::PersonId.in_subquery(access::granted_people(tenant.membership())),
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
    let size = page.per_page.unwrap_or(20).min(100);
    if number < 1 || size < 1 {
        return Err(OperationError::Validation {
            details: json!({"errors":{"page":["must be positive"]}}),
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

pub async fn mutate(
    tenant: &TenantTransaction,
    id: Option<&str>,
    body: &Value,
    if_match: Option<&str>,
    provenance: &CredentialProvenance,
) -> Result<(Value, String), OperationError> {
    sync::lock(tenant).await?;
    if let Some(id) = id {
        let row = selected(tenant, id).await?;
        access::require_person_access(tenant, row.person_id, PersonAccess::Manage).await?;
    }
    let fields = body
        .as_object()
        .filter(|outer| outer.len() == 1)
        .and_then(|outer| outer.get("health_event"))
        .and_then(Value::as_object)
        .ok_or_else(|| OperationError::Validation {
            details: json!({"status":400,"code":"bad_request","message":"Invalid request body"}),
        })?;
    let operation = sync::Operation {
        resource_type: "health_event".into(),
        action: if id.is_some() { "update" } else { "create" }.into(),
        id: id.map(str::to_owned),
        if_match: if_match.map(str::to_owned),
        attributes: fields.clone(),
    };
    let result = sync::health_events::apply(tenant, &operation, provenance).await?;
    let id = result["record_id"]
        .as_str()
        .ok_or(OperationError::Unavailable)?;
    read(tenant, id).await
}
