use super::*;

pub(super) fn collection_path(household_id: i64) -> String {
    format!("/api/v1/households/{household_id}/admin/invitations")
}

pub(super) fn valid_id(value: &str) -> Option<i64> {
    let mut chars = value.bytes();
    if !matches!(chars.next(), Some(b'1'..=b'9')) || !chars.all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

pub(super) fn summary(row: &household_invitation::Model, now: NaiveDateTime) -> Value {
    json!({
        "id": row.id,
        "email": row.email,
        "membership_role": row.membership_role,
        "pending": row.accepted_at.is_none() && row.revoked_at.is_none() && row.expires_at > now,
        "accepted_at": row.accepted_at.map(|at| at.and_utc().to_rfc3339()),
        "revoked_at": row.revoked_at.map(|at| at.and_utc().to_rfc3339()),
        "expires_at": row.expires_at.and_utc().to_rfc3339(),
    })
}

pub(super) fn state_row(row: &household_invitation::Model) -> Value {
    json!({
        "email": row.email,
        "membership_role": row.membership_role,
        "accepted_at": row.accepted_at,
        "revoked_at": row.revoked_at,
        "expires_at": row.expires_at,
        "invited_by_membership_id": row.invited_by_membership_id,
    })
}

pub(super) fn valid_email(email: &str) -> bool {
    email.len() <= 320
        && !email.is_empty()
        && !email.contains(char::is_whitespace)
        && email.matches('@').count() == 1
        && email.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty()
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
        })
}

pub(super) fn parse_create(body: &Value) -> Result<(String, String), StatusCode> {
    let outer = body.as_object().ok_or(StatusCode::BAD_REQUEST)?;
    let inner = outer
        .get("household_invitation")
        .and_then(Value::as_object)
        .ok_or(StatusCode::BAD_REQUEST)?;
    if outer.len() != 1
        || inner.len() != 2
        || !inner.contains_key("email")
        || !inner.contains_key("membership_role")
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let email = inner
        .get("email")
        .and_then(Value::as_str)
        .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?
        .trim()
        .to_lowercase();
    let role = inner
        .get("membership_role")
        .and_then(Value::as_str)
        .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
    if !valid_email(&email) || !matches!(role, "administrator" | "member") {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    Ok((email, role.to_owned()))
}

pub(super) fn accepted_body(row: &membership::Model) -> Value {
    json!({"data": {
        "household_id": row.household_id.to_string(),
        "membership_id": row.id.to_string(),
        "person_id": row.person_id.map(|value| value.to_string()),
        "role": row.role,
    }})
}
