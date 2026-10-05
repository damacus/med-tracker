use super::*;
use sea_orm::sea_query::ExprTrait;

pub(super) fn granted_people(
    membership: &membership::Model,
) -> sea_orm::sea_query::SelectStatement {
    grant::Entity::find()
        .select_only()
        .column(grant::Column::PersonId)
        .filter(grant::Column::HouseholdId.eq(membership.household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(membership.id))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(grant::Column::AccessLevel.is_in(["view", "record", "manage"]))
        .filter(
            Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(sea_orm::sea_query::Expr::col(grant::Column::ExpiresAt).gt(
                    sea_orm::sea_query::Expr::cust("timezone('UTC', clock_timestamp())"),
                )),
        )
        .into_query()
}

pub(super) fn scope(
    household_id: i64,
    membership: &membership::Model,
) -> sea_orm::Select<medication::Entity> {
    let query = medication::Entity::find().filter(medication::Column::HouseholdId.eq(household_id));
    if membership.role == "owner" || membership.role == "administrator" {
        return query;
    }
    let granted_schedules = schedule::Entity::find()
        .select_only()
        .column(schedule::Column::MedicationId)
        .filter(schedule::Column::HouseholdId.eq(household_id))
        .filter(schedule::Column::PersonId.in_subquery(granted_people(membership)))
        .into_query();
    let granted_assignments = person_medication::Entity::find()
        .select_only()
        .column(person_medication::Column::MedicationId)
        .filter(person_medication::Column::HouseholdId.eq(household_id))
        .filter(person_medication::Column::PersonId.in_subquery(granted_people(membership)))
        .into_query();
    let linked_schedules = schedule::Entity::find()
        .select_only()
        .column(schedule::Column::MedicationId)
        .filter(schedule::Column::HouseholdId.eq(household_id))
        .into_query();
    let linked_assignments = person_medication::Entity::find()
        .select_only()
        .column(person_medication::Column::MedicationId)
        .filter(person_medication::Column::HouseholdId.eq(household_id))
        .into_query();
    query.filter(
        Condition::any()
            .add(medication::Column::Id.in_subquery(granted_schedules))
            .add(medication::Column::Id.in_subquery(granted_assignments))
            .add(
                Condition::all()
                    .add(medication::Column::CreatedByMembershipId.eq(membership.id))
                    .add(medication::Column::Id.not_in_subquery(linked_schedules))
                    .add(medication::Column::Id.not_in_subquery(linked_assignments)),
            ),
    )
}
