use super::*;

pub(super) fn data(
    slug: &str,
    household_name: &str,
    records: &invitations::browser::BrowsePage,
    submitted: Option<&[(String, String)]>,
    error: Option<&OperationError>,
    labels: &Value,
) -> Value {
    let mut data = super::super::medications::rendering::appearance_context();
    let index = &labels["admin"]["invitations"]["index"];
    data["title"] = json!(format!(
        "{household_name} · {}",
        index["title"].as_str().unwrap_or_default()
    ));
    data["slug"] = json!(slug);
    data["household_name"] = json!(household_name);
    data["admin_section"] = json!("invitations");
    let defaults = vec![
        ("email".into(), String::new()),
        ("membership_role".into(), "member".into()),
    ];
    let draft = submitted.unwrap_or(&defaults);
    data["draft"] =
        json!({"email":field(draft,"email"),"membership_role":field(draft,"membership_role")});
    data["roles"] = json!(["member", "administrator"]);
    let rows = records
        .rows
        .iter()
        .map(|row| {
            use invitations::browser::InvitationStatus;
            let status_key = match row.status {
                InvitationStatus::Pending => "pending",
                InvitationStatus::Accepted => "accepted",
                InvitationStatus::Revoked => "revoked",
                InvitationStatus::Expired => "expired",
            };
            InvitationView {
                id: row.id,
                email: &row.email,
                membership_role: row.membership_role.key(),
                status: index["status"][status_key].as_str().unwrap_or_default(),
                actionable: row.status.actionable(),
                expires_at: row.expires_at,
                expires_at_display: row.expires_at.format("%Y-%m-%d %H:%M UTC").to_string(),
            }
        })
        .collect::<Vec<_>>();
    data["invitations"] = json!(rows);
    let errors = errors(error, index);
    for field in ["email", "membership_role"] {
        data[format!("{field}_invalid")] =
            json!(errors.iter().any(|error| error["field"] == field));
    }
    data["errors"] = json!(errors);
    data["form_error_heading"] = index["validation"]["heading"].clone();
    data
}

fn errors(error: Option<&OperationError>, index: &Value) -> Vec<Value> {
    let Some(error) = error else {
        return Vec::new();
    };
    if let OperationError::Validation { details } = error
        && let Some(fields) = details.get("errors").and_then(Value::as_object)
    {
        return fields.iter().map(|(field,messages)| {
            let message = messages.as_array().and_then(|values| values.first()).and_then(Value::as_str).or_else(|| messages.as_str()).unwrap_or_default();
            let key = match (field.as_str(), message) {
                ("email", "is invalid") => "email_invalid",
                ("email", "has already been taken") => "email_taken",
                ("membership_role", "is invalid") => "role_invalid",
                ("base", "Household invitations cannot assign care authority.") => "care_authority",
                ("base", "Invitation cannot be cancelled") => "cannot_cancel",
                ("base", "Invitation cannot be resent") => "cannot_resend",
                _ => "unavailable",
            };
            let target = if matches!(field.as_str(), "email" | "membership_role") { field.as_str() } else { "invitation-form" };
            json!({"field":target,"message":index["validation"][key].as_str().unwrap_or_default()})
        }).collect();
    }
    vec![
        json!({"field":"invitation-form","message":index["validation"]["unavailable"].as_str().unwrap_or_default()}),
    ]
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

#[derive(serde::Serialize)]
struct InvitationView<'a> {
    id: invitations::browser::InvitationId,
    email: &'a str,
    membership_role: &'a str,
    status: &'a str,
    actionable: bool,
    expires_at: chrono::DateTime<chrono::Utc>,
    expires_at_display: String,
}
