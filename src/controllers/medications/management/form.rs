use super::*;
use serde_json::{Value, json};

const FIELDS: &[(&str, &str, &str, bool)] = &[
    ("name", "Name", "text", true),
    ("friendly_name", "Display name", "text", false),
    ("dose_amount", "Dose", "number", false),
    ("current_supply", "Starting Supply", "number", false),
    ("reorder_threshold", "Reorder Threshold", "number", true),
    ("description", "Description", "textarea", false),
    ("warnings", "Warnings", "textarea", false),
];

pub(super) fn draft(context: &FormContext) -> HashMap<String, String> {
    let mut draft: HashMap<String, String> = FIELDS
        .iter()
        .map(|(field, _, _, _)| ((*field).into(), String::new()))
        .collect();
    draft.insert("reorder_threshold".into(), "0".into());
    draft.insert("location_id".into(), String::new());
    draft.insert("dose_unit".into(), String::new());
    draft.insert("etag".into(), String::new());
    if let Some(snapshot) = &context.snapshot {
        let record = &snapshot.medication;
        for (field, value) in [
            ("name", &record.name),
            ("friendly_name", &record.friendly_name),
            ("description", &record.description),
            ("warnings", &record.warnings),
            ("barcode", &record.barcode),
            ("dose_unit", &record.dose_unit),
        ] {
            draft.insert(field.into(), value.clone().unwrap_or_default());
        }
        draft.insert(
            "dose_amount".into(),
            record
                .dose_amount
                .map(|value| value.to_string())
                .unwrap_or_default(),
        );
        draft.insert(
            "current_supply".into(),
            record
                .current_supply
                .map(|value| value.normalize().to_string())
                .unwrap_or_default(),
        );
        draft.insert(
            "reorder_threshold".into(),
            record.reorder_threshold.normalize().to_string(),
        );
        draft.insert("location_id".into(), record.location_id.to_string());
        draft.insert("etag".into(), snapshot.etag.clone());
    }
    draft
}

pub(super) fn attributes(
    draft: &HashMap<String, String>,
    context: &FormContext,
) -> std::result::Result<Value, OperationError> {
    let options_mode = context.options_mode;
    if !options_mode
        && context
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.medication.dose_amount.is_some())
        && forms::field(draft, "dose_amount").is_empty()
    {
        return Err(OperationError::Validation {
            details: json!({"error":"Dose cannot be blank.","errors":{"dose_amount":["cannot be blank"]}}),
        });
    }
    if options_mode
        && ["dose_amount", "dose_unit", "current_supply"]
            .iter()
            .any(|field| draft.contains_key(*field))
    {
        return Err(OperationError::Validation {
            details: json!({"error":"Dosage options remain unchanged when editing medication details."}),
        });
    }
    let mut attributes = serde_json::Map::new();
    for field in ["name", "friendly_name", "description", "warnings"] {
        attributes.insert(field.into(), json!(forms::field(draft, field)));
    }
    let location = forms::field(draft, "location_id");
    attributes.insert(
        "location_id".into(),
        location
            .parse::<i64>()
            .map_or_else(|_| json!(location), |value| json!(value)),
    );
    if !options_mode {
        attributes.insert(
            "reorder_threshold".into(),
            json!(forms::field(draft, "reorder_threshold")),
        );
        let unit = forms::field(draft, "dose_unit");
        if !unit.is_empty() {
            attributes.insert("dose_unit".into(), json!(unit));
        }
        for field in ["dose_amount", "current_supply"] {
            let value = forms::field(draft, field);
            attributes.insert(
                field.into(),
                if value.is_empty() {
                    Value::Null
                } else {
                    json!(value)
                },
            );
        }
    }
    Ok(Value::Object(attributes))
}

pub(super) fn view(
    slug: &str,
    context: &FormContext,
    draft: &HashMap<String, String>,
    error: Option<&OperationError>,
    authenticity: String,
) -> Value {
    let editing = context.snapshot.is_some();
    let id = context
        .snapshot
        .as_ref()
        .map(|snapshot| snapshot.medication.id);
    let mut data = rendering::appearance_context();
    let errors = errors(error);
    data["title"] = json!(if editing {
        "Edit Medication"
    } else {
        "Add a New Medication"
    });
    data["slug"] = json!(slug);
    data["dosage_options_url"] =
        json!(id.map(|id| format!("/households/{slug}/medications/{id}/dosage_options")));
    data["action"] = json!(id.map_or_else(
        || format!("/households/{slug}/medications"),
        |id| format!("/households/{slug}/medications/{id}")
    ));
    data["latest_url"] = json!(id.map(|id| format!("/households/{slug}/medications/{id}/edit")));
    data["changed"] = json!(matches!(error, Some(OperationError::Conflict { .. })));
    data["draft"] = json!(draft);
    data["authenticity_token"] = json!(authenticity);
    data["options_mode"] = json!(context.options_mode);
    data["locations"] = json!(context.locations);
    data["selected_location"] = json!(forms::field(draft, "location_id").parse::<i64>().ok());
    data["location_invalid"] = json!(errors.iter().any(|error| error["field"] == "location_id"));
    data["unit_invalid"] = json!(errors.iter().any(|error| error["field"] == "dose_unit"));
    data["units"] = json!([
        "tablet", "capsule", "gummy", "mg", "ml", "g", "mcg", "IU", "spray", "drop", "sachet",
        "pad"
    ]);
    for (group, names) in [
        ("identity_fields", &["name", "friendly_name"][..]),
        ("dose_fields", &["dose_amount"][..]),
        ("stock_fields", &["current_supply", "reorder_threshold"][..]),
        ("note_fields", &["description", "warnings"][..]),
    ] {
        data[group] = json!(names.iter().filter_map(|name| FIELDS.iter().find(|(field,_,_,_)| field == name)).filter(|(field,_,_,_)| !context.options_mode || !["dose_amount","current_supply","reorder_threshold"].contains(field)).map(|(field,label,kind,required)| json!({"name":field,"label":if editing && *field=="current_supply" { "Remaining Supply" } else { label },"kind":kind,"required":required,"value":forms::field(draft,field),"invalid":errors.iter().any(|error| error["field"]==*field)})).collect::<Vec<_>>());
    }
    data["errors"] = json!(errors);
    data
}

pub(super) fn errors(error: Option<&OperationError>) -> Vec<Value> {
    let Some(error) = error else {
        return Vec::new();
    };
    if let OperationError::Conflict { .. } = error {
        return vec![
            json!({"field":"medication-form","message":"Medication changed while this form was open. Review the latest details before saving."}),
        ];
    }
    if let OperationError::Validation { details } = error
        && let Some(fields) = details.get("errors").and_then(Value::as_object)
    {
        return fields
            .iter()
            .map(|(field, messages)| {
                let label = FIELDS
                    .iter()
                    .find(|(name, _, _, _)| *name == field)
                    .map(|(_, label, _, _)| *label)
                    .unwrap_or(match field.as_str() {
                        "location_id" => "Location",
                        "dose_unit" => "Unit",
                        _ => field,
                    });
                let message = messages
                    .as_array()
                    .map(|values| {
                        values
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_else(|| messages.as_str().unwrap_or("is invalid").into());
                json!({"field":field,"message":format!("{label} {message}")})
            })
            .collect();
    }
    vec![json!({"field":"medication-form","message":forms::message(error)})]
}
