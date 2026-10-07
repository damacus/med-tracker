use super::*;

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

pub(super) fn body(
    kind: Kind,
    person_id: &str,
    draft: &HashMap<String, String>,
    existing: Option<&Value>,
) -> Result<Value, OperationError> {
    let mut attrs = serde_json::Map::new();
    attrs.insert("person_id".into(), json!(person_id));
    for field in [
        "medication_id",
        "source_dosage_option_id",
        "dose_amount",
        "dose_unit",
        "dose_cycle",
        "notes",
        "min_hours_between_doses",
    ] {
        if let Some(value) = browser_forms::optional(draft, field) {
            if field == "min_hours_between_doses"
                && !whole_hours_or_unchanged(&value, existing.and_then(|record| record.get(field)))
            {
                return Err(invalid(field, "must be a whole number"));
            }
            attrs.insert(field.into(), json!(value));
        }
    }
    if draft.contains_key("notes") {
        attrs.insert("notes".into(), json!(browser_forms::field(draft, "notes")));
    }
    if draft.contains_key("min_hours_between_doses")
        && browser_forms::optional(draft, "min_hours_between_doses").is_none()
    {
        attrs.insert("min_hours_between_doses".into(), Value::Null);
    }
    if let Some(value) = browser_forms::optional(draft, "max_daily_doses") {
        attrs.insert(
            "max_daily_doses".into(),
            json!(
                value
                    .parse::<i64>()
                    .map_err(|_| invalid("max_daily_doses", "must be a whole number"))?
            ),
        );
    }
    match kind {
        Kind::Schedule => {
            for field in ["frequency", "start_date", "end_date", "schedule_type"] {
                attrs.insert(field.into(), json!(browser_forms::field(draft, field)));
            }
            attrs.insert("schedule_config".into(), configuration(draft, existing)?);
        }
        Kind::Assignment => {
            attrs.insert(
                "administration_kind".into(),
                json!(browser_forms::field(draft, "administration_kind")),
            );
        }
    }
    Ok(json!({kind.envelope():attrs}))
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

fn configuration(
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
    if !steps.is_empty() {
        config.insert("taper_steps".into(), json!(steps));
    }
    Ok(Value::Object(config))
}
