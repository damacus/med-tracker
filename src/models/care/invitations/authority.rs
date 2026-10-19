use super::*;
use crate::models::{
    access, authorization,
    entities::{account, membership, user},
};
use sea_orm::DatabaseTransaction;

pub(super) async fn authorize(
    db: &DatabaseTransaction,
    inviter: &membership::Model,
    invitation: &household_invitation::Model,
) -> Result<(), OperationError> {
    if !authorization::household_manager(inviter, invitation.household_id) {
        return Err(super::acceptance::unavailable());
    }
    let owner = account::Entity::find()
        .filter(account::Column::PersonId.eq(inviter.person_id))
        .one(db)
        .await?
        .ok_or_else(super::acceptance::unavailable)?;
    access::verify_account(db, owner.id)
        .await
        .map_err(|_| super::acceptance::unavailable())?;
    user::Entity::find()
        .filter(user::Column::PersonId.eq(owner.person_id))
        .filter(user::Column::Active.eq(true))
        .one(db)
        .await?
        .ok_or_else(super::acceptance::unavailable)?;
    Ok(())
}
