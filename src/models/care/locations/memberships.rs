use super::*;
use crate::models::{
    access::PersonAccess,
    care::{administration, doses},
    entities::{location_membership, person},
};

fn invalid_membership() -> OperationError {
    OperationError::Validation {
        details: json!({"errors":{"location_membership":["is invalid"]}}),
    }
}

fn snapshot(record: &location_membership::Model) -> Value {
    json!({"id":record.id,"household_id":record.household_id,"location_id":record.location_id,"person_id":record.person_id,"created_at":record.created_at,"updated_at":record.updated_at})
}

pub fn representation(
    record: &location_membership::Model,
    location: &location::Model,
    person: &person::Model,
) -> Value {
    json!({"data":{"id":record.id.to_string(),"location_id":location.id.to_string(),"location_portable_id":location.portable_id,"person_id":person.id.to_string(),"person_portable_id":person.portable_id,"created_at":record.created_at.and_utc().to_rfc3339()}})
}

async fn person_for(
    tenant: &TenantTransaction,
    person_id: &str,
) -> Result<person::Model, OperationError> {
    let query =
        person::Entity::find().filter(person::Column::HouseholdId.eq(tenant.scope().household_id));
    let query = if let Ok(id) = person_id.parse::<i64>() {
        query.filter(person::Column::Id.eq(id))
    } else {
        query.filter(person::Column::PortableId.eq(person_id))
    };
    query
        .one(tenant.transaction())
        .await
        .map_err(|_| OperationError::Unavailable)?
        .ok_or(OperationError::NotFound)
}

async fn require_manage(tenant: &TenantTransaction, person_id: i64) -> Result<(), OperationError> {
    if !access::can_access_person(tenant, person_id, PersonAccess::View).await? {
        return Err(OperationError::NotFound);
    }
    if !access::can_access_person(tenant, person_id, PersonAccess::Manage).await? {
        return Err(OperationError::Forbidden);
    }
    Ok(())
}

async fn record_person_change(
    tenant: &TenantTransaction,
    person: &person::Model,
) -> Result<(), OperationError> {
    let mut active: person::ActiveModel = person.clone().into();
    active.updated_at = Set(Utc::now().naive_utc());
    active
        .update(tenant.transaction())
        .await
        .map_err(|_| OperationError::Unavailable)?;
    let now = Utc::now().naive_utc();
    api_change_event::ActiveModel{household_id:Set(tenant.scope().household_id),household_membership_id:Set(Some(tenant.membership().id)),account_id:Set(Some(tenant.scope().actor.account_id)),action:Set("update".into()),record_type:Set("Person".into()),record_id:Set(person.id),record_portable_id:Set(Some(person.portable_id.clone())),request_id:Set(Some(tenant.scope().request_id.clone())),metadata:Set(json!({"record_type":"Person","record_id":person.id,"portable_id":person.portable_id,"person_portable_id":person.portable_id})),occurred_at:Set(now),created_at:Set(now),updated_at:Set(now),..Default::default()}.insert(tenant.transaction()).await.map_err(|_|OperationError::Unavailable)?;
    Ok(())
}

pub async fn create_membership(
    tenant: &TenantTransaction,
    location: &location::Model,
    attributes: &Value,
    provenance: Option<&CredentialProvenance>,
) -> Result<(location_membership::Model, person::Model), OperationError> {
    let outer = attributes
        .as_object()
        .filter(|outer| outer.len() == 1)
        .ok_or_else(invalid_membership)?;
    let inner = outer
        .get("location_membership")
        .and_then(Value::as_object)
        .filter(|inner| inner.len() == 1)
        .ok_or_else(invalid_membership)?;
    let person_id = inner
        .get("person_id")
        .and_then(Value::as_str)
        .filter(|id| doses::valid_identifier(id))
        .ok_or_else(invalid_membership)?;
    let person = person_for(tenant, person_id).await?;
    require_manage(tenant, person.id).await?;
    if let Some(existing) = location_membership::Entity::find()
        .filter(location_membership::Column::LocationId.eq(location.id))
        .filter(location_membership::Column::PersonId.eq(person.id))
        .one(tenant.transaction())
        .await
        .map_err(|_| OperationError::Unavailable)?
    {
        return Ok((existing, person));
    }
    let now = Utc::now().naive_utc();
    let record = location_membership::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        location_id: Set(location.id),
        person_id: Set(person.id),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await
    .map_err(|_| OperationError::Unavailable)?;
    administration::persistence::record_version_as(
        tenant,
        "LocationMembership",
        record.id,
        "create",
        None,
        snapshot(&record),
        provenance,
    )
    .await?;
    record_person_change(tenant, &person).await?;
    Ok((record, person))
}

pub async fn destroy_membership(
    tenant: &TenantTransaction,
    location: &location::Model,
    id: i64,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    let membership = location_membership::Entity::find_by_id(id)
        .filter(location_membership::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(location_membership::Column::LocationId.eq(location.id))
        .lock_exclusive()
        .one(tenant.transaction())
        .await
        .map_err(|_| OperationError::Unavailable)?
        .ok_or(OperationError::NotFound)?;
    require_manage(tenant, membership.person_id).await?;
    let person = person::Entity::find_by_id(membership.person_id)
        .one(tenant.transaction())
        .await
        .map_err(|_| OperationError::Unavailable)?
        .ok_or(OperationError::NotFound)?;
    lifecycle::record_membership_destroy(
        tenant,
        &membership,
        provenance,
        "LocationMembershipPolicy",
    )
    .await?;
    location_membership::Entity::delete_by_id(membership.id)
        .exec(tenant.transaction())
        .await
        .map_err(|_| OperationError::Unavailable)?;
    record_person_change(tenant, &person).await?;
    Ok(())
}
