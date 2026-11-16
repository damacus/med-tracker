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
) -> std::result::Result<medications::crud::MedicationInput, OperationError> {
    use medications::crud::{Field, MedicationInput};
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
    let mut attributes = MedicationInput::default();
    attributes.name = Field::Value(forms::field(draft, "name").to_owned());
    attributes.friendly_name = Field::Value(forms::field(draft, "friendly_name").to_owned());
    attributes.description = Field::Value(forms::field(draft, "description").to_owned());
    attributes.warnings = Field::Value(forms::field(draft, "warnings").to_owned());
    let location = forms::field(draft, "location_id");
    attributes.location_id = location.parse::<i64>().map_or(Field::Invalid, Field::Value);
    if !options_mode {
        attributes.reorder_threshold = decimal_field(forms::field(draft, "reorder_threshold"));
        let unit = forms::field(draft, "dose_unit");
        if !unit.is_empty() {
            attributes.dose_unit = Field::Value(unit.to_owned());
        }
        attributes.dose_amount = nullable_decimal_field(forms::field(draft, "dose_amount"));
        attributes.current_supply = nullable_decimal_field(forms::field(draft, "current_supply"));
    }
    Ok(attributes)
}

fn decimal_field(value: &str) -> medications::crud::Field<sea_orm::prelude::Decimal> {
    medications::crud::parse_decimal_text(value).map_or(
        medications::crud::Field::InvalidText,
        medications::crud::Field::Value,
    )
}

fn nullable_decimal_field(value: &str) -> medications::crud::Field<sea_orm::prelude::Decimal> {
    if value.is_empty() {
        medications::crud::Field::Null
    } else {
        decimal_field(value)
    }
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

fn validation_field_errors<'a>(
    details: &'a Value,
    label_for: impl Fn(&'a str) -> &'a str,
) -> Option<Vec<Value>> {
    let fields = details.get("errors").and_then(Value::as_object)?;
    Some(
        fields
            .iter()
            .map(|(field, messages)| {
                let label = label_for(field.as_str());
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
                json!({"field": field, "message": format!("{label} {message}")})
            })
            .collect(),
    )
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
        && let Some(mapped) = validation_field_errors(details, |field| {
            FIELDS
                .iter()
                .find(|(name, _, _, _)| *name == field)
                .map(|(_, label, _, _)| *label)
                .unwrap_or(match field {
                    "location_id" => "Location",
                    "dose_unit" => "Unit",
                    _ => field,
                })
        })
    {
        return mapped;
    }
    vec![json!({"field":"medication-form","message":forms::message(error)})]
}

pub(super) struct MoveView<'a> {
    pub slug: &'a str,
    pub context: &'a FormContext,
    pub draft: &'a HashMap<String, String>,
    pub error: Option<&'a OperationError>,
    pub authenticity: String,
    pub language: &'a str,
}

pub(super) fn move_draft(
    context: &FormContext,
    destination_id: Option<i64>,
    filter_location_id: Option<i64>,
    filter_category: Option<&str>,
) -> HashMap<String, String> {
    let mut draft = HashMap::from([
        (
            "etag".into(),
            context
                .snapshot
                .as_ref()
                .map(|snapshot| snapshot.etag.clone())
                .unwrap_or_default(),
        ),
        (
            "location_id".into(),
            destination_id.map(|id| id.to_string()).unwrap_or_default(),
        ),
    ]);
    if let Some(id) = filter_location_id {
        draft.insert("filter_location_id".into(), id.to_string());
    }
    if let Some(category) = filter_category.filter(|value| !value.is_empty()) {
        draft.insert("filter_category".into(), category.to_owned());
    }
    draft
}

pub(super) fn move_attributes(
    draft: &HashMap<String, String>,
    current_location_id: i64,
    etag_matches: bool,
    labels: &Value,
) -> std::result::Result<medications::crud::MedicationInput, OperationError> {
    use medications::crud::{Field, MedicationInput};
    let parsed = forms::field(draft, "location_id")
        .trim()
        .parse::<i64>()
        .ok();
    if etag_matches && parsed == Some(current_location_id) {
        let message = labels["medications"]["move"]["same_location"]
            .as_str()
            .unwrap_or("must be different from the current location");
        return Err(OperationError::Validation {
            details: json!({
                "error": "Destination location must be different from the current location.",
                "errors": { "location_id": [message] }
            }),
        });
    }
    let mut attributes = MedicationInput::default();
    attributes.location_id = parsed.map_or(Field::Invalid, Field::Value);
    Ok(attributes)
}

pub(super) fn move_view(input: MoveView<'_>) -> std::result::Result<Value, OperationError> {
    let snapshot = input
        .context
        .snapshot
        .as_ref()
        .ok_or(OperationError::NotFound)?;
    let locale = crate::models::care::report_pdf::locale(input.language);
    let labels = crate::models::care::report_pdf::translations(locale)
        .map_err(|_| OperationError::Unavailable)?;
    let record = &snapshot.medication;
    let id = record.id;
    let name = snapshot.representation["data"]["display_name"]
        .as_str()
        .or(record.name.as_deref())
        .unwrap_or("Medication");
    let current_location_name = input
        .context
        .locations
        .iter()
        .find(|location| location.id == record.location_id)
        .map(|location| location.name.as_str())
        .unwrap_or("Unknown");
    let quantity_label = match record.current_supply {
        Some(value) => {
            let qty = value.normalize().to_string();
            let unit = rendering::unit_label(record.dose_unit.as_deref().unwrap_or_default(), &qty);
            if unit.is_empty() {
                qty
            } else {
                format!("{qty} {unit}")
            }
        }
        None => labels["medications"]["index"]["untracked_stock"]
            .as_str()
            .unwrap_or("Stock is not tracked.")
            .to_owned(),
    };
    let selected_location = forms::field(input.draft, "location_id").parse::<i64>().ok();
    let filter_location_id = forms::optional(input.draft, "filter_location_id");
    let filter_category = forms::optional(input.draft, "filter_category");
    let filter_query = rendering::query_string(&[
        ("filter_location_id", filter_location_id.clone()),
        ("filter_category", filter_category.clone()),
    ]);
    let return_to = format!(
        "/households/{}/medications/{id}/move{filter_query}",
        input.slug
    );
    let add_location_url = format!(
        "/households/{}/locations/new{}",
        input.slug,
        rendering::query_string(&[("return_to", Some(return_to))])
    );
    let latest_query = rendering::query_string(&[
        (
            "destination_id",
            selected_location.map(|loc| loc.to_string()),
        ),
        ("filter_location_id", filter_location_id),
        ("filter_category", filter_category),
    ]);
    let latest_url = format!(
        "/households/{}/medications/{id}/move{latest_query}",
        input.slug
    );
    let back_url = format!("/households/{}/medications/{id}{filter_query}", input.slug);
    let errors = move_errors(input.error, &labels);
    let mut data = rendering::appearance_context();
    data["lang"] = json!(locale);
    data["labels"] = labels.clone();
    data["title"] = labels["medications"]["move"]["title"].clone();
    data["slug"] = json!(input.slug);
    data["medication_id"] = json!(id);
    data["name"] = json!(name);
    data["current_location_name"] = json!(current_location_name);
    data["quantity_label"] = json!(quantity_label);
    data["action"] = json!(format!("/households/{}/medications/{id}/move", input.slug));
    data["latest_url"] = json!(latest_url);
    data["back_url"] = json!(back_url);
    data["add_location_url"] = json!(add_location_url);
    data["changed"] = json!(matches!(input.error, Some(OperationError::Conflict { .. })));
    data["etag"] = json!(forms::field(input.draft, "etag"));
    data["filter_location_id"] = json!(forms::optional(input.draft, "filter_location_id"));
    data["filter_category"] = json!(forms::optional(input.draft, "filter_category"));
    data["draft"] = json!(input.draft);
    data["authenticity_token"] = json!(input.authenticity);
    data["locations"] = json!(input.context.locations);
    data["selected_location"] = json!(selected_location);
    data["location_invalid"] = json!(errors.iter().any(|entry| entry["field"] == "location_id"));
    data["errors"] = json!(errors);
    Ok(data)
}

fn move_errors(error: Option<&OperationError>, labels: &Value) -> Vec<Value> {
    let Some(error) = error else {
        return Vec::new();
    };
    let move_labels = &labels["medications"]["move"];
    if let OperationError::Conflict { .. } = error {
        let message = move_labels["conflict"].as_str().unwrap_or(
            "Medication changed while this form was open. Review the latest details before saving.",
        );
        return vec![json!({"field": "medication-move-form", "message": message})];
    }
    if let OperationError::NotFound = error {
        let field_label = move_labels["destination"]
            .as_str()
            .unwrap_or("Destination location");
        let suffix = move_labels["unavailable_location"]
            .as_str()
            .unwrap_or("is not available in this household");
        return vec![json!({"field": "location_id", "message": format!("{field_label} {suffix}")})];
    }
    if let OperationError::Validation { details } = error
        && let Some(mapped) = validation_field_errors(details, |field| {
            if field == "location_id" {
                move_labels["destination"]
                    .as_str()
                    .unwrap_or("Destination location")
            } else {
                field
            }
        })
    {
        return mapped;
    }
    vec![json!({"field": "medication-move-form", "message": forms::message(error)})]
}
