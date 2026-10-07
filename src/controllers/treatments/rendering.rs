use super::*;

pub(super) fn index(mut collection: Value) -> Value {
    let mut data = browser::rendering::appearance_context();
    let name = collection["person"]["name"].as_str().unwrap_or("Person");
    data["title"] = json!(format!("Treatments for {name}"));
    let medications = collection["medications"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    for key in ["schedules", "assignments"] {
        if let Some(records) = collection[key].as_array_mut() {
            for record in records {
                let id = record["medication_id"]
                    .to_string()
                    .trim_matches('"')
                    .to_owned();
                record["medication_name"] = json!(
                    medications
                        .iter()
                        .find(|value| value["id"].to_string().trim_matches('"') == id)
                        .and_then(|value| value["name"].as_str())
                        .unwrap_or("Medication")
                );
            }
        }
    }
    for (key, value) in collection.as_object().into_iter().flatten() {
        data[key] = value.clone();
    }
    data["groups"] = json!([{"kind":"schedules","label":"Schedules","records":collection["schedules"]},{"kind":"assignments","label":"Medication assignments","records":collection["assignments"]}]);
    data
}

pub(super) fn form(
    kind: Kind,
    options: Value,
    id: Option<&str>,
    draft: &HashMap<String, String>,
    error: Option<&OperationError>,
) -> Value {
    let mut data = browser::rendering::appearance_context();
    let title = match (kind, id.is_some()) {
        (Kind::Schedule, false) => "Add schedule",
        (Kind::Schedule, true) => "Edit schedule",
        (Kind::Assignment, false) => "Assign medication",
        (Kind::Assignment, true) => "Edit assignment",
    };
    data["title"] = json!(title);
    data["kind"] = json!(kind.path());
    data["record_id"] = json!(id);
    data["person_name"] = options["person_name"].clone();
    data["draft"] = json!(draft);
    data["is_schedule"] = json!(matches!(kind, Kind::Schedule));
    let errors = match error {
        Some(OperationError::Validation { details }) => {
            details.get("errors").cloned().unwrap_or(json!({}))
        }
        _ => json!({}),
    };
    data["errors"] = errors.clone();
    data["error"] = json!(error.map(|error| match error {
        OperationError::Conflict { .. } =>
            "Treatment changed while this form was open. Review the latest treatment before saving.".into(),
        OperationError::Validation { details } => details["errors"]["schedule_config"]
            .as_array()
            .and_then(|messages| messages.first())
            .and_then(Value::as_str)
            .map_or_else(|| browser_forms::message(error), |message| format!("Taper plan {message}.")),
        _ => browser_forms::message(error),
    }));
    let mut fields = vec![
        field("medication_id", "Medication", "select", draft, &errors),
        field(
            "source_dosage_option_id",
            "Dose option",
            "select",
            draft,
            &errors,
        ),
        field("dose_amount", "Dose amount", "text", draft, &errors),
        field("dose_unit", "Dose unit", "select", draft, &errors),
        field("dose_cycle", "Dose cycle", "select", draft, &errors),
        field(
            "max_daily_doses",
            "Maximum daily doses",
            "number",
            draft,
            &errors,
        ),
        field(
            "min_hours_between_doses",
            "Minimum hours between doses",
            "number",
            draft,
            &errors,
        ),
        field("notes", "Notes", "textarea", draft, &errors),
    ];
    fields[0]["options"] = json!(
        options["medications"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|value| json!({"value":value["id"].to_string(),"label":value["name"]}))
            .collect::<Vec<_>>()
    );
    fields[1]["options"] = json!(options["dosages"].as_array().into_iter().flatten().map(|value|json!({"value":value["id"].to_string(),"label":format!("{} · {} {}", options["medications"].as_array().into_iter().flatten().find(|row|row["id"]==value["medication_id"]).and_then(|row|row["name"].as_str()).unwrap_or("Medication"), value["amount"].as_str().unwrap_or(""),value["unit"].as_str().unwrap_or(""))})).collect::<Vec<_>>());
    fields[3]["options"] = choices(&[
        "tablet", "capsule", "gummy", "mg", "ml", "g", "mcg", "IU", "spray", "drop", "sachet",
        "pad",
    ]);
    fields[4]["options"] = choices(&["daily", "weekly", "monthly"]);
    if matches!(kind, Kind::Schedule) {
        let mut schedule_type = field("schedule_type", "Schedule type", "select", draft, &errors);
        schedule_type["options"] = choices(&[
            "daily",
            "multiple_daily",
            "weekly",
            "specific_dates",
            "prn",
            "tapering",
            "every_other_day",
        ]);
        fields.extend([
            schedule_type,
            field("frequency", "Frequency", "text", draft, &errors),
            field("start_date", "Start date", "date", draft, &errors),
            field("end_date", "End date", "date", draft, &errors),
            field("times", "Times", "text", draft, &errors),
            field("dates", "Specific dates", "textarea", draft, &errors),
        ]);
    } else {
        let mut administration = field(
            "administration_kind",
            "Administration",
            "select",
            draft,
            &errors,
        );
        administration["options"] = choices(&["routine", "as_needed"]);
        fields.push(administration);
    }
    for (group, names) in [
        (
            "dose_fields",
            &[
                "medication_id",
                "source_dosage_option_id",
                "dose_amount",
                "dose_unit",
            ][..],
        ),
        (
            "timing_fields",
            &[
                "schedule_type",
                "dose_cycle",
                "start_date",
                "end_date",
                "frequency",
                "times",
                "dates",
                "administration_kind",
            ][..],
        ),
        (
            "limit_fields",
            &["max_daily_doses", "min_hours_between_doses"][..],
        ),
        ("note_fields", &["notes"][..]),
    ] {
        data[group] = json!(
            names
                .iter()
                .filter_map(|name| fields
                    .iter()
                    .find(|field| field["name"].as_str() == Some(*name)))
                .collect::<Vec<_>>()
        );
    }
    data["weekdays"] = json!(
        [
            "monday",
            "tuesday",
            "wednesday",
            "thursday",
            "friday",
            "saturday",
            "sunday"
        ]
        .iter()
        .map(|day| json!({"value":day,"checked":draft.contains_key(&format!("weekday_{day}"))}))
        .collect::<Vec<_>>()
    );
    let steps = (0..forms::step_count(draft).unwrap_or(0))
        .map(|index| {
            let fields = [
                ("start_date", "Start date", "date"),
                ("end_date", "End date", "date"),
                ("amount", "Dose amount", "text"),
                ("unit", "Dose unit", "text"),
                ("times", "Times", "text"),
                ("max_daily_doses", "Maximum daily doses", "number"),
                (
                    "min_hours_between_doses",
                    "Minimum hours between doses",
                    "number",
                ),
            ]
            .iter()
            .map(|(name, label, kind)| {
                field(&format!("step_{index}_{name}"), label, kind, draft, &errors)
            })
            .collect::<Vec<_>>();
            json!({"index":index,"original_index":draft.get(&format!("step_{index}_original_index")),"fields":fields})
        })
        .collect::<Vec<_>>();
    data["steps"] = json!(steps);
    data["step_count"] = json!(steps.len());
    data
}

fn field(
    name: &str,
    label: &str,
    kind: &str,
    draft: &HashMap<String, String>,
    errors: &Value,
) -> Value {
    let original = browser_forms::field(draft, name);
    let hours = name.ends_with("min_hours_between_doses");
    let decimal = hours
        .then(|| original.parse::<sea_orm::prelude::Decimal>().ok())
        .flatten();
    let whole = decimal.is_some_and(|value| value.fract().is_zero());
    let value = decimal.filter(|_| whole).map_or_else(
        || original.to_owned(),
        |value| value.normalize().to_string(),
    );
    json!({"name":name,"label":label,"kind":kind,"value":value,"step":if hours && decimal.is_some_and(|number| !number.fract().is_zero()) {"any"} else {"1"},"errors":errors.get(name).cloned().unwrap_or(json!([])),"options":[]})
}
fn choices(values: &[&str]) -> Value {
    json!(
        values
            .iter()
            .map(|value| json!({"value":value,"label":value.replace('_'," ")}))
            .collect::<Vec<_>>()
    )
}
