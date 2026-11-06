use super::*;

pub(crate) struct Preflight {
    counts: Value,
    pub(crate) conflicts: Vec<NameConflict>,
    pub(crate) errors: Vec<String>,
    original_errors: usize,
    schedule_diagnostics: Vec<PreflightDiagnostic>,
}

#[derive(Clone)]
pub(crate) struct PreflightDiagnostic {
    pub(crate) collection: Option<&'static str>,
    pub(crate) portable_id: Option<String>,
    pub(crate) field: Option<String>,
    pub(crate) detail: String,
}

#[derive(serde::Serialize)]
pub(crate) struct NameConflict {
    pub(crate) record_type: &'static str,
    pub(crate) portable_id: String,
    pub(crate) field: &'static str,
    pub(crate) existing_portable_id: String,
}

impl Preflight {
    pub(crate) fn body(&self, applied: bool) -> Value {
        json!({"applied": applied, "counts": self.counts, "conflicts": self.conflicts, "errors": self.errors})
    }

    pub(crate) fn diagnostics(
        &self,
        plan: &ImportPlan,
        source: &Value,
    ) -> Vec<PreflightDiagnostic> {
        self.errors
            .iter()
            .enumerate()
            .map(|(position, message)| {
                if let Some(offset) = position.checked_sub(self.original_errors)
                    && let Some(diagnostic) = self.schedule_diagnostics.get(offset)
                {
                    return diagnostic.clone();
                }
                let mut diagnostic = PreflightDiagnostic {
                    collection: None,
                    portable_id: None,
                    field: None,
                    detail: message.clone(),
                };
                let Some((kind, remainder)) = message.split_once('[') else {
                    return diagnostic;
                };
                let Some(collection) = BASE_TYPES
                    .iter()
                    .chain(V2_TYPES)
                    .copied()
                    .find(|candidate| *candidate == kind)
                else {
                    return diagnostic;
                };
                let Some((identifier, suffix)) = remainder.split_once(']') else {
                    return diagnostic;
                };
                let portable_id = if let Ok(row_index) = identifier.parse::<usize>() {
                    let rows = if position < self.original_errors {
                        source["records"][kind]
                            .as_array()
                            .map(Vec::as_slice)
                            .unwrap_or(&[])
                    } else {
                        plan.rows(kind)
                    };
                    rows.get(row_index)
                        .and_then(|row| row["portable_id"].as_str())
                        .map(str::to_owned)
                } else {
                    None
                };
                let field = if let Some(suffix) = suffix.strip_prefix('.') {
                    suffix.split_whitespace().next().map(str::to_owned)
                } else {
                    None
                };
                diagnostic.collection = Some(collection);
                diagnostic.portable_id = portable_id;
                diagnostic.field = field;
                diagnostic
            })
            .collect()
    }
}

#[cfg(test)]
mod typed_conflict_tests {
    use super::*;

    #[test]
    fn name_conflict_retains_the_existing_response_shape() {
        let result = Preflight {
            counts: json!({}),
            conflicts: vec![NameConflict {
                record_type: "medications",
                portable_id: "source".into(),
                field: "name",
                existing_portable_id: "destination".into(),
            }],
            errors: Vec::new(),
            original_errors: 0,
            schedule_diagnostics: Vec::new(),
        };
        assert_eq!(
            result.body(false)["conflicts"],
            json!([{"record_type":"medications", "portable_id":"source", "field":"name", "existing_portable_id":"destination"}])
        );
    }

    #[test]
    fn diagnostics_distinguish_original_and_normalized_people_indices() {
        let source = json!({"format":"medtracker.portable.v3","records":{"people":[
            {"portable_id":"matched", "name":"A"}, {"portable_id":"new", "name":"B"}
        ]}});
        let plan = ImportPlan::parse(
            json!({"format":"medtracker.portable.v3","records":{"people":[
                {"portable_id":"new", "name":"B"}
            ]}}),
        )
        .unwrap();
        let result = Preflight {
            counts: json!({}),
            conflicts: vec![],
            errors: vec![
                "people[0].email is invalid".into(),
                "people[0].name is required".into(),
            ],
            original_errors: 1,
            schedule_diagnostics: vec![],
        };
        let diagnostics = result.diagnostics(&plan, &source);
        assert_eq!(diagnostics[0].portable_id.as_deref(), Some("matched"));
        assert_eq!(diagnostics[0].field.as_deref(), Some("email"));
        assert_eq!(diagnostics[1].portable_id.as_deref(), Some("new"));
        assert_eq!(diagnostics[1].field.as_deref(), Some("name"));
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

pub(crate) async fn preflight(
    db: &DatabaseTransaction,
    tenant: &TenantTransaction,
    plan: &ImportPlan,
    index: &ExistingIndex,
) -> Result<Preflight, OperationError> {
    let mut errors = plan.errors.clone();
    let original_errors = errors.len();
    let schedule_conflicts = super::person_schedules::conflicts(db, tenant, plan, index).await?;
    let schedule_diagnostics = schedule_conflicts
        .iter()
        .map(|conflict| {
            let (source, field) = match conflict {
                super::person_schedules::RestoreConflict::Clinical { source, field } => {
                    (source, *field)
                }
                super::person_schedules::RestoreConflict::Pause { source } => (source, "pause"),
            };
            PreflightDiagnostic {
                collection: Some("schedules"),
                portable_id: Some(source.clone()),
                field: Some(field.into()),
                detail: conflict.to_string(),
            }
        })
        .collect();
    errors.extend(
        schedule_conflicts
            .into_iter()
            .map(|conflict| conflict.to_string()),
    );
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
            if let Some(other) = current
                && other != portable_id
            {
                conflicts.push(NameConflict {
                    record_type: kind,
                    portable_id: portable_id.to_owned(),
                    field,
                    existing_portable_id: other.to_owned(),
                });
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
                if let Some((amount, unit)) = index.dosage_value(plan, dosage_id)
                    && (decimal(&row["dose_amount"]) != Some(amount)
                        || row["dose_unit"].as_str() != Some(unit))
                {
                    errors.push(format!("{kind}[{i}] dose must match source dosage option"));
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
            if plan.format != crate::models::care::sync::PortableFormat::V3
                && let Some(stock_location) = row["taken_from_location_portable_id"].as_str()
                && index.medication_location(plan, stock_medication) != Some(stock_location)
            {
                errors.push(format!("medication_takes[{i}].taken_from_location_portable_id does not match medication location"));
            }
        }
    }
    for (i, row) in plan.rows("dose_occurrences").iter().enumerate() {
        if let Some(take_id) = row["medication_take_portable_id"].as_str()
            && index.take_source(plan, take_id)
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
    for kind in ["medication_takes", "dose_occurrences"] {
        for (i, row) in plan.rows(kind).iter().enumerate() {
            let Some(portable_id) = row["portable_id"].as_str() else {
                continue;
            };
            if index.get(kind, portable_id).is_none() {
                continue;
            }
            if let Ok(fields) = mapped_row(kind, row, index, tenant)
                && let Err(error) = verify_immutable(db, kind, &fields).await
            {
                match error {
                    OperationError::Validation { details } => {
                        let message = details
                            .get("error")
                            .and_then(Value::as_str)
                            .unwrap_or("is invalid");
                        errors.push(format!("{kind}[{i}] {message}"));
                    }
                    error => return Err(error),
                }
            }
        }
    }
    let open_periods = db.query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT period.portable_id, CASE WHEN period.schedule_id IS NOT NULL THEN 'schedule' ELSE 'person_medication' END AS source_type, COALESCE(schedule.portable_id, assignment.portable_id) AS source_portable_id FROM medication_pause_periods period LEFT JOIN schedules schedule ON schedule.id = period.schedule_id AND schedule.household_id = period.household_id LEFT JOIN person_medications assignment ON assignment.id = period.person_medication_id AND assignment.household_id = period.household_id WHERE period.household_id = $1 AND period.ended_at IS NULL",
        [tenant.scope().household_id.into()])).await?;
    let mut open_by_source = HashMap::new();
    let mut open_period_sources = HashMap::new();
    for row in open_periods {
        let kind: String = row.try_get("", "source_type")?;
        let id: String = row.try_get("", "source_portable_id")?;
        let period_id: String = row.try_get("", "portable_id")?;
        open_by_source.insert((kind.clone(), id.clone()), period_id.clone());
        open_period_sources.insert(period_id, (kind, id));
    }
    for (i, row) in plan.rows("medication_pause_periods").iter().enumerate() {
        if !row["ended_at"].is_null()
            && let Some(portable_id) = row["portable_id"].as_str()
            && let Some(source) = open_period_sources.get(portable_id)
        {
            open_by_source.remove(source);
        }
        if row["ended_at"].is_null()
            && let (Some(kind), Some(source), Some(portable_id)) = (
                row["source_type"].as_str(),
                row["source_portable_id"].as_str(),
                row["portable_id"].as_str(),
            )
            && open_by_source
                .insert((kind.to_owned(), source.to_owned()), portable_id.to_owned())
                .is_some_and(|existing| existing != portable_id)
        {
            errors.push(format!(
                "medication_pause_periods[{i}] source already has an open pause period"
            ));
        }
        if let Some(portable_id) = row["portable_id"]
            .as_str()
            .filter(|id| index.get("medication_pause_periods", id).is_some())
            && let Ok(fields) = mapped_row("medication_pause_periods", row, index, tenant)
            && let Err(error) = verify_pause(db, tenant, portable_id, &fields).await
        {
            match error {
                OperationError::Validation { details } => {
                    let message = details
                        .get("error")
                        .and_then(Value::as_str)
                        .unwrap_or("is invalid");
                    errors.push(format!("medication_pause_periods[{i}] {message}"));
                }
                error => return Err(error),
            }
        }
    }
    if plan.format.includes_extended_records() {
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
        original_errors,
        schedule_diagnostics,
    })
}
