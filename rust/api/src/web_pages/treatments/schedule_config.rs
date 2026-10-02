use super::*;
use serde_json::Map;

fn original(draft: &TreatmentDraft) -> Result<Map<String, Value>, Errors> {
    if draft.value("config_original").is_empty()
        || draft.value("config_type_original") != draft.value("schedule_type")
    {
        return Ok(Map::new());
    }
    serde_json::from_str::<Value>(draft.value("config_original"))
        .ok()
        .and_then(|value| value.as_object().cloned())
        .ok_or_else(|| payload::invalid("schedule_config"))
}

fn rows(draft: &TreatmentDraft, prefix: &str) -> Vec<Value> {
    draft
        .indices(prefix, 0)
        .into_iter()
        .filter_map(|index| {
            let value = draft.value(&format!("{prefix}{index}"));
            (!value.is_empty()).then(|| Value::String(value.to_owned()))
        })
        .collect()
}

fn put_string(map: &mut Map<String, Value>, name: &str, value: &str) {
    if value.is_empty() {
        map.remove(name);
    } else if !map.get(name).is_some_and(|old| {
        old.as_str()
            .map_or_else(|| old.to_string().as_str() == value, |old| old == value)
    }) {
        map.insert(name.into(), value.into());
    }
}

fn canonical_day(value: &Value) -> Option<&'static str> {
    match value.as_str()? {
        "monday" | "mon" | "1" => Some("monday"),
        "tuesday" | "tue" | "2" => Some("tuesday"),
        "wednesday" | "wed" | "3" => Some("wednesday"),
        "thursday" | "thu" | "4" => Some("thursday"),
        "friday" | "fri" | "5" => Some("friday"),
        "saturday" | "sat" | "6" => Some("saturday"),
        "sunday" | "sun" | "0" | "7" => Some("sunday"),
        _ => None,
    }
}

fn weekdays(draft: &TreatmentDraft, config: &mut Map<String, Value>) {
    let selected = [
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
        "sunday",
    ]
    .into_iter()
    .filter(|day| draft.value(&format!("weekday_{day}")) == "true")
    .collect::<Vec<_>>();
    let original = config
        .get("weekdays")
        .and_then(Value::as_array)
        .map(|days| {
            days.iter()
                .filter_map(canonical_day)
                .collect::<std::collections::BTreeSet<_>>()
        });
    if original.as_ref() != Some(&selected.iter().copied().collect()) {
        config.insert("weekdays".into(), json!(selected));
    }
}

fn taper_steps(draft: &TreatmentDraft, config: &Map<String, Value>) -> Result<Vec<Value>, Errors> {
    let original = config.get("taper_steps").and_then(Value::as_array);
    let mut steps = Vec::new();
    for index in draft.indices("step_", 0) {
        let prefix = format!("step_{index}_");
        if !draft
            .fields
            .iter()
            .any(|(name, value)| name.starts_with(&prefix) && !value.is_empty())
        {
            continue;
        }
        let mut step = original
            .and_then(|steps| steps.get(index))
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        for name in [
            "start_date",
            "end_date",
            "dose_amount",
            "dose_unit",
            "min_hours_between_doses",
        ] {
            let field = format!("{prefix}{name}");
            let alias = match name {
                "dose_amount"
                    if step.contains_key("amount") && !step.contains_key("dose_amount") =>
                {
                    "amount"
                }
                "dose_unit" if step.contains_key("unit") && !step.contains_key("dose_unit") => {
                    "unit"
                }
                _ => name,
            };
            put_string(&mut step, alias, draft.value(&field));
        }
        let max_field = format!("{prefix}max_daily_doses");
        let max = payload::integer(draft.value(&max_field), &max_field)?;
        if max.is_null() {
            step.remove("max_daily_doses");
        } else {
            step.insert("max_daily_doses".into(), max);
        }
        let times = rows(draft, &format!("{prefix}time_"));
        if !times.is_empty() || step.contains_key("times") {
            step.insert("times".into(), json!(times));
        }
        steps.push(Value::Object(step));
    }
    Ok(steps)
}

pub(super) fn build(draft: &TreatmentDraft) -> Result<Value, Errors> {
    let mut config = original(draft)?;
    match draft.value("schedule_type") {
        "prn" => {
            config.insert("as_needed".into(), true.into());
        }
        "tapering" => {
            let steps = taper_steps(draft, &config)?;
            config.insert("taper_steps".into(), json!(steps));
            let times = rows(draft, "time_");
            if !times.is_empty() || config.contains_key("times") {
                config.insert("times".into(), json!(times));
            }
        }
        "daily" | "multiple_daily" | "weekly" | "specific_dates" | "every_other_day" => {
            config.insert("times".into(), json!(rows(draft, "time_")));
            if draft.value("schedule_type") == "weekly" {
                weekdays(draft, &mut config);
            }
            if draft.value("schedule_type") == "specific_dates" {
                config.insert("dates".into(), json!(rows(draft, "date_")));
            }
        }
        _ => return Err(payload::invalid("schedule_type")),
    }
    Ok(Value::Object(config))
}
