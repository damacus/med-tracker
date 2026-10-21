use super::*;
use crate::models::entities::{account, user};

pub(crate) async fn me_read(tenant: &TenantTransaction, zone: Tz) -> Result<Value, OperationError> {
    access::recheck(tenant).await?;
    let account = account::Entity::find_by_id(tenant.scope().actor.account_id)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    let user_id = tenant
        .legacy_user_id()
        .ok_or(OperationError::Unauthenticated)?;
    let user = user::Entity::find_by_id(user_id)
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
    Ok(values(tenant, records, zone).await?.remove(0))
}
