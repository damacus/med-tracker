use super::*;

pub(super) fn text(row: &Value, field: &str) -> Result<String, String> {
    row[field]
        .as_str()
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| format!("{field} is required"))
}

fn foreign(index: &ExistingIndex, kind: &str, row: &Value, field: &str) -> Result<Value, String> {
    match row[field].as_str() {
        Some(portable_id) => index
            .get(kind, portable_id)
            .map(|id| json!(id))
            .ok_or_else(|| format!("{field} references unknown {kind} portable ID")),
        None if row[field].is_null() => Ok(Value::Null),
        _ => Err(format!("{field} must be a portable ID")),
    }
}

fn enum_value(
    row: &Value,
    field: &str,
    variants: &[&str],
    default: Option<&str>,
) -> Result<Value, String> {
    let selected = row[field]
        .as_str()
        .or(default)
        .ok_or_else(|| format!("{field} is required"))?;
    variants
        .iter()
        .position(|candidate| candidate == &selected)
        .map(|value| json!(value))
        .ok_or_else(|| format!("{field} is unsupported"))
}

fn copy_fields(row: &Value, fields: &[&str]) -> Map<String, Value> {
    fields
        .iter()
        .map(|field| ((*field).to_owned(), row[*field].clone()))
        .collect()
}

fn required_id(
    index: &ExistingIndex,
    kind: &str,
    row: &Value,
    field: &str,
) -> Result<Value, String> {
    let result = foreign(index, kind, row, field)?;
    if result.is_null() {
        Err(format!("{field} is required"))
    } else {
        Ok(result)
    }
}

pub(super) fn mapped_row(
    kind: &str,
    row: &Value,
    index: &ExistingIndex,
    context: &AuthContext,
) -> Result<Map<String, Value>, String> {
    let mut fields = match kind {
        "locations" => copy_fields(row, &["name", "description"]),
        "people" => {
            let mut fields = copy_fields(row, &["name", "email", "date_of_birth", "has_capacity"]);
            fields.insert(
                "person_type".to_owned(),
                enum_value(
                    row,
                    "person_type",
                    &["adult", "minor", "dependent_adult"],
                    None,
                )?,
            );
            fields
        }
        "medications" => {
            let mut fields = copy_fields(
                row,
                &[
                    "name",
                    "friendly_name",
                    "category",
                    "description",
                    "dose_amount",
                    "dose_unit",
                    "current_supply",
                    "reorder_threshold",
                    "barcode",
                    "dmd_code",
                    "dmd_system",
                    "dmd_concept_class",
                ],
            );
            fields.insert(
                "location_id".to_owned(),
                required_id(index, "locations", row, "location_portable_id")?,
            );
            fields.insert(
                "default_schedule_type".to_owned(),
                enum_value(
                    row,
                    "default_schedule_type",
                    &[
                        "daily",
                        "multiple_daily",
                        "weekly",
                        "specific_dates",
                        "prn",
                        "tapering",
                        "every_other_day",
                    ],
                    Some("multiple_daily"),
                )?,
            );
            fields
        }
        "dosage_options" => {
            let mut fields = copy_fields(
                row,
                &[
                    "amount",
                    "unit",
                    "frequency",
                    "description",
                    "default_for_adults",
                    "default_for_children",
                    "default_max_daily_doses",
                    "default_min_hours_between_doses",
                    "current_supply",
                    "reorder_threshold",
                ],
            );
            fields.insert(
                "medication_id".to_owned(),
                required_id(index, "medications", row, "medication_portable_id")?,
            );
            fields.insert(
                "default_dose_cycle".to_owned(),
                enum_value(
                    row,
                    "default_dose_cycle",
                    &["daily", "weekly", "monthly"],
                    Some("daily"),
                )?,
            );
            fields
        }
        "schedules" => {
            let mut fields = copy_fields(
                row,
                &[
                    "dose_amount",
                    "dose_unit",
                    "frequency",
                    "max_daily_doses",
                    "min_hours_between_doses",
                    "schedule_config",
                    "start_date",
                    "end_date",
                    "active",
                    "notes",
                    "retired_at",
                ],
            );
            fields.insert(
                "person_id".to_owned(),
                required_id(index, "people", row, "person_portable_id")?,
            );
            fields.insert(
                "medication_id".to_owned(),
                required_id(index, "medications", row, "medication_portable_id")?,
            );
            fields.insert(
                "source_dosage_option_id".to_owned(),
                foreign(
                    index,
                    "dosage_options",
                    row,
                    "source_dosage_option_portable_id",
                )?,
            );
            fields.insert(
                "dose_cycle".to_owned(),
                if row["dose_cycle"].is_null() {
                    Value::Null
                } else {
                    enum_value(row, "dose_cycle", &["daily", "weekly", "monthly"], None)?
                },
            );
            fields.insert(
                "schedule_type".to_owned(),
                enum_value(
                    row,
                    "schedule_type",
                    &[
                        "daily",
                        "multiple_daily",
                        "weekly",
                        "specific_dates",
                        "prn",
                        "tapering",
                        "every_other_day",
                    ],
                    Some("daily"),
                )?,
            );
            fields
        }
        "person_medications" => {
            let mut fields = copy_fields(
                row,
                &[
                    "dose_amount",
                    "dose_unit",
                    "max_daily_doses",
                    "min_hours_between_doses",
                    "active",
                    "notes",
                    "position",
                    "retired_at",
                ],
            );
            fields.insert(
                "person_id".to_owned(),
                required_id(index, "people", row, "person_portable_id")?,
            );
            fields.insert(
                "medication_id".to_owned(),
                required_id(index, "medications", row, "medication_portable_id")?,
            );
            fields.insert(
                "source_dosage_option_id".to_owned(),
                foreign(
                    index,
                    "dosage_options",
                    row,
                    "source_dosage_option_portable_id",
                )?,
            );
            fields.insert(
                "dose_cycle".to_owned(),
                if row["dose_cycle"].is_null() {
                    Value::Null
                } else {
                    enum_value(row, "dose_cycle", &["daily", "weekly", "monthly"], None)?
                },
            );
            fields.insert(
                "administration_kind".to_owned(),
                enum_value(
                    row,
                    "administration_kind",
                    &["routine", "as_needed"],
                    Some("as_needed"),
                )?,
            );
            fields
        }
        "medication_takes" => {
            let mut fields = copy_fields(
                row,
                &["client_uuid", "taken_at", "dose_amount", "dose_unit"],
            );
            let (schedule_id, assignment_id) = source_ids(index, row)?;
            fields.insert("schedule_id".to_owned(), schedule_id);
            fields.insert("person_medication_id".to_owned(), assignment_id);
            fields.insert(
                "taken_from_medication_id".to_owned(),
                foreign(
                    index,
                    "medications",
                    row,
                    "taken_from_medication_portable_id",
                )?,
            );
            fields.insert(
                "taken_from_location_id".to_owned(),
                foreign(index, "locations", row, "taken_from_location_portable_id")?,
            );
            fields
        }
        "notification_preferences" => {
            let mut fields = copy_fields(
                row,
                &[
                    "enabled",
                    "dose_due_enabled",
                    "missed_dose_enabled",
                    "low_stock_enabled",
                    "private_text_enabled",
                    "morning_time",
                    "afternoon_time",
                    "evening_time",
                    "night_time",
                ],
            );
            fields.insert(
                "person_id".to_owned(),
                required_id(index, "people", row, "person_portable_id")?,
            );
            fields
        }
        "medication_pause_periods" => {
            let mut fields = copy_fields(
                row,
                &["reason", "note", "started_at", "ended_at", "legacy_context"],
            );
            if !row["created_at"].is_null() {
                fields.insert("created_at".to_owned(), row["created_at"].clone());
            }
            let (schedule_id, assignment_id) = source_ids(index, row)?;
            fields.insert("schedule_id".to_owned(), schedule_id);
            fields.insert("person_medication_id".to_owned(), assignment_id);
            fields.insert("imported_context".to_owned(), json!(true));
            fields.insert(
                "imported_actor_references".to_owned(),
                json!({
                    "recorded_by_person_portable_id": row["recorded_by_person_portable_id"],
                    "resumed_by_person_portable_id": row["resumed_by_person_portable_id"],
                }),
            );
            fields
        }
        "dose_occurrences" => {
            let mut fields = copy_fields(
                row,
                &[
                    "window_starts_on",
                    "window_ends_on",
                    "position",
                    "scheduled_at",
                    "outcome",
                    "reason",
                    "note",
                    "resolved_at",
                ],
            );
            let (schedule_id, assignment_id) = source_ids(index, row)?;
            fields.insert("schedule_id".to_owned(), schedule_id);
            fields.insert("person_medication_id".to_owned(), assignment_id);
            fields.insert(
                "medication_take_id".to_owned(),
                foreign(
                    index,
                    "medication_takes",
                    row,
                    "medication_take_portable_id",
                )?,
            );
            fields.insert(
                "resolved_by_membership_id".to_owned(),
                if row["outcome"] == "open" {
                    Value::Null
                } else {
                    json!(context.membership.id)
                },
            );
            fields
        }
        "health_events" => {
            let mut fields = copy_fields(row, &["title", "notes", "started_on", "ended_on"]);
            fields.insert(
                "person_id".to_owned(),
                required_id(index, "people", row, "person_portable_id")?,
            );
            fields.insert(
                "event_kind".to_owned(),
                enum_value(
                    row,
                    "event_kind",
                    &["illness", "suspected_side_effect"],
                    None,
                )?,
            );
            fields.insert(
                "severity".to_owned(),
                if row["severity"].is_null() {
                    Value::Null
                } else {
                    enum_value(row, "severity", &["mild", "moderate", "severe"], None)?
                },
            );
            fields
        }
        _ => return Err("Unsupported portable record type".to_owned()),
    };
    fields.insert(
        "household_id".to_owned(),
        json!(context.membership.household_id),
    );
    fields.insert("portable_id".to_owned(), row["portable_id"].clone());
    Ok(fields)
}

fn source_ids(index: &ExistingIndex, row: &Value) -> Result<(Value, Value), String> {
    let portable_id = row["source_portable_id"]
        .as_str()
        .ok_or("source_portable_id is required")?;
    match row["source_type"].as_str() {
        Some("schedule") => Ok((
            json!(index
                .get("schedules", portable_id)
                .ok_or("Unknown schedule source")?),
            Value::Null,
        )),
        Some("person_medication") => Ok((
            Value::Null,
            json!(index
                .get("person_medications", portable_id)
                .ok_or("Unknown person medication source")?),
        )),
        _ => Err("Unsupported medication source type".to_owned()),
    }
}

pub(super) fn record_person<'a>(
    kind: &str,
    row: &'a Value,
    plan: &'a ImportPlan,
    index: &'a ExistingIndex,
) -> Option<&'a str> {
    if kind == "people" {
        return row["portable_id"].as_str();
    }
    if let Some(person) = row["person_portable_id"].as_str() {
        return Some(person);
    }
    if matches!(
        kind,
        "medication_takes" | "medication_pause_periods" | "dose_occurrences"
    ) {
        let source_type = row["source_type"].as_str()?;
        let source_id = row["source_portable_id"].as_str()?;
        let source_kind = if source_type == "schedule" {
            "schedules"
        } else {
            "person_medications"
        };
        return plan
            .rows(source_kind)
            .iter()
            .find(|source| source["portable_id"] == source_id)
            .and_then(|source| source["person_portable_id"].as_str())
            .or_else(|| index.owner(source_kind, source_id));
    }
    None
}
