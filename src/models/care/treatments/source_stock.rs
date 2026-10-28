use super::projection::SourceRef;
use super::*;
use crate::models::entities::{location, person_medication};
use loco_rs::model::TenantQueryExt;
use sea_orm::{Condition, JoinType, QueryOrder};
use std::collections::{HashMap, HashSet};
pub(super) async fn source_stock(
    db: &DatabaseTransaction,
    context: &AuthContext,
    sources: &[SourceRef],
    medications: &HashMap<i64, medication::Model>,
    recordable_people: &HashSet<i64>,
) -> Result<HashMap<i64, Vec<i64>>, ApiError> {
    let recordable_sources: Vec<&SourceRef> = sources
        .iter()
        .filter(|source| recordable_people.contains(&source.person_id))
        .collect();
    if recordable_sources.is_empty() {
        return Ok(HashMap::new());
    }
    let household_id = context.scope().household_id;
    let all_stock = access::can_manage_household(context);
    let mut linked_stock: HashMap<i64, HashSet<i64>> = HashMap::new();
    if !all_stock {
        let person_ids: Vec<i64> = recordable_sources
            .iter()
            .map(|source| source.person_id)
            .collect();
        let assignments: Vec<(i64, i64)> = person_medication::Entity::find()
            .select_only()
            .columns([
                person_medication::Column::PersonId,
                person_medication::Column::MedicationId,
            ])
            .in_tenant(household_id)
            .filter(person_medication::Column::PersonId.is_in(person_ids.clone()))
            .into_tuple()
            .all(db)
            .await
            .map_err(database_error)?;
        let schedules: Vec<(i64, i64)> = schedule::Entity::find()
            .select_only()
            .columns([schedule::Column::PersonId, schedule::Column::MedicationId])
            .in_tenant(household_id)
            .filter(schedule::Column::PersonId.is_in(person_ids))
            .into_tuple()
            .all(db)
            .await
            .map_err(database_error)?;
        for (person_id, medication_id) in assignments.into_iter().chain(schedules) {
            linked_stock
                .entry(person_id)
                .or_default()
                .insert(medication_id);
        }
    }
    let mut identities = Condition::any();
    for source in &recordable_sources {
        if let Some(original) = medications.get(&source.medication_id) {
            identities = identities.add(crate::models::care::doses::stock_candidate_condition(
                original,
            ));
        }
    }
    let mut query = access::medication_scope(context).filter(identities);
    if !all_stock {
        let candidate_ids: HashSet<i64> = linked_stock
            .values()
            .flat_map(|ids| ids.iter().copied())
            .chain(recordable_sources.iter().map(|source| source.medication_id))
            .collect();
        query = query.filter(medication::Column::Id.is_in(candidate_ids));
    }
    let candidates = query
        .join(
            JoinType::LeftJoin,
            medication::Entity::belongs_to(location::Entity)
                .from((
                    medication::Column::LocationId,
                    medication::Column::HouseholdId,
                ))
                .to((location::Column::Id, location::Column::HouseholdId))
                .into(),
        )
        .order_by_asc(location::Column::Name)
        .order_by_asc(medication::Column::Id)
        .all(db)
        .await
        .map_err(database_error)?;
    Ok(recordable_sources
        .into_iter()
        .filter_map(|source| {
            let original = medications.get(&source.medication_id)?;
            let ids = eligible_stock_ids(
                household_id,
                all_stock,
                original,
                &candidates,
                linked_stock.get(&source.person_id),
            );
            Some((source.id, ids))
        })
        .collect())
}

fn eligible_stock_ids(
    household_id: i64,
    all_stock: bool,
    original: &medication::Model,
    candidates: &[medication::Model],
    linked_stock: Option<&HashSet<i64>>,
) -> Vec<i64> {
    candidates
        .iter()
        .filter(|candidate| {
            candidate.household_id == household_id
                && (all_stock
                    || candidate.id == original.id
                    || linked_stock.is_some_and(|ids| ids.contains(&candidate.id)))
                && crate::models::care::doses::same_stock_signature(candidate, original)
                && candidate
                    .current_supply
                    .is_none_or(|supply| supply > sea_orm::prelude::Decimal::ZERO)
        })
        .map(|candidate| candidate.id)
        .collect()
}
