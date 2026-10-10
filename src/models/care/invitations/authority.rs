use super::*;
use crate::models::{
    access, authorization,
    entities::{grant, membership, person},
};
use sea_orm::sea_query::{Expr, ExprTrait};
use sea_orm::{DatabaseTransaction, QuerySelect};
use std::collections::HashSet;

pub(super) async fn authorize(
    db: &DatabaseTransaction,
    inviter: &membership::Model,
    invitation: &household_invitation::Model,
) -> Result<(), OperationError> {
    if authorization::household_manager(inviter, invitation.household_id) {
        return Ok(());
    }
    if inviter.household_id != invitation.household_id
        || inviter.status != "active"
        || inviter.revoked_at.is_some()
        || invitation.membership_role != "member"
    {
        return Err(super::acceptance::unavailable());
    }
    access::verify_account_actor(db, inviter.account_id)
        .await
        .map_err(|_| super::acceptance::unavailable())?;
    let additional = household_invitation_grant::Entity::find()
        .filter(household_invitation_grant::Column::HouseholdId.eq(invitation.household_id))
        .filter(household_invitation_grant::Column::HouseholdInvitationId.eq(invitation.id))
        .lock_exclusive()
        .all(db)
        .await?;
    if additional.is_empty()
        || additional.iter().any(|row| {
            row.relationship_type != "parent"
                || row.access_level != "manage"
                || row.expires_at.is_some()
        })
    {
        return Err(super::acceptance::unavailable());
    }
    let ids = additional
        .iter()
        .map(|row| row.person_id)
        .collect::<HashSet<_>>();
    let mut manageable = access::granted_people(inviter);
    manageable.and_where(Expr::col(grant::Column::AccessLevel).eq("manage"));
    let people = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(invitation.household_id))
        .filter(person::Column::Id.is_in(ids.iter().copied()))
        .filter(person::Column::Id.in_subquery(manageable))
        .filter(person::Column::PersonType.is_in([1, 2]))
        .filter(person::Column::HasCapacity.eq(false))
        .all(db)
        .await?;
    if people.len() != ids.len()
        || people
            .iter()
            .any(|row| !authorization::may_delegate(inviter, row))
    {
        return Err(super::acceptance::unavailable());
    }
    Ok(())
}
