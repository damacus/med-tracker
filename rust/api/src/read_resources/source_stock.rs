use super::source_projection::SourceRef;
use crate::entities::medication;
use crate::read_entities::{person_medication, schedule};
use crate::{database_error, ApiError, AuthContext};
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, DatabaseTransaction, DbBackend, EntityTrait,
    QueryFilter, QuerySelect, Statement,
};
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
    let household_id = context.membership.household_id;
    let all_stock = matches!(context.membership.role.as_str(), "owner" | "administrator");
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
            .filter(person_medication::Column::HouseholdId.eq(household_id))
            .filter(person_medication::Column::PersonId.is_in(person_ids.clone()))
            .into_tuple()
            .all(db)
            .await
            .map_err(database_error)?;
        let schedules: Vec<(i64, i64)> = schedule::Entity::find()
            .select_only()
            .columns([schedule::Column::PersonId, schedule::Column::MedicationId])
            .filter(schedule::Column::HouseholdId.eq(household_id))
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
    let mut names = Condition::any();
    for source in &recordable_sources {
        if let Some(original) = medications.get(&source.medication_id) {
            names = names.add(match &original.name {
                Some(name) => medication::Column::Name.eq(name),
                None => medication::Column::Name.is_null(),
            });
        }
    }
    let mut query = crate::scope(household_id, &context.membership).filter(names);
    if !all_stock {
        let candidate_ids: HashSet<i64> = linked_stock
            .values()
            .flat_map(|ids| ids.iter().copied())
            .chain(recordable_sources.iter().map(|source| source.medication_id))
            .collect();
        query = query.filter(medication::Column::Id.is_in(candidate_ids));
    }
    let mut candidates = query.all(db).await.map_err(database_error)?;
    if !candidates.is_empty() {
        let placeholders = (2..=candidates.len() + 1)
            .map(|index| format!("${index}"))
            .collect::<Vec<_>>()
            .join(", ");
        let values = std::iter::once(household_id.into())
            .chain(candidates.iter().map(|candidate| candidate.id.into()))
            .collect::<Vec<sea_orm::Value>>();
        let rows = db.query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            format!("SELECT m.id FROM medications m LEFT JOIN locations l ON l.id=m.location_id AND l.household_id=m.household_id WHERE m.household_id=$1 AND m.id IN ({placeholders}) ORDER BY l.name ASC, m.id ASC"), values,
        )).await.map_err(database_error)?;
        let order = rows
            .into_iter()
            .enumerate()
            .map(|(rank, row)| {
                row.try_get::<i64>("", "id")
                    .map(|id| (id, rank))
                    .map_err(database_error)
            })
            .collect::<Result<HashMap<_, _>, _>>()?;
        if order.len() != candidates.len()
            || candidates
                .iter()
                .any(|candidate| !order.contains_key(&candidate.id))
        {
            return Err(ApiError::internal());
        }
        candidates.sort_by_key(|candidate| {
            (
                order.get(&candidate.id).copied().unwrap_or(usize::MAX),
                candidate.id,
            )
        });
    }
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
                && crate::dose::same_stock_signature(candidate, original)
                && candidate
                    .current_supply
                    .is_none_or(|supply| supply > sea_orm::prelude::Decimal::ZERO)
        })
        .map(|candidate| candidate.id)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sea_orm::prelude::Decimal;

    fn medication(id: i64) -> medication::Model {
        medication::Model {
            id,
            household_id: 2,
            created_at: Utc::now().naive_utc(),
            created_by_membership_id: Some(4),
            portable_id: format!("medication-{id}"),
            name: Some("Paracetamol".into()),
            friendly_name: None,
            category: None,
            description: None,
            barcode: None,
            dmd_code: None,
            dmd_system: None,
            dmd_concept_class: None,
            dose_amount: Some(500.0),
            dose_unit: Some("mg".into()),
            current_supply: Some(Decimal::ONE),
            supply_at_last_restock: None,
            reorder_threshold: Decimal::ZERO,
            reorder_status: None,
            order_supplier: None,
            order_quantity: None,
            expected_arrival_on: None,
            ordered_at: None,
            reordered_at: None,
            warnings: None,
            default_schedule_type: 0,
            location_id: 8,
            updated_at: Utc::now().naive_utc(),
        }
    }

    fn stock_candidates() -> Vec<medication::Model> {
        let mut candidates: Vec<_> = (10..=19).map(medication).collect();
        candidates[2].household_id = 9;
        candidates[3].name = Some("Ibuprofen".into());
        candidates[4].dose_amount = Some(250.0);
        candidates[5].dose_unit = Some("ml".into());
        candidates[6].current_supply = Some(Decimal::ZERO);
        candidates[7].current_supply = Some(-Decimal::ONE);
        candidates[8].current_supply = None;
        candidates
    }

    #[test]
    fn owner_stock_excludes_foreign_mismatched_and_exhausted_inventory() {
        let candidates = stock_candidates();
        assert_eq!(
            eligible_stock_ids(2, true, &candidates[0], &candidates, None),
            vec![10, 11, 18, 19]
        );
    }

    #[test]
    fn delegated_stock_uses_only_source_person_links_and_original_inventory() {
        let candidates = stock_candidates();
        let links = HashSet::from([11, 12, 13, 14, 15, 16, 17, 18]);
        assert_eq!(
            eligible_stock_ids(2, false, &candidates[0], &candidates, Some(&links)),
            vec![10, 11, 18]
        );
        assert_eq!(
            eligible_stock_ids(2, false, &candidates[0], &candidates, None),
            vec![10]
        );
    }
}
