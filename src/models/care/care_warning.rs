use crate::models::{
    access::{self, TenantTransaction},
    entities::{carer_relationship, person},
    errors::OperationError,
};
use loco_rs::model::TenantQueryExt;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

pub async fn project(
    tenant: &TenantTransaction,
    selected: Option<&[i64]>,
) -> Result<HashMap<i64, Value>, OperationError> {
    access::recheck(tenant).await?;
    let mut query = crate::models::access::person_scope(
        crate::models::ids::HouseholdId::from_database(tenant.scope().household_id),
    )
    .filter(person::Column::Id.in_subquery(access::granted_people(tenant.membership())))
    .filter(person::Column::PersonType.is_in([1, 2]))
    .filter(person::Column::HasCapacity.eq(false));
    if let Some(ids) = selected {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        query = query.filter(person::Column::Id.is_in(ids.iter().copied()));
    }
    let rows = query
        .order_by_asc(person::Column::Id)
        .all(tenant.transaction())
        .await?;
    if rows.is_empty() {
        return Ok(HashMap::new());
    }
    let assigned: HashSet<i64> = carer_relationship::Entity::find()
        .select_only()
        .column(carer_relationship::Column::PatientId)
        .in_tenant(tenant.scope().household_id)
        .filter(carer_relationship::Column::Active.eq(true))
        .filter(carer_relationship::Column::PatientId.is_in(rows.iter().map(|row| row.id)))
        .into_tuple::<i64>()
        .all(tenant.transaction())
        .await?
        .into_iter()
        .collect();
    let names = crate::models::person_identity::display_names(
        tenant,
        rows.iter()
            .map(|row| crate::models::ids::PersonId::from_database(row.id)),
    )
    .await?;
    Ok(rows
        .into_iter()
        .filter(|row| !assigned.contains(&row.id))
        .map(|row| {
            let can_assign = access::can_manage_household(tenant);
            (
                row.id,
                json!({"person_id":row.id,"person_name":names.get(&crate::models::ids::PersonId::from_database(row.id)),"can_assign":can_assign}),
            )
        })
        .collect())
}
