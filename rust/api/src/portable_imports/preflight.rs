use super::*;

pub(super) struct Preflight {
    counts: Value,
    pub(super) conflicts: Vec<Value>,
    pub(super) errors: Vec<String>,
}

impl Preflight {
    pub(super) fn body(&self, applied: bool) -> Value {
        json!({"applied": applied, "counts": self.counts, "conflicts": self.conflicts, "errors": self.errors})
    }
}

fn reference(
    plan: &ImportPlan,
    index: &ExistingIndex,
    errors: &mut Vec<String>,
    target_field: (&str, usize, &str, &str, bool),
) {
    let (kind, row_index, field, target, required) = target_field;
    let row = &plan.rows(kind)[row_index];
    let value = row[field].as_str().filter(|value| !value.is_empty());
    match value {
        Some(value) if index.has(plan, target, value) => {}
        None if !required && row[field].is_null() => {}
        _ => errors.push(format!(
            "{kind}[{row_index}].{field} references unknown {target} portable ID"
        )),
    }
}

pub(super) async fn preflight(
    db: &DatabaseTransaction,
    context: &AuthContext,
    plan: &ImportPlan,
    index: &ExistingIndex,
) -> Result<Preflight, ApiError> {
    let mut errors = plan.errors.clone();
    let mut conflicts = Vec::new();
    for kind in BASE_TYPES.iter().chain(V2_TYPES) {
        for (index, row) in plan.rows(kind).iter().enumerate() {
            errors.extend(validate_row(kind, index, row));
        }
    }
    for kind in ["locations", "people", "medications"] {
        let mut names = HashMap::new();
        let field = if kind == "people" { "email" } else { "name" };
        for row in plan.rows(kind) {
            let Some(value) = row[field].as_str().filter(|value| !value.is_empty()) else {
                continue;
            };
            let portable_id = row["portable_id"].as_str().unwrap_or_default();
            let lower = value.to_lowercase();
            let current = names.get(&lower).copied().or_else(|| {
                index
                    .names
                    .get(kind)
                    .and_then(|rows| rows.get(&lower).map(String::as_str))
            });
            if let Some(other) = current {
                if other != portable_id {
                    conflicts.push(json!({"record_type": kind, "portable_id": portable_id,
                        "field": field, "existing_portable_id": other}));
                }
            }
            names.insert(lower, portable_id);
        }
    }
    for (row_index, row) in plan.rows("people").iter().enumerate() {
        if row["person_type"]
            .as_str()
            .is_some_and(|kind| matches!(kind, "minor" | "dependent_adult"))
            && row["has_capacity"] == true
        {
            errors.push(format!(
                "people[{row_index}].has_capacity must be false for minors and dependent adults"
            ));
        }
    }
    for (i, row) in plan.rows("people").iter().enumerate() {
        if let Some(locations) = row["location_portable_ids"].as_array() {
            for location in locations {
                if location
                    .as_str()
                    .is_none_or(|id| !index.has(plan, "locations", id))
                {
                    errors.push(format!("people[{i}].location_portable_ids references unknown locations portable ID"));
                }
            }
        } else if !row["location_portable_ids"].is_null() {
            errors.push(format!(
                "people[{i}].location_portable_ids must be an array"
            ));
        }
    }
    for (kind, field, target, required) in [
        ("medications", "location_portable_id", "locations", true),
        (
            "dosage_options",
            "medication_portable_id",
            "medications",
            true,
        ),
        ("schedules", "person_portable_id", "people", true),
        ("schedules", "medication_portable_id", "medications", true),
        (
            "schedules",
            "source_dosage_option_portable_id",
            "dosage_options",
            false,
        ),
        ("person_medications", "person_portable_id", "people", true),
        (
            "person_medications",
            "medication_portable_id",
            "medications",
            true,
        ),
        (
            "person_medications",
            "source_dosage_option_portable_id",
            "dosage_options",
            false,
        ),
        (
            "medication_takes",
            "taken_from_medication_portable_id",
            "medications",
            false,
        ),
        (
            "medication_takes",
            "taken_from_location_portable_id",
            "locations",
            false,
        ),
        (
            "notification_preferences",
            "person_portable_id",
            "people",
            true,
        ),
        (
            "dose_occurrences",
            "medication_take_portable_id",
            "medication_takes",
            false,
        ),
        ("health_events", "person_portable_id", "people", true),
    ] {
        for i in 0..plan.rows(kind).len() {
            reference(plan, index, &mut errors, (kind, i, field, target, required));
        }
    }
    for kind in [
        "medication_takes",
        "medication_pause_periods",
        "dose_occurrences",
    ] {
        for (i, row) in plan.rows(kind).iter().enumerate() {
            let target = match row["source_type"].as_str() {
                Some("schedule") => "schedules",
                Some("person_medication") => "person_medications",
                _ => {
                    errors.push(format!("{kind}[{i}].source_type is unsupported"));
                    continue;
                }
            };
            reference(
                plan,
                index,
                &mut errors,
                (kind, i, "source_portable_id", target, true),
            );
        }
    }
    for (i, row) in plan.rows("health_events").iter().enumerate() {
        if let Some(medications) = row["medication_portable_ids"].as_array() {
            for medication in medications {
                if medication
                    .as_str()
                    .is_none_or(|id| !index.has(plan, "medications", id))
                {
                    errors.push(format!("health_events[{i}].medication_portable_ids references unknown medications portable ID"));
                }
            }
        } else if !row["medication_portable_ids"].is_null() {
            errors.push(format!(
                "health_events[{i}].medication_portable_ids must be an array"
            ));
        }
    }
    for kind in ["schedules", "person_medications"] {
        for (i, row) in plan.rows(kind).iter().enumerate() {
            let Some(medication_id) = row["medication_portable_id"].as_str() else {
                continue;
            };
            if let Some(dosage_id) = row["source_dosage_option_portable_id"].as_str() {
                if index.dosage_medication(plan, dosage_id) != Some(medication_id) {
                    errors.push(format!(
                        "{kind}[{i}].source_dosage_option_portable_id must belong to medication"
                    ));
                }
                if let Some((amount, unit)) = index.dosage_value(plan, dosage_id) {
                    if decimal(&row["dose_amount"]) != Some(amount)
                        || row["dose_unit"].as_str() != Some(unit)
                    {
                        errors.push(format!("{kind}[{i}] dose must match source dosage option"));
                    }
                }
            }
        }
    }
    for (i, row) in plan.rows("medication_takes").iter().enumerate() {
        let Some(source_type) = row["source_type"].as_str() else {
            continue;
        };
        let Some(source_id) = row["source_portable_id"].as_str() else {
            continue;
        };
        let source_kind = if source_type == "schedule" {
            "schedules"
        } else {
            "person_medications"
        };
        let Some(source_medication) = index.source_medication(plan, source_kind, source_id) else {
            continue;
        };
        if let Some(stock_medication) = row["taken_from_medication_portable_id"].as_str() {
            if stock_medication != source_medication {
                errors.push(format!("medication_takes[{i}].taken_from_medication_portable_id does not match source medication"));
            }
            if let Some(stock_location) = row["taken_from_location_portable_id"].as_str() {
                if index.medication_location(plan, stock_medication) != Some(stock_location) {
                    errors.push(format!("medication_takes[{i}].taken_from_location_portable_id does not match medication location"));
                }
            }
        }
    }
    for (i, row) in plan.rows("dose_occurrences").iter().enumerate() {
        if let Some(take_id) = row["medication_take_portable_id"].as_str() {
            if index.take_source(plan, take_id)
                != Some((
                    row["source_type"].as_str().unwrap_or_default(),
                    row["source_portable_id"].as_str().unwrap_or_default(),
                ))
            {
                errors.push(format!(
                    "dose_occurrences[{i}].medication_take_portable_id does not match source"
                ));
            }
        }
    }
    for kind in ["medication_takes", "dose_occurrences"] {
        for (i, row) in plan.rows(kind).iter().enumerate() {
            let Some(portable_id) = row["portable_id"].as_str() else {
                continue;
            };
            if index.get(kind, portable_id).is_none() {
                continue;
            }
            if let Ok(fields) = mapped_row(kind, row, index, context) {
                if let Err(message) = verify_immutable(db, kind, &fields).await {
                    errors.push(format!("{kind}[{i}] {message}"));
                }
            }
        }
    }
    let open_periods = db.query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT period.portable_id, CASE WHEN period.schedule_id IS NOT NULL THEN 'schedule' ELSE 'person_medication' END AS source_type, COALESCE(schedule.portable_id, assignment.portable_id) AS source_portable_id FROM medication_pause_periods period LEFT JOIN schedules schedule ON schedule.id = period.schedule_id AND schedule.household_id = period.household_id LEFT JOIN person_medications assignment ON assignment.id = period.person_medication_id AND assignment.household_id = period.household_id WHERE period.household_id = $1 AND period.ended_at IS NULL",
        [context.membership.household_id.into()])).await.map_err(database_error)?;
    let mut open_by_source = HashMap::new();
    for row in open_periods {
        let kind: String = row.try_get("", "source_type").map_err(database_error)?;
        let id: String = row
            .try_get("", "source_portable_id")
            .map_err(database_error)?;
        let period_id: String = row.try_get("", "portable_id").map_err(database_error)?;
        open_by_source.insert((kind, id), period_id);
    }
    for (i, row) in plan.rows("medication_pause_periods").iter().enumerate() {
        if row["ended_at"].is_null() {
            if let (Some(kind), Some(source), Some(portable_id)) = (
                row["source_type"].as_str(),
                row["source_portable_id"].as_str(),
                row["portable_id"].as_str(),
            ) {
                if open_by_source
                    .insert((kind.to_owned(), source.to_owned()), portable_id.to_owned())
                    .is_some_and(|existing| existing != portable_id)
                {
                    errors.push(format!(
                        "medication_pause_periods[{i}] source already has an open pause period"
                    ));
                }
            }
        }
        if let Some(portable_id) = row["portable_id"]
            .as_str()
            .filter(|id| index.get("medication_pause_periods", id).is_some())
        {
            if let Ok(fields) = mapped_row("medication_pause_periods", row, index, context) {
                if let Err(message) = verify_pause(db, context, portable_id, &fields).await {
                    errors.push(format!("medication_pause_periods[{i}] {message}"));
                }
            }
        }
    }
    if plan.v2 {
        for (kind, source_type) in [
            ("schedules", "schedule"),
            ("person_medications", "person_medication"),
        ] {
            for (i, row) in plan.rows(kind).iter().enumerate() {
                let has_open = open_by_source.contains_key(&(
                    source_type.to_owned(),
                    row["portable_id"].as_str().unwrap_or_default().to_owned(),
                ));
                if row["active"] == false && row["retired_at"].is_null() && !has_open {
                    errors.push(format!(
                        "{kind}[{i}] inactive source requires an open pause period"
                    ));
                }
            }
        }
    }
    Ok(Preflight {
        counts: plan.counts.clone(),
        conflicts,
        errors,
    })
}
