use super::*;
use crate::models::entities::{api_tombstone, medication};
use loco_rs::model::TenantQueryExt;
use sea_orm::QueryOrder;

pub async fn can_manage(tenant: &TenantTransaction) -> Result<bool, OperationError> {
    access::recheck(tenant).await?;
    Ok(access::can_manage_household(tenant))
}

pub async fn list(tenant: &TenantTransaction) -> Result<Vec<location::Model>, OperationError> {
    access::recheck(tenant).await?;
    location::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .filter(location::Column::RetiredAt.is_null())
        .order_by_asc(location::Column::Id)
        .all(tenant.transaction())
        .await
        .map_err(|_| OperationError::Unavailable)
}

pub async fn read(tenant: &TenantTransaction, id: &str) -> Result<location::Model, OperationError> {
    access::recheck(tenant).await?;
    let query = location::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .filter(location::Column::RetiredAt.is_null());
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
        .filter(location::Column::RetiredAt.is_null())
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
    attributes: Value,
    etag: &str,
    provenance: Option<&CredentialProvenance>,
) -> Result<location::Model, OperationError> {
    let found = locked(tenant, id, etag).await?;
    let mut merged = json!({"name":found.name,"description":found.description});
    let inner = attributes
        .as_object()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| invalid("location", "is invalid"))?;
    for (key, value) in inner {
        merged[key] = value.clone();
    }
    let (name, description) = validate(&merged)?;
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
    let medicines = medication::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .filter(medication::Column::LocationId.eq(found.id))
        .order_by_asc(medication::Column::Id)
        .lock_exclusive()
        .all(tenant.transaction())
        .await
        .map_err(OperationError::from)?;
    for medicine in medicines {
        crate::models::care::medications::crud::update(
            tenant,
            &medicine.id.to_string(),
            json!({"location_id":null}),
            None,
            provenance,
        )
        .await?;
    }
    let before = snapshot(&found);
    let now = Utc::now().naive_utc();
    let mut retired: location::ActiveModel = found.clone().into();
    retired.retired_at = Set(Some(now));
    retired.updated_at = Set(now);
    retired
        .update(tenant.transaction())
        .await
        .map_err(OperationError::from)?;
    record_change(tenant, &found, "destroy", Some(before), provenance).await?;
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
    .map_err(OperationError::from)?;
    Ok(())
}
