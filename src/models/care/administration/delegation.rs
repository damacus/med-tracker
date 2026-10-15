use super::*;
use loco_rs::model::TenantQueryExt;

fn snapshot(row: &carer_relationship::Model) -> Value {
    json!({"id":row.id,"household_id":row.household_id,"carer_id":row.carer_id,"patient_id":row.patient_id,"relationship_type":row.relationship_type,"active":row.active,"created_at":row.created_at,"updated_at":row.updated_at})
}
pub async fn list(tenant: &TenantTransaction) -> Result<Value, OperationError> {
    authorize(tenant).await?;
    let people: HashMap<_, _> = crate::models::person_identity::current_summaries(tenant)
        .await?
        .into_iter()
        .map(|row| (row.id.get(), row))
        .collect();
    let rows = carer_relationship::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .order_by_asc(carer_relationship::Column::Id)
        .all(tenant.transaction())
        .await?;
    let data: Vec<Value> = rows
        .iter()
        .map(|row| {
            let mut value = snapshot(row);
            value["carer_name"] = people
                .get(&row.carer_id)
                .map_or(Value::Null, |row| json!(row.name));
            value["patient_name"] = people
                .get(&row.patient_id)
                .map_or(Value::Null, |row| json!(row.name));
            value
        })
        .collect();
    Ok(json!({"data":data}))
}
pub async fn options(tenant: &TenantTransaction) -> Result<Value, OperationError> {
    access::recheck(tenant).await?;
    authorize(tenant).await?;
    let records = access::current_people(tenant).await?;
    let capable: std::collections::HashSet<_> = records
        .iter()
        .filter(|row| row.has_capacity)
        .map(|row| row.id)
        .collect();
    let rows = crate::models::person_identity::summaries(tenant, records).await?;
    let patients: Vec<Value> = rows
        .iter()
        .map(|row| json!({"id":row.id,"name":row.name}))
        .collect();
    let carers: Vec<Value> = rows
        .iter()
        .filter(|row| capable.contains(&row.id.get()))
        .map(|row| json!({"id":row.id,"name":row.name}))
        .collect();
    Ok(json!({"carers":carers,"patients":patients}))
}
pub async fn assign(
    tenant: &TenantTransaction,
    carer_id: i64,
    patient_id: i64,
    relationship_type: &str,
    provenance: Option<&CredentialProvenance>,
) -> Result<carer_relationship::Model, OperationError> {
    access::lock_household(tenant).await?;
    access::recheck(tenant).await?;
    let carer = crate::models::access::person_scope(
        crate::models::ids::HouseholdId::from_database(tenant.scope().household_id),
    )
    .filter(person::Column::Id.eq(carer_id))
    .one(tenant.transaction())
    .await?
    .ok_or_else(|| invalid("carer", "must belong to the relationship household"))?;
    crate::models::access::person_scope(crate::models::ids::HouseholdId::from_database(
        tenant.scope().household_id,
    ))
    .filter(person::Column::Id.eq(patient_id))
    .one(tenant.transaction())
    .await?
    .ok_or_else(|| invalid("patient", "must belong to the relationship household"))?;
    authorize(tenant).await?;
    if !carer.has_capacity {
        return Err(invalid("carer", "must have capacity"));
    }
    if !matches!(
        relationship_type,
        "parent" | "family_member" | "professional_carer" | "self"
    ) {
        return Err(invalid("relationship_type", "is invalid"));
    }
    let existing = carer_relationship::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .filter(carer_relationship::Column::CarerId.eq(carer_id))
        .filter(carer_relationship::Column::PatientId.eq(patient_id))
        .lock_exclusive()
        .one(tenant.transaction())
        .await?;
    let now = Utc::now().naive_utc();
    let before = existing.as_ref().map(snapshot);
    let row = if let Some(row) = existing {
        if row.active && row.relationship_type.as_deref() == Some(relationship_type) {
            row
        } else {
            let mut active: carer_relationship::ActiveModel = row.into();
            active.relationship_type = Set(Some(relationship_type.into()));
            active.active = Set(true);
            active.updated_at = Set(now);
            active.update(tenant.transaction()).await?
        }
    } else {
        carer_relationship::ActiveModel {
            household_id: Set(tenant.scope().household_id),
            carer_id: Set(carer_id),
            patient_id: Set(patient_id),
            relationship_type: Set(Some(relationship_type.into())),
            active: Set(true),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(tenant.transaction())
        .await?
    };
    if before.as_ref().is_none_or(|value| {
        value["active"] != true || value["relationship_type"] != relationship_type
    }) {
        persistence::record_version(
            tenant,
            "CarerRelationship",
            row.id,
            before,
            snapshot(&row),
            provenance,
        )
        .await?;
    }
    Ok(row)
}
pub async fn deactivate(
    tenant: &TenantTransaction,
    id: i64,
    provenance: Option<&CredentialProvenance>,
) -> Result<carer_relationship::Model, OperationError> {
    authorize(tenant).await?;
    let row = carer_relationship::Entity::find_by_id(id)
        .in_tenant(tenant.scope().household_id)
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    if !row.active {
        return Ok(row);
    }
    let before = snapshot(&row);
    let mut active: carer_relationship::ActiveModel = row.into();
    active.active = Set(false);
    active.updated_at = Set(Utc::now().naive_utc());
    let row = active.update(tenant.transaction()).await?;
    persistence::record_version(
        tenant,
        "CarerRelationship",
        row.id,
        Some(before),
        snapshot(&row),
        provenance,
    )
    .await?;
    Ok(row)
}
pub async fn reactivate(
    tenant: &TenantTransaction,
    id: i64,
    provenance: Option<&CredentialProvenance>,
) -> Result<carer_relationship::Model, OperationError> {
    authorize(tenant).await?;
    let row = carer_relationship::Entity::find_by_id(id)
        .in_tenant(tenant.scope().household_id)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    assign(
        tenant,
        row.carer_id,
        row.patient_id,
        row.relationship_type
            .as_deref()
            .ok_or(OperationError::Unavailable)?,
        provenance,
    )
    .await
}
