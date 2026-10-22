use super::*;
use crate::models::entities::schedule;
use loco_rs::model::TenantQueryExt;

pub struct Overview {
    pub household_name: String,
    pub active_members: u64,
    pub people: u64,
    pub pending_invitations: u64,
    pub expired_invitations: u64,
    pub active_schedules: u64,
    pub recent_memberships: u64,
    pub recent_audit_events: u64,
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
    let cutoff = Utc::now();
    let now = cutoff.naive_utc();
    let today = cutoff
        .with_timezone(&crate::models::care::doses::app_zone(tenant.settings()))
        .date_naive();
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
        .filter(household_invitation::Column::ExpiresAt.gt(now))
        .count(tenant.transaction())
        .await?;
    let expired_invitations = household_invitation::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .filter(household_invitation::Column::AcceptedAt.is_null())
        .filter(household_invitation::Column::RevokedAt.is_null())
        .filter(household_invitation::Column::ExpiresAt.lte(now))
        .count(tenant.transaction())
        .await?;
    let active_schedules = access::schedule_scope(tenant)
        .filter(schedule::Column::Active.eq(true))
        .filter(schedule::Column::StartDate.lte(today))
        .filter(schedule::Column::EndDate.gte(today))
        .count(tenant.transaction())
        .await?;
    let recent_memberships = membership::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .filter(membership::Column::Status.eq("active"))
        .filter(membership::Column::RevokedAt.is_null())
        .filter(membership::Column::JoinedAt.gte(now - chrono::Duration::days(7)))
        .filter(membership::Column::JoinedAt.lte(now))
        .count(tenant.transaction())
        .await?;
    let recent_audit_events = security_audit_event::Entity::find()
        .filter(security_audit_event::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(security_audit_event::Column::CreatedAt.gte(now - chrono::Duration::hours(24)))
        .filter(security_audit_event::Column::CreatedAt.lte(now))
        .count(tenant.transaction())
        .await?;
    Ok(Overview {
        household_name: name,
        active_members,
        people,
        pending_invitations,
        expired_invitations,
        active_schedules,
        recent_memberships,
        recent_audit_events,
    })
}
