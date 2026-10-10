use crate::models::{access::TenantTransaction, entities::medication, errors::OperationError};
use sea_orm::{ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, Statement};

pub(crate) async fn record_crossing(
    tenant: &TenantTransaction,
    previous: &medication::Model,
    take_id: i64,
) -> Result<(), OperationError> {
    let Some(before) = previous.current_supply else {
        return Ok(());
    };
    let current = medication::Entity::find_by_id(previous.id)
        .filter(medication::Column::HouseholdId.eq(tenant.scope().household_id))
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    if before <= current.reorder_threshold
        || current
            .current_supply
            .is_none_or(|amount| amount > current.reorder_threshold)
    {
        return Ok(());
    }
    tenant.transaction().execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO notification_events(household_id,person_id,event_type,event_key,metadata,created_at,updated_at) SELECT m.household_id,m.person_id,'browser_low_stock_pending','low-stock:'||$2::text||':'||$3::text||':'||m.person_id::text,jsonb_build_object('account_id',m.account_id,'medication_id',$2::bigint,'take_id',$3::bigint),timezone('UTC',clock_timestamp()),timezone('UTC',clock_timestamp()) FROM household_memberships m JOIN notification_preferences p ON p.household_id=m.household_id AND p.person_id=m.person_id WHERE m.household_id=$1 AND m.status='active' AND m.revoked_at IS NULL AND p.enabled AND p.low_stock_enabled AND (EXISTS(SELECT 1 FROM person_medications pm WHERE pm.household_id=m.household_id AND pm.person_id=m.person_id AND pm.medication_id=$2 AND pm.active) OR EXISTS(SELECT 1 FROM schedules s WHERE s.household_id=m.household_id AND s.person_id=m.person_id AND s.medication_id=$2 AND s.active)) ON CONFLICT DO NOTHING",
        [tenant.scope().household_id.into(),current.id.into(),take_id.into()])).await?;
    Ok(())
}
