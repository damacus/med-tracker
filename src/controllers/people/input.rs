use super::*;
use crate::models::care::people::{
    DependentInput, DependentKind, PersonKind, PersonUpdateInput, UpdateField,
};

pub(super) fn update(draft: &HashMap<String, String>) -> PersonUpdateInput {
    PersonUpdateInput {
        name: UpdateField::Value(forms::field(draft, "name").into()),
        email: UpdateField::Value(Some(forms::field(draft, "email").into())),
        date_of_birth: forms::field(draft, "date_of_birth")
            .parse()
            .map_or(UpdateField::Invalid, UpdateField::Value),
        person_type: match forms::field(draft, "person_type") {
            "adult" => UpdateField::Value(PersonKind::Adult),
            "minor" => UpdateField::Value(PersonKind::Minor),
            "dependent_adult" => UpdateField::Value(PersonKind::DependentAdult),
            _ => UpdateField::Invalid,
        },
        has_capacity: match draft.get("has_capacity").map(String::as_str) {
            None | Some("false") => UpdateField::Value(false),
            Some("true") => UpdateField::Value(true),
            _ => UpdateField::Invalid,
        },
        invalid_shape: draft.contains_key("initial_guardian_declaration"),
    }
}

fn invalid() -> OperationError {
    OperationError::Validation {
        details: json!({"errors":{"person":["contains unsupported or invalid fields"]}}),
    }
}

pub(super) fn creation(draft: &HashMap<String, String>) -> Result<DependentInput, OperationError> {
    let person_type = match forms::field(draft, "person_type") {
        "minor" => DependentKind::Minor,
        "dependent_adult" => DependentKind::DependentAdult,
        _ => return Err(invalid()),
    };
    let has_capacity = match draft.get("has_capacity").map(String::as_str) {
        None | Some("false") => Some(false),
        Some("true") => Some(true),
        _ => return Err(invalid()),
    };
    let initial_guardian_declaration = match draft
        .get("initial_guardian_declaration")
        .map(String::as_str)
    {
        None => None,
        Some("true") => Some(true),
        Some("false") => Some(false),
        _ => return Err(invalid()),
    };
    Ok(DependentInput {
        name: forms::field(draft, "name").into(),
        email: Some(forms::field(draft, "email").into()),
        date_of_birth: forms::field(draft, "date_of_birth")
            .parse()
            .map_err(|_| invalid())?,
        person_type,
        has_capacity,
        initial_guardian_declaration,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft() -> HashMap<String, String> {
        HashMap::from([
            ("name".into(), "Synthetic child".into()),
            ("date_of_birth".into(), "2020-01-01".into()),
            ("person_type".into(), "minor".into()),
            ("initial_guardian_declaration".into(), "true".into()),
        ])
    }

    #[test]
    fn dependent_input_form_preserves_missing_capacity_and_email_defaults() {
        let input = creation(&draft()).unwrap();
        assert!(matches!(input.person_type, DependentKind::Minor));
        assert_eq!(input.has_capacity, Some(false));
        assert_eq!(input.initial_guardian_declaration, Some(true));
        assert_eq!(input.email.as_deref(), Some(""));
        assert_eq!(input.date_of_birth.to_string(), "2020-01-01");
    }

    #[test]
    fn dependent_input_form_rejects_coerced_booleans_and_invalid_dates() {
        for (field, value) in [
            ("has_capacity", "on"),
            ("has_capacity", ""),
            ("initial_guardian_declaration", "1"),
            ("initial_guardian_declaration", "TRUE"),
            ("date_of_birth", "2020-02-30"),
            ("person_type", "adult"),
        ] {
            let mut draft = draft();
            draft.insert(field.into(), value.into());
            assert!(matches!(
                creation(&draft),
                Err(OperationError::Validation { details })
                    if details == json!({"errors":{"person":["contains unsupported or invalid fields"]}})
            ));
        }
    }
}
