use super::*;

pub(crate) struct Preflight {
    counts: Value,
    pub(crate) conflicts: Vec<NameConflict>,
    pub(crate) errors: Vec<String>,
    diagnostics: Vec<PreflightDiagnostic>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DiagnosticReason {
    InvalidSource,
    MissingDependency,
}

#[derive(Clone)]
pub(crate) struct PreflightDiagnostic {
    pub(crate) collection: Option<String>,
    pub(crate) portable_id: Option<String>,
    pub(crate) field: Option<String>,
    pub(crate) reason: DiagnosticReason,
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
        _plan: &ImportPlan,
        _source: &Value,
    ) -> Vec<PreflightDiagnostic> {
        self.diagnostics.clone()
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
            diagnostics: Vec::new(),
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
        let original = ImportPlan::parse(source).unwrap();
        let a = row_diagnostic(
            &original,
            "people",
            0,
            Some("email"),
            DiagnosticReason::InvalidSource,
            "invalid".into(),
        );
        let b = row_diagnostic(
            &plan,
            "people",
            0,
            Some("name"),
            DiagnosticReason::InvalidSource,
            "invalid".into(),
        );
        assert_eq!(a.portable_id.as_deref(), Some("matched"));
        assert_eq!(b.portable_id.as_deref(), Some("new"));
    }

    #[test]
    fn schedule_diagnostics_do_not_infer_reason_or_invent_pause_rows() {
        let clinical = super::super::person_schedules::RestoreConflict::Clinical {
            source: "references unknown".into(),
            field: "dose_amount",
        };
        let diagnostic = schedule_diagnostic(&clinical);
        assert_eq!(diagnostic.reason, DiagnosticReason::InvalidSource);
        assert_eq!(
            diagnostic.portable_id.as_deref(),
            Some("references unknown")
        );
        let pause = super::super::person_schedules::RestoreConflict::Pause {
            source: "destination-only".into(),
        };
        let diagnostic = schedule_diagnostic(&pause);
        assert!(diagnostic.collection.is_none());
        assert!(diagnostic.portable_id.is_none());
        assert!(diagnostic.field.is_none());
    }
}

fn reference(
    plan: &ImportPlan,
    index: &ExistingIndex,
    errors: &mut Vec<String>,
    diagnostics: &mut Vec<PreflightDiagnostic>,
    target_field: (&str, usize, &str, &str, bool),
) {
    let (kind, row_index, field, target, required) = target_field;
    let row = &plan.rows(kind)[row_index];
    let value = row[field].as_str().filter(|value| !value.is_empty());
    match value {
        Some(value) if index.has(plan, target, value) => {}
        None if !required && row[field].is_null() => {}
        _ => {
            let detail =
                format!("{kind}[{row_index}].{field} references unknown {target} portable ID");
            errors.push(detail.clone());
            diagnostics.push(row_diagnostic(
                plan,
                kind,
                row_index,
                Some(field),
                DiagnosticReason::MissingDependency,
                detail,
            ));
        }
    }
}

fn row_diagnostic(
    plan: &ImportPlan,
    kind: &str,
    index: usize,
    field: Option<&str>,
    reason: DiagnosticReason,
    detail: String,
) -> PreflightDiagnostic {
    PreflightDiagnostic {
        collection: Some(kind.to_owned()),
        portable_id: plan
            .rows(kind)
            .get(index)
            .and_then(|row| row["portable_id"].as_str())
            .map(str::to_owned),
        field: field.map(str::to_owned),
        reason,
        detail,
    }
}

fn schedule_diagnostic(conflict: &super::person_schedules::RestoreConflict) -> PreflightDiagnostic {
    let (collection, portable_id, field) = match conflict {
        super::person_schedules::RestoreConflict::Clinical { source, field } => (
            Some("schedules".into()),
            Some(source.clone()),
            Some((*field).into()),
        ),
        super::person_schedules::RestoreConflict::Pause { .. } => (None, None, None),
    };
    PreflightDiagnostic {
        collection,
        portable_id,
        field,
        reason: DiagnosticReason::InvalidSource,
        detail: conflict.to_string(),
    }
}

pub(crate) async fn preflight(
    db: &DatabaseTransaction,
    tenant: &TenantTransaction,
    plan: &ImportPlan,
    index: &ExistingIndex,
) -> Result<Preflight, OperationError> {
    let mut errors = plan.errors.clone();
    let mut diagnostics = plan.source_diagnostics.clone();
    macro_rules! row_error {
        ($kind:expr, $index:expr, $field:expr, $reason:expr, $detail:expr) => {{
            let detail = $detail;
            diagnostics.push(row_diagnostic(
                plan,
                $kind,
                $index,
                $field,
                $reason,
                detail.clone(),
            ));
            errors.push(detail);
        }};
    }
    let schedule_conflicts = super::person_schedules::conflicts(db, tenant, plan, index).await?;
    diagnostics.extend(schedule_conflicts.iter().map(schedule_diagnostic));
    errors.extend(
        schedule_conflicts
            .into_iter()
            .map(|conflict| conflict.to_string()),
    );
    let mut conflicts = Vec::new();
    for kind in BASE_TYPES.iter().chain(V2_TYPES) {
        for (index, row) in plan.rows(kind).iter().enumerate() {
            for detail in validate_row(kind, index, row) {
                row_error!(kind, index, None, DiagnosticReason::InvalidSource, detail);
            }
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
            row_error!(
                "people",
                row_index,
                Some("has_capacity"),
                DiagnosticReason::InvalidSource,
                format!(
                    "people[{row_index}].has_capacity must be false for minors and dependent adults"
                )
            );
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
            reference(
                plan,
                index,
                &mut errors,
                &mut diagnostics,
                (kind, i, field, target, required),
            );
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
                    row_error!(
                        kind,
                        i,
                        Some("source_type"),
                        DiagnosticReason::InvalidSource,
                        format!("{kind}[{i}].source_type is unsupported")
                    );
                    continue;
                }
            };
            reference(
                plan,
                index,
                &mut errors,
                &mut diagnostics,
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
                    row_error!(
                        "health_events",
                        i,
                        Some("medication_portable_ids"),
                        DiagnosticReason::MissingDependency,
                        format!(
                            "health_events[{i}].medication_portable_ids references unknown medications portable ID"
                        )
                    );
                }
            }
        } else if !row["medication_portable_ids"].is_null() {
            row_error!(
                "health_events",
                i,
                Some("medication_portable_ids"),
                DiagnosticReason::InvalidSource,
                format!("health_events[{i}].medication_portable_ids must be an array")
            );
        }
    }
    for kind in ["schedules", "person_medications"] {
        for (i, row) in plan.rows(kind).iter().enumerate() {
            let Some(medication_id) = row["medication_portable_id"].as_str() else {
                continue;
            };
            if let Some(dosage_id) = row["source_dosage_option_portable_id"].as_str() {
                if index.dosage_medication(plan, dosage_id) != Some(medication_id) {
                    row_error!(
                        kind,
                        i,
                        Some("source_dosage_option_portable_id"),
                        DiagnosticReason::InvalidSource,
                        format!(
                            "{kind}[{i}].source_dosage_option_portable_id must belong to medication"
                        )
                    );
                }
                if let Some((amount, unit)) = index.dosage_value(plan, dosage_id)
                    && (decimal(&row["dose_amount"]) != Some(amount)
                        || row["dose_unit"].as_str() != Some(unit))
                {
                    row_error!(
                        kind,
                        i,
                        None,
                        DiagnosticReason::InvalidSource,
                        format!("{kind}[{i}] dose must match source dosage option")
                    );
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
                row_error!(
                    "medication_takes",
                    i,
                    Some("taken_from_medication_portable_id"),
                    DiagnosticReason::InvalidSource,
                    format!(
                        "medication_takes[{i}].taken_from_medication_portable_id does not match source medication"
                    )
                );
            }
            if plan.format != crate::models::care::sync::PortableFormat::V3
                && let Some(stock_location) = row["taken_from_location_portable_id"].as_str()
                && index.medication_location(plan, stock_medication) != Some(stock_location)
            {
                row_error!(
                    "medication_takes",
                    i,
                    Some("taken_from_location_portable_id"),
                    DiagnosticReason::InvalidSource,
                    format!(
                        "medication_takes[{i}].taken_from_location_portable_id does not match medication location"
                    )
                );
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
            row_error!(
                "dose_occurrences",
                i,
                Some("medication_take_portable_id"),
                DiagnosticReason::InvalidSource,
                format!("dose_occurrences[{i}].medication_take_portable_id does not match source")
            );
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
                        row_error!(
                            kind,
                            i,
                            None,
                            DiagnosticReason::InvalidSource,
                            format!("{kind}[{i}] {message}")
                        );
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
            row_error!(
                "medication_pause_periods",
                i,
                None,
                DiagnosticReason::InvalidSource,
                format!("medication_pause_periods[{i}] source already has an open pause period")
            );
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
                    row_error!(
                        "medication_pause_periods",
                        i,
                        None,
                        DiagnosticReason::InvalidSource,
                        format!("medication_pause_periods[{i}] {message}")
                    );
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
                    row_error!(
                        kind,
                        i,
                        None,
                        DiagnosticReason::InvalidSource,
                        format!("{kind}[{i}] inactive source requires an open pause period")
                    );
                }
            }
        }
    }
    Ok(Preflight {
        counts: plan.counts.clone(),
        conflicts,
        errors,
        diagnostics,
    })
}
