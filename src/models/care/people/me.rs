use super::*;
use crate::models::entities::{account, user};
use sea_orm::{ConnectionTrait, DbBackend, Statement};

pub(crate) async fn me_read(tenant: &TenantTransaction, zone: Tz) -> Result<Value, OperationError> {
    access::recheck(tenant).await?;
    let account = account::Entity::find_by_id(tenant.scope().actor.account_id)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    let user = user::Entity::find_by_id(tenant.user_id())
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    let record = person::Entity::find_by_id(user.person_id)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    let own = own_person(tenant, &record, zone).await?;
    let status = match account.status {
        1 => "unverified",
        2 => "verified",
        3 => "closed",
        _ => return Err(OperationError::Unavailable),
    };
    Ok(json!({"data": {
        "id": user.id,
        "email_address": user.email_address,
        "membership_role": tenant.membership().role,
        "active": user.active,
        "person": own,
        "account": {"id": account.id, "email": account.email, "status": status},
    }}))
}

async fn own_person(
    tenant: &TenantTransaction,
    record: &person::Model,
    zone: Tz,
) -> Result<Value, OperationError> {
    let records = std::slice::from_ref(record);
    if record.household_id == tenant.scope().household_id {
        return Ok(values(tenant, records, zone).await?.remove(0));
    }
    set_household(tenant, record.household_id).await?;
    let result = super::projection::values_in(tenant, record.household_id, records, zone).await;
    set_household(tenant, tenant.scope().household_id).await?;
    Ok(result?.remove(0))
}

async fn set_household(
    tenant: &TenantTransaction,
    household_id: i64,
) -> Result<(), OperationError> {
    tenant
        .transaction()
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT set_config('med_tracker.current_household_id', $1, true)",
            [household_id.to_string().into()],
        ))
        .await?;
    Ok(())
}
