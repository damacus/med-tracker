use crate::models::{
    access::{self, TenantTransaction},
    authorization,
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
    let mut query =
        crate::models::access::person_queries::household_participants(tenant.scope().household_id)
            .filter(
                person::Column::Id.in_subquery(access::participating_people(tenant.membership())),
            )
            .filter(person::Column::PersonType.is_in([1, 2]))
            .filter(person::Column::HasCapacity.eq(false));
    if let Some(ids) = selected {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        query = query.filter(person::Column::Id.is_in(ids.iter().copied()));
    }
    let rows = query
        .order_by_asc(person::Column::Name)
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
    let manageable = access::managed_people(tenant.membership());
    let manageable: HashSet<i64> = access::person_queries::household_people(tenant.scope())
        .select_only()
        .column(person::Column::Id)
        .filter(person::Column::Id.in_subquery(manageable))
        .filter(person::Column::Id.is_in(rows.iter().map(|row| row.id)))
        .into_tuple::<i64>()
        .all(tenant.transaction())
        .await?
        .into_iter()
        .collect();
    Ok(rows
        .into_iter()
        .filter(|row| !assigned.contains(&row.id))
        .map(|row| {
            let can_assign = authorization::may_delegate(tenant.membership(), &row)
                && (access::can_manage_household(tenant) || manageable.contains(&row.id));
            (
                row.id,
                json!({"person_id":row.id,"person_name":row.name,"can_assign":can_assign}),
            )
        })
        .collect())
}
