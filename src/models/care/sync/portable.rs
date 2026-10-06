use super::*;
use crate::models::entities::{location, location_membership, membership, notification_preference};
use chrono::NaiveDateTime;
use sea_orm::Condition;
use sha2::{Digest, Sha256};
fn database_error(_: sea_orm::DbErr) -> OperationError {
    OperationError::Unavailable
}
fn representation_etag(value: &Value) -> String {
    let mut value = value.clone();
    value.sort_all_objects();
    format!(
        "\"{}\"",
        hex::encode(Sha256::digest(value.to_string().as_bytes()))
    )
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
    tenant: &TenantTransaction,
) -> Result<Vec<person::Model>, OperationError> {
    let query = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(tenant.scope().household_id))
        .order_by_asc(person::Column::Id);
    let query = if access::can_manage_household(tenant) {
        query
    } else {
        query.filter(person::Column::Id.in_subquery(access::granted_people(tenant.membership())))
    };
    Ok(query.all(tenant.transaction()).await?)
}
pub(super) fn occurrence_etag(row: &dose_occurrence::Model) -> String {
    crate::models::care::dose_occurrences::record_etag(row)
}
pub(super) async fn payload(
    tenant: &TenantTransaction,
    zone: chrono_tz::Tz,
) -> Result<AppliedBatch, OperationError> {
    let db = tenant.transaction();
    let format = "medtracker.portable.v2";
    let include_health_events = true;
    let household_id = tenant.scope().household_id;
    let people = visible_people(tenant).await?;
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
    let medication_ids = super::reading::visible_medication_ids(db, tenant).await?;
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
    let locations = location::Entity::find()
        .filter(location::Column::HouseholdId.eq(household_id))
        .filter(location::Column::Id.is_in(location_ids.iter().copied().collect::<Vec<_>>()))
        .order_by_asc(location::Column::Id)
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
    let canonical_health = health_events::values(db, &health).await?;
    let health_medications: HashMap<i64, Value> = canonical_health
        .iter()
        .filter_map(|row| Some((row["id"].as_i64()?, row["medication_portable_ids"].clone())))
        .collect();

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
        "current_supply": row.current_supply.map(|value| medications::decimal_string(value.to_string())),
        "reorder_threshold": medications::decimal_string(row.reorder_threshold.to_string()),
        "barcode": row.barcode, "dmd_code": row.dmd_code, "dmd_system": row.dmd_system,
        "dmd_concept_class": row.dmd_concept_class,
    }))).collect();
    let dosage_values: Vec<Value> = dosages.iter().map(|row| merge(identity(&row.portable_id, row.updated_at), json!({
        "medication_portable_id": medication_ids_by_id.get(&row.medication_id),
        "amount": medications::decimal_string(row.amount.to_string()),
        "unit": row.unit, "frequency": row.frequency, "description": row.description,
        "default_for_adults": row.default_for_adults, "default_for_children": row.default_for_children,
        "default_max_daily_doses": row.default_max_daily_doses,
        "default_min_hours_between_doses": medications::decimal_string(row.default_min_hours_between_doses.to_string()),
        "default_dose_cycle": cycle(row.default_dose_cycle),
        "current_supply": row.current_supply.map(|value| medications::decimal_string(value.to_string())),
        "reorder_threshold": row.reorder_threshold.map(|value| medications::decimal_string(value.to_string())),
    }))).collect();
    let schedule_values: Vec<Value> = schedules.iter().map(|row| merge(identity(&row.portable_id, row.updated_at), json!({
        "source_dosage_option_portable_id": row.source_dosage_option_id.and_then(|id| dosage_ids_by_id.get(&id)),
        "retired_at": row.retired_at.map(timestamp), "person_portable_id": person_ids_by_id.get(&row.person_id),
        "medication_portable_id": medication_ids_by_id.get(&row.medication_id),
        "dose_amount": row.dose_amount.map(|value| medications::decimal_string(value.to_string())),
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
        "dose_amount": row.dose_amount.map(|value| medications::decimal_string(value.to_string())),
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
            "dose_amount": row.dose_amount.map(|value| medications::decimal_string(value.to_string())),
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
            "medication_portable_ids": health_medications.get(&row.id).cloned().unwrap_or_else(|| json!([])),
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
        records["dose_occurrences"] = json!(
            occurrences
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
                .collect::<Vec<_>>()
        );
    }
    let mut etags = HashMap::new();
    for row in &locations {
        let (_, etag) = locations::representation(row);
        etags.insert(row.portable_id.clone(), etag);
    }
    add_etags(&mut etags, people::values(tenant, &people, zone).await?);
    add_etags(
        &mut etags,
        treatments::projection::serialize_schedules(db, tenant, schedules.clone()).await?,
    );
    add_etags(
        &mut etags,
        treatments::projection::serialize_assignments(db, tenant, assignments.clone()).await?,
    );
    add_etags(
        &mut etags,
        medications::serialize_many(db, medications.clone()).await?,
    );
    for row in &dosages {
        let parent = medication_ids_by_id
            .get(&row.medication_id)
            .ok_or(OperationError::Unavailable)?;
        let dto = dosages::value(row, parent)?;
        etags.insert(
            row.portable_id.clone(),
            representation_etag(&json!({"data":dto})),
        );
    }
    add_etags(&mut etags, canonical_health);
    let schedules_by_id = schedules.iter().cloned().map(|row| (row.id, row)).collect();
    let assignments_by_id = assignments
        .iter()
        .cloned()
        .map(|row| (row.id, row))
        .collect();
    for (row, (_, etag)) in periods.iter().zip(
        pause_periods::period_values(db, &periods, &schedules_by_id, &assignments_by_id).await?,
    ) {
        etags.insert(row.portable_id.clone(), etag);
    }
    for row in &preferences {
        let owner = person_ids_by_id
            .get(&row.person_id)
            .ok_or(OperationError::Unavailable)?;
        let formatted =
            |time: Option<chrono::NaiveTime>| time.map(|time| time.format("%H:%M:%S").to_string());
        let canonical = json!({"data":{
            "id":row.id,"portable_id":row.portable_id,"person_id":row.person_id,
            "person_portable_id":owner,"enabled":row.enabled,"dose_due_enabled":row.dose_due_enabled,
            "missed_dose_enabled":row.missed_dose_enabled,"low_stock_enabled":row.low_stock_enabled,
            "private_text_enabled":row.private_text_enabled,"morning_time":formatted(row.morning_time),
            "afternoon_time":formatted(row.afternoon_time),"evening_time":formatted(row.evening_time),
            "night_time":formatted(row.night_time),"updated_at":row.updated_at.and_utc().to_rfc3339()
        }});
        etags.insert(row.portable_id.clone(), representation_etag(&canonical));
    }
    for row in &occurrences {
        etags.insert(row.portable_id.clone(), occurrence_etag(row));
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
    Ok(AppliedBatch {
        body: json!({"format":format,"scope":"single_person","exported_at":Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs,true),"source_instance_id":"hosted:rust","records":records}),
        takes,
    })
}
fn add_etags(etags: &mut HashMap<String, String>, rows: Vec<Value>) {
    for row in rows {
        if let Some(id) = row["portable_id"].as_str() {
            etags.insert(id.into(), representation_etag(&json!({"data":row})));
        }
    }
}
