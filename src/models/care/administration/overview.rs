use super::*;
use loco_rs::model::TenantQueryExt;

pub struct Overview {
    pub household_name: String,
    pub active_members: u64,
    pub people: u64,
    pub pending_invitations: u64,
}

pub async fn household_name(tenant: &TenantTransaction) -> Result<String, OperationError> {
    authorize(tenant).await?;
    let household = household::Entity::find_by_id(tenant.scope().household_id)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    Ok(household.name)
}

pub async fn read(tenant: &TenantTransaction) -> Result<Overview, OperationError> {
    let name = household_name(tenant).await?;
    let active_members = membership::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .filter(membership::Column::Status.eq("active"))
        .filter(membership::Column::RevokedAt.is_null())
        .count(tenant.transaction())
        .await?;
    let people = access::person_queries::household_people(tenant.scope())
        .count(tenant.transaction())
        .await?;
    let pending_invitations = household_invitation::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .filter(household_invitation::Column::AcceptedAt.is_null())
        .filter(household_invitation::Column::RevokedAt.is_null())
        .filter(household_invitation::Column::ExpiresAt.gt(Utc::now().naive_utc()))
        .count(tenant.transaction())
        .await?;
    Ok(Overview {
        household_name: name,
        active_members,
        people,
        pending_invitations,
    })
}
