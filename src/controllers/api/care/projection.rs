use super::*;
use crate::models::entities::{location, medication, person, person_medication, schedule};
use std::collections::HashMap;

pub(super) fn take_etag(take: &medication_take::Model) -> String {
    let input = format!(
        "MedicationTake:{}:{}",
        take.id,
        take.updated_at.and_utc().timestamp_micros()
    );
    format!("\"{}\"", hex::encode(Sha256::digest(input.as_bytes())))
}

pub(super) async fn serialize(
    db: &DatabaseTransaction,
    takes: &[medication_take::Model],
    household_id: i64,
) -> Result<Vec<Value>, OperationError> {
    let schedule_ids: Vec<i64> = takes.iter().filter_map(|take| take.schedule_id).collect();
    let assignment_ids: Vec<i64> = takes
        .iter()
        .filter_map(|take| take.person_medication_id)
        .collect();
    let inventory_ids: Vec<i64> = takes
        .iter()
        .filter_map(|take| take.taken_from_medication_id)
        .collect();
    let location_ids: Vec<i64> = takes
        .iter()
        .filter_map(|take| take.taken_from_location_id)
        .collect();
    let schedules: HashMap<_, _> = if schedule_ids.is_empty() {
        HashMap::new()
    } else {
        schedule::Entity::find()
            .filter(schedule::Column::HouseholdId.eq(household_id))
            .filter(schedule::Column::Id.is_in(schedule_ids))
            .all(db)
            .await
            .map_err(|_| OperationError::Unavailable)?
            .into_iter()
            .map(|value| (value.id, value))
            .collect()
    };
    let assignments: HashMap<_, _> = if assignment_ids.is_empty() {
        HashMap::new()
    } else {
        person_medication::Entity::find()
            .filter(person_medication::Column::HouseholdId.eq(household_id))
            .filter(person_medication::Column::Id.is_in(assignment_ids))
            .all(db)
            .await
            .map_err(|_| OperationError::Unavailable)?
            .into_iter()
            .map(|value| (value.id, value))
            .collect()
    };
    let inventory: HashMap<_, _> = if inventory_ids.is_empty() {
        HashMap::new()
    } else {
        medication::Entity::find()
            .filter(medication::Column::HouseholdId.eq(household_id))
            .filter(medication::Column::Id.is_in(inventory_ids))
            .all(db)
            .await
            .map_err(|_| OperationError::Unavailable)?
            .into_iter()
            .map(|value| (value.id, value.portable_id))
            .collect()
    };
    let locations: HashMap<_, _> = if location_ids.is_empty() {
        HashMap::new()
    } else {
        location::Entity::find()
            .filter(location::Column::HouseholdId.eq(household_id))
            .filter(location::Column::Id.is_in(location_ids))
            .all(db)
            .await
            .map_err(|_| OperationError::Unavailable)?
            .into_iter()
            .map(|value| (value.id, value.portable_id))
            .collect()
    };
    let med_ids: Vec<i64> = schedules
        .values()
        .map(|v| v.medication_id)
        .chain(assignments.values().map(|v| v.medication_id))
        .collect();
    let meds: HashMap<_, _> = if med_ids.is_empty() {
        HashMap::new()
    } else {
        medication::Entity::find()
            .filter(medication::Column::HouseholdId.eq(household_id))
            .filter(medication::Column::Id.is_in(med_ids))
            .all(db)
            .await
            .map_err(|_| OperationError::Unavailable)?
            .into_iter()
            .map(|value| (value.id, value.portable_id))
            .collect()
    };
    let person_ids: Vec<i64> = schedules
        .values()
        .map(|v| v.person_id)
        .chain(assignments.values().map(|v| v.person_id))
        .collect();
    let people: HashMap<_, _> = if person_ids.is_empty() {
        HashMap::new()
    } else {
        person::Entity::find()
            .filter(person::Column::HouseholdId.eq(household_id))
            .filter(person::Column::Id.is_in(person_ids))
            .all(db)
            .await
            .map_err(|_| OperationError::Unavailable)?
            .into_iter()
            .map(|value| (value.id, value))
            .collect()
    };
    Ok(takes.iter().map(|take| {
        let schedule = take.schedule_id.and_then(|id| schedules.get(&id));
        let assignment = take.person_medication_id.and_then(|id| assignments.get(&id));
        let person_id = schedule.map(|v| v.person_id).or_else(|| assignment.map(|v| v.person_id));
        let medication_id = schedule.map(|v| v.medication_id).or_else(|| assignment.map(|v| v.medication_id));
        json!({
            "id": take.id, "portable_id": take.portable_id, "client_uuid": take.client_uuid,
            "schedule_id": take.schedule_id, "schedule_portable_id": schedule.map(|v| &v.portable_id),
            "person_medication_id": take.person_medication_id, "person_medication_portable_id": assignment.map(|v| &v.portable_id),
            "taken_from_medication_id": take.taken_from_medication_id, "taken_from_medication_portable_id": take.taken_from_medication_id.and_then(|id| inventory.get(&id)),
            "taken_from_location_id": take.taken_from_location_id, "taken_from_location_portable_id": take.taken_from_location_id.and_then(|id| locations.get(&id)),
            "dose_amount": take.dose_amount.map(|amount| decimal_string(amount.to_string())), "dose_unit": take.dose_unit,
            "taken_at": take.taken_at.map(|value| value.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
            "updated_at": take.updated_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            "person_id": person_id, "person_portable_id": person_id.and_then(|id| people.get(&id)).map(|v| &v.portable_id),
            "medication_id": medication_id, "medication_portable_id": medication_id.and_then(|id| meds.get(&id))
        })
    }).collect())
}

fn decimal_string(value: String) -> String {
    if value.contains('.') {
        let trimmed = value.trim_end_matches('0');
        if trimmed.ends_with('.') {
            format!("{trimmed}0")
        } else {
            trimmed.into()
        }
    } else {
        format!("{value}.0")
    }
}
