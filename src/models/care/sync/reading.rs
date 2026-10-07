use super::*;
use sea_orm::DatabaseTransaction;
fn database_error(_: sea_orm::DbErr) -> OperationError {
    OperationError::Unavailable
}

pub async fn changes(tenant: &TenantTransaction, cursor: &str) -> Result<Value, OperationError> {
    lock(tenant).await?;
    let since = chrono::DateTime::parse_from_rfc3339(cursor)
        .map_err(|_| OperationError::Validation {
            details: json!({"error":"cursor must be ISO8601"}),
        })?
        .naive_utc();
    let db = tenant.transaction();
    let household_id = tenant.scope().household_id;
    let response_cursor = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true);
    let people = super::portable::visible_people(tenant).await?;
    let visible_people: HashSet<&str> = people
        .iter()
        .map(|person| person.portable_id.as_str())
        .collect();
    let manager = matches!(tenant.membership().role.as_str(), "owner" | "administrator");
    let visible_medications = if manager {
        HashSet::new()
    } else {
        visible_medication_ids(db, tenant).await?
    };
    let (visible_schedule_ids, visible_assignment_ids) =
        visible_source_ids(db, household_id, &people).await?;
    let visible_periods = if manager {
        HashSet::new()
    } else {
        visible_pause_period_ids(
            db,
            household_id,
            &visible_schedule_ids,
            &visible_assignment_ids,
        )
        .await?
    };
    let events = api_change_event::Entity::find()
        .filter(api_change_event::Column::HouseholdId.eq(household_id))
        .filter(api_change_event::Column::OccurredAt.gte(since))
        .order_by_asc(api_change_event::Column::OccurredAt)
        .order_by_asc(api_change_event::Column::Id)
        .all(db)
        .await
        .map_err(database_error)?;
    let tombstones = api_tombstone::Entity::find()
        .filter(api_tombstone::Column::HouseholdId.eq(household_id))
        .filter(api_tombstone::Column::DeletedAt.gte(since))
        .order_by_asc(api_tombstone::Column::DeletedAt)
        .order_by_asc(api_tombstone::Column::Id)
        .all(db)
        .await
        .map_err(database_error)?;
    let reassigned_ids: Vec<String> = tombstones
        .iter()
        .filter(|row| {
            row.record_type == "HealthEvent"
                && row.metadata["reassigned_to_person_portable_id"].is_string()
        })
        .map(|row| row.record_portable_id.clone())
        .collect();
    let visible_person_ids: HashSet<i64> = people.iter().map(|person| person.id).collect();
    let visible_reassigned_events: HashSet<String> = health_event::Entity::find()
        .filter(health_event::Column::HouseholdId.eq(household_id))
        .filter(health_event::Column::PortableId.is_in(reassigned_ids))
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .filter(|row| manager || visible_person_ids.contains(&row.person_id))
        .map(|row| row.portable_id)
        .collect();
    let outcome_ids: Vec<i64> = events
        .iter()
        .filter(|event| event.record_type == "MedicationDoseOccurrence")
        .map(|event| event.record_id)
        .collect();
    let outcomes: HashMap<i64, dose_occurrence::Model> = dose_occurrence::Entity::find()
        .filter(dose_occurrence::Column::HouseholdId.eq(household_id))
        .filter(dose_occurrence::Column::Id.is_in(outcome_ids))
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|outcome| (outcome.id, outcome))
        .collect();
    let outcome_schedule_ids: Vec<i64> = outcomes
        .values()
        .filter_map(|row| row.schedule_id)
        .collect();
    let outcome_assignment_ids: Vec<i64> = outcomes
        .values()
        .filter_map(|row| row.person_medication_id)
        .collect();
    let outcome_take_ids: Vec<i64> = outcomes
        .values()
        .filter_map(|row| row.medication_take_id)
        .collect();
    let outcome_schedules: HashMap<i64, String> = schedule::Entity::find()
        .filter(schedule::Column::HouseholdId.eq(household_id))
        .filter(schedule::Column::Id.is_in(outcome_schedule_ids))
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|row| (row.id, row.portable_id))
        .collect();
    let outcome_assignments: HashMap<i64, String> = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(household_id))
        .filter(person_medication::Column::Id.is_in(outcome_assignment_ids))
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|row| (row.id, row.portable_id))
        .collect();
    let outcome_takes: HashMap<i64, String> = medication_take::Entity::find()
        .filter(medication_take::Column::HouseholdId.eq(household_id))
        .filter(medication_take::Column::Id.is_in(outcome_take_ids))
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|row| (row.id, row.portable_id))
        .collect();
    let dosage_ids: Vec<i64> = events
        .iter()
        .filter(|event| event.record_type == "MedicationDosageOption")
        .map(|event| event.record_id)
        .collect();
    let dosage_parents: HashMap<i64, i64> = dosage::Entity::find()
        .filter(dosage::Column::HouseholdId.eq(household_id))
        .filter(dosage::Column::Id.is_in(dosage_ids))
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|row| (row.id, row.medication_id))
        .collect();
    let parent_ids: HashSet<i64> = tombstones
        .iter()
        .filter(|row| row.record_type == "MedicationDosageOption")
        .filter_map(|row| row.metadata["medication_id"].as_i64())
        .chain(
            events
                .iter()
                .filter(|row| row.record_type == "MedicationDosageOption")
                .filter_map(|row| row.metadata["medication_id"].as_i64()),
        )
        .chain(dosage_parents.values().copied())
        .collect();
    let live_parents: HashSet<i64> = medication::Entity::find()
        .filter(medication::Column::HouseholdId.eq(household_id))
        .filter(medication::Column::Id.is_in(parent_ids.clone()))
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|row| row.id)
        .collect();
    let deleted_parents: HashMap<i64, Value> = api_tombstone::Entity::find()
        .filter(api_tombstone::Column::HouseholdId.eq(household_id))
        .filter(api_tombstone::Column::RecordType.eq("Medication"))
        .order_by_asc(api_tombstone::Column::DeletedAt)
        .order_by_asc(api_tombstone::Column::Id)
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .filter_map(|row| {
            let id = row.metadata["record_id"].as_i64()?;
            parent_ids.contains(&id).then_some((id, row.metadata))
        })
        .collect();
    let dosage_visible = |parent: Option<i64>| {
        let Some(parent) = parent else {
            return false;
        };
        if manager {
            return true;
        }
        if live_parents.contains(&parent) {
            return visible_medications.contains(&parent);
        }
        deleted_parents.get(&parent).is_some_and(|metadata| {
            visible(
                "Medication",
                None,
                None,
                metadata,
                &visible_people,
                &visible_medications,
                false,
                tenant.membership().id,
            )
        })
    };
    let changes: Vec<Value> = events.iter().filter_map(|event| {
        if event.record_type == "MedicationDosageOption" {
            let parent=dosage_parents.get(&event.record_id).copied()
                .or_else(|| event.metadata["medication_id"].as_i64());
            if !dosage_visible(parent) { return None; }
        }
        if event.record_type == "MedicationDoseOccurrence" && !manager {
            let outcome = outcomes.get(&event.record_id)?;
            if !outcome.schedule_id.is_some_and(|id| visible_schedule_ids.contains(&id))
                && !outcome.person_medication_id.is_some_and(|id| visible_assignment_ids.contains(&id)) {
                return None;
            }
        }
        if event.record_type == "MedicationPausePeriod" && !manager && !visible_periods.contains(&event.record_id) {
            return None;
        }
        if !matches!(event.record_type.as_str(), "MedicationPausePeriod" | "MedicationDoseOccurrence" | "MedicationDosageOption")
            && !visible(&event.record_type, Some(event.record_id), event.record_portable_id.as_deref(),
            &event.metadata, &visible_people, &visible_medications, manager, tenant.membership().id) {
            return None;
        }
        let portable_id = event.record_portable_id.as_deref()?;
        let mut value = json!({
            "id": event.id, "record_type": event.record_type, "record_id": event.record_id,
            "record_portable_id": portable_id, "action": event.action,
            "occurred_at": event.occurred_at.and_utc().to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true),
            "metadata": event.metadata,
        });
        if event.record_type == "MedicationDoseOccurrence" {
            let outcome = outcomes.get(&event.record_id)?;
            let (source_type, source_id) = if let Some(id) = outcome.schedule_id {
                ("schedule", outcome_schedules.get(&id).map(String::as_str))
            } else { ("person_medication", outcome.person_medication_id
                .and_then(|id| outcome_assignments.get(&id).map(String::as_str))) };
            value["record"] = super::portable::occurrence_value(outcome, source_type, source_id,
                outcome.medication_take_id.and_then(|id| outcome_takes.get(&id).map(String::as_str)));
            value["record"]["etag"] = json!(super::portable::occurrence_etag(outcome));
        }
        Some(value)
    }).collect();
    let tombstones: Vec<Value> = tombstones.iter().filter_map(|row| {
        if row.record_type == "MedicationPausePeriod" {
            return None;
        }
        if row.record_type == "HealthEvent"
            && row.metadata["reassigned_to_person_portable_id"].is_string()
            && visible_reassigned_events.contains(&row.record_portable_id)
        {
            return None;
        }
        if row.record_type == "MedicationDosageOption" && !dosage_visible(row.metadata["medication_id"].as_i64()) {
            return None;
        }
        if row.record_type != "MedicationDosageOption" && !visible(&row.record_type, None, Some(&row.record_portable_id), &row.metadata,
            &visible_people, &visible_medications, manager, tenant.membership().id) {
            return None;
        }
        let mut metadata = row.metadata.clone();
        if let Some(metadata) = metadata.as_object_mut() {
            metadata.remove("sync_person_portable_ids");
            metadata.remove("sync_creator_membership_id");
            metadata.remove("reassigned_to_person_portable_id");
            metadata.remove("medication_id");
        }
        Some(json!({
            "id": row.id, "record_type": row.record_type, "record_portable_id": row.record_portable_id,
            "action": row.action, "deleted_at": row.deleted_at.and_utc().to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true),
            "metadata": metadata,
        }))
    }).collect();
    Ok(json!({"data":{"cursor":response_cursor,"changes":changes,"tombstones":tombstones}}))
}

pub async fn snapshot(
    tenant: &TenantTransaction,
    zone: chrono_tz::Tz,
) -> Result<AppliedBatch, OperationError> {
    lock(tenant).await?;
    let mut payload = super::portable::payload(tenant, zone).await?;
    payload.body["cursor"] = json!(Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true));
    payload.body = json!({"data":payload.body});
    Ok(payload)
}

#[allow(clippy::too_many_arguments)]
fn visible(
    record_type: &str,
    record_id: Option<i64>,
    portable_id: Option<&str>,
    metadata: &Value,
    people: &HashSet<&str>,
    medications: &HashSet<i64>,
    manager: bool,
    membership_id: i64,
) -> bool {
    if manager {
        return true;
    }
    match record_type {
        "Location" => true,
        "MedicationDosageOption" => {
            metadata["medication_id"]
                .as_i64()
                .is_some_and(|id| medications.contains(&id))
                || visible(
                    "Medication",
                    None,
                    portable_id,
                    metadata,
                    people,
                    medications,
                    false,
                    membership_id,
                )
        }
        "Person" => portable_id.is_some_and(|id| people.contains(id)),
        "Medication" => {
            if let Some(id) = record_id {
                return medications.contains(&id);
            }
            metadata["sync_person_portable_ids"]
                .as_array()
                .is_some_and(|ids| {
                    ids.iter()
                        .filter_map(Value::as_str)
                        .any(|id| people.contains(id))
                })
                || metadata["sync_creator_membership_id"].as_str()
                    == Some(membership_id.to_string().as_str())
        }
        _ => metadata["person_portable_id"]
            .as_str()
            .is_some_and(|id| people.contains(id)),
    }
}

pub(super) async fn visible_medication_ids(
    db: &DatabaseTransaction,
    context: &TenantTransaction,
) -> Result<HashSet<i64>, OperationError> {
    Ok(access::medication_scope(context)
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|row| row.id)
        .collect())
}

async fn visible_source_ids(
    db: &DatabaseTransaction,
    household_id: i64,
    people: &[person::Model],
) -> Result<(HashSet<i64>, HashSet<i64>), OperationError> {
    let person_ids: Vec<i64> = people.iter().map(|person| person.id).collect();
    let schedule_ids: HashSet<i64> = schedule::Entity::find()
        .filter(schedule::Column::HouseholdId.eq(household_id))
        .filter(schedule::Column::PersonId.is_in(person_ids.clone()))
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|row| row.id)
        .collect();
    let assignment_ids: HashSet<i64> = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(household_id))
        .filter(person_medication::Column::PersonId.is_in(person_ids))
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|row| row.id)
        .collect();
    Ok((schedule_ids, assignment_ids))
}

async fn visible_pause_period_ids(
    db: &DatabaseTransaction,
    household_id: i64,
    schedule_ids: &HashSet<i64>,
    assignment_ids: &HashSet<i64>,
) -> Result<HashSet<i64>, OperationError> {
    let periods = pause_period::Entity::find()
        .filter(pause_period::Column::HouseholdId.eq(household_id))
        .filter(
            sea_orm::Condition::any()
                .add(
                    pause_period::Column::ScheduleId
                        .is_in(schedule_ids.iter().copied().collect::<Vec<_>>()),
                )
                .add(
                    pause_period::Column::PersonMedicationId
                        .is_in(assignment_ids.iter().copied().collect::<Vec<_>>()),
                ),
        )
        .all(db)
        .await
        .map_err(database_error)?;
    Ok(periods.into_iter().map(|row| row.id).collect())
}
