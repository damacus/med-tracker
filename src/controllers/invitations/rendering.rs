use super::*;

pub(super) fn data(
    slug: &str,
    household_name: &str,
    records: &invitations::browser::BrowsePage,
    submitted: Option<&[(String, String)]>,
    error: Option<&OperationError>,
    labels: &Value,
    zone: chrono_tz::Tz,
) -> Result<Value, OperationError> {
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
            Ok(InvitationView {
                id: row.id,
                email: &row.email,
                membership_role: row.membership_role.key(),
                status: index["status"][status_key].as_str().unwrap_or_default(),
                actionable: row.status.actionable(),
                expires_at: row.expires_at,
                expires_at_display: timestamp(row.expires_at, labels, zone)?,
                created_at: row.created_at,
                created_at_display: timestamp(row.created_at, labels, zone)?,
                delivery_status: index["delivery"]["status"][row.delivery.status.key()]
                    .as_str()
                    .unwrap_or_default(),
                delivery_failed: matches!(
                    row.delivery.status,
                    invitations::browser::DeliveryStatus::Failed
                ),
                delivery_category: row
                    .delivery
                    .category
                    .and_then(|category| index["delivery"]["category"][category.key()].as_str()),
                delivery_retry: !matches!(
                    row.delivery.category,
                    Some(invitations::browser::FailureCategory::Unavailable)
                ),
                delivery_at: row.delivery.at,
                delivery_at_display: row
                    .delivery
                    .at
                    .map(|at| timestamp(at, labels, zone))
                    .transpose()?,
            })
        })
        .collect::<Result<Vec<_>, OperationError>>()?;
    data["invitations"] = json!(rows);
    let errors = errors(error, index);
    for field in ["email", "membership_role"] {
        data[format!("{field}_invalid")] =
            json!(errors.iter().any(|error| error["field"] == field));
    }
    data["errors"] = json!(errors);
    data["form_error_heading"] = index["validation"]["heading"].clone();
    Ok(data)
}

fn timestamp(
    at: chrono::DateTime<chrono::Utc>,
    labels: &Value,
    zone: chrono_tz::Tz,
) -> Result<String, OperationError> {
    let local = at.with_timezone(&zone);
    let date = crate::models::care::people::browser::birth_date(
        Some(&local.date_naive().to_string()),
        labels,
    )
    .ok_or(OperationError::Unavailable)?;
    Ok(format!("{} {}", date, local.format("%H:%M %Z")))
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
    created_at: chrono::DateTime<chrono::Utc>,
    created_at_display: String,
    delivery_status: &'a str,
    delivery_failed: bool,
    delivery_retry: bool,
    delivery_category: Option<&'a str>,
    delivery_at: Option<chrono::DateTime<chrono::Utc>>,
    delivery_at_display: Option<String>,
}
