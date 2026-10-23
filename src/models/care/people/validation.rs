use super::*;

pub(super) fn assign(
    record: &mut person::Model,
    input: PersonUpdateInput,
) -> Result<(), OperationError> {
    if !input.valid_shape() {
        return Err(invalid("person", "is invalid"));
    }
    match input.name {
        UpdateField::Missing => {}
        UpdateField::Value(value) if !value.trim().is_empty() => record.name = value,
        UpdateField::Value(_) | UpdateField::Invalid => {
            return Err(invalid("name", "can't be blank"));
        }
    }
    match input.email {
        UpdateField::Missing => {}
        UpdateField::Value(value) => {
            let value = value
                .map(|value| value.trim().to_lowercase())
                .filter(|value| !value.is_empty());
            if value.as_deref().is_some_and(|value| !email_valid(value)) {
                return Err(invalid("email", "is invalid"));
            }
            record.email = value;
        }
        UpdateField::Invalid => return Err(invalid("email", "is invalid")),
    }
    match input.date_of_birth {
        UpdateField::Missing => {}
        UpdateField::Value(value) => record.date_of_birth = Some(value),
        UpdateField::Invalid => return Err(invalid("date_of_birth", "is invalid")),
    }
    match input.person_type {
        UpdateField::Missing => {}
        UpdateField::Value(value) => record.person_type = value.code(),
        UpdateField::Invalid => return Err(invalid("person_type", "is invalid")),
    }
    match input.has_capacity {
        UpdateField::Missing => {}
        UpdateField::Value(value) => record.has_capacity = value,
        UpdateField::Invalid => return Err(invalid("has_capacity", "is invalid")),
    }
    Ok(())
}

pub(super) fn email_valid(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && local
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".!#$%&'*+/=?^_`{|}~-".contains(&b))
        && domain.split('.').all(|label| {
            label.len() <= 63
                && label
                    .bytes()
                    .next()
                    .is_some_and(|b| b.is_ascii_alphanumeric())
                && label
                    .bytes()
                    .last()
                    .is_some_and(|b| b.is_ascii_alphanumeric())
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
}

pub(super) fn capacity(
    record: &mut person::Model,
    today: NaiveDate,
) -> Result<bool, OperationError> {
    let birth = record
        .date_of_birth
        .ok_or_else(|| invalid("date_of_birth", "can't be blank"))?;
    let years = age(birth, today);
    if (years < 18 && record.person_type == 2) || (years >= 18 && record.person_type == 1) {
        return Err(invalid("person_type", "does not match age"));
    }
    let dependent =
        (years < 18 && record.person_type == 1) || (years >= 18 && record.person_type == 2);
    if dependent {
        record.has_capacity = false;
    }
    Ok(dependent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn person_update_input_preserves_patch_and_validation_order() {
        let mut record = person::Model {
            name: "Existing".into(),
            email: Some("existing@example.test".into()),
            id: 1,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
            date_of_birth: None,
            has_capacity: true,
            person_type: 0,
            portable_id: String::new(),
            professional_title: None,
        };
        let input = PersonUpdateInput {
            name: UpdateField::Value(" ".into()),
            email: UpdateField::Invalid,
            ..Default::default()
        };
        assert_eq!(
            assign(&mut record, input),
            Err(invalid("name", "can't be blank"))
        );
        assign(
            &mut record,
            PersonUpdateInput {
                email: UpdateField::Value(None),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(record.name, "Existing");
        assert_eq!(record.email, None);
    }

    #[test]
    fn person_update_input_retains_invalid_fields_until_authorised_assignment() {
        for (body, field, message) in [
            (json!({}), "person", "is invalid"),
            (
                json!({"initial_guardian_declaration":true}),
                "person",
                "is invalid",
            ),
            (json!({"name":null,"email":42}), "name", "can't be blank"),
            (json!({"email":false}), "email", "is invalid"),
            (json!({"date_of_birth":null}), "date_of_birth", "is invalid"),
            (
                json!({"date_of_birth":"2020-02-30"}),
                "date_of_birth",
                "is invalid",
            ),
            (
                json!({"person_type":"guardian"}),
                "person_type",
                "is invalid",
            ),
            (json!({"has_capacity":"true"}), "has_capacity", "is invalid"),
        ] {
            let input = PersonUpdateInput::from_fields(body.as_object().unwrap());
            let mut record = person::Model {
                id: 1,
                created_at: Utc::now().naive_utc(),
                updated_at: Utc::now().naive_utc(),
                name: "Existing".into(),
                email: None,
                date_of_birth: None,
                has_capacity: true,
                person_type: 0,
                portable_id: String::new(),
                professional_title: None,
            };
            assert_eq!(
                assign(&mut record, input),
                Err(invalid(field, message)),
                "{body}"
            );
        }
    }
}
