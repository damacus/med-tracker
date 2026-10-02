use crate::entities::{
    dosage, dose_occurrence, grant, health_event, health_event_medication, medication,
    medication_take, membership, pause_period, person, person_medication, schedule,
};
use crate::read_entities::{
    location_membership, notification_preference, person as read_person,
    person_medication as read_assignment, schedule as read_schedule, stock_location,
};
use crate::read_resources::{
    location_value, serialize_assignments, serialize_people, serialize_schedules,
};
use crate::{database_error, representation_etag, ApiError, AuthContext};
use chrono::{NaiveDateTime, Utc};
use sea_orm::{ColumnTrait, Condition, DatabaseTransaction, EntityTrait, QueryFilter, QueryOrder};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

pub(super) enum Scope {
    Manage,
    View,
}

fn timestamp(value: NaiveDateTime) -> String {
    value
        .and_utc()
        .to_rfc3339_opts(chrono::SecondsFormat::Micros, true)
}

fn identity(portable_id: &str, updated_at: NaiveDateTime) -> Value {
    let mut value = json!({"portable_id": portable_id, "updated_at": timestamp(updated_at)});
    value["etag"] = json!(representation_etag(&value));
    value
}

fn merge(mut identity: Value, fields: Value) -> Value {
    if let (Some(target), Some(source)) = (identity.as_object_mut(), fields.as_object()) {
        target.extend(source.clone());
    }
    identity
}

pub(super) fn occurrence_value(
    row: &dose_occurrence::Model,
    source_type: &str,
    source_portable_id: Option<&str>,
    take_portable_id: Option<&str>,
) -> Value {
    merge(
        identity(&row.portable_id, row.updated_at),
        json!({
            "source_type": source_type, "source_portable_id": source_portable_id,
            "window_starts_on": row.window_starts_on.to_string(),
            "window_ends_on": row.window_ends_on.map(|date| date.to_string()).unwrap_or_else(|| row.window_starts_on.to_string()),
            "position": row.position, "scheduled_at": row.scheduled_at.map(timestamp),
            "outcome": row.outcome, "reason": row.reason, "note": row.note,
            "resolved_at": row.resolved_at.map(timestamp),
            "medication_take_portable_id": take_portable_id,
        }),
    )
}

fn person_type(value: i32) -> &'static str {
    match value {
        1 => "minor",
        2 => "dependent_adult",
        _ => "adult",
    }
}

fn cycle(value: i32) -> &'static str {
    match value {
        1 => "weekly",
        2 => "monthly",
        _ => "daily",
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

fn event_kind(value: i32) -> &'static str {
    if value == 1 {
        "suspected_side_effect"
    } else {
        "illness"
    }
}

fn severity(value: Option<i32>) -> Option<&'static str> {
    value.map(|value| match value {
        1 => "moderate",
        2 => "severe",
        _ => "mild",
    })
}

pub(super) async fn visible_people(
    db: &DatabaseTransaction,
    context: &AuthContext,
    scope: Scope,
) -> Result<Vec<person::Model>, ApiError> {
    if context.membership.role == "owner" || context.membership.role == "administrator" {
        return person::Entity::find()
            .filter(person::Column::HouseholdId.eq(context.membership.household_id))
            .order_by_asc(person::Column::Id)
            .all(db)
            .await
            .map_err(database_error);
    }
    let now = Utc::now().naive_utc();
    let grants = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(context.membership.household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(context.membership.id))
        .filter(grant::Column::RevokedAt.is_null())
        .all(db)
        .await
        .map_err(database_error)?;
    let ids: HashSet<i64> = grants
        .into_iter()
        .filter(|grant| grant.expires_at.is_none_or(|expires| expires > now))
        .filter(|grant| match scope {
            Scope::Manage => grant.access_level == "manage",
            Scope::View => matches!(grant.access_level.as_str(), "view" | "record" | "manage"),
        })
        .map(|grant| grant.person_id)
        .collect();
    person::Entity::find()
        .filter(person::Column::HouseholdId.eq(context.membership.household_id))
        .filter(person::Column::Id.is_in(ids.into_iter().collect::<Vec<_>>()))
        .order_by_asc(person::Column::Id)
        .all(db)
        .await
        .map_err(database_error)
}

pub(super) async fn payload(
    db: &DatabaseTransaction,
    context: &AuthContext,
    scope: Scope,
    format: &str,
    include_health_events: bool,
) -> Result<Value, ApiError> {
    let household_id = context.membership.household_id;
    let people = visible_people(db, context, scope).await?;
    let person_ids: Vec<i64> = people.iter().map(|person| person.id).collect();
    let schedules = schedule::Entity::find()
        .filter(schedule::Column::HouseholdId.eq(household_id))
        .filter(schedule::Column::PersonId.is_in(person_ids.clone()))
        .order_by_asc(schedule::Column::Id)
        .all(db)
        .await
        .map_err(database_error)?;
    let assignments = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(household_id))
        .filter(person_medication::Column::PersonId.is_in(person_ids.clone()))
        .order_by_asc(person_medication::Column::Id)
        .all(db)
        .await
        .map_err(database_error)?;
    let schedule_ids: HashSet<i64> = schedules.iter().map(|row| row.id).collect();
    let assignment_ids: HashSet<i64> = assignments.iter().map(|row| row.id).collect();
    let sources = Condition::any()
        .add(
            medication_take::Column::ScheduleId
                .is_in(schedule_ids.iter().copied().collect::<Vec<_>>()),
        )
        .add(
            medication_take::Column::PersonMedicationId
                .is_in(assignment_ids.iter().copied().collect::<Vec<_>>()),
        );
    let takes = medication_take::Entity::find()
        .filter(medication_take::Column::HouseholdId.eq(household_id))
        .filter(sources)
        .order_by_asc(medication_take::Column::Id)
        .all(db)
        .await
        .map_err(database_error)?;
    let health = if include_health_events {
        health_event::Entity::find()
            .filter(health_event::Column::HouseholdId.eq(household_id))
            .filter(health_event::Column::PersonId.is_in(person_ids.clone()))
            .order_by_asc(health_event::Column::Id)
            .all(db)
            .await
            .map_err(database_error)?
    } else {
        Vec::new()
    };
    let health_ids: HashSet<i64> = health.iter().map(|row| row.id).collect();
    let health_links = if include_health_events {
        health_event_medication::Entity::find()
            .filter(health_event_medication::Column::HouseholdId.eq(household_id))
            .filter(
                health_event_medication::Column::HealthEventId
                    .is_in(health_ids.iter().copied().collect::<Vec<_>>()),
            )
            .all(db)
            .await
            .map_err(database_error)?
    } else {
        Vec::new()
    };
    let mut medication_ids: HashSet<i64> = schedules
        .iter()
        .map(|row| row.medication_id)
        .chain(assignments.iter().map(|row| row.medication_id))
        .collect();
    if include_health_events {
        medication_ids.extend(health_links.iter().filter_map(|row| row.medication_id));
    }
    let medications = medication::Entity::find()
        .filter(medication::Column::HouseholdId.eq(household_id))
        .filter(medication::Column::Id.is_in(medication_ids.iter().copied().collect::<Vec<_>>()))
        .order_by_asc(medication::Column::Id)
        .all(db)
        .await
        .map_err(database_error)?;
    let dosages = dosage::Entity::find()
        .filter(dosage::Column::HouseholdId.eq(household_id))
        .filter(
            dosage::Column::MedicationId.is_in(medication_ids.iter().copied().collect::<Vec<_>>()),
        )
        .order_by_asc(dosage::Column::Id)
        .all(db)
        .await
        .map_err(database_error)?;
    let memberships = location_membership::Entity::find()
        .filter(location_membership::Column::HouseholdId.eq(household_id))
        .filter(location_membership::Column::PersonId.is_in(person_ids.clone()))
        .all(db)
        .await
        .map_err(database_error)?;
    let mut location_ids: HashSet<i64> = memberships
        .iter()
        .map(|row| row.location_id)
        .chain(medications.iter().map(|row| row.location_id))
        .chain(takes.iter().filter_map(|row| row.taken_from_location_id))
        .collect();
    location_ids.retain(|id| *id > 0);
    let locations = stock_location::Entity::find()
        .filter(stock_location::Column::HouseholdId.eq(household_id))
        .filter(stock_location::Column::Id.is_in(location_ids.iter().copied().collect::<Vec<_>>()))
        .order_by_asc(stock_location::Column::Id)
        .all(db)
        .await
        .map_err(database_error)?;
    let preferences = notification_preference::Entity::find()
        .filter(notification_preference::Column::HouseholdId.eq(household_id))
        .filter(notification_preference::Column::PersonId.is_in(person_ids.clone()))
        .order_by_asc(notification_preference::Column::Id)
        .all(db)
        .await
        .map_err(database_error)?;
    let v2 = format == "medtracker.portable.v2";
    let periods = if v2 {
        pause_period::Entity::find()
            .filter(pause_period::Column::HouseholdId.eq(household_id))
            .filter(
                Condition::any()
                    .add(
                        pause_period::Column::ScheduleId
                            .is_in(schedule_ids.iter().copied().collect::<Vec<_>>()),
                    )
                    .add(
                        pause_period::Column::PersonMedicationId
                            .is_in(assignment_ids.iter().copied().collect::<Vec<_>>()),
                    ),
            )
            .order_by_asc(pause_period::Column::Id)
            .all(db)
            .await
            .map_err(database_error)?
    } else {
        Vec::new()
    };
    let occurrences = if v2 {
        dose_occurrence::Entity::find()
            .filter(dose_occurrence::Column::HouseholdId.eq(household_id))
            .filter(
                Condition::any()
                    .add(
                        dose_occurrence::Column::ScheduleId
                            .is_in(schedule_ids.iter().copied().collect::<Vec<_>>()),
                    )
                    .add(
                        dose_occurrence::Column::PersonMedicationId
                            .is_in(assignment_ids.iter().copied().collect::<Vec<_>>()),
                    ),
            )
            .order_by_asc(dose_occurrence::Column::Id)
            .all(db)
            .await
            .map_err(database_error)?
    } else {
        Vec::new()
    };
    let actor_membership_ids: HashSet<i64> = periods
        .iter()
        .flat_map(|row| [row.recorded_by_membership_id, row.resumed_by_membership_id])
        .flatten()
        .collect();
    let actor_memberships = membership::Entity::find()
        .filter(membership::Column::HouseholdId.eq(household_id))
        .filter(membership::Column::Id.is_in(actor_membership_ids.into_iter().collect::<Vec<_>>()))
        .all(db)
        .await
        .map_err(database_error)?;
    let actor_person_ids: HashSet<i64> = actor_memberships
        .iter()
        .filter_map(|row| row.person_id)
        .collect();
    let actor_people = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(household_id))
        .filter(person::Column::Id.is_in(actor_person_ids.into_iter().collect::<Vec<_>>()))
        .all(db)
        .await
        .map_err(database_error)?;
    let actor_person_ids_by_id: HashMap<i64, &str> = actor_people
        .iter()
        .map(|row| (row.id, row.portable_id.as_str()))
        .collect();
    let actor_by_membership: HashMap<i64, &str> = actor_memberships
        .iter()
        .filter_map(|row| {
            Some((
                row.id,
                actor_person_ids_by_id.get(&row.person_id?).copied()?,
            ))
        })
        .collect();

    let person_ids_by_id: HashMap<i64, &str> = people
        .iter()
        .map(|row| (row.id, row.portable_id.as_str()))
        .collect();
    let medication_ids_by_id: HashMap<i64, &str> = medications
        .iter()
        .map(|row| (row.id, row.portable_id.as_str()))
        .collect();
    let location_ids_by_id: HashMap<i64, &str> = locations
        .iter()
        .map(|row| (row.id, row.portable_id.as_str()))
        .collect();
    let dosage_ids_by_id: HashMap<i64, &str> = dosages
        .iter()
        .map(|row| (row.id, row.portable_id.as_str()))
        .collect();
    let source_ids: HashMap<i64, &str> = schedules
        .iter()
        .map(|row| (row.id, row.portable_id.as_str()))
        .collect();
    let assignment_source_ids: HashMap<i64, &str> = assignments
        .iter()
        .map(|row| (row.id, row.portable_id.as_str()))
        .collect();
    let take_ids_by_id: HashMap<i64, &str> = takes
        .iter()
        .map(|row| (row.id, row.portable_id.as_str()))
        .collect();
    let preference_ids: HashMap<i64, &str> = preferences
        .iter()
        .map(|row| (row.person_id, row.portable_id.as_str()))
        .collect();
    let mut person_locations: HashMap<i64, Vec<&str>> = HashMap::new();
    for row in &memberships {
        if let Some(portable_id) = location_ids_by_id.get(&row.location_id) {
            person_locations
                .entry(row.person_id)
                .or_default()
                .push(portable_id);
        }
    }
    let mut health_medications: HashMap<i64, Vec<&str>> = HashMap::new();
    for row in &health_links {
        if let Some(portable_id) = row
            .medication_id
            .and_then(|id| medication_ids_by_id.get(&id))
        {
            health_medications
                .entry(row.health_event_id)
                .or_default()
                .push(portable_id);
        }
    }

    let people_values: Vec<Value> = people.iter().map(|row| merge(identity(&row.portable_id, row.updated_at), json!({
        "name": row.name, "email": row.email, "date_of_birth": row.date_of_birth.map(|date| date.to_string()),
        "person_type": person_type(row.person_type), "has_capacity": row.has_capacity,
        "location_portable_ids": person_locations.get(&row.id).cloned().unwrap_or_default(),
        "notification_preference_portable_id": preference_ids.get(&row.id),
    }))).collect();
    let location_values: Vec<Value> = locations
        .iter()
        .map(|row| {
            merge(
                identity(&row.portable_id, row.updated_at),
                json!({
                    "name": row.name, "description": row.description,
                }),
            )
        })
        .collect();
    let medication_values: Vec<Value> = medications.iter().map(|row| merge(identity(&row.portable_id, row.updated_at), json!({
        "location_portable_id": location_ids_by_id.get(&row.location_id), "name": row.name,
        "friendly_name": row.friendly_name, "category": row.category, "description": row.description,
        "dose_amount": row.dose_amount, "dose_unit": row.dose_unit,
        "default_schedule_type": schedule_type(row.default_schedule_type),
        "current_supply": row.current_supply.map(|value| crate::decimal_string(value.to_string())),
        "reorder_threshold": crate::decimal_string(row.reorder_threshold.to_string()),
        "barcode": row.barcode, "dmd_code": row.dmd_code, "dmd_system": row.dmd_system,
        "dmd_concept_class": row.dmd_concept_class,
    }))).collect();
    let dosage_values: Vec<Value> = dosages.iter().map(|row| merge(identity(&row.portable_id, row.updated_at), json!({
        "medication_portable_id": medication_ids_by_id.get(&row.medication_id),
        "amount": crate::decimal_string(row.amount.to_string()),
        "unit": row.unit, "frequency": row.frequency, "description": row.description,
        "default_for_adults": row.default_for_adults, "default_for_children": row.default_for_children,
        "default_max_daily_doses": row.default_max_daily_doses,
        "default_min_hours_between_doses": crate::decimal_string(row.default_min_hours_between_doses.to_string()),
        "default_dose_cycle": cycle(row.default_dose_cycle),
        "current_supply": row.current_supply.map(|value| crate::decimal_string(value.to_string())),
        "reorder_threshold": row.reorder_threshold.map(|value| crate::decimal_string(value.to_string())),
    }))).collect();
    let schedule_values: Vec<Value> = schedules.iter().map(|row| merge(identity(&row.portable_id, row.updated_at), json!({
        "source_dosage_option_portable_id": row.source_dosage_option_id.and_then(|id| dosage_ids_by_id.get(&id)),
        "retired_at": row.retired_at.map(timestamp), "person_portable_id": person_ids_by_id.get(&row.person_id),
        "medication_portable_id": medication_ids_by_id.get(&row.medication_id),
        "dose_amount": row.dose_amount.map(|value| crate::decimal_string(value.to_string())),
        "dose_unit": row.dose_unit, "frequency": row.frequency, "dose_cycle": row.dose_cycle.map(cycle),
        "max_daily_doses": row.max_daily_doses, "min_hours_between_doses": row.min_hours_between_doses,
        "schedule_type": schedule_type(row.schedule_type), "schedule_config": row.schedule_config,
        "start_date": row.start_date.map(|date| date.to_string()), "end_date": row.end_date.map(|date| date.to_string()),
        "active": row.active, "notes": row.notes,
    }))).collect();
    let assignment_values: Vec<Value> = assignments.iter().map(|row| merge(identity(&row.portable_id, row.updated_at), json!({
        "source_dosage_option_portable_id": row.source_dosage_option_id.and_then(|id| dosage_ids_by_id.get(&id)),
        "retired_at": row.retired_at.map(timestamp), "person_portable_id": person_ids_by_id.get(&row.person_id),
        "medication_portable_id": medication_ids_by_id.get(&row.medication_id),
        "dose_amount": row.dose_amount.map(|value| crate::decimal_string(value.to_string())),
        "dose_unit": row.dose_unit, "dose_cycle": row.dose_cycle.map(cycle),
        "max_daily_doses": row.max_daily_doses, "min_hours_between_doses": row.min_hours_between_doses,
        "administration_kind": if row.administration_kind == 1 { "as_needed" } else { "routine" },
        "active": row.active, "notes": row.notes, "position": row.position,
    }))).collect();
    let take_values: Vec<Value> = takes.iter().map(|row| {
        let (source_type, source_id) = if let Some(id) = row.schedule_id {
            ("schedule", source_ids.get(&id))
        } else { ("person_medication", row.person_medication_id.and_then(|id| assignment_source_ids.get(&id))) };
        merge(identity(&row.portable_id, row.updated_at), json!({
            "client_uuid": row.client_uuid, "source_type": source_type, "source_portable_id": source_id,
            "taken_at": row.taken_at.map(timestamp),
            "dose_amount": row.dose_amount.map(|value| crate::decimal_string(value.to_string())),
            "dose_unit": row.dose_unit,
            "taken_from_medication_portable_id": row.taken_from_medication_id.and_then(|id| medication_ids_by_id.get(&id)),
            "taken_from_location_portable_id": row.taken_from_location_id.and_then(|id| location_ids_by_id.get(&id)),
        }))
    }).collect();
    let preference_values: Vec<Value> = preferences.iter().map(|row| merge(identity(&row.portable_id, row.updated_at), json!({
        "person_portable_id": person_ids_by_id.get(&row.person_id), "enabled": row.enabled,
        "dose_due_enabled": row.dose_due_enabled, "missed_dose_enabled": row.missed_dose_enabled,
        "low_stock_enabled": row.low_stock_enabled, "private_text_enabled": row.private_text_enabled,
        "morning_time": row.morning_time.map(|time| time.format("%H:%M:%S").to_string()),
        "afternoon_time": row.afternoon_time.map(|time| time.format("%H:%M:%S").to_string()),
        "evening_time": row.evening_time.map(|time| time.format("%H:%M:%S").to_string()),
        "night_time": row.night_time.map(|time| time.format("%H:%M:%S").to_string()),
    }))).collect();
    let mut records = json!({
        "people": people_values, "locations": location_values, "medications": medication_values,
        "dosage_options": dosage_values, "schedules": schedule_values,
        "person_medications": assignment_values, "medication_takes": take_values,
        "notification_preferences": preference_values,
    });
    if include_health_events {
        records["health_events"] = json!(health.iter().map(|row| merge(identity(&row.portable_id, row.updated_at), json!({
            "person_portable_id": person_ids_by_id.get(&row.person_id), "event_kind": event_kind(row.event_kind),
            "severity": severity(row.severity), "title": row.title, "notes": row.notes,
            "started_on": row.started_on.to_string(), "ended_on": row.ended_on.map(|date| date.to_string()),
            "medication_portable_ids": health_medications.get(&row.id).cloned().unwrap_or_default(),
        }))).collect::<Vec<_>>());
    }
    if v2 {
        records["medication_pause_periods"] = json!(periods.iter().map(|row| {
            let (source_type, source_id) = if let Some(id) = row.schedule_id {
                ("schedule", source_ids.get(&id))
            } else { ("person_medication", row.person_medication_id.and_then(|id| assignment_source_ids.get(&id))) };
            merge(identity(&row.portable_id, row.updated_at), json!({
                "source_type": source_type, "source_portable_id": source_id,
                "reason": row.reason, "note": row.note, "started_at": row.started_at.map(timestamp),
                "ended_at": row.ended_at.map(timestamp), "created_at": timestamp(row.created_at),
                "legacy_context": row.legacy_context, "imported_context": row.imported_context,
                "recorded_by_person_portable_id": row.recorded_by_membership_id
                    .and_then(|id| actor_by_membership.get(&id).copied())
                    .map(Value::from)
                    .or_else(|| row.imported_actor_references.get("recorded_by_person_portable_id").cloned()),
                "resumed_by_person_portable_id": row.resumed_by_membership_id
                    .and_then(|id| actor_by_membership.get(&id).copied())
                    .map(Value::from)
                    .or_else(|| row.imported_actor_references.get("resumed_by_person_portable_id").cloned()),
            }))
        }).collect::<Vec<_>>());
        records["dose_occurrences"] = json!(occurrences
            .iter()
            .map(|row| {
                let (source_type, source_id) = if let Some(id) = row.schedule_id {
                    ("schedule", source_ids.get(&id).copied())
                } else {
                    (
                        "person_medication",
                        row.person_medication_id
                            .and_then(|id| assignment_source_ids.get(&id).copied()),
                    )
                };
                occurrence_value(
                    row,
                    source_type,
                    source_id,
                    row.medication_take_id
                        .and_then(|id| take_ids_by_id.get(&id).copied()),
                )
            })
            .collect::<Vec<_>>());
    }
    let mut etags = HashMap::new();
    for row in &locations {
        let dto = location_value(row.clone());
        etags.insert(
            row.portable_id.clone(),
            representation_etag(&json!({"data": dto})),
        );
    }
    let read_people = read_person::Entity::find()
        .filter(read_person::Column::HouseholdId.eq(household_id))
        .filter(read_person::Column::Id.is_in(person_ids.clone()))
        .order_by_asc(read_person::Column::Id)
        .all(db)
        .await
        .map_err(database_error)?;
    add_etags(&mut etags, serialize_people(db, read_people).await?);
    let read_schedules = read_schedule::Entity::find()
        .filter(read_schedule::Column::HouseholdId.eq(household_id))
        .filter(read_schedule::Column::Id.is_in(schedule_ids.iter().copied().collect::<Vec<_>>()))
        .order_by_asc(read_schedule::Column::Id)
        .all(db)
        .await
        .map_err(database_error)?;
    add_etags(
        &mut etags,
        serialize_schedules(db, context, read_schedules).await?,
    );
    let read_assignments = read_assignment::Entity::find()
        .filter(read_assignment::Column::HouseholdId.eq(household_id))
        .filter(
            read_assignment::Column::Id.is_in(assignment_ids.iter().copied().collect::<Vec<_>>()),
        )
        .order_by_asc(read_assignment::Column::Id)
        .all(db)
        .await
        .map_err(database_error)?;
    add_etags(
        &mut etags,
        serialize_assignments(db, context, read_assignments).await?,
    );
    add_etags(
        &mut etags,
        crate::serialize_many(db, medications.clone()).await?,
    );
    for dosage in &dosages {
        let medication_portable_id = medication_ids_by_id
            .get(&dosage.medication_id)
            .ok_or_else(ApiError::internal)?;
        let dto = crate::dosage_options::dosage_value(dosage.clone(), medication_portable_id)?;
        etags.insert(
            dosage.portable_id.clone(),
            representation_etag(&json!({"data": dto})),
        );
    }
    add_etags(&mut etags, crate::health_events::values(db, &health).await?);
    for take in &takes {
        etags.insert(take.portable_id.clone(), crate::dose::take_etag(take));
    }
    let people_by_id: HashMap<i64, &person::Model> =
        people.iter().map(|row| (row.id, row)).collect();
    for preference in &preferences {
        let owner = people_by_id
            .get(&preference.person_id)
            .ok_or_else(ApiError::internal)?;
        let (_, etag) = crate::notification_preferences::representation(preference, owner);
        etags.insert(preference.portable_id.clone(), etag);
    }
    if v2 {
        let schedules_by_id: HashMap<i64, schedule::Model> =
            schedules.iter().cloned().map(|row| (row.id, row)).collect();
        let assignments_by_id: HashMap<i64, person_medication::Model> = assignments
            .iter()
            .cloned()
            .map(|row| (row.id, row))
            .collect();
        let period_values = crate::pause_lifecycle::period_values(
            db,
            &periods,
            &schedules_by_id,
            &assignments_by_id,
        )
        .await?;
        for (period, (_, etag)) in periods.iter().zip(period_values) {
            etags.insert(period.portable_id.clone(), etag);
        }
        for occurrence in &occurrences {
            etags.insert(
                occurrence.portable_id.clone(),
                crate::dose_occurrences::record_etag(occurrence),
            );
        }
    }
    if let Some(collections) = records.as_object_mut() {
        for rows in collections.values_mut() {
            if let Some(rows) = rows.as_array_mut() {
                for row in rows {
                    if let Some(etag) = row["portable_id"].as_str().and_then(|id| etags.get(id)) {
                        row["etag"] = json!(etag);
                    }
                }
            }
        }
    }
    Ok(json!({
        "format": format, "scope": "single_person", "exported_at": Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "source_instance_id": "hosted:rust", "records": records,
    }))
}

fn add_etags(etags: &mut HashMap<String, String>, rows: Vec<Value>) {
    for row in rows {
        if let Some(portable_id) = row["portable_id"].as_str() {
            let key = portable_id.to_owned();
            let etag = representation_etag(&json!({"data": row}));
            etags.insert(key, etag);
        }
    }
}
