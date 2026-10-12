use super::*;

pub async fn index(tenant: &TenantTransaction) -> Result<Vec<Value>, OperationError> {
    access::recheck(tenant).await?;
    if !access::can_manage_household(tenant) {
        return Err(OperationError::Forbidden);
    }
    let events = security_audit_event::Entity::find()
        .filter(security_audit_event::Column::HouseholdId.eq(tenant.scope().household_id))
        .order_by_desc(security_audit_event::Column::CreatedAt)
        .order_by_desc(security_audit_event::Column::Id)
        .limit(100)
        .all(tenant.transaction())
        .await
        .map_err(|_| OperationError::Unavailable)?;
    Ok(events
        .iter()
        .map(|event| {
            json!({
                "id": event.id,
                "event_type": event.event_type,
                "actor_account_id": event.actor_account_id,
                "actor_membership_id": event.actor_membership_id,
                "request_id": event.request_id,
                "metadata": event.metadata,
                "created_at": event.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string()
            })
        })
        .collect())
}
