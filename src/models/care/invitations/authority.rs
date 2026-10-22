use super::*;
use crate::models::{
    access, authorization,
    entities::{account, membership},
};

pub(super) async fn authorize(
    db: &impl sea_orm::ConnectionTrait,
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
        .map_err(|error| match error {
            OperationError::Unavailable => OperationError::Unavailable,
            _ => super::acceptance::unavailable(),
        })?;
    Ok(())
}
