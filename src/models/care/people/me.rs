use super::*;
use crate::models::entities::account;
use crate::views::people::{Me, MeAccount, MeAccountStatus, MeResponse};

pub(crate) async fn me_read(
    tenant: &TenantTransaction,
    zone: Tz,
) -> Result<MeResponse, OperationError> {
    access::recheck(tenant).await?;
    let account = account::Entity::find_by_id(tenant.scope().actor.account_id)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    let record = person::Entity::find_by_id(account.person_id)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    let person = values(tenant, std::slice::from_ref(&record), zone)
        .await?
        .remove(0);
    let status = match account.status {
        1 => MeAccountStatus::Unverified,
        2 => MeAccountStatus::Verified,
        3 => MeAccountStatus::Closed,
        _ => return Err(OperationError::Unavailable),
    };
    Ok(MeResponse {
        data: Me {
            id: account.id,
            email_address: account.email.clone(),
            membership_role: tenant.membership().role.clone(),
            active: account.active,
            person,
            account: MeAccount {
                id: account.id,
                email: account.email,
                status,
            },
        },
    })
}
