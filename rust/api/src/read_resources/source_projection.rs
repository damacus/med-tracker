use super::people::today;
use super::response::timestamp;
use super::source_stock::source_stock;
use crate::entities::{medication, membership};
use crate::read_entities::{pause_period, person, person_medication, schedule};
use crate::{database_error, decimal_string, ApiError, AuthContext};
use chrono::{NaiveDate, Utc};
use sea_orm::{ColumnTrait, Condition, DatabaseTransaction, EntityTrait, QueryFilter};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

pub(super) struct SourceRef {
    pub(super) id: i64,
    pub(super) person_id: i64,
    pub(super) medication_id: i64,
    portable_id: String,
}

struct SourceContext {
    people: HashMap<i64, String>,
    medications: HashMap<i64, String>,
    manageable_people: HashSet<i64>,
    recordable_people: HashSet<i64>,
    eligible_stock: HashMap<i64, Vec<i64>>,
    pauses: HashMap<i64, Value>,
}

enum SourceKind {
    Schedule,
    Assignment,
}

async fn source_context(
    db: &DatabaseTransaction,
    context: &AuthContext,
    sources: &[SourceRef],
    kind: SourceKind,
) -> Result<SourceContext, ApiError> {
    if sources.is_empty() {
        return Ok(SourceContext {
            people: HashMap::new(),
            medications: HashMap::new(),
            manageable_people: HashSet::new(),
            recordable_people: HashSet::new(),
            eligible_stock: HashMap::new(),
            pauses: HashMap::new(),
        });
    }
    let person_ids: Vec<i64> = sources.iter().map(|source| source.person_id).collect();
    let medication_ids: Vec<i64> = sources.iter().map(|source| source.medication_id).collect();
    let people = person::Entity::find()
        .filter(person::Column::Id.is_in(person_ids.clone()))
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|person| (person.id, person.portable_id))
        .collect();
    let source_medications: HashMap<i64, medication::Model> = medication::Entity::find()
        .filter(medication::Column::Id.is_in(medication_ids))
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|medication| (medication.id, medication))
        .collect();
    let permission_grants = crate::entities::grant::Entity::find()
        .filter(crate::entities::grant::Column::HouseholdId.eq(context.membership.household_id))
        .filter(crate::entities::grant::Column::HouseholdMembershipId.eq(context.membership.id))
        .filter(crate::entities::grant::Column::PersonId.is_in(person_ids))
        .filter(crate::entities::grant::Column::AccessLevel.is_in(["record", "manage"]))
        .filter(crate::entities::grant::Column::RevokedAt.is_null())
        .filter(
            Condition::any()
                .add(crate::entities::grant::Column::ExpiresAt.is_null())
                .add(crate::entities::grant::Column::ExpiresAt.gt(Utc::now().naive_utc())),
        )
        .all(db)
        .await
        .map_err(database_error)?;
    let (recordable_people, manageable_people) = source_permissions(&permission_grants);
    let eligible_stock = source_stock(
        db,
        context,
        sources,
        &source_medications,
        &recordable_people,
    )
    .await?;
    let medications = source_medications
        .into_iter()
        .map(|(id, medication)| (id, medication.portable_id))
        .collect();
    let source_ids: Vec<i64> = sources.iter().map(|source| source.id).collect();
    let pauses = match kind {
        SourceKind::Schedule => {
            pause_period::Entity::find()
                .filter(pause_period::Column::ScheduleId.is_in(source_ids))
                .filter(pause_period::Column::EndedAt.is_null())
                .all(db)
                .await
        }
        SourceKind::Assignment => {
            pause_period::Entity::find()
                .filter(pause_period::Column::PersonMedicationId.is_in(source_ids))
                .filter(pause_period::Column::EndedAt.is_null())
                .all(db)
                .await
        }
    }
    .map_err(database_error)?;
    let actor_ids: Vec<i64> = pauses
        .iter()
        .flat_map(|pause| {
            [
                pause.recorded_by_membership_id,
                pause.resumed_by_membership_id,
            ]
        })
        .flatten()
        .collect();
    let actor_memberships = if actor_ids.is_empty() {
        Vec::new()
    } else {
        membership::Entity::find()
            .filter(membership::Column::Id.is_in(actor_ids))
            .all(db)
            .await
            .map_err(database_error)?
    };
    let actor_person_ids: Vec<i64> = actor_memberships
        .iter()
        .filter_map(|membership| membership.person_id)
        .collect();
    let actor_people: HashMap<i64, String> = if actor_person_ids.is_empty() {
        HashMap::new()
    } else {
        person::Entity::find()
            .filter(person::Column::Id.is_in(actor_person_ids))
            .all(db)
            .await
            .map_err(database_error)?
            .into_iter()
            .map(|person| (person.id, person.name))
            .collect()
    };
    let actor_names: HashMap<i64, String> = actor_memberships
        .into_iter()
        .filter_map(|membership| {
            membership
                .person_id
                .and_then(|person_id| actor_people.get(&person_id).cloned())
                .map(|name| (membership.id, name))
        })
        .collect();
    let source_portable_ids: HashMap<i64, &str> = sources
        .iter()
        .map(|source| (source.id, source.portable_id.as_str()))
        .collect();
    let pauses = pauses
        .into_iter()
        .filter_map(|pause| {
            let source_id = match kind {
                SourceKind::Schedule => pause.schedule_id?,
                SourceKind::Assignment => pause.person_medication_id?,
            };
            let source_portable_id = source_portable_ids.get(&source_id)?;
            let source_type = match kind {
                SourceKind::Schedule => "schedule",
                SourceKind::Assignment => "person_medication",
            };
            Some((
                source_id,
                json!({
                    "id": pause.portable_id,
                    "portable_id": pause.portable_id,
                    "source_type": source_type,
                    "source_id": source_portable_id,
                    "reason": pause.reason,
                    "note": pause.note,
                    "legacy_context": pause.legacy_context,
                    "recorded_by_membership_id": pause.recorded_by_membership_id.map(|id| id.to_string()),
                    "resumed_by_membership_id": pause.resumed_by_membership_id.map(|id| id.to_string()),
                    "recorded_by_name": pause.recorded_by_membership_id.and_then(|id| actor_names.get(&id)),
                    "resumed_by_name": pause.resumed_by_membership_id.and_then(|id| actor_names.get(&id)),
                    "started_at": pause.started_at.map(timestamp),
                    "ended_at": pause.ended_at.map(timestamp),
                    "created_at": timestamp(pause.created_at),
                    "updated_at": timestamp(pause.updated_at)
                }),
            ))
        })
        .collect();
    Ok(SourceContext {
        people,
        medications,
        manageable_people,
        recordable_people,
        eligible_stock,
        pauses,
    })
}

fn source_permissions(grants: &[crate::entities::grant::Model]) -> (HashSet<i64>, HashSet<i64>) {
    let recordable = grants
        .iter()
        .filter(|grant| matches!(grant.access_level.as_str(), "record" | "manage"))
        .map(|grant| grant.person_id)
        .collect();
    let manageable = grants
        .iter()
        .filter(|grant| grant.access_level == "manage")
        .map(|grant| grant.person_id)
        .collect();
    (recordable, manageable)
}

fn dose_cycle(value: Option<i32>) -> Option<&'static str> {
    match value {
        Some(0) => Some("daily"),
        Some(1) => Some("weekly"),
        Some(2) => Some("monthly"),
        _ => None,
    }
}

fn schedule_type(value: i32) -> &'static str {
    match value {
        1 => "multiple_daily",
        2 => "weekly",
        3 => "specific_dates",
        4 => "prn",
        5 => "tapering",
        6 => "every_other_day",
        _ => "daily",
    }
}

pub(crate) async fn serialize_schedules(
    db: &DatabaseTransaction,
    context: &AuthContext,
    records: Vec<schedule::Model>,
) -> Result<Vec<Value>, ApiError> {
    let reference_date = today();
    let sources: Vec<SourceRef> = records
        .iter()
        .map(|record| SourceRef {
            id: record.id,
            person_id: record.person_id,
            medication_id: record.medication_id,
            portable_id: record.portable_id.clone(),
        })
        .collect();
    let associations = source_context(db, context, &sources, SourceKind::Schedule).await?;
    Ok(records
        .into_iter()
        .map(|record| schedule_row(record, &associations, reference_date))
        .collect())
}

fn schedule_row(
    record: schedule::Model,
    associations: &SourceContext,
    reference_date: NaiveDate,
) -> Value {
    let active = record.active
        && record
            .start_date
            .is_some_and(|start| start <= reference_date)
        && record.end_date.is_some_and(|end| end >= reference_date);
    json!({
        "id": record.id,
        "portable_id": record.portable_id,
        "person_id": record.person_id,
        "person_portable_id": associations.people.get(&record.person_id),
        "medication_id": record.medication_id,
        "medication_portable_id": associations.medications.get(&record.medication_id),
        "dose_amount": record.dose_amount.map(|value| decimal_string(value.to_string())),
        "dose_unit": record.dose_unit,
        "frequency": record.frequency,
        "dose_cycle": dose_cycle(record.dose_cycle),
        "start_date": record.start_date.map(|date| date.to_string()),
        "end_date": record.end_date.map(|date| date.to_string()),
        "active": active,
        "paused": !record.active,
        "can_manage": associations.manageable_people.contains(&record.person_id),
        "can_record": associations.recordable_people.contains(&record.person_id),
        "eligible_stock_medication_ids": associations.eligible_stock.get(&record.id).cloned().unwrap_or_default(),
        "notes": record.notes,
        "updated_at": timestamp(record.updated_at),
        "schedule_type": schedule_type(record.schedule_type),
        "schedule_config": record.schedule_config,
        "max_daily_doses": record.max_daily_doses,
        "min_hours_between_doses": record.min_hours_between_doses.map(|value| decimal_string(value.to_string())),
        "current_pause_period": associations.pauses.get(&record.id)
    })
}

pub(crate) async fn serialize_assignments(
    db: &DatabaseTransaction,
    context: &AuthContext,
    records: Vec<person_medication::Model>,
) -> Result<Vec<Value>, ApiError> {
    let sources: Vec<SourceRef> = records
        .iter()
        .map(|record| SourceRef {
            id: record.id,
            person_id: record.person_id,
            medication_id: record.medication_id,
            portable_id: record.portable_id.clone(),
        })
        .collect();
    let associations = source_context(db, context, &sources, SourceKind::Assignment).await?;
    Ok(records
        .into_iter()
        .map(|record| assignment_row(record, &associations))
        .collect())
}

fn assignment_row(record: person_medication::Model, associations: &SourceContext) -> Value {
    json!({
        "id": record.id,
        "portable_id": record.portable_id,
        "person_id": record.person_id,
        "person_portable_id": associations.people.get(&record.person_id),
        "medication_id": record.medication_id,
        "medication_portable_id": associations.medications.get(&record.medication_id),
        "dose_amount": record.dose_amount.map(|value| decimal_string(value.to_string())),
        "dose_unit": record.dose_unit,
        "active": record.active,
        "paused": !record.active,
        "can_manage": associations.manageable_people.contains(&record.person_id),
        "can_record": associations.recordable_people.contains(&record.person_id),
        "eligible_stock_medication_ids": associations.eligible_stock.get(&record.id).cloned().unwrap_or_default(),
        "dose_cycle": dose_cycle(record.dose_cycle),
        "administration_kind": if record.administration_kind == 0 { "routine" } else { "as_needed" },
        "notes": record.notes,
        "position": record.position,
        "updated_at": timestamp(record.updated_at),
        "max_daily_doses": record.max_daily_doses,
        "min_hours_between_doses": record.min_hours_between_doses.map(|value| decimal_string(value.to_string())),
        "current_pause_period": associations.pauses.get(&record.id)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::prelude::Decimal;

    fn assignment() -> person_medication::Model {
        person_medication::Model {
            id: 1,
            household_id: 2,
            portable_id: "assignment".into(),
            person_id: 3,
            medication_id: 10,
            source_dosage_option_id: None,
            dose_amount: Some(Decimal::ONE),
            dose_unit: Some("tablet".into()),
            active: true,
            notes: None,
            updated_at: Utc::now().naive_utc(),
            dose_cycle: Some(0),
            administration_kind: 0,
            position: 0,
            max_daily_doses: None,
            min_hours_between_doses: None,
            retired_at: None,
        }
    }

    fn scheduled() -> schedule::Model {
        schedule::Model {
            id: 1,
            household_id: 2,
            portable_id: "schedule".into(),
            person_id: 3,
            medication_id: 10,
            source_dosage_option_id: None,
            dose_amount: Some(Decimal::ONE),
            dose_unit: Some("tablet".into()),
            frequency: Some("daily".into()),
            dose_cycle: Some(0),
            start_date: Some(today()),
            end_date: Some(today()),
            active: true,
            notes: None,
            updated_at: Utc::now().naive_utc(),
            schedule_type: 0,
            schedule_config: json!({}),
            max_daily_doses: None,
            min_hours_between_doses: None,
            retired_at: None,
        }
    }

    fn associations() -> SourceContext {
        SourceContext {
            people: HashMap::from([(3, "person".into())]),
            medications: HashMap::from([(10, "medication".into())]),
            manageable_people: HashSet::from([3]),
            pauses: HashMap::new(),
            recordable_people: HashSet::from([3]),
            eligible_stock: HashMap::from([(1, vec![10])]),
        }
    }

    #[test]
    fn assignment_projection_supplies_dose_recording_permission() {
        let row = assignment_row(assignment(), &associations());
        assert_eq!(row["can_record"], true);
    }

    #[test]
    fn schedule_projection_supplies_dose_recording_permission() {
        let row = schedule_row(scheduled(), &associations(), today());
        assert_eq!(row["can_record"], true);
    }

    #[test]
    fn source_projections_supply_eligible_stock_inventory() {
        let associations = associations();
        for row in [
            assignment_row(assignment(), &associations),
            schedule_row(scheduled(), &associations, today()),
        ] {
            assert_eq!(row["eligible_stock_medication_ids"], json!([10]));
        }
    }

    fn grant(person_id: i64, access_level: &str) -> crate::entities::grant::Model {
        crate::entities::grant::Model {
            id: person_id,
            household_id: 2,
            household_membership_id: 4,
            person_id,
            access_level: access_level.into(),
            expires_at: None,
            revoked_at: None,
            relationship_type: "carer".into(),
            granted_by_membership_id: None,
            carer_relationship_id: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        }
    }

    #[test]
    fn source_permissions_keep_record_only_manage_and_view_grants_distinct() {
        let (recordable, manageable) =
            source_permissions(&[grant(3, "record"), grant(5, "manage"), grant(6, "view")]);
        assert_eq!(recordable, HashSet::from([3, 5]));
        assert_eq!(manageable, HashSet::from([5]));
        for (person_id, can_record, can_manage) in [
            (3, true, false),
            (5, true, true),
            (6, false, false),
            (7, false, false),
        ] {
            let mut associations = associations();
            associations.recordable_people = recordable.clone();
            associations.manageable_people = manageable.clone();
            let mut assignment = assignment();
            assignment.person_id = person_id;
            let mut schedule = scheduled();
            schedule.person_id = person_id;
            for row in [
                assignment_row(assignment, &associations),
                schedule_row(schedule, &associations, today()),
            ] {
                assert_eq!(row["can_record"], can_record);
                assert_eq!(row["can_manage"], can_manage);
            }
        }
    }

    #[test]
    fn missing_stock_projection_serializes_an_empty_inventory_list() {
        let mut associations = associations();
        associations.eligible_stock.clear();
        for row in [
            assignment_row(assignment(), &associations),
            schedule_row(scheduled(), &associations, today()),
        ] {
            assert_eq!(row["eligible_stock_medication_ids"], json!([]));
        }
    }
}
