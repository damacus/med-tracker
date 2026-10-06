use super::*;

pub async fn update(
    tenant: &TenantTransaction,
    id: &str,
    attributes: Value,
    zone: Tz,
    provenance: Option<&CredentialProvenance>,
) -> Result<person::Model, OperationError> {
    let found = authorize_update(tenant, id).await?;
    let record = person::Entity::find_by_id(found.id)
        .filter(person::Column::HouseholdId.eq(tenant.scope().household_id))
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    let mut proposed = record.clone();
    validation::assign(&mut proposed, &attributes, false)?;
    validation::capacity(&mut proposed, zone)?;
    if !proposed.has_capacity
        && carer_relationship::Entity::find()
            .filter(carer_relationship::Column::HouseholdId.eq(tenant.scope().household_id))
            .filter(carer_relationship::Column::PatientId.eq(record.id))
            .filter(carer_relationship::Column::Active.eq(true))
            .one(tenant.transaction())
            .await?
            .is_none()
    {
        return Err(invalid("has_capacity", "requires an active carer"));
    }
    if proposed == record {
        return Ok(record);
    }
    let before = persistence::snapshot(&record);
    let mut active: person::ActiveModel = record.into();
    active.name = Set(proposed.name);
    active.email = Set(proposed.email);
    active.date_of_birth = Set(proposed.date_of_birth);
    active.person_type = Set(proposed.person_type);
    active.has_capacity = Set(proposed.has_capacity);
    active.updated_at = Set(Utc::now().naive_utc());
    let saved = active
        .update(tenant.transaction())
        .await
        .map_err(write_error)?;
    persistence::record_version(
        tenant,
        "Person",
        saved.id,
        "update",
        Some(before),
        persistence::snapshot(&saved),
        provenance,
    )
    .await?;
    persistence::change(
        tenant,
        "Person",
        saved.id,
        &saved.portable_id,
        "update",
        provenance,
    )
    .await?;
    Ok(saved)
}
