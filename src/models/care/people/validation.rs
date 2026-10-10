use super::*;

pub(super) fn assign(
    record: &mut person::Model,
    attributes: &Value,
    create: bool,
) -> Result<(), OperationError> {
    let fields = attributes
        .as_object()
        .ok_or_else(|| invalid("person", "is invalid"))?;
    if fields.is_empty()
        || fields.keys().any(|name| {
            !matches!(
                name.as_str(),
                "name" | "email" | "date_of_birth" | "person_type" | "has_capacity"
            )
        })
    {
        return Err(invalid("person", "is invalid"));
    }
    if let Some(value) = fields.get("name") {
        record.name = value
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| invalid("name", "can't be blank"))?
            .to_owned();
    } else if create {
        return Err(invalid("name", "can't be blank"));
    }
    if let Some(value) = fields.get("email") {
        record.email = match value {
            Value::Null => None,
            Value::String(value) => {
                let value = value.trim().to_lowercase();
                if !value.is_empty() && !email_valid(&value) {
                    return Err(invalid("email", "is invalid"));
                }
                (!value.is_empty()).then_some(value)
            }
            _ => return Err(invalid("email", "is invalid")),
        };
    }
    if let Some(value) = fields.get("date_of_birth") {
        record.date_of_birth = Some(
            NaiveDate::parse_from_str(
                value
                    .as_str()
                    .ok_or_else(|| invalid("date_of_birth", "is invalid"))?,
                "%Y-%m-%d",
            )
            .map_err(|_| invalid("date_of_birth", "is invalid"))?,
        );
    } else if create {
        return Err(invalid("date_of_birth", "can't be blank"));
    }
    if let Some(value) = fields.get("person_type") {
        record.person_type = match value.as_str() {
            Some("adult") => 0,
            Some("minor") => 1,
            Some("dependent_adult") => 2,
            _ => return Err(invalid("person_type", "is invalid")),
        };
    }
    if let Some(value) = fields.get("has_capacity") {
        record.has_capacity = value
            .as_bool()
            .ok_or_else(|| invalid("has_capacity", "is invalid"))?;
    }
    Ok(())
}

fn email_valid(value: &str) -> bool {
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
