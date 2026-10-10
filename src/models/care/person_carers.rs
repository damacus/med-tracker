use crate::models::{
    access::{self, PersonAccess, TenantTransaction},
    authorization,
    care::{administration, doses::CredentialProvenance, invitations, sync},
    entities::{account, carer_relationship, membership, person, user},
    errors::OperationError,
};
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
    let row = person::Entity::find_by_id(id)
        .filter(person::Column::HouseholdId.eq(tenant.scope().household_id))
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    if !matches!(row.person_type, 1 | 2)
        || row.has_capacity
        || !authorization::may_delegate(tenant.membership(), &row)
    {
        return Err(OperationError::Forbidden);
    }
    if !access::can_manage_household(tenant) {
        access::require_person_access(tenant, id, PersonAccess::Manage).await?;
    }
    Ok(row)
}

fn eligible(tenant: &TenantTransaction, visible: bool) -> sea_orm::Select<person::Entity> {
    let active_users = user::Entity::find()
        .select_only()
        .column(user::Column::PersonId)
        .filter(user::Column::Active.eq(true))
        .into_query();
    let user_accounts = person::Entity::find()
        .select_only()
        .column(person::Column::AccountId)
        .filter(person::Column::AccountId.is_not_null())
        .filter(person::Column::Id.in_subquery(active_users))
        .into_query();
    let active_accounts = account::Entity::find()
        .select_only()
        .column(account::Column::Id)
        .filter(account::Column::Status.eq(2))
        .filter(account::Column::Id.in_subquery(user_accounts))
        .into_query();
    let mut query = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(person::Column::PersonType.eq(0))
        .filter(person::Column::HasCapacity.eq(true))
        .filter(person::Column::AccountId.in_subquery(active_accounts));
    if visible {
        query = query
            .filter(person::Column::Id.in_subquery(access::granted_people(tenant.membership())));
    }
    query
}

pub async fn page(tenant: &TenantTransaction, id: i64) -> Result<Value, OperationError> {
    let patient = selected(tenant, id).await?;
    let manager = access::can_manage_household(tenant);
    let mut query = carer_relationship::Entity::find()
        .filter(carer_relationship::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(carer_relationship::Column::PatientId.eq(id));
    if !manager {
        query = query.filter(carer_relationship::Column::Active.eq(true));
    }
    let rows = query
        .order_by_asc(carer_relationship::Column::Id)
        .all(tenant.transaction())
        .await?;
    let names: HashMap<_, _> = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(tenant.scope().household_id))
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
    let options = if manager {
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
    target: &url::Url,
    provenance: &CredentialProvenance,
) -> Result<AssignmentOutcome, OperationError> {
    sync::lock(tenant).await?;
    selected(tenant, id).await?;
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
            .filter(person::Column::AccountId.eq(account.id))
            .filter(Expr::cust("NULLIF(btrim(professional_title), '') IS NULL"))
            .one(tenant.transaction())
            .await?
            .ok_or_else(invalid)?;
        let self_relation = carer_relationship::Entity::find()
            .filter(carer_relationship::Column::HouseholdId.eq(tenant.scope().household_id))
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
            .filter(carer_relationship::Column::HouseholdId.eq(tenant.scope().household_id))
            .filter(carer_relationship::Column::CarerId.eq(carer.id))
            .filter(carer_relationship::Column::PatientId.eq(id))
            .one(tenant.transaction())
            .await?;
        let member = membership::Entity::find()
            .filter(membership::Column::HouseholdId.eq(tenant.scope().household_id))
            .filter(membership::Column::AccountId.eq(account.id))
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
        if user::Entity::find()
            .filter(Expr::cust_with_values(
                "LOWER(email_address) = $1",
                [email.clone()],
            ))
            .one(tenant.transaction())
            .await?
            .is_some()
        {
            return Err(invalid());
        }
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
        .filter(carer_relationship::Column::HouseholdId.eq(tenant.scope().household_id))
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

pub async fn can_assign(tenant: &TenantTransaction, id: i64) -> Result<bool, OperationError> {
    match selected(tenant, id).await {
        Ok(_) => Ok(true),
        Err(OperationError::Forbidden | OperationError::NotFound) => Ok(false),
        Err(error) => Err(error),
    }
}
