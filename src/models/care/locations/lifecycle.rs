use super::*;
use crate::models::entities::{api_tombstone, location_membership, medication};
use sea_orm::QueryOrder;

async fn record_membership_destroy(
    tenant: &TenantTransaction,
    record: &location_membership::Model,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    let before = json!({"id":record.id,"household_id":record.household_id,"location_id":record.location_id,"person_id":record.person_id,"created_at":record.created_at,"updated_at":record.updated_at});
    let changes: serde_json::Map<String, Value> = before
        .as_object()
        .ok_or(OperationError::Unavailable)?
        .iter()
        .map(|(key, value)| (key.clone(), json!([value, null])))
        .collect();
    let mut context = json!({"actor_account_id":tenant.scope().actor.account_id,"actor_user_id":tenant.user_id(),"actor_membership_id":tenant.membership().id,"household_id":tenant.scope().household_id,"request_id":tenant.scope().request_id,"active_role":tenant.membership().role,"permissions_version":tenant.membership().permissions_version,"policy_class":"LocationPolicy","policy_query":"destroy?"});
    if let Some(provenance) = provenance {
        let (method, prefix) = match provenance.method {
            CredentialMethod::ApiSession => ("api_session", "api_session"),
            CredentialMethod::ApiAppToken => ("api_app_token", "api_app_token"),
            CredentialMethod::OauthGrant => ("oauth", "oauth_grant"),
            CredentialMethod::PersonalApiKey => ("personal_api_key", "personal_api_key"),
            CredentialMethod::BrowserSession => ("browser_session", "browser_session"),
        };
        context["authentication_method"] = json!(method);
        context["session_reference"] = json!(format!("{prefix}:{}", provenance.reference));
    }
    version::ActiveModel {
        item_type: Set("LocationMembership".into()),
        item_id: Set(record.id),
        event: Set("destroy".into()),
        object: Set(Some(before.to_string())),
        object_changes: Set(Some(Value::Object(changes).to_string())),
        whodunnit: Set(Some(tenant.user_id().to_string())),
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
        .filter(location::Column::HouseholdId.eq(tenant.scope().household_id))
        .order_by_asc(location::Column::Id)
        .all(tenant.transaction())
        .await
        .map_err(|_| OperationError::Unavailable)
}

pub async fn read(tenant: &TenantTransaction, id: &str) -> Result<location::Model, OperationError> {
    access::recheck(tenant).await?;
    let query = location::Entity::find()
        .filter(location::Column::HouseholdId.eq(tenant.scope().household_id));
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
        .filter(location::Column::HouseholdId.eq(tenant.scope().household_id))
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
        .filter(medication::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(medication::Column::LocationId.eq(found.id))
        .order_by_asc(medication::Column::Id)
        .lock_exclusive()
        .all(tenant.transaction())
        .await
        .map_err(|_| OperationError::Unavailable)?;
    for medicine in medicines {
        crate::models::care::medications::crud::retire_cascade(
            tenant,
            &medicine.id.to_string(),
            provenance,
            "destroy",
        )
        .await?;
    }
    let memberships = location_membership::Entity::find()
        .filter(location_membership::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(location_membership::Column::LocationId.eq(found.id))
        .lock_exclusive()
        .all(tenant.transaction())
        .await
        .map_err(|_| OperationError::Unavailable)?;
    for membership in memberships {
        record_membership_destroy(tenant, &membership, provenance).await?;
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
