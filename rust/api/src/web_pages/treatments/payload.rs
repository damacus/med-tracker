use super::*;
use serde_json::Map;

pub(super) fn invalid(name: &str) -> Errors {
    BTreeMap::from([(name.to_owned(), vec!["is invalid".into()])])
}

pub(super) fn nullable_string(value: &str) -> Value {
    if value.is_empty() {
        Value::Null
    } else {
        Value::String(value.to_owned())
    }
}

pub(super) fn integer(value: &str, name: &str) -> Result<Value, Errors> {
    if value.is_empty() {
        return Ok(Value::Null);
    }
    value
        .parse::<i32>()
        .map(|value| json!(value))
        .map_err(|_| invalid(name))
}

pub(super) fn build(
    context: &Context,
    kind: Kind,
    draft: &TreatmentDraft,
) -> Result<Value, Errors> {
    let mut attributes = Map::new();
    attributes.insert("person_id".into(), context.person_id.to_string().into());
    attributes.insert("medication_id".into(), draft.value("medication_id").into());
    for name in ["dose_amount", "dose_unit", "source_dosage_option_id"] {
        let value = draft.value(name);
        if !value.is_empty() {
            attributes.insert(name.into(), value.into());
        }
    }
    if !draft.value("max_daily_doses").is_empty() {
        attributes.insert(
            "max_daily_doses".into(),
            integer(draft.value("max_daily_doses"), "max_daily_doses")?,
        );
    }
    attributes.insert("notes".into(), draft.value("notes").into());
    attributes.insert(
        "min_hours_between_doses".into(),
        nullable_string(draft.value("min_hours_between_doses")),
    );
    if !draft.value("dose_cycle").is_empty() {
        attributes.insert("dose_cycle".into(), draft.value("dose_cycle").into());
    }
    match kind {
        Kind::Assignment => {
            if !draft.value("administration_kind").is_empty() {
                attributes.insert(
                    "administration_kind".into(),
                    draft.value("administration_kind").into(),
                );
            }
        }
        Kind::Schedule => {
            for name in ["start_date", "schedule_type"] {
                attributes.insert(name.into(), draft.value(name).into());
            }
            if draft.value("end_date").is_empty() {
                return Err(invalid("end_date"));
            }
            attributes.insert("end_date".into(), draft.value("end_date").into());
            attributes.insert("frequency".into(), draft.value("frequency").into());
            attributes.insert("schedule_config".into(), schedule_config::build(draft)?);
        }
    }
    Ok(json!({(kind.body_key()): attributes}))
}
