use super::*;
use crate::controllers::shell::ShellView as TeraView;
use crate::models::care::people::PersonKind;
use crate::views::{PaginationMeta, people::Person};

const FIELDS: &[(&str, &str, &str, bool)] = &[
    ("name", "Name", "text", true),
    ("email", "Email", "email", false),
    ("date_of_birth", "Date of Birth", "date", true),
];

fn context(slug: &str) -> Value {
    let mut data = super::super::medications::rendering::appearance_context();
    data["slug"] = json!(slug);
    let labels = crate::models::care::report_pdf::translations("en").unwrap_or(Value::Null);
    data["care_warning_labels"] = labels["care_warning"].clone();
    data["health_events_label"] = labels["people"]["show"]["health_events"].clone();
    data["manage_carers_label"] =
        labels["people"]["carer_relationships"]["browser"]["relationships"].clone();
    data["capacity_hint"] = labels["people"]["form"]["capacity_hint"].clone();
    data["guardian_labels"] = labels["guardian_declarations"].clone();
    data
}

pub(super) fn index(
    slug: &str,
    records: Vec<Value>,
    meta: PaginationMeta,
    can_create: bool,
) -> Value {
    let mut data = context(slug);
    data["title"] = json!("People");
    let page = meta.page;
    let size = meta.per_page;
    data["people"] = Value::Array(records);
    data["per_page"] = json!(size);
    data["previous_page"] = json!((page > 1).then_some(page - 1));
    data["next_page"] = json!(
        ((page as u64).saturating_mul(size as u64) < meta.total_count)
            .then_some(page.saturating_add(1))
    );
    data["can_create"] = json!(can_create);
    data
}

pub(super) fn detail(
    slug: &str,
    person: &Person,
    medications: &[Value],
    can_manage: bool,
    can_manage_household: bool,
) -> Value {
    let mut data = context(slug);
    data["title"] = json!(person.name);
    data["avatar_initials"] = json!(
        person
            .name
            .split_whitespace()
            .filter_map(|word| word.chars().next())
            .take(2)
            .map(|letter| letter.to_uppercase().to_string())
            .collect::<Vec<_>>()
            .join("")
    );
    data["type_label"] = json!(type_label(person.person_type));
    data["person"] = json!(person);
    data["age_present"] = json!(person.age.is_some());
    data["date_of_birth_display"] = json!(
        person
            .date_of_birth
            .as_deref()
            .filter(|value| !value.is_empty())
            .unwrap_or("Not recorded")
    );
    data["email_display"] = json!(
        person
            .email
            .as_deref()
            .filter(|value| !value.is_empty())
            .unwrap_or("Not recorded")
    );
    data["medications"] = json!(medications);
    data["can_manage"] = json!(can_manage);
    data["can_manage_household"] = json!(can_manage_household);
    data
}

pub(super) fn draft(person: Option<&Person>) -> HashMap<String, String> {
    HashMap::from([
        (
            "name".into(),
            person.map_or("", |person| &person.name).into(),
        ),
        (
            "email".into(),
            person
                .and_then(|person| person.email.as_deref())
                .unwrap_or_default()
                .into(),
        ),
        (
            "date_of_birth".into(),
            person
                .and_then(|person| person.date_of_birth.as_deref())
                .unwrap_or_default()
                .into(),
        ),
        (
            "person_type".into(),
            person
                .map_or("minor", |person| person.person_type.as_str())
                .into(),
        ),
        (
            "has_capacity".into(),
            person.is_some_and(|person| person.has_capacity).to_string(),
        ),
    ])
}

pub(super) fn form(
    slug: &str,
    person: Option<&Person>,
    draft: &HashMap<String, String>,
    error: Option<&OperationError>,
    language: &str,
    labels: &Value,
) -> Value {
    let mut data = context(slug);
    data["lang"] = json!(language);
    data["labels"] = labels.clone();
    data["capacity_hint"] = labels["people"]["form"]["capacity_hint"].clone();
    data["cancel_url"] = json!(person.map(|record| record.id).map_or_else(
        || format!("/households/{slug}/people"),
        |id| format!("/households/{slug}/people/{id}")
    ));
    data["guardian_labels"] = labels["guardian_declarations"].clone();
    let errors = errors(error);
    data["title"] = labels["people"]["form"][if person.is_some() {
        "edit_heading"
    } else {
        "new_heading"
    }]
    .clone();
    data["editing"] = json!(person.is_some());
    data["action"] = json!(person.map(|person| person.id).map_or_else(
        || format!("/households/{slug}/people"),
        |id| format!("/households/{slug}/people/{id}")
    ));
    let mut values = draft.clone();
    values
        .entry("has_capacity".into())
        .or_insert_with(|| "false".into());
    data["draft"] = json!(values);
    data["fields"] = json!(FIELDS.iter().map(|(field,_,kind,required)|json!({"name":field,"label":labels["people"]["form"][*field],"kind":kind,"required":required,"value":forms::field(draft,field),"invalid":errors.iter().any(|error|error["field"]==*field)})).collect::<Vec<_>>());
    let types: &[&str] = if person.is_some() {
        &["adult", "minor", "dependent_adult"]
    } else {
        &["minor", "dependent_adult"]
    };
    data["types"] = json!(
        types
            .iter()
            .map(|value| json!({"value":value,"label":labels["forms"]["medications"]["wizard"]["dose"]["person_types"][*value]}))
            .collect::<Vec<_>>()
    );
    data["type_invalid"] = json!(errors.iter().any(|error| error["field"] == "person_type"));
    data["capacity_invalid"] = json!(errors.iter().any(|error| error["field"] == "has_capacity"));
    data["focus_field"] = errors
        .first()
        .map_or(Value::Null, |error| error["field"].clone());
    data["form_error_heading"] = labels["people"]["index"]["error_heading"].clone();
    data["errors"] = json!(errors);
    data
}

fn type_label(value: PersonKind) -> &'static str {
    match value {
        PersonKind::Minor => "Minor",
        PersonKind::DependentAdult => "Dependent adult",
        PersonKind::Adult => "Adult",
    }
}

fn errors(error: Option<&OperationError>) -> Vec<Value> {
    let Some(error) = error else {
        return Vec::new();
    };
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
                        "person_type" => "Person Type",
                        "has_capacity" => "Capacity",
                        _ => "Person",
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
    vec![json!({"field":"person-form","message":forms::message(error)})]
}

pub(super) fn render(
    view: &TeraView,
    token: &CsrfToken,
    template: &str,
    mut data: Value,
    error: Option<&OperationError>,
) -> Response {
    let Ok(authenticity) = token.authenticity_token() else {
        return unavailable();
    };
    data["authenticity_token"] = json!(authenticity);
    match format::render().view(view, template, data) {
        Ok(response) => (
            error.map(forms::status).unwrap_or(StatusCode::OK),
            token.clone(),
            [(header::CACHE_CONTROL, "no-store")],
            response,
        )
            .into_response(),
        Err(_) => unavailable(),
    }
}
