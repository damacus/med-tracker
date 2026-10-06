use super::source_stock::source_stock;
use super::*;
use crate::models::{
    authorization,
    entities::{grant, membership, pause_period, person_medication},
};
use chrono::NaiveDate;
use sea_orm::sea_query::{Expr, ExprTrait};
use sea_orm::{Condition, ConnectionTrait, DbBackend, Statement};
use std::collections::{HashMap, HashSet};
fn timestamp(value: chrono::NaiveDateTime) -> String {
    value.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}
fn decimal_string(value: String) -> String {
    medications::decimal_string(value)
}
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
    let subjects = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(context.scope().household_id))
        .filter(person::Column::Id.is_in(person_ids.clone()))
        .all(db)
        .await
        .map_err(database_error)?;
    let people = subjects
        .iter()
        .map(|person| (person.id, person.portable_id.clone()))
        .collect();
    let source_medications: HashMap<i64, medication::Model> = medication::Entity::find()
        .filter(medication::Column::HouseholdId.eq(context.scope().household_id))
        .filter(medication::Column::Id.is_in(medication_ids))
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|medication| (medication.id, medication))
        .collect();
    let permission_grants = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(context.scope().household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(context.membership().id))
        .filter(grant::Column::PersonId.is_in(person_ids))
        .filter(grant::Column::AccessLevel.is_in(["record", "manage"]))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(
            Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(
                    Expr::col((grant::Entity, grant::Column::ExpiresAt))
                        .gt(Expr::cust("timezone('UTC', clock_timestamp())")),
                ),
        )
        .all(db)
        .await
        .map_err(database_error)?;
    let clock = db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT timezone('UTC', clock_timestamp()) AS now",
        ))
        .await?
        .ok_or(OperationError::Unavailable)?;
    let now: chrono::NaiveDateTime = clock.try_get("", "now")?;
    let recordable_people = permission_grants
        .iter()
        .filter(|record| {
            subjects
                .iter()
                .find(|subject| subject.id == record.person_id)
                .is_some_and(|subject| {
                    authorization::person_access(
                        context.membership(),
                        subject,
                        record,
                        PersonAccess::Record,
                        now,
                    )
                })
        })
        .map(|record| record.person_id)
        .collect();
    let manageable_people = permission_grants
        .iter()
        .filter(|record| {
            subjects
                .iter()
                .find(|subject| subject.id == record.person_id)
                .is_some_and(|subject| {
                    authorization::person_access(
                        context.membership(),
                        subject,
                        record,
                        PersonAccess::Manage,
                        now,
                    )
                })
        })
        .map(|record| record.person_id)
        .collect();
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
    let source_column = match kind {
        SourceKind::Schedule => pause_period::Column::ScheduleId,
        SourceKind::Assignment => pause_period::Column::PersonMedicationId,
    };
    let pauses = pause_period::Entity::find()
        .filter(pause_period::Column::HouseholdId.eq(context.scope().household_id))
        .filter(source_column.is_in(source_ids))
        .filter(pause_period::Column::EndedAt.is_null())
        .all(db)
        .await
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
            .filter(membership::Column::HouseholdId.eq(context.scope().household_id))
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
            .filter(person::Column::HouseholdId.eq(context.scope().household_id))
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
    let reference_date = Utc::now()
        .with_timezone(&crate::models::care::doses::app_zone())
        .date_naive();
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
