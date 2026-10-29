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
    let plans: HashMap<_, _> = crate::models::entities::person_schedule::Entity::find()
        .filter(
            crate::models::entities::person_schedule::Column::Id.is_in(
                recordable_sources
                    .iter()
                    .filter_map(|source| source.person_schedule_id.clone()),
            ),
        )
        .all(db)
        .await?
        .into_iter()
        .map(|plan| (plan.id.clone(), plan))
        .collect();
    let missing: Vec<_> = recordable_sources
        .iter()
        .filter_map(|source| source.person_schedule_id.as_ref())
        .filter(|id| !plans.contains_key(*id))
        .cloned()
        .collect();
    let shared_plans: HashMap<_, _> = crate::models::entities::shared_schedule::Entity::find()
        .filter(crate::models::entities::shared_schedule::Column::Id.is_in(missing))
        .all(db)
        .await?
        .into_iter()
        .map(|plan| (plan.id.clone(), plan))
        .collect();
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
        let prescribed = source.person_schedule_id.as_ref().and_then(|id| {
            use crate::models::care::stock_identity::ExactProductIdentity;
            if let Some(plan) = plans.get(id) {
                ExactProductIdentity::from_parts(
                    plan.dmd_system.as_deref(),
                    plan.dmd_concept_class.as_deref(),
                    plan.dmd_code.as_deref(),
                    plan.stock_unit.as_deref(),
                )
            } else {
                shared_plans.get(id).and_then(|plan| {
                    ExactProductIdentity::from_parts(
                        plan.dmd_system.as_deref(),
                        plan.dmd_concept_class.as_deref(),
                        plan.dmd_code.as_deref(),
                        plan.stock_unit.as_deref(),
                    )
                })
            }
        });
        if let Some(identity) = prescribed {
            identities = identities.add(identity.condition());
        }
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
            let ids = if let Some(id) = source.person_schedule_id.as_ref() {
                let matches = |candidate: &medication::Model| {
                    if let Some(plan) = plans.get(id) {
                        crate::models::care::person_schedules::stock_matches(plan, candidate)
                    } else {
                        shared_plans.get(id).is_some_and(|plan| {
                            crate::models::care::person_schedules::projection::stock_matches(
                                plan, candidate,
                            )
                        })
                    }
                };
                if !matches(original) {
                    return Some((source.id, Vec::new()));
                }
                candidates
                    .iter()
                    .filter(|candidate| {
                        candidate.household_id == household_id
                            && (all_stock
                                || candidate.id == original.id
                                || linked_stock
                                    .get(&source.person_id)
                                    .is_some_and(|ids| ids.contains(&candidate.id)))
                            && candidate
                                .current_supply
                                .is_none_or(|supply| supply > sea_orm::prelude::Decimal::ZERO)
                            && matches(candidate)
                    })
                    .map(|candidate| candidate.id)
                    .collect()
            } else {
                ids
            };
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
