use super::*;
use crate::models::entities::{location, medication, person, person_medication, schedule};
use crate::views::medication_takes::MedicationTake;
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
) -> Result<Vec<MedicationTake>, OperationError> {
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
        crate::models::access::person_queries::household_participants(household_id)
            .filter(person::Column::Id.is_in(person_ids))
            .all(db)
            .await
            .map_err(|_| OperationError::Unavailable)?
            .into_iter()
            .map(|value| (value.id, value))
            .collect()
    };
    Ok(takes
        .iter()
        .map(|take| {
            let schedule = take.schedule_id.and_then(|id| schedules.get(&id));
            let assignment = take
                .person_medication_id
                .and_then(|id| assignments.get(&id));
            let person_id = schedule
                .map(|v| v.person_id)
                .or_else(|| assignment.map(|v| v.person_id));
            let medication_id = schedule
                .map(|v| v.medication_id)
                .or_else(|| assignment.map(|v| v.medication_id));
            MedicationTake {
                id: take.id,
                portable_id: take.portable_id.clone(),
                client_uuid: take.client_uuid.clone(),
                schedule_id: take.schedule_id,
                schedule_portable_id: schedule.map(|row| row.portable_id.clone()),
                person_medication_id: take.person_medication_id,
                person_medication_portable_id: assignment.map(|row| row.portable_id.clone()),
                taken_from_medication_id: take.taken_from_medication_id,
                taken_from_medication_portable_id: take
                    .taken_from_medication_id
                    .and_then(|id| inventory.get(&id))
                    .cloned(),
                taken_from_location_id: take.taken_from_location_id,
                taken_from_location_portable_id: take
                    .taken_from_location_id
                    .and_then(|id| locations.get(&id))
                    .cloned(),
                dose_amount: take
                    .dose_amount
                    .map(|amount| decimal_string(amount.to_string())),
                dose_unit: take.dose_unit.clone(),
                taken_at: take
                    .taken_at
                    .map(|value| value.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
                updated_at: take.updated_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                person_id,
                person_portable_id: person_id
                    .and_then(|id| people.get(&id))
                    .map(|row| row.portable_id.clone()),
                medication_id,
                medication_portable_id: medication_id.and_then(|id| meds.get(&id)).cloned(),
            }
        })
        .collect())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::medication_takes::MedicationTakeResponse;

    #[test]
    fn typed_medication_take_keeps_wire_ids_and_nulls() -> Result<(), serde_json::Error> {
        let response = MedicationTakeResponse {
            data: MedicationTake {
                id: 12,
                portable_id: "synthetic-take".into(),
                client_uuid: None,
                schedule_id: Some(23),
                schedule_portable_id: Some("synthetic-schedule".into()),
                person_medication_id: None,
                person_medication_portable_id: None,
                taken_from_medication_id: None,
                taken_from_medication_portable_id: None,
                taken_from_location_id: None,
                taken_from_location_portable_id: None,
                dose_amount: Some(decimal_string("2.000".into())),
                dose_unit: Some("tablet".into()),
                taken_at: None,
                updated_at: "2026-11-24T00:00:00Z".into(),
                person_id: Some(34),
                person_portable_id: Some("synthetic-person".into()),
                medication_id: Some(45),
                medication_portable_id: None,
            },
        };
        assert_eq!(
            serde_json::to_value(response)?,
            json!({"data": {
                "id":12,"portable_id":"synthetic-take","client_uuid":null,
                "schedule_id":23,"schedule_portable_id":"synthetic-schedule",
                "person_medication_id":null,"person_medication_portable_id":null,
                "taken_from_medication_id":null,"taken_from_medication_portable_id":null,
                "taken_from_location_id":null,"taken_from_location_portable_id":null,
                "dose_amount":"2.0","dose_unit":"tablet","taken_at":null,
                "updated_at":"2026-11-24T00:00:00Z","person_id":34,
                "person_portable_id":"synthetic-person","medication_id":45,
                "medication_portable_id":null
            }})
        );
        Ok(())
    }
}
