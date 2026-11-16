use super::*;
use crate::controllers::shell::ShellView as TeraView;

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

pub(super) fn valid_return_to(slug: &str, value: Option<&str>) -> Option<String> {
    let value = value.map(str::trim).filter(|value| !value.is_empty())?;
    let prefix = format!("/households/{slug}/medications/");
    if !value.starts_with(&prefix)
        || value.contains('#')
        || value.contains('\\')
        || value.chars().any(|ch| ch.is_control())
    {
        return None;
    }
    let parsed = url::Url::parse(&format!("http://localhost{value}")).ok()?;
    if parsed.host_str() != Some("localhost")
        || !parsed.path().starts_with(&prefix)
        || parsed.path().contains("//")
        || parsed.path().contains("..")
    {
        return None;
    }
    Some(value.to_owned())
}

pub(super) fn return_to_destination(
    slug: &str,
    return_to: Option<&str>,
    destination_id: i64,
) -> Option<String> {
    let return_to = valid_return_to(slug, return_to)?;
    let mut parsed = url::Url::parse(&format!("http://localhost{return_to}")).ok()?;
    let pairs: Vec<(String, String)> = parsed
        .query_pairs()
        .filter(|(key, _)| key != "destination_id")
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    {
        let mut query = parsed.query_pairs_mut();
        query.clear();
        for (key, value) in &pairs {
            query.append_pair(key, value);
        }
        query.append_pair("destination_id", &destination_id.to_string());
    }
    Some(match parsed.query() {
        Some(query) if !query.is_empty() => format!("{}?{query}", parsed.path()),
        _ => parsed.path().to_owned(),
    })
}

pub(super) fn draft(
    slug: &str,
    record: Option<&location::Model>,
    return_to: Option<&str>,
) -> HashMap<String, String> {
    let mut draft = HashMap::from([
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
    ]);
    if record.is_none()
        && let Some(return_to) = valid_return_to(slug, return_to)
    {
        draft.insert("return_to".into(), return_to);
    }
    draft
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
    if record.is_none()
        && let Some(return_to) = valid_return_to(slug, draft.get("return_to").map(String::as_str))
    {
        data["return_to"] = json!(return_to);
        data["back_url"] = json!(return_to);
    }
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
