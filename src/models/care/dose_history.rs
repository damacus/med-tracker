use crate::models::{
    access::{self, TenantTransaction},
    entities::{medication_take, person_medication, schedule},
    errors::OperationError,
};
use chrono::NaiveDateTime;
use sea_orm::{
    ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
    QueryTrait,
};

pub struct History {
    pub records: Vec<medication_take::Model>,
    pub total_count: u64,
}

pub async fn list(
    tenant: &TenantTransaction,
    page: i64,
    per_page: i64,
    updated_since: Option<NaiveDateTime>,
) -> Result<History, OperationError> {
    access::recheck(tenant).await?;
    if page < 1 || !(1..=100).contains(&per_page) {
        return Err(OperationError::Validation {
            details: serde_json::json!({"error":"Invalid pagination"}),
        });
    }
    let household_id = tenant.scope().household_id;
    let mut query = medication_take::Entity::find()
        .filter(medication_take::Column::HouseholdId.eq(household_id));
    if !access::can_manage_household(tenant) {
        let people = access::granted_people(tenant.membership());
        let schedules = schedule::Entity::find()
            .select_only()
            .column(schedule::Column::Id)
            .filter(schedule::Column::HouseholdId.eq(household_id))
            .filter(schedule::Column::PersonId.in_subquery(people.clone()))
            .into_query();
        let assignments = person_medication::Entity::find()
            .select_only()
            .column(person_medication::Column::Id)
            .filter(person_medication::Column::HouseholdId.eq(household_id))
            .filter(person_medication::Column::PersonId.in_subquery(people))
            .into_query();
        query = query.filter(
            Condition::any()
                .add(medication_take::Column::ScheduleId.in_subquery(schedules))
                .add(medication_take::Column::PersonMedicationId.in_subquery(assignments)),
        );
    }
    if let Some(timestamp) = updated_since {
        query = query.filter(medication_take::Column::UpdatedAt.gte(timestamp));
    }
    let total_count = query.clone().count(tenant.transaction()).await?;
    let records = query
        .order_by_asc(medication_take::Column::Id)
        .limit(per_page as u64)
        .offset(page.saturating_sub(1).saturating_mul(per_page) as u64)
        .all(tenant.transaction())
        .await?;
    Ok(History {
        records,
        total_count,
    })
}
