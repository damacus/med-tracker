use super::{OperationError, TeraView, format, forms, unavailable};
use axum::{
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use axum_csrf::CsrfToken;
use serde_json::{Map, Value, json};
use std::collections::HashMap;

pub(super) fn context(
    slug: &str,
    error: Option<&OperationError>,
) -> Result<Map<String, Value>, OperationError> {
    let Value::Object(mut data) = super::super::medications::rendering::appearance_context() else {
        return Err(OperationError::Unavailable);
    };
    data.insert("slug".into(), json!(slug));
    data.insert("title".into(), json!("Administration"));
    let errors = field_errors(error);
    for field in [
        "name",
        "role",
        "carer_id",
        "patient_id",
        "relationship_type",
    ] {
        data.insert(
            format!("{field}_invalid"),
            json!(
                errors
                    .iter()
                    .any(|error| error.get("field").and_then(Value::as_str) == Some(field))
            ),
        );
    }
    data.insert("errors".into(), json!(errors));
    data.insert("roles".into(), json!(["owner", "administrator", "member"]));
    data.insert(
        "relationship_types".into(),
        json!(["parent", "family_member", "professional_carer", "self"]),
    );
    Ok(data)
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
            let message = messages.as_array().map_or_else(||messages.as_str().unwrap_or("is invalid").into(), |items| items.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", "));
            json!({"field":if field=="base" {"administration-form"}else{field},"message":format!("{label} {message}").trim()})
        }).collect();
    }
    vec![json!({"field":"administration-form","message":forms::message(error)})]
}

pub(super) fn render(
    view: &TeraView,
    token: &CsrfToken,
    template: &str,
    mut data: Map<String, Value>,
    error: Option<&OperationError>,
) -> Response {
    let Ok(authenticity) = token.authenticity_token() else {
        return unavailable();
    };
    data.insert("authenticity_token".into(), json!(authenticity));
    let data = Value::Object(data);
    match format::render().view(view, template, crate::views::template_data(&data)) {
        Ok(response) => (
            error.map_or(StatusCode::OK, forms::status),
            token.clone(),
            [(header::CACHE_CONTROL, "no-store")],
            response,
        )
            .into_response(),
        Err(_) => unavailable(),
    }
}

pub(super) fn members(value: &Value) -> std::result::Result<Vec<Value>, OperationError> {
    let rows = value
        .get("data")
        .and_then(Value::as_array)
        .ok_or(OperationError::Unavailable)?;
    let mut active = Vec::new();
    for row in rows {
        let row = row.as_object().ok_or(OperationError::Unavailable)?;
        let status = row
            .get("status")
            .and_then(Value::as_str)
            .ok_or(OperationError::Unavailable)?;
        if status != "active" {
            continue;
        }
        let id = row
            .get("id")
            .and_then(Value::as_i64)
            .ok_or(OperationError::Unavailable)?;
        let name = row
            .get("person_name")
            .and_then(Value::as_str)
            .ok_or(OperationError::Unavailable)?;
        let role = row
            .get("role")
            .and_then(Value::as_str)
            .ok_or(OperationError::Unavailable)?;
        active.push(json!({"id":id,"name":name,"role":role}));
    }
    Ok(active)
}

pub(super) fn selected(
    options: &mut Value,
    draft: &HashMap<String, String>,
) -> Result<(), OperationError> {
    for key in ["carers", "patients"] {
        let rows = options
            .get(key)
            .and_then(Value::as_array)
            .ok_or(OperationError::Unavailable)?;
        if rows.iter().any(|row| {
            row.as_object()
                .and_then(|row| row.get("id"))
                .and_then(Value::as_i64)
                .is_none()
        }) {
            return Err(OperationError::Unavailable);
        }
    }
    for (key, field) in [("carers", "carer_id"), ("patients", "patient_id")] {
        let rows = options
            .get_mut(key)
            .and_then(Value::as_array_mut)
            .ok_or(OperationError::Unavailable)?;
        for row in rows {
            let row = row.as_object_mut().ok_or(OperationError::Unavailable)?;
            let id = row
                .get("id")
                .and_then(Value::as_i64)
                .ok_or(OperationError::Unavailable)?;
            row.insert(
                "selected".into(),
                json!(id.to_string() == forms::field(draft, field)),
            );
        }
    }
    Ok(())
}
