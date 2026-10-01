use super::persistence::tombstone;
use crate::database_error;
use crate::entities::dosage;
use crate::entities::medication;
use crate::entities::medication_take;
use crate::entities::person;
use crate::entities::person_medication;
use crate::entities::schedule;
use crate::medication_management::record_version;
use crate::read_entities::dose_occurrence;
use crate::read_entities::location_membership;
use crate::read_entities::pause_period;
use crate::read_entities::stock_location;
use crate::ApiError;
use crate::AuthContext;
use sea_orm::ColumnTrait;
use sea_orm::Condition;
use sea_orm::ConnectionTrait;
use sea_orm::DatabaseTransaction;
use sea_orm::DbErr;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::QueryOrder;
use sea_orm::QuerySelect;
use serde_json::json;
use std::collections::HashMap;
use std::collections::HashSet;

pub(super) struct CascadeSnapshot {
    pub(super) medications: Vec<medication::Model>,
    pub(super) dosages: Vec<dosage::Model>,
    pub(super) schedules: Vec<schedule::Model>,
    pub(super) assignments: Vec<person_medication::Model>,
    pub(super) memberships: Vec<location_membership::Model>,
}

pub(super) async fn record_cascade_effects(
    db: &DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    cascade: &CascadeSnapshot,
) -> Result<(), ApiError> {
    let person_ids: HashSet<i64> = cascade
        .schedules
        .iter()
        .map(|row| row.person_id)
        .chain(cascade.assignments.iter().map(|row| row.person_id))
        .collect();
    let people = if person_ids.is_empty() {
        Vec::new()
    } else {
        person::Entity::find()
            .filter(person::Column::Id.is_in(person_ids))
            .all(db)
            .await
            .map_err(database_error)?
    };
    let person_portable_ids: HashMap<i64, String> = people
        .into_iter()
        .map(|row| (row.id, row.portable_id))
        .collect();
    for row in &cascade.memberships {
        record_version(
            db,
            context,
            request_id,
            "LocationMembership",
            row.id,
            "destroy",
            Some(json!({"id": row.id, "household_id": row.household_id, "location_id": row.location_id, "person_id": row.person_id, "created_at": row.created_at, "updated_at": row.updated_at})),
            None,
        )
        .await?;
    }
    for row in &cascade.schedules {
        record_version(
            db, context, request_id, "Schedule", row.id, "destroy",
            Some(json!({"id":row.id,"household_id":row.household_id,"portable_id":row.portable_id,"person_id":row.person_id,"medication_id":row.medication_id,"source_dosage_option_id":row.source_dosage_option_id,"dose_amount":row.dose_amount,"dose_unit":row.dose_unit,"frequency":row.frequency,"dose_cycle":row.dose_cycle,"start_date":row.start_date,"end_date":row.end_date,"active":row.active,"notes":row.notes,"created_at":row.created_at,"updated_at":row.updated_at,"schedule_type":row.schedule_type,"schedule_config":row.schedule_config,"max_daily_doses":row.max_daily_doses,"min_hours_between_doses":row.min_hours_between_doses,"retired_at":row.retired_at})),
            None,
        ).await?;
        let person_metadata = person_portable_ids.get(&row.person_id).map_or_else(
            || json!({}),
            |portable_id| json!({"person_portable_id": portable_id}),
        );
        tombstone(
            db,
            context,
            "Schedule",
            row.id,
            &row.portable_id,
            person_metadata,
        )
        .await?;
    }
    for row in &cascade.assignments {
        record_version(
            db, context, request_id, "PersonMedication", row.id, "destroy",
            Some(json!({"id":row.id,"household_id":row.household_id,"portable_id":row.portable_id,"person_id":row.person_id,"medication_id":row.medication_id,"source_dosage_option_id":row.source_dosage_option_id,"dose_amount":row.dose_amount,"dose_unit":row.dose_unit,"active":row.active,"notes":row.notes,"created_at":row.created_at,"updated_at":row.updated_at,"dose_cycle":row.dose_cycle,"administration_kind":row.administration_kind,"position":row.position,"max_daily_doses":row.max_daily_doses,"min_hours_between_doses":row.min_hours_between_doses,"retired_at":row.retired_at})),
            None,
        ).await?;
        let person_metadata = person_portable_ids.get(&row.person_id).map_or_else(
            || json!({}),
            |portable_id| json!({"person_portable_id": portable_id}),
        );
        tombstone(
            db,
            context,
            "PersonMedication",
            row.id,
            &row.portable_id,
            person_metadata,
        )
        .await?;
    }
    for row in &cascade.dosages {
        record_version(
            db, context, request_id, "MedicationDosageOption", row.id, "destroy",
            Some(json!({"id":row.id,"household_id":row.household_id,"medication_id":row.medication_id,"portable_id":row.portable_id,"amount":row.amount,"unit":row.unit,"current_supply":row.current_supply,"reorder_threshold":row.reorder_threshold,"frequency":row.frequency,"description":row.description,"default_for_adults":row.default_for_adults,"default_for_children":row.default_for_children,"default_max_daily_doses":row.default_max_daily_doses,"default_min_hours_between_doses":row.default_min_hours_between_doses,"default_dose_cycle":row.default_dose_cycle,"created_at":row.created_at,"updated_at":row.updated_at})),
            None,
        ).await?;
        tombstone(
            db,
            context,
            "MedicationDosageOption",
            row.id,
            &row.portable_id,
            json!({}),
        )
        .await?;
    }
    for row in &cascade.medications {
        let mut visible_people: Vec<&str> = cascade
            .schedules
            .iter()
            .filter(|source| source.medication_id == row.id)
            .filter_map(|source| {
                person_portable_ids
                    .get(&source.person_id)
                    .map(String::as_str)
            })
            .chain(
                cascade
                    .assignments
                    .iter()
                    .filter(|source| source.medication_id == row.id)
                    .filter_map(|source| {
                        person_portable_ids
                            .get(&source.person_id)
                            .map(String::as_str)
                    }),
            )
            .collect();
        visible_people.sort_unstable();
        visible_people.dedup();
        let visibility = if !visible_people.is_empty() {
            json!({"sync_person_portable_ids": visible_people})
        } else if let Some(creator) = row.created_by_membership_id {
            json!({"sync_creator_membership_id": creator.to_string()})
        } else {
            json!({})
        };
        tombstone(
            db,
            context,
            "Medication",
            row.id,
            &row.portable_id,
            visibility,
        )
        .await?;
    }
    Ok(())
}

pub(crate) async fn delete_medication_tree(
    db: &DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    medication: &medication::Model,
) -> Result<bool, ApiError> {
    let household_id = context.membership.household_id;
    if medication.household_id != household_id {
        return Err(ApiError::not_found());
    }
    let schedules = schedule::Entity::find()
        .filter(schedule::Column::HouseholdId.eq(household_id))
        .filter(schedule::Column::MedicationId.eq(medication.id))
        .order_by_asc(schedule::Column::Id)
        .lock_exclusive()
        .all(db)
        .await
        .map_err(database_error)?;
    let assignments = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(household_id))
        .filter(person_medication::Column::MedicationId.eq(medication.id))
        .order_by_asc(person_medication::Column::Id)
        .lock_exclusive()
        .all(db)
        .await
        .map_err(database_error)?;
    let dosages = dosage::Entity::find()
        .filter(dosage::Column::HouseholdId.eq(household_id))
        .filter(dosage::Column::MedicationId.eq(medication.id))
        .order_by_asc(dosage::Column::Id)
        .lock_exclusive()
        .all(db)
        .await
        .map_err(database_error)?;
    let schedule_ids: Vec<i64> = schedules.iter().map(|row| row.id).collect();
    let assignment_ids: Vec<i64> = assignments.iter().map(|row| row.id).collect();
    let mut takes =
        Condition::any().add(medication_take::Column::TakenFromMedicationId.eq(medication.id));
    if !schedule_ids.is_empty() {
        takes = takes.add(medication_take::Column::ScheduleId.is_in(schedule_ids.clone()));
    }
    if !assignment_ids.is_empty() {
        takes =
            takes.add(medication_take::Column::PersonMedicationId.is_in(assignment_ids.clone()));
    }
    if medication_take::Entity::find()
        .filter(takes)
        .one(db)
        .await
        .map_err(database_error)?
        .is_some()
    {
        return Ok(false);
    }
    if !schedule_ids.is_empty() || !assignment_ids.is_empty() {
        let mut sources = Condition::any();
        let mut pauses = Condition::any();
        if !schedule_ids.is_empty() {
            sources = sources.add(dose_occurrence::Column::ScheduleId.is_in(schedule_ids.clone()));
            pauses = pauses.add(pause_period::Column::ScheduleId.is_in(schedule_ids.clone()));
        }
        if !assignment_ids.is_empty() {
            sources = sources
                .add(dose_occurrence::Column::PersonMedicationId.is_in(assignment_ids.clone()));
            pauses =
                pauses.add(pause_period::Column::PersonMedicationId.is_in(assignment_ids.clone()));
        }
        if dose_occurrence::Entity::find()
            .filter(sources)
            .one(db)
            .await
            .map_err(database_error)?
            .is_some()
            || pause_period::Entity::find()
                .filter(pauses)
                .one(db)
                .await
                .map_err(database_error)?
                .is_some()
        {
            return Ok(false);
        }
    }
    let retained: bool = db.query_one_raw(sea_orm::Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        "SELECT EXISTS (SELECT 1 FROM health_event_medications WHERE household_id = $1 AND medication_id = $2 UNION ALL SELECT 1 FROM medication_review_prompts WHERE household_id = $1 AND (primary_medication_id = $2 OR interacting_medication_id = $2)) AS retained",
        [household_id.into(), medication.id.into()],
    )).await.map_err(database_error)?.ok_or_else(ApiError::internal)?
        .try_get("", "retained").map_err(|_| ApiError::internal())?;
    if retained {
        return Ok(false);
    }
    if !schedule_ids.is_empty() {
        schedule::Entity::delete_many()
            .filter(schedule::Column::Id.is_in(schedule_ids))
            .exec(db)
            .await
            .map_err(database_error)?;
    }
    if !assignment_ids.is_empty() {
        person_medication::Entity::delete_many()
            .filter(person_medication::Column::Id.is_in(assignment_ids))
            .exec(db)
            .await
            .map_err(database_error)?;
    }
    if !dosages.is_empty() {
        dosage::Entity::delete_many()
            .filter(dosage::Column::MedicationId.eq(medication.id))
            .exec(db)
            .await
            .map_err(database_error)?;
    }
    medication::Entity::delete_by_id(medication.id)
        .exec(db)
        .await
        .map_err(database_error)?;
    record_cascade_effects(
        db,
        context,
        request_id,
        &CascadeSnapshot {
            medications: vec![medication.clone()],
            dosages,
            schedules,
            assignments,
            memberships: Vec::new(),
        },
    )
    .await?;
    Ok(true)
}

pub(super) async fn delete_dependents(
    db: &DatabaseTransaction,
    record: &stock_location::Model,
) -> Result<Option<CascadeSnapshot>, DbErr> {
    let medications = medication::Entity::find()
        .filter(medication::Column::HouseholdId.eq(record.household_id))
        .filter(medication::Column::LocationId.eq(record.id))
        .order_by_asc(medication::Column::Id)
        .lock_exclusive()
        .all(db)
        .await?;
    let medication_ids: Vec<i64> = medications.iter().map(|medication| medication.id).collect();
    let schedules = if medication_ids.is_empty() {
        Vec::new()
    } else {
        schedule::Entity::find()
            .filter(schedule::Column::MedicationId.is_in(medication_ids.clone()))
            .order_by_asc(schedule::Column::Id)
            .lock_exclusive()
            .all(db)
            .await?
    };
    let assignments = if medication_ids.is_empty() {
        Vec::new()
    } else {
        person_medication::Entity::find()
            .filter(person_medication::Column::MedicationId.is_in(medication_ids.clone()))
            .order_by_asc(person_medication::Column::Id)
            .lock_exclusive()
            .all(db)
            .await?
    };
    let schedule_ids: Vec<i64> = schedules.iter().map(|schedule| schedule.id).collect();
    let assignment_ids: Vec<i64> = assignments.iter().map(|assignment| assignment.id).collect();
    let dosages = if medication_ids.is_empty() {
        Vec::new()
    } else {
        dosage::Entity::find()
            .filter(dosage::Column::MedicationId.is_in(medication_ids.clone()))
            .order_by_asc(dosage::Column::Id)
            .lock_exclusive()
            .all(db)
            .await?
    };
    let memberships = location_membership::Entity::find()
        .filter(location_membership::Column::LocationId.eq(record.id))
        .order_by_asc(location_membership::Column::Id)
        .lock_exclusive()
        .all(db)
        .await?;

    let mut takes =
        Condition::any().add(medication_take::Column::TakenFromLocationId.eq(record.id));
    if !medication_ids.is_empty() {
        takes =
            takes.add(medication_take::Column::TakenFromMedicationId.is_in(medication_ids.clone()));
    }
    if !schedule_ids.is_empty() {
        takes = takes.add(medication_take::Column::ScheduleId.is_in(schedule_ids.clone()));
    }
    if !assignment_ids.is_empty() {
        takes =
            takes.add(medication_take::Column::PersonMedicationId.is_in(assignment_ids.clone()));
    }
    if medication_take::Entity::find()
        .filter(takes)
        .one(db)
        .await?
        .is_some()
    {
        return Ok(None);
    }

    let mut sources = Condition::any();
    if !schedule_ids.is_empty() {
        sources = sources.add(dose_occurrence::Column::ScheduleId.is_in(schedule_ids.clone()));
    }
    if !assignment_ids.is_empty() {
        sources =
            sources.add(dose_occurrence::Column::PersonMedicationId.is_in(assignment_ids.clone()));
    }
    if !schedule_ids.is_empty() || !assignment_ids.is_empty() {
        if dose_occurrence::Entity::find()
            .filter(sources)
            .one(db)
            .await?
            .is_some()
        {
            return Ok(None);
        }
        let mut pauses = Condition::any();
        if !schedule_ids.is_empty() {
            pauses = pauses.add(pause_period::Column::ScheduleId.is_in(schedule_ids.clone()));
        }
        if !assignment_ids.is_empty() {
            pauses =
                pauses.add(pause_period::Column::PersonMedicationId.is_in(assignment_ids.clone()));
        }
        if pause_period::Entity::find()
            .filter(pauses)
            .one(db)
            .await?
            .is_some()
        {
            return Ok(None);
        }
    }

    location_membership::Entity::delete_many()
        .filter(location_membership::Column::LocationId.eq(record.id))
        .exec(db)
        .await?;
    if !medication_ids.is_empty() {
        schedule::Entity::delete_many()
            .filter(schedule::Column::MedicationId.is_in(medication_ids.clone()))
            .exec(db)
            .await?;
        person_medication::Entity::delete_many()
            .filter(person_medication::Column::MedicationId.is_in(medication_ids.clone()))
            .exec(db)
            .await?;
        dosage::Entity::delete_many()
            .filter(dosage::Column::MedicationId.is_in(medication_ids.clone()))
            .exec(db)
            .await?;
        medication::Entity::delete_many()
            .filter(medication::Column::Id.is_in(medication_ids))
            .exec(db)
            .await?;
    }
    stock_location::Entity::delete_by_id(record.id)
        .exec(db)
        .await?;
    Ok(Some(CascadeSnapshot {
        medications,
        dosages,
        schedules,
        assignments,
        memberships,
    }))
}
