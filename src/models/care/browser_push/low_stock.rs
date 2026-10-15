use crate::models::{
    access::TenantTransaction, entities::medication, errors::OperationError, ids::MedicationId,
};
use loco_rs::model::TenantQueryExt;
use sea_orm::prelude::Decimal;
use sea_orm::{ConnectionTrait, DbBackend, EntityTrait, Statement};

pub(crate) async fn record_crossing(
    tenant: &TenantTransaction,
    medication_id: MedicationId,
    previous_supply: Option<Decimal>,
    take_id: i64,
) -> Result<(), OperationError> {
    let Some(before) = previous_supply else {
        return Ok(());
    };
    let current = medication::Entity::find_by_id(medication_id.get())
        .in_tenant(tenant.scope().household_id)
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
    tenant
        .transaction()
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT med_tracker.record_low_stock_notifications($1,$2,$3)",
            [
                current.id.into(),
                take_id.into(),
                tenant.scope().request_id.clone().into(),
            ],
        ))
        .await?;
    Ok(())
}
