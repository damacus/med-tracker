use super::*;

pub async fn unassign(
    tenant: &TenantTransaction,
    id: &str,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    household::Entity::find_by_id(tenant.scope().household_id)
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    access::recheck(tenant).await?;
    let row = reading::find(tenant, id, true).await?;
    access::require_person_access(tenant, row.person_id, PersonAccess::Manage).await?;
    let before = writing::clinical_snapshot(&row);
    let now = Utc::now().naive_utc();
    let mut active = row.into_active_model();
    active.active = Set(false);
    active.retired_at = Set(Some(now));
    active.updated_at = Set(now);
    let row = active.update(tenant.transaction()).await?;
    administration::persistence::record_version_as(
        tenant,
        "PersonMedication",
        row.id,
        "update",
        Some(before),
        writing::clinical_snapshot(&row),
        provenance,
    )
    .await?;
    let person = person::Entity::find_by_id(row.person_id)
        .filter(person::Column::HouseholdId.eq(tenant.scope().household_id))
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    api_change_event::ActiveModel{household_id:Set(tenant.scope().household_id),household_membership_id:Set(Some(tenant.membership().id)),account_id:Set(Some(tenant.scope().actor.account_id)),action:Set("update".into()),record_type:Set("PersonMedication".into()),record_id:Set(row.id),record_portable_id:Set(Some(row.portable_id.clone())),request_id:Set(Some(tenant.scope().request_id.clone())),metadata:Set(json!({"record_type":"PersonMedication","record_id":row.id,"portable_id":row.portable_id,"person_portable_id":person.portable_id})),occurred_at:Set(now),created_at:Set(now),updated_at:Set(now),..Default::default()}.insert(tenant.transaction()).await?;
    Ok(())
}
