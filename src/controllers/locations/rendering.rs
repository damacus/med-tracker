use super::*;

fn context(slug: &str, error: Option<&OperationError>) -> Value {
    let mut data = super::super::medications::rendering::appearance_context();
    data["slug"] = json!(slug);
    let errors = errors(error);
    data["name_invalid"] = json!(errors.iter().any(|error| error["field"] == "name"));
    data["description_invalid"] = json!(errors.iter().any(|error| error["field"] == "description"));
    data["changed"] = json!(matches!(error, Some(OperationError::Conflict { .. })));
    data["errors"] = json!(errors);
    data
}

pub(super) fn index(slug: &str, records: &[location::Model], can_manage: bool) -> Value {
    let mut data = context(slug, None);
    data["title"] = json!("Locations");
    data["locations"] = json!(records.iter().map(location_data).collect::<Vec<_>>());
    data["can_manage"] = json!(can_manage);
    data
}

pub(super) fn detail(
    slug: &str,
    record: &location::Model,
    medications: &[browser_query::MedicationCard],
    can_manage: bool,
    error: Option<&OperationError>,
) -> Value {
    let mut data = context(slug, error);
    data["title"] = json!(record.name);
    data["location"] = location_data(record);
    data["etag"] = json!(locations::representation(record).1);
    data["medications"] = json!(medications);
    data["can_manage"] = json!(can_manage);
    data
}

fn location_data(record: &location::Model) -> Value {
    json!({"id":record.id,"name":record.name,"description":record.description})
}

pub(super) fn draft(record: Option<&location::Model>) -> HashMap<String, String> {
    HashMap::from([
        (
            "name".into(),
            record.map(|record| record.name.clone()).unwrap_or_default(),
        ),
        (
            "description".into(),
            record
                .and_then(|record| record.description.clone())
                .unwrap_or_default(),
        ),
        (
            "etag".into(),
            record
                .map(|record| locations::representation(record).1)
                .unwrap_or_default(),
        ),
    ])
}

pub(super) fn form(
    slug: &str,
    record: Option<&location::Model>,
    draft: &HashMap<String, String>,
    error: Option<&OperationError>,
) -> Value {
    let mut data = context(slug, error);
    data["title"] = json!(if record.is_some() {
        "Edit Location"
    } else {
        "New Location"
    });
    data["action"] = json!(record.map_or_else(
        || format!("/households/{slug}/locations"),
        |record| format!("/households/{slug}/locations/{}", record.id)
    ));
    data["latest_url"] =
        json!(record.map(|record| format!("/households/{slug}/locations/{}/edit", record.id)));
    data["draft"] = json!(draft);
    data
}

fn errors(error: Option<&OperationError>) -> Vec<Value> {
    let Some(error) = error else {
        return Vec::new();
    };
    if let OperationError::Conflict { .. } = error {
        return vec![
            json!({"field":"location-form","message":"Location changed while this form was open. Review the latest details before saving."}),
        ];
    }
    if let OperationError::Validation { details } = error
        && let Some(fields) = details.get("errors").and_then(Value::as_object)
    {
        return fields
            .iter()
            .map(|(field, messages)| {
                let label = match field.as_str() {
                    "name" => "Name",
                    "description" => "Description",
                    _ => "Location",
                };
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
    vec![json!({"field":"location-form","message":forms::message(error)})]
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
