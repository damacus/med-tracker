use super::*;
use sea_orm::{ConnectionTrait, DatabaseTransaction, DbBackend, Statement};

pub(crate) async fn signup_invitation(
    transaction: &DatabaseTransaction,
    token: &str,
) -> Result<household_invitation::Model, OperationError> {
    if token.trim().is_empty() {
        return Err(super::acceptance::unavailable());
    }
    let digest = tokens::digest(token);
    transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT set_config('med_tracker.current_invitation_token_digest',$1,true)",
            [digest.clone().into()],
        ))
        .await?;
    let invitation = household_invitation::Entity::find()
        .filter(household_invitation::Column::TokenDigest.eq(&digest))
        .filter(household_invitation::Column::AcceptedAt.is_null())
        .filter(household_invitation::Column::RevokedAt.is_null())
        .filter(household_invitation::Column::ExpiresAt.gt(Utc::now().naive_utc()))
        .one(transaction)
        .await?
        .ok_or_else(super::acceptance::unavailable)?;
    Ok(invitation)
}
