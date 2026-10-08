use super::*;

const FIELDS: &[(&str, &str, &str, bool)] = &[
    ("amount", "Amount", "number", true),
    ("frequency", "Frequency", "text", true),
    ("description", "Description", "text", false),
    ("current_supply", "Current Supply", "number", false),
    ("reorder_threshold", "Reorder Threshold", "number", false),
    (
        "default_max_daily_doses",
        "Maximum daily doses",
        "number",
        true,
    ),
    (
        "default_min_hours_between_doses",
        "Minimum hours between doses",
        "number",
        true,
    ),
];

pub(super) fn defaults() -> Value {
    json!({"amount":"","unit":"tablet","frequency":"daily","description":"","current_supply":"","reorder_threshold":"","default_max_daily_doses":1,"default_min_hours_between_doses":"0","default_dose_cycle":"daily","default_for_adults":false,"default_for_children":false})
}

pub(super) fn draft(record: &Value, etag: &str) -> HashMap<String, String> {
    let mut draft = HashMap::new();
    for field in FIELDS.iter().map(|(name, _, _, _)| *name).chain([
        "unit",
        "default_dose_cycle",
        "default_for_adults",
        "default_for_children",
    ]) {
        let value = &record[field];
        draft.insert(
            field.into(),
            value.as_str().map(str::to_owned).unwrap_or_else(|| {
                if value.is_null() {
                    String::new()
                } else {
                    value.to_string()
                }
            }),
        );
    }
    draft.insert("etag".into(), etag.into());
    draft
}

pub(super) fn fields(draft: &Value, errors: &Value, names: &[&str]) -> Value {
    json!(FIELDS.iter().filter(|(name, _, _, _)| names.contains(name)).map(|(name, label, kind, required)| {
        let value = draft[name].as_str().unwrap_or_default();
        let (value, step) = if *name == "default_min_hours_between_doses" {
            let decimal = value.parse::<sea_orm::prelude::Decimal>().ok();
            let whole = decimal.is_some_and(|number| number.fract().is_zero());
            (decimal.filter(|_| whole).map_or_else(|| value.to_owned(), |number| number.normalize().to_string()), if whole { "1" } else { "any" })
        } else {
            (value.to_owned(), if *name == "default_max_daily_doses" { "1" } else { "any" })
        };
        json!({"name":name,"label":label,"kind":kind,"required":required,"value":value,"step":step,"errors":errors[name].as_array().cloned().unwrap_or_default()})
    }).collect::<Vec<_>>())
}

pub(super) fn browser_hours_valid(
    draft: &HashMap<String, String>,
    existing: Option<&Value>,
) -> bool {
    let entered =
        forms::field(draft, "default_min_hours_between_doses").parse::<sea_orm::prelude::Decimal>();
    match entered {
        Ok(value) if value.fract().is_zero() => true,
        Ok(value) => {
            existing
                .and_then(Value::as_str)
                .and_then(|stored| stored.parse::<sea_orm::prelude::Decimal>().ok())
                == Some(value)
        }
        Err(_) => false,
    }
}

pub(super) fn attributes(draft: &HashMap<String, String>, medication: Option<i64>) -> Value {
    let mut attributes = serde_json::Map::new();
    for name in [
        "amount",
        "unit",
        "frequency",
        "description",
        "default_min_hours_between_doses",
        "default_dose_cycle",
    ] {
        attributes.insert(name.into(), json!(forms::field(draft, name)));
    }
    for name in ["current_supply", "reorder_threshold"] {
        let value = forms::field(draft, name).trim();
        attributes.insert(
            name.into(),
            if value.is_empty() {
                Value::Null
            } else {
                json!(value)
            },
        );
    }
    let maximum = forms::field(draft, "default_max_daily_doses");
    attributes.insert(
        "default_max_daily_doses".into(),
        maximum
            .parse::<i64>()
            .map_or_else(|_| json!(maximum), |value| json!(value)),
    );
    for name in ["default_for_adults", "default_for_children"] {
        attributes.insert(name.into(), json!(forms::field(draft, name) == "true"));
    }
    if let Some(id) = medication {
        attributes.insert("medication_id".into(), json!(id.to_string()));
    }
    json!({"dosage_option":attributes})
}
