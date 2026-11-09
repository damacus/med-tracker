use crate::models::{errors::OperationError, ids::AccountId, platform};
use sea_orm::DatabaseConnection;
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct AccountAccess {
    platform_admin: bool,
    support_access: bool,
}

pub(crate) async fn load(
    db: &DatabaseConnection,
    account_id: AccountId,
) -> Result<AccountAccess, OperationError> {
    let transaction = platform::begin(db).await?;
    let platform_admin = match platform::administrator(&transaction, account_id.get()).await {
        Ok(_) => true,
        Err(OperationError::Forbidden | OperationError::Unauthenticated) => false,
        Err(error) => return Err(error),
    };
    let support_access = platform::support::support_link(&transaction, account_id.get()).await?;
    transaction.commit().await?;
    Ok(AccountAccess {
        platform_admin,
        support_access,
    })
}
