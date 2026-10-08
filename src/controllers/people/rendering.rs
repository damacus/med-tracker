use super::*;

const FIELDS: &[(&str, &str, &str, bool)] = &[
    ("name", "Name", "text", true),
    ("email", "Email", "email", false),
    ("date_of_birth", "Date of Birth", "date", true),
];

fn context(slug: &str) -> Value {
    let mut data = super::super::medications::rendering::appearance_context();
    data["slug"] = json!(slug);
    data
}

pub(super) fn index(slug: &str, collection: Value, can_create: bool) -> Value {
    let mut data = context(slug);
    data["title"] = json!("People");
    let mut records = collection["data"].clone();
    if let Some(records) = records.as_array_mut() {
        for record in records {
            record["age_present"] = json!(!record["age"].is_null());
        }
    }
    let page = collection["meta"]["page"].as_i64().unwrap_or(1);
    let size = collection["meta"]["per_page"].as_i64().unwrap_or(20);
    let total = collection["meta"]["total_count"].as_i64().unwrap_or(0);
    data["people"] = records;
    data["per_page"] = json!(size);
    data["previous_page"] = json!((page > 1).then_some(page - 1));
    data["next_page"] = json!((page.saturating_mul(size) < total).then_some(page + 1));
    data["can_create"] = json!(can_create);
    data
}

pub(super) fn detail(
    slug: &str,
    person: Value,
    medications: &[Value],
    can_manage: bool,
    can_manage_household: bool,
) -> Value {
    let mut data = context(slug);
    data["title"] = person["name"].clone();
    data["avatar_initials"] = json!(
        person["name"]
            .as_str()
            .unwrap_or_default()
            .split_whitespace()
            .filter_map(|word| word.chars().next())
            .take(2)
            .map(|letter| letter.to_uppercase().to_string())
            .collect::<Vec<_>>()
            .join("")
    );
    data["type_label"] = json!(type_label(
        person["person_type"].as_str().unwrap_or("adult")
    ));
    data["person"] = person;
    data["age_present"] = json!(!data["person"]["age"].is_null());
    data["date_of_birth_display"] = json!(
        data["person"]["date_of_birth"]
            .as_str()
            .filter(|value| !value.is_empty())
            .unwrap_or("Not recorded")
    );
    data["email_display"] = json!(
        data["person"]["email"]
            .as_str()
            .filter(|value| !value.is_empty())
            .unwrap_or("Not recorded")
    );
    data["medications"] = json!(medications);
    data["can_manage"] = json!(can_manage);
    data["can_manage_household"] = json!(can_manage_household);
    data
}

pub(super) fn draft(person: Option<&Value>) -> HashMap<String, String> {
    let mut draft = FIELDS
        .iter()
        .map(|(field, _, _, _)| {
            (
                (*field).into(),
                person
                    .and_then(|person| person[*field].as_str())
                    .unwrap_or_default()
                    .into(),
            )
        })
        .collect::<HashMap<_, _>>();
    draft.insert(
        "person_type".into(),
        person
            .and_then(|person| person["person_type"].as_str())
            .unwrap_or("adult")
            .into(),
    );
    draft.insert(
        "has_capacity".into(),
        person
            .is_none_or(|person| person["has_capacity"].as_bool().unwrap_or(false))
            .to_string(),
    );
    draft
}

pub(super) fn attributes(draft: &HashMap<String, String>) -> Value {
    let capacity = match draft.get("has_capacity").map(String::as_str) {
        None | Some("false") => json!(false),
        Some("true") => json!(true),
        Some(value) => json!(value),
    };
    json!({"name":forms::field(draft,"name"),"email":forms::field(draft,"email"),"date_of_birth":forms::field(draft,"date_of_birth"),"person_type":forms::field(draft,"person_type"),"has_capacity":capacity})
}

pub(super) fn form(
    slug: &str,
    person: Option<&Value>,
    draft: &HashMap<String, String>,
    error: Option<&OperationError>,
) -> Value {
    let mut data = context(slug);
    let errors = errors(error);
    data["title"] = json!(if person.is_some() {
        "Edit Person"
    } else {
        "New Person"
    });
    data["editing"] = json!(person.is_some());
    data["action"] = json!(person.and_then(|person| person["id"].as_i64()).map_or_else(
        || format!("/households/{slug}/people"),
        |id| format!("/households/{slug}/people/{id}")
    ));
    let mut values = draft.clone();
    values
        .entry("has_capacity".into())
        .or_insert_with(|| "false".into());
    data["draft"] = json!(values);
    data["fields"] = json!(FIELDS.iter().map(|(field,label,kind,required)|json!({"name":field,"label":label,"kind":kind,"required":required,"value":forms::field(draft,field),"invalid":errors.iter().any(|error|error["field"]==*field)})).collect::<Vec<_>>());
    data["types"] = json!(
        ["adult", "minor", "dependent_adult"]
            .map(|value| json!({"value":value,"label":type_label(value)}))
    );
    data["type_invalid"] = json!(errors.iter().any(|error| error["field"] == "person_type"));
    data["capacity_invalid"] = json!(errors.iter().any(|error| error["field"] == "has_capacity"));
    data["errors"] = json!(errors);
    data
}

fn type_label(value: &str) -> &str {
    match value {
        "minor" => "Minor",
        "dependent_adult" => "Dependent adult",
        _ => "Adult",
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
