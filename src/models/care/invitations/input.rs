use super::*;

#[derive(Debug)]
pub(crate) struct EmailValidationError;

impl From<EmailValidationError> for OperationError {
    fn from(_: EmailValidationError) -> Self {
        invalid("email", "is invalid")
    }
}

pub(crate) fn validate_email(value: &str) -> Result<String, EmailValidationError> {
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
        return Err(EmailValidationError);
    }
    Ok(email)
}

pub(crate) fn validated_email(value: &str) -> Result<String, OperationError> {
    validate_email(value).map_err(Into::into)
}

pub struct InvitationInput {
    email: String,
    role: String,
    care_authority_requested: bool,
}

impl InvitationInput {
    pub fn from_fields(fields: &serde_json::Map<String, Value>) -> Self {
        let email = fields.get("email").and_then(Value::as_str).unwrap_or("");
        let role = fields
            .get("membership_role")
            .and_then(Value::as_str)
            .unwrap_or("member");
        let care_authority_requested = [
            "dependent_ids",
            "dependent_ids[]",
            "relationship_type",
            "access_level",
        ]
        .iter()
        .any(|field| fields.contains_key(*field));
        Self::from_form(email, role, care_authority_requested)
    }

    pub fn from_form(email: &str, role: &str, care_authority_requested: bool) -> Self {
        Self {
            email: email.into(),
            role: role.into(),
            care_authority_requested,
        }
    }

    pub(super) fn validated(self) -> Result<(String, String), OperationError> {
        if self.care_authority_requested {
            return Err(invalid(
                "base",
                "Household invitations cannot assign care authority.",
            ));
        }
        let email = validate_email(&self.email).map_err(OperationError::from)?;
        if !matches!(self.role.as_str(), "member" | "administrator") {
            return Err(invalid("membership_role", "is invalid"));
        }
        Ok((email, self.role))
    }
}
