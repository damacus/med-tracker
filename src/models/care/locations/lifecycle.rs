use super::*;
use crate::models::entities::{api_tombstone, location_membership, medication, medication_take};
use loco_rs::model::TenantQueryExt;
use sea_orm::QueryOrder;

pub(super) async fn record_membership_destroy(
    tenant: &TenantTransaction,
    record: &location_membership::Model,
    provenance: Option<&CredentialProvenance>,
    policy_class: &str,
) -> Result<(), OperationError> {
    let before = json!({"id":record.id,"household_id":record.household_id,"location_id":record.location_id,"person_id":record.person_id,"created_at":record.created_at,"updated_at":record.updated_at});
    let changes: serde_json::Map<String, Value> = before
        .as_object()
        .ok_or(OperationError::Unavailable)?
        .iter()
        .map(|(key, value)| (key.clone(), json!([value, null])))
        .collect();
    let mut context = crate::models::audit_context::household(tenant, provenance);
    context["policy_class"] = json!(policy_class);
    context["policy_query"] = json!("destroy?");
    version::ActiveModel {
        item_type: Set("LocationMembership".into()),
        item_id: Set(record.id),
        event: Set("destroy".into()),
        object: Set(Some(before.to_string())),
        object_changes: Set(Some(Value::Object(changes).to_string())),
        whodunnit: Set(Some(tenant.scope().actor.account_id.to_string())),
        request_id: Set(Some(tenant.scope().request_id.clone())),
        household_id: Set(Some(tenant.scope().household_id)),
        actor_membership_id: Set(Some(tenant.membership().id)),
        audit_context: Set(context),
        created_at: Set(Some(Utc::now().naive_utc())),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await
    .map_err(|_| OperationError::Unavailable)?;
    Ok(())
}

pub async fn can_manage(tenant: &TenantTransaction) -> Result<bool, OperationError> {
    access::recheck(tenant).await?;
    Ok(access::can_manage_household(tenant))
}

pub async fn list(tenant: &TenantTransaction) -> Result<Vec<location::Model>, OperationError> {
    access::recheck(tenant).await?;
    location::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .order_by_asc(location::Column::Id)
        .all(tenant.transaction())
        .await
        .map_err(|_| OperationError::Unavailable)
}

pub async fn read(tenant: &TenantTransaction, id: &str) -> Result<location::Model, OperationError> {
    access::recheck(tenant).await?;
    let query = location::Entity::find().in_tenant(tenant.scope().household_id);
    let query = if let Ok(id) = id.parse::<i64>() {
        query.filter(location::Column::Id.eq(id))
    } else {
        query.filter(location::Column::PortableId.eq(id))
    };
    query
        .one(tenant.transaction())
        .await
        .map_err(|_| OperationError::Unavailable)?
        .ok_or(OperationError::NotFound)
}

async fn locked(
    tenant: &TenantTransaction,
    id: &str,
    etag: &str,
) -> Result<location::Model, OperationError> {
    authorize(tenant).await?;
    let found = read(tenant, id).await?;
    let found = location::Entity::find_by_id(found.id)
        .in_tenant(tenant.scope().household_id)
        .lock_exclusive()
        .one(tenant.transaction())
        .await
        .map_err(|_| OperationError::Unavailable)?
        .ok_or(OperationError::NotFound)?;
    if representation(&found).1 != etag {
        return Err(OperationError::Conflict {
            code: "conflict".into(),
            details: json!({"message":"Record has changed since it was last read"}),
        });
    }
    Ok(found)
}

pub async fn update(
    tenant: &TenantTransaction,
    id: &str,
    input: LocationInput,
    etag: &str,
    provenance: Option<&CredentialProvenance>,
) -> Result<location::Model, OperationError> {
    let found = locked(tenant, id, etag).await?;
    let (name, description) = input.values(Some(&found))?;
    let before = snapshot(&found);
    let mut active: location::ActiveModel = found.into();
    active.name = Set(name);
    active.description = Set(description);
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active
        .update(tenant.transaction())
        .await
        .map_err(write_error)?;
    record_change(tenant, &updated, "update", Some(before), provenance).await?;
    Ok(updated)
}

pub async fn retire(
    tenant: &TenantTransaction,
    id: &str,
    etag: &str,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    let found = locked(tenant, id, etag).await?;
    if medication::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .filter(medication::Column::LocationId.eq(found.id))
        .one(tenant.transaction())
        .await
        .map_err(OperationError::from)?
        .is_some()
    {
        return Err(invalid(
            "location",
            "cannot be deleted while medications are stored here; move them to another location first",
        ));
    }
    if medication_take::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .filter(medication_take::Column::TakenFromLocationId.eq(found.id))
        .one(tenant.transaction())
        .await
        .map_err(OperationError::from)?
        .is_some()
    {
        return Err(invalid(
            "location",
            "cannot be deleted while administration history references it",
        ));
    }
    let memberships = location_membership::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .filter(location_membership::Column::LocationId.eq(found.id))
        .lock_exclusive()
        .all(tenant.transaction())
        .await
        .map_err(|_| OperationError::Unavailable)?;
    for membership in memberships {
        record_membership_destroy(tenant, &membership, provenance, "LocationPolicy").await?;
        location_membership::Entity::delete_by_id(membership.id)
            .exec(tenant.transaction())
            .await
            .map_err(|_| OperationError::Unavailable)?;
    }
    location::Entity::delete_by_id(found.id)
        .exec(tenant.transaction())
        .await
        .map_err(|_| OperationError::Unavailable)?;
    record_change(
        tenant,
        &found,
        "destroy",
        Some(snapshot(&found)),
        provenance,
    )
    .await?;
    let now = Utc::now().naive_utc();
    api_tombstone::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        household_membership_id: Set(Some(tenant.membership().id)),
        account_id: Set(Some(tenant.scope().actor.account_id)),
        action: Set("delete".into()),
        record_type: Set("Location".into()),
        record_portable_id: Set(found.portable_id.clone()),
        metadata: Set(
            json!({"record_type":"Location","record_id":found.id,"portable_id":found.portable_id}),
        ),
        deleted_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await
    .map_err(|_| OperationError::Unavailable)?;
    Ok(())
}
