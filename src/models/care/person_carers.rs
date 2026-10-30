use crate::models::{
    access::{self, PersonAccess, TenantTransaction},
    authorization,
    care::{administration, invitations, sync},
    entities::{account, carer_relationship, membership, person},
    errors::OperationError,
    identity::credentials::CredentialProvenance,
};
use loco_rs::model::TenantQueryExt;
use sea_orm::sea_query::Expr;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, QueryTrait};
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
    access::require_person_access(tenant, id, PersonAccess::View).await?;
    let row =
        crate::models::access::person_queries::household_participants(tenant.scope().household_id)
            .filter(crate::models::entities::person::Column::Id.eq(id))
            .one(tenant.transaction())
            .await?
            .ok_or(OperationError::NotFound)?;
    if !matches!(row.person_type, 1 | 2) || row.has_capacity {
        return Err(OperationError::Forbidden);
    }
    Ok(row)
}

fn eligible(tenant: &TenantTransaction, visible: bool) -> sea_orm::Select<person::Entity> {
    let active_accounts = account::Entity::find()
        .select_only()
        .column(account::Column::PersonId)
        .filter(account::Column::Status.eq(2))
        .filter(account::Column::Active.eq(true))
        .into_query();
    let mut query =
        crate::models::access::person_queries::household_participants(tenant.scope().household_id)
            .filter(person::Column::PersonType.eq(0))
            .filter(person::Column::HasCapacity.eq(true))
            .filter(person::Column::Id.in_subquery(active_accounts));
    if visible {
        query = query.filter(
            person::Column::Id.in_subquery(access::participating_people(tenant.membership())),
        );
    }
    query
}

pub async fn page(tenant: &TenantTransaction, id: i64) -> Result<Value, OperationError> {
    let patient = selected(tenant, id).await?;
    let manager = access::can_manage_household(tenant);
    let can_assign = authorization::may_delegate(tenant.membership(), &patient);
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
    let names: HashMap<_, _> =
        crate::models::access::person_queries::household_participants(tenant.scope().household_id)
            .filter(person::Column::Id.is_in(rows.iter().map(|row| row.carer_id)))
            .all(tenant.transaction())
            .await?
            .into_iter()
            .map(|row| (row.id, row.name))
            .collect();
    let records = rows
        .into_iter()
        .map(|row| {
            json!({
                "id":row.id,"name":names.get(&row.carer_id),"kind":row.relationship_type.as_deref().filter(|kind|matches!(*kind,"parent"|"family_member"|"professional_carer")).unwrap_or("unknown"),
                "active":row.active,"version":row.updated_at.to_string()
            })
        })
        .collect::<Vec<_>>();
    let options = if manager && can_assign {
        eligible(tenant, true)
            .order_by_asc(person::Column::Name)
            .order_by_asc(person::Column::Id)
            .all(tenant.transaction())
            .await?
            .into_iter()
            .map(|row| json!({"id":row.id,"name":row.name}))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    Ok(
        json!({"person":{"id":patient.id,"name":patient.name},"manager":manager,"can_assign":can_assign,"records":records,"options":options}),
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
    target: &url::Url,
    provenance: &CredentialProvenance,
) -> Result<AssignmentOutcome, OperationError> {
    sync::lock(tenant).await?;
    let patient = selected(tenant, id).await?;
    if !authorization::may_delegate(tenant.membership(), &patient) {
        return Err(OperationError::Forbidden);
    }
    if access::can_manage_household(tenant) {
        let carer_id = draft
            .get("carer_id")
            .and_then(|value| value.parse::<i64>().ok())
            .ok_or_else(invalid)?;
        let carer = eligible(tenant, true)
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
    let email = invitations::validated_email(draft.get("email").map_or("", String::as_str))?;
    let account = account::Entity::find()
        .filter(account::Column::Email.eq(&email))
        .one(tenant.transaction())
        .await?;
    if let Some(account) = account {
        let carer = eligible(tenant, false)
            .filter(person::Column::Id.eq(account.person_id))
            .filter(Expr::cust("NULLIF(btrim(professional_title), '') IS NULL"))
            .one(tenant.transaction())
            .await?
            .ok_or_else(invalid)?;
        let self_relation = carer_relationship::Entity::find()
            .in_tenant(tenant.scope().household_id)
            .filter(carer_relationship::Column::CarerId.eq(carer.id))
            .filter(carer_relationship::Column::PatientId.eq(carer.id))
            .filter(carer_relationship::Column::Active.eq(true))
            .filter(carer_relationship::Column::RelationshipType.is_in(["self", "0"]))
            .one(tenant.transaction())
            .await?;
        if self_relation.is_some() {
            return Err(invalid());
        }
        let existing = carer_relationship::Entity::find()
            .in_tenant(tenant.scope().household_id)
            .filter(carer_relationship::Column::CarerId.eq(carer.id))
            .filter(carer_relationship::Column::PatientId.eq(id))
            .one(tenant.transaction())
            .await?;
        let member = membership::Entity::find()
            .in_tenant(tenant.scope().household_id)
            .filter(membership::Column::PersonId.eq(account.person_id))
            .one(tenant.transaction())
            .await?;
        if existing.is_some_and(|row| !row.active)
            || member.is_some_and(|row| row.status != "active" || row.revoked_at.is_some())
        {
            return Err(OperationError::Forbidden);
        }
        administration::delegation::assign(tenant, carer.id, id, "parent", Some(provenance))
            .await?;
    } else {
        invitations::create_for_parent(tenant, id, &email, target, Some(provenance)).await?;
        return Ok(AssignmentOutcome::Invited);
    }
    Ok(AssignmentOutcome::Assigned)
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
        eligible(tenant, false)
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

pub async fn can_read(tenant: &TenantTransaction, id: i64) -> Result<bool, OperationError> {
    match selected(tenant, id).await {
        Ok(_) => Ok(true),
        Err(OperationError::Forbidden | OperationError::NotFound) => Ok(false),
        Err(error) => Err(error),
    }
}

pub async fn can_assign(tenant: &TenantTransaction, id: i64) -> Result<bool, OperationError> {
    match selected(tenant, id).await {
        Ok(patient) => Ok(authorization::may_delegate(tenant.membership(), &patient)),
        Err(OperationError::Forbidden | OperationError::NotFound) => Ok(false),
        Err(error) => Err(error),
    }
}
