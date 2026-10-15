use crate::models::{
    access::{self, TenantTransaction},
    care::{administration, doses::CredentialProvenance, sync},
    entities::{carer_relationship, person},
    errors::OperationError,
};
use loco_rs::model::TenantQueryExt;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde_json::{Value, json};
use std::collections::HashMap;

fn invalid() -> OperationError {
    OperationError::Validation {
        details: json!({"errors":{"carer":["is not eligible for this assignment"]}}),
    }
}

pub async fn selected(
    tenant: &TenantTransaction,
    id: i64,
) -> Result<person::Model, OperationError> {
    access::recheck(tenant).await?;
    access::require_participation(tenant, id).await?;
    let row = crate::models::access::person_scope(crate::models::ids::HouseholdId::from_database(
        tenant.scope().household_id,
    ))
    .filter(person::Column::Id.eq(id))
    .one(tenant.transaction())
    .await?
    .ok_or(OperationError::NotFound)?;
    if !matches!(row.person_type, 1 | 2)
        || row.has_capacity
        || !access::can_manage_household(tenant)
    {
        return Err(OperationError::Forbidden);
    }
    if !access::can_manage_household(tenant) {
        access::require_participation(tenant, id).await?;
    }
    Ok(row)
}

fn eligible(tenant: &TenantTransaction) -> sea_orm::Select<person::Entity> {
    access::person_scope(crate::models::ids::HouseholdId::from_database(
        tenant.scope().household_id,
    ))
    .filter(person::Column::PersonType.eq(0))
    .filter(person::Column::HasCapacity.eq(true))
}

pub async fn page(tenant: &TenantTransaction, id: i64) -> Result<Value, OperationError> {
    let patient = selected(tenant, id).await?;
    let manager = access::can_manage_household(tenant);
    let mut query = carer_relationship::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .filter(carer_relationship::Column::PatientId.eq(id));
    if !manager {
        query = query.filter(carer_relationship::Column::Active.eq(true));
    }
    let rows = query
        .order_by_asc(carer_relationship::Column::Id)
        .all(tenant.transaction())
        .await?;
    let names: HashMap<_, _> = crate::models::person_identity::display_names(
        tenant,
        rows.iter()
            .map(|row| crate::models::ids::PersonId::from_database(row.carer_id)),
    )
    .await?
    .into_iter()
    .map(|(id, name)| (id.get(), name))
    .collect();
    let patient = crate::models::person_identity::summaries(tenant, vec![patient])
        .await?
        .pop()
        .ok_or(OperationError::NotFound)?;
    let records = rows
        .into_iter()
        .map(|row| {
            json!({
                "id":row.id,"name":names.get(&row.carer_id),"kind":row.relationship_type.as_deref().filter(|kind|matches!(*kind,"parent"|"family_member"|"professional_carer")).unwrap_or("unknown"),
                "active":row.active,"version":row.updated_at.to_string()
            })
        })
        .collect::<Vec<_>>();
    let options = if manager {
        let people = eligible(tenant).all(tenant.transaction()).await?;
        crate::models::person_identity::summaries(tenant, people)
            .await?
            .into_iter()
            .map(|row| json!({"id":row.id,"name":row.name}))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    Ok(
        json!({"person":{"id":patient.id,"name":patient.name},"manager":manager,"records":records,"options":options}),
    )
}

pub enum AssignmentOutcome {
    Assigned,
    Invited,
}

pub async fn assign(
    tenant: &TenantTransaction,
    id: i64,
    draft: &HashMap<String, String>,
    provenance: &CredentialProvenance,
) -> Result<AssignmentOutcome, OperationError> {
    sync::lock(tenant).await?;
    selected(tenant, id).await?;
    if access::can_manage_household(tenant) {
        let carer_id = draft
            .get("carer_id")
            .and_then(|value| value.parse::<i64>().ok())
            .ok_or_else(invalid)?;
        let carer = eligible(tenant)
            .filter(person::Column::Id.eq(carer_id))
            .one(tenant.transaction())
            .await?
            .ok_or_else(invalid)?;
        let kind = draft
            .get("relationship_type")
            .map(String::as_str)
            .filter(|kind| matches!(*kind, "parent" | "family_member" | "professional_carer"))
            .ok_or_else(invalid)?;
        administration::delegation::assign(tenant, carer.id, id, kind, Some(provenance)).await?;
        return Ok(AssignmentOutcome::Assigned);
    }
    Err(OperationError::Forbidden)
}

pub async fn change(
    tenant: &TenantTransaction,
    patient_id: i64,
    id: i64,
    active: bool,
    expected: &str,
    provenance: &CredentialProvenance,
) -> Result<(), OperationError> {
    sync::lock(tenant).await?;
    administration::authorize(tenant).await?;
    selected(tenant, patient_id).await?;
    let row = carer_relationship::Entity::find_by_id(id)
        .in_tenant(tenant.scope().household_id)
        .filter(carer_relationship::Column::PatientId.eq(patient_id))
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    if row.active != active && row.updated_at.to_string() != expected {
        return Err(OperationError::Conflict {
            code: "stale_relationship".into(),
            details: json!({"error":"Relationship has changed"}),
        });
    }
    if active {
        eligible(tenant)
            .filter(person::Column::Id.eq(row.carer_id))
            .one(tenant.transaction())
            .await?
            .ok_or_else(invalid)?;
        administration::delegation::reactivate(tenant, id, Some(provenance)).await?;
    } else {
        administration::delegation::deactivate(tenant, id, Some(provenance)).await?;
    }
    Ok(())
}

pub async fn can_assign(tenant: &TenantTransaction, id: i64) -> Result<bool, OperationError> {
    match selected(tenant, id).await {
        Ok(_) => Ok(true),
        Err(OperationError::Forbidden | OperationError::NotFound) => Ok(false),
        Err(error) => Err(error),
    }
}
