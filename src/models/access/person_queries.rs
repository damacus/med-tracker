use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Select};

use super::{Actor, HouseholdScope};
use crate::models::entities::person;

pub(crate) fn household_people(scope: &HouseholdScope) -> Select<person::Entity> {
    person::Entity::find().filter(person::Column::HouseholdId.eq(scope.household_id))
}

pub(crate) fn account_person(actor: Actor) -> Select<person::Entity> {
    person::Entity::find()
        .filter(person::Column::AccountId.eq(actor.account_id))
        .order_by_asc(person::Column::Id)
}
