use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, QueryTrait, Select};

use super::{Actor, HouseholdScope};
use crate::models::entities::{account, membership, person};

pub(crate) fn household_people(scope: &HouseholdScope) -> Select<person::Entity> {
    household_participants(scope.household_id)
}

pub(crate) fn household_participants(household_id: i64) -> Select<person::Entity> {
    person::Entity::find().filter(
        person::Column::Id.in_subquery(
            membership::Entity::find()
                .select_only()
                .column(membership::Column::PersonId)
                .filter(membership::Column::HouseholdId.eq(household_id))
                .filter(membership::Column::Status.eq("active"))
                .filter(membership::Column::RevokedAt.is_null())
                .into_query(),
        ),
    )
}

pub(crate) fn account_person(actor: Actor) -> Select<person::Entity> {
    person::Entity::find()
        .filter(
            person::Column::Id.in_subquery(
                account::Entity::find()
                    .select_only()
                    .column(account::Column::PersonId)
                    .filter(account::Column::Id.eq(actor.account_id))
                    .into_query(),
            ),
        )
        .order_by_asc(person::Column::Id)
}
