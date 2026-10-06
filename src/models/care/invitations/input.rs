use super::*;

pub(crate) fn validated_email(value: &str) -> Result<String, OperationError> {
    let email = value.trim().to_lowercase();
    let valid_email = email.len() <= 320
        && !email.is_empty()
        && !email.contains(char::is_whitespace)
        && email.matches('@').count() == 1
        && email.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty()
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
        });
    if !valid_email {
        return Err(invalid("email", "is invalid"));
    }
    Ok(email)
}

pub(super) fn attributes(value: &Value) -> Result<(String, String), OperationError> {
    let email = validated_email(value.get("email").and_then(Value::as_str).unwrap_or(""))?;
    let role = value
        .get("membership_role")
        .and_then(Value::as_str)
        .unwrap_or("member");
    if !matches!(role, "member" | "administrator") {
        return Err(invalid("membership_role", "is invalid"));
    }
    Ok((email, role.into()))
}
