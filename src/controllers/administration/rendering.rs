use super::*;

pub(super) fn error_page(view: &TeraView, language: &str, error: &OperationError) -> Response {
    let key = match error {
        OperationError::Forbidden => "forbidden",
        OperationError::NotFound => "not_found",
        _ => "unavailable",
    };
    render_error_page(view, language, key, forms::status(error))
}

pub(super) fn invalid_link(view: &TeraView, language: &str) -> Response {
    render_error_page(
        view,
        language,
        "invalid_link",
        StatusCode::UNPROCESSABLE_ENTITY,
    )
}

fn render_error_page(view: &TeraView, language: &str, key: &str, status: StatusCode) -> Response {
    let Ok(labels) = crate::models::care::report_pdf::translations(language) else {
        return unavailable();
    };
    let mut data = super::super::medications::rendering::appearance_context();
    data["lang"] = json!(language);
    data["labels"] = labels.clone();
    data["error_labels"] = labels["administration"]["errors"].clone();
    data["title"] = labels["administration"]["errors"][key].clone();
    let response = match format::render().view(view, "administration/error.html", data) {
        Ok(response) => response,
        Err(_) => return unavailable(),
    };
    (status, [(header::CACHE_CONTROL, "no-store")], response).into_response()
}

pub(super) fn context(slug: &str, error: Option<&OperationError>) -> Value {
    let mut data = super::super::medications::rendering::appearance_context();
    data["slug"] = json!(slug);
    let errors = field_errors(error);
    for field in [
        "name",
        "role",
        "carer_id",
        "patient_id",
        "relationship_type",
    ] {
        data[format!("{field}_invalid")] =
            json!(errors.iter().any(|error| error["field"] == field));
    }
    data["errors"] = json!(errors);
    data["roles"] = json!(["owner", "administrator", "member"]);
    data["relationship_types"] = json!(["parent", "family_member", "professional_carer", "self"]);
    data
}

fn field_errors(error: Option<&OperationError>) -> Vec<Value> {
    let Some(error) = error else {
        return Vec::new();
    };
    if let OperationError::Validation { details } = error
        && let Some(fields) = details.get("errors").and_then(Value::as_object)
    {
        return fields.iter().map(|(field, messages)| {
            let field=match field.as_str() {"carer"=>"carer_id","patient"=>"patient_id",field=>field};
            let label = match field {
                "name" => "Household name", "role" => "Household role", "carer_id" => "Carer", "patient_id" => "Patient", "relationship_type" => "Relationship type", _ => "",
            };
            let message = messages.as_array().map(|items| items.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", ")).unwrap_or_else(||messages.as_str().unwrap_or("is invalid").into());
            json!({"field":if field=="base" {"administration-form"}else{field},"message":format!("{label} {message}").trim()})
        }).collect();
    }
    vec![json!({"field":"administration-form","message":forms::message(error)})]
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

pub(super) fn selected(options: &mut Value, draft: &HashMap<String, String>) {
    for (key, field) in [("carers", "carer_id"), ("patients", "patient_id")] {
        if let Some(rows) = options[key].as_array_mut() {
            for row in rows {
                row["selected"] = json!(
                    row["id"]
                        .as_i64()
                        .is_some_and(|id| id.to_string() == forms::field(draft, field))
                );
            }
        }
    }
}
