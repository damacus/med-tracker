use super::*;
use crate::models::care::treatment_command::{
    AdministrationKind, DosageOptionId, DoseCycle, MedicationId, PersonId, ScheduleType,
};

pub(super) fn defaults(record: Option<&Value>) -> HashMap<String, String> {
    let mut draft = HashMap::from([
        ("dose_unit".into(), "tablet".into()),
        ("dose_cycle".into(), "daily".into()),
        ("schedule_type".into(), "daily".into()),
        ("administration_kind".into(), "routine".into()),
        ("step_count".into(), "0".into()),
    ]);
    if let Some(record) = record {
        for field in [
            "medication_id",
            "source_dosage_option_id",
            "dose_amount",
            "dose_unit",
            "dose_cycle",
            "notes",
            "frequency",
            "start_date",
            "end_date",
            "schedule_type",
            "administration_kind",
            "max_daily_doses",
            "min_hours_between_doses",
        ] {
            if let Some(value) = record.get(field).filter(|value| !value.is_null()) {
                draft.insert(
                    field.into(),
                    value
                        .as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| value.to_string()),
                );
            }
        }
        let config = &record["schedule_config"];
        for field in ["times", "dates"] {
            if let Some(values) = config[field].as_array() {
                draft.insert(
                    field.into(),
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(", "),
                );
            }
        }
        if let Some(days) = config["weekdays"].as_array() {
            for day in days.iter().filter_map(Value::as_str) {
                let day = match day.trim().to_ascii_lowercase().as_str() {
                    "0" | "7" | "sun" | "sunday" => "sunday",
                    "1" | "mon" | "monday" => "monday",
                    "2" | "tue" | "tuesday" => "tuesday",
                    "3" | "wed" | "wednesday" => "wednesday",
                    "4" | "thu" | "thursday" => "thursday",
                    "5" | "fri" | "friday" => "friday",
                    "6" | "sat" | "saturday" => "saturday",
                    _ => continue,
                };
                draft.insert(format!("weekday_{day}"), "on".into());
            }
        }
        if let Some(steps) = config["taper_steps"].as_array() {
            draft.insert("step_count".into(), steps.len().to_string());
            for (index, step) in steps.iter().enumerate() {
                draft.insert(format!("step_{index}_original_index"), index.to_string());
                for (field, alternate) in [
                    ("start_date", "start_date"),
                    ("end_date", "end_date"),
                    ("amount", "dose_amount"),
                    ("unit", "dose_unit"),
                ] {
                    if let Some(value) = step
                        .get(field)
                        .or_else(|| step.get(alternate))
                        .and_then(Value::as_str)
                    {
                        draft.insert(format!("step_{index}_{field}"), value.into());
                    }
                }
                if let Some(times) = step["times"].as_array() {
                    draft.insert(
                        format!("step_{index}_times"),
                        times
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(", "),
                    );
                }
                for field in ["max_daily_doses", "min_hours_between_doses"] {
                    if let Some(value) = step.get(field) {
                        draft.insert(
                            format!("step_{index}_{field}"),
                            value
                                .as_str()
                                .map(str::to_owned)
                                .unwrap_or_else(|| value.to_string()),
                        );
                    }
                }
            }
        }
    }
    draft
}

pub(crate) enum FormInput {
    Schedule(treatments::ValidatedScheduleInput),
    Assignment(assignments::ValidatedAssignmentInput),
}

pub(crate) fn body(
    kind: Kind,
    person_id: &str,
    draft: &HashMap<String, String>,
    existing: Option<&Value>,
) -> Result<FormInput, OperationError> {
    let person_id = person_id
        .parse::<PersonId>()
        .map_err(|_| invalid("person_id", "is invalid"))?;
    let medication_id = browser_forms::optional(draft, "medication_id")
        .map(|value| {
            value
                .parse::<MedicationId>()
                .map_err(|_| invalid("medication_id", "is invalid"))
        })
        .transpose()?;
    let source_dosage_option_id = browser_forms::optional(draft, "source_dosage_option_id")
        .map(|value| {
            value
                .parse::<DosageOptionId>()
                .map_err(|_| invalid("source_dosage_option_id", "is invalid"))
        })
        .transpose()?;
    let dose_amount = browser_forms::optional(draft, "dose_amount")
        .map(|value| {
            value
                .parse::<sea_orm::prelude::Decimal>()
                .map_err(|_| invalid("dose_amount", "must be a decimal string"))
        })
        .transpose()?;
    let dose_unit = browser_forms::optional(draft, "dose_unit");
    let dose_cycle = browser_forms::optional(draft, "dose_cycle")
        .map(|value| DoseCycle::parse(&value).ok_or_else(|| invalid("dose_cycle", "is invalid")))
        .transpose()?;
    let notes = draft
        .contains_key("notes")
        .then(|| browser_forms::field(draft, "notes").to_owned());
    let min_hours_between_doses = if let Some(value) =
        browser_forms::optional(draft, "min_hours_between_doses")
    {
        if !whole_hours_or_unchanged(
            &value,
            existing.and_then(|record| record.get("min_hours_between_doses")),
        ) {
            return Err(invalid("min_hours_between_doses", "must be a whole number"));
        }
        let value = value
            .parse::<sea_orm::prelude::Decimal>()
            .ok()
            .filter(|value| *value > sea_orm::prelude::Decimal::ZERO && value.fract().is_zero())
            .and_then(|value| value.normalize().to_string().parse::<i32>().ok())
            .ok_or_else(|| invalid("min_hours_between_doses", "must be a whole positive number"))?;
        Some(Some(value))
    } else if draft.contains_key("min_hours_between_doses") {
        Some(None)
    } else {
        None
    };
    let max_daily_doses = browser_forms::optional(draft, "max_daily_doses")
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|_| invalid("max_daily_doses", "must be a whole number"))
                .and_then(|value| {
                    i32::try_from(value)
                        .ok()
                        .filter(|value| *value > 0)
                        .ok_or_else(|| invalid("max_daily_doses", "must be positive"))
                })
        })
        .transpose()?;
    match kind {
        Kind::Schedule => {
            let parse_date = |field| {
                let raw = browser_forms::field(draft, field);
                if raw.len() != 10 {
                    return Err(invalid(field, "must be a date"));
                }
                chrono::NaiveDate::parse_from_str(raw, "%Y-%m-%d")
                    .map_err(|_| invalid(field, "must be a date"))
            };
            let schedule_type = ScheduleType::parse(browser_forms::field(draft, "schedule_type"))
                .ok_or_else(|| invalid("schedule_type", "is invalid"))?;
            Ok(FormInput::Schedule(
                treatments::ScheduleInput {
                    person_id: Some(person_id),
                    medication_id,
                    source_dosage_option_id,
                    dose_amount,
                    dose_unit,
                    frequency: Some(browser_forms::field(draft, "frequency").to_owned()),
                    start_date: Some(parse_date("start_date")?),
                    end_date: Some(parse_date("end_date")?),
                    notes,
                    max_daily_doses,
                    min_hours_between_doses,
                    dose_cycle,
                    schedule_type: Some(schedule_type),
                    schedule_config: Some(configuration(draft, existing)?),
                }
                .validated()?,
            ))
        }
        Kind::Assignment => {
            let administration_kind =
                AdministrationKind::parse(browser_forms::field(draft, "administration_kind"))
                    .ok_or_else(|| invalid("administration_kind", "is invalid"))?;
            Ok(FormInput::Assignment(
                assignments::Attributes {
                    person_id: Some(person_id),
                    medication_id,
                    source_dosage_option_id,
                    dose_amount,
                    dose_unit,
                    administration_kind: Some(administration_kind),
                    notes,
                    max_daily_doses,
                    min_hours_between_doses,
                    dose_cycle,
                }
                .validated(existing.is_none())?,
            ))
        }
    }
}

fn invalid(field: &str, message: &str) -> OperationError {
    OperationError::Validation {
        details: json!({"errors":{field:[message]}}),
    }
}

pub(super) fn step_count(draft: &HashMap<String, String>) -> Result<usize, OperationError> {
    browser_forms::field(draft, "step_count")
        .parse::<usize>()
        .ok()
        .filter(|value| *value <= draft.len())
        .ok_or_else(|| invalid("schedule_config", "contains invalid taper steps"))
}

fn list(value: &str) -> Vec<&str> {
    value
        .split([',', '\n'])
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect()
}

fn whole_hours_or_unchanged(value: &str, old: Option<&Value>) -> bool {
    let Ok(entered) = value.parse::<sea_orm::prelude::Decimal>() else {
        return false;
    };
    entered.fract().is_zero()
        || old
            .and_then(|stored| {
                stored
                    .as_str()
                    .map(str::to_owned)
                    .or_else(|| stored.as_number().map(ToString::to_string))
            })
            .and_then(|stored| stored.parse::<sea_orm::prelude::Decimal>().ok())
            == Some(entered)
}

fn same_taper_step(draft: &HashMap<String, String>, index: usize, original: &Value) -> bool {
    for field in ["start_date", "end_date", "unit"] {
        let stored = if field == "unit" {
            original.get("unit").or_else(|| original.get("dose_unit"))
        } else {
            original.get(field)
        };
        if stored.and_then(Value::as_str)
            != Some(browser_forms::field(
                draft,
                &format!("step_{index}_{field}"),
            ))
        {
            return false;
        }
    }
    let stored_amount = original
        .get("amount")
        .or_else(|| original.get("dose_amount"))
        .and_then(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .or_else(|| value.as_number().map(ToString::to_string))
        })
        .and_then(|value| value.parse::<sea_orm::prelude::Decimal>().ok());
    let entered_amount = browser_forms::field(draft, &format!("step_{index}_amount"))
        .parse::<sea_orm::prelude::Decimal>()
        .ok();
    let stored_times = original["times"]
        .as_array()
        .map(|values| values.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default();
    stored_amount.is_some()
        && stored_amount == entered_amount
        && stored_times == list(browser_forms::field(draft, &format!("step_{index}_times")))
}

pub(crate) fn configuration(
    draft: &HashMap<String, String>,
    existing: Option<&Value>,
) -> Result<Value, OperationError> {
    let mut config = serde_json::Map::new();
    for field in ["times", "dates"] {
        let values = list(browser_forms::field(draft, field));
        if !values.is_empty() {
            config.insert(field.into(), json!(values));
        }
    }
    let weekdays: Vec<_> = [
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
        "sunday",
    ]
    .into_iter()
    .filter(|day| draft.contains_key(&format!("weekday_{day}")))
    .collect();
    if !weekdays.is_empty() {
        config.insert("weekdays".into(), json!(weekdays));
    }
    if browser_forms::field(draft, "schedule_type") == "prn" {
        config.insert("as_needed".into(), json!(true));
    }
    let mut steps = Vec::new();
    let mut used_origins = Vec::new();
    for index in 0..step_count(draft)? {
        let original = browser_forms::optional(draft, &format!("step_{index}_original_index"))
            .and_then(|value| value.parse::<usize>().ok());
        let original_step = original.and_then(|original| {
            existing
                .and_then(|record| record.get("schedule_config"))
                .and_then(|config| config.get("taper_steps"))
                .and_then(|steps| steps.get(original))
        });
        if let Some(original) = original {
            if used_origins.contains(&original) || original_step.is_none() {
                return Err(invalid(
                    "schedule_config",
                    "contains an invalid taper step origin",
                ));
            }
            used_origins.push(original);
        }
        let mut step = serde_json::Map::new();
        for field in ["start_date", "end_date", "amount", "unit"] {
            step.insert(
                field.into(),
                json!(browser_forms::field(
                    draft,
                    &format!("step_{index}_{field}")
                )),
            );
        }
        let times = list(browser_forms::field(draft, &format!("step_{index}_times")));
        if !times.is_empty() {
            step.insert("times".into(), json!(times));
        }
        for field in ["max_daily_doses", "min_hours_between_doses"] {
            if let Some(value) = browser_forms::optional(draft, &format!("step_{index}_{field}")) {
                let value = if field == "max_daily_doses" {
                    json!(value.parse::<i64>().map_err(|_| invalid(
                        "schedule_config",
                        "must use whole numbers for maximum doses"
                    ))?)
                } else {
                    let old = original_step.and_then(|step| step.get(field));
                    if !whole_hours_or_unchanged(&value, old) {
                        return Err(invalid(
                            "schedule_config",
                            "must use whole hours between doses",
                        ));
                    }
                    if value
                        .parse::<sea_orm::prelude::Decimal>()
                        .is_ok_and(|hours| !hours.fract().is_zero())
                        && !original_step.is_some_and(|step| same_taper_step(draft, index, step))
                    {
                        return Err(invalid(
                            "schedule_config",
                            "must use whole hours when a taper step changes",
                        ));
                    }
                    json!(value)
                };
                step.insert(field.into(), value);
            }
        }
        steps.push(Value::Object(step));
    }
    if browser_forms::field(draft, "schedule_type") == "tapering"
        && draft.contains_key("tapering_plan")
    {
        let instruction = browser_forms::optional(draft, "tapering_plan")
            .ok_or_else(|| invalid("tapering_plan", "is required"))?;
        config.insert("tapering_plan".into(), json!(instruction));
        if steps.is_empty() {
            steps.push(json!({"start_date":browser_forms::field(draft,"start_date"),"end_date":browser_forms::field(draft,"end_date"),"amount":browser_forms::field(draft,"dose_amount"),"unit":browser_forms::field(draft,"dose_unit"),"max_daily_doses":browser_forms::field(draft,"max_daily_doses").parse::<i64>().map_err(|_|invalid("max_daily_doses","must be a whole number"))?,"min_hours_between_doses":browser_forms::field(draft,"min_hours_between_doses")}));
        }
    }
    if !steps.is_empty() {
        config.insert("taper_steps".into(), json!(steps));
    }
    Ok(Value::Object(config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::prelude::Decimal;

    #[test]
    fn form_builds_typed_schedule_with_exact_amount_and_date() {
        let draft = HashMap::from([
            ("medication_id".into(), "80001".into()),
            ("dose_amount".into(), "0.10".into()),
            ("dose_unit".into(), "tablet".into()),
            ("start_date".into(), "2026-10-22".into()),
            ("end_date".into(), "2026-11-22".into()),
            ("schedule_type".into(), "daily".into()),
            ("times".into(), "08:00".into()),
            ("step_count".into(), "0".into()),
        ]);
        let FormInput::Schedule(input) = body(Kind::Schedule, "73001", &draft, None).unwrap()
        else {
            panic!("expected schedule input");
        };
        assert_eq!(
            input.as_ref().dose_amount,
            Some("0.10".parse::<Decimal>().unwrap())
        );
        assert_eq!(
            input.as_ref().start_date,
            Some("2026-10-22".parse().unwrap())
        );
        assert_eq!(
            input.as_ref().schedule_config.as_ref().unwrap()["times"],
            json!(["08:00"])
        );
    }

    #[test]
    fn form_rejects_non_padded_schedule_dates() {
        let draft = HashMap::from([
            ("medication_id".into(), "80001".into()),
            ("dose_amount".into(), "1".into()),
            ("dose_unit".into(), "tablet".into()),
            ("start_date".into(), "2026-1-2".into()),
            ("end_date".into(), "2026-11-22".into()),
            ("schedule_type".into(), "daily".into()),
            ("step_count".into(), "0".into()),
        ]);
        let result = body(Kind::Schedule, "73001", &draft, None);
        assert!(matches!(result, Err(OperationError::Validation { .. })));
    }
}
