use super::*;

pub(super) fn data(
    slug: &str,
    records: &Value,
    options: &Value,
    submitted: Option<&[(String, String)]>,
    error: Option<&OperationError>,
) -> Value {
    let mut data = super::super::medications::rendering::appearance_context();
    data["title"] = json!("Invitations");
    data["slug"] = json!(slug);
    let defaults = vec![
        ("email".into(), String::new()),
        ("membership_role".into(), "member".into()),
        ("relationship_type".into(), String::new()),
        ("access_level".into(), "record".into()),
    ];
    let draft = submitted.unwrap_or(&defaults);
    data["draft"] = json!({"email":field(draft,"email"),"membership_role":field(draft,"membership_role"),"relationship_type":field(draft,"relationship_type"),"access_level":field(draft,"access_level")});
    data["roles"] = json!(["member", "administrator"]);
    data["relationship_types"] = json!(["parent", "carer", "family_member", "professional"]);
    data["access_levels"] = json!(["view", "record", "manage"]);
    data["dependents"] = json!(
        options["dependents"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|dependent| {
                let mut row = dependent.clone();
                row["selected"] = json!(dependent["id"].as_i64().is_some_and(|id| {
                    draft
                        .iter()
                        .any(|(key, value)| key == "dependent_ids[]" && value == &id.to_string())
                }));
                row
            })
            .collect::<Vec<_>>()
    );
    data["invitations"] = json!(
        records["data"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|record| {
                let mut row = record.clone();
                row["actionable"] =
                    json!(record["accepted_at"].is_null() && record["revoked_at"].is_null());
                row["status"] = json!(if !record["accepted_at"].is_null() {
                    "Accepted"
                } else if !record["revoked_at"].is_null() {
                    "Revoked"
                } else if record["pending"] == true {
                    "Pending"
                } else {
                    "Expired"
                });
                row
            })
            .collect::<Vec<_>>()
    );
    let errors = errors(error);
    for field in [
        "email",
        "membership_role",
        "relationship_type",
        "access_level",
        "dependent_ids",
    ] {
        data[format!("{field}_invalid")] =
            json!(errors.iter().any(|error| error["field"] == field));
    }
    data["errors"] = json!(errors);
    data
}

fn errors(error: Option<&OperationError>) -> Vec<Value> {
    let Some(error) = error else {
        return Vec::new();
    };
    if let OperationError::Validation { details } = error
        && let Some(fields) = details.get("errors").and_then(Value::as_object)
    {
        return fields.iter().map(|(field,messages)| {
            let label=match field.as_str(){"email"=>"Email","membership_role"=>"Household role","relationship_type"=>"Dependent relationship","access_level"=>"Dependent access","dependent_ids"=>"Existing dependents",_=>""};
            let message=messages.as_array().map(|values|values.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", ")).unwrap_or_else(||messages.as_str().unwrap_or("is invalid").into());
            json!({"field":if field=="base"{"invitation-form"}else{field},"message":format!("{label} {message}").trim()})
        }).collect();
    }
    vec![json!({"field":"invitation-form","message":forms::message(error)})]
}

pub(super) fn render(
    view: &TeraView,
    token: &CsrfToken,
    mut data: Value,
    error: Option<&OperationError>,
) -> Response {
    let Ok(authenticity) = token.authenticity_token() else {
        return unavailable();
    };
    data["authenticity_token"] = json!(authenticity);
    match format::render().view(view, "invitations/index.html", data) {
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
