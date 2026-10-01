use crate::read_entities::person;
use crate::read_resources::age;
use crate::read_resources::today;
use axum::http::StatusCode;
use chrono::NaiveDate;
use serde_json::json;
use serde_json::Value;

pub(super) struct Attributes {
    pub(super) name: Option<String>,
    pub(super) email: Option<Option<String>>,
    pub(super) date_of_birth: Option<NaiveDate>,
    pub(super) person_type: Option<i32>,
    pub(super) has_capacity: Option<bool>,
}

pub(super) fn valid_identifier(value: &str) -> bool {
    let numeric = value
        .as_bytes()
        .first()
        .is_some_and(|byte| (b'1'..=b'9').contains(byte))
        && value.bytes().all(|byte| byte.is_ascii_digit());
    numeric
        || (value.len() == 36
            && value.bytes().enumerate().all(|(index, byte)| match index {
                8 | 13 | 18 | 23 => byte == b'-',
                19 => matches!(byte, b'8' | b'9' | b'a' | b'b' | b'A' | b'B'),
                _ => byte.is_ascii_hexdigit(),
            }))
}

fn email_valid(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    if local.is_empty()
        || !local
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b".!#$%&'*+/=?^_`{|}~-".contains(&byte))
    {
        return false;
    }
    domain.split('.').all(|label| {
        label.len() <= 63
            && label
                .bytes()
                .next()
                .is_some_and(|byte| byte.is_ascii_alphanumeric())
            && label
                .bytes()
                .last()
                .is_some_and(|byte| byte.is_ascii_alphanumeric())
            && label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    })
}

pub(super) fn parse_attributes(body: &Value, create: bool) -> Result<Attributes, StatusCode> {
    let outer = body.as_object().ok_or(StatusCode::BAD_REQUEST)?;
    let inner = outer.get("person").ok_or(StatusCode::BAD_REQUEST)?;
    let inner = inner.as_object().ok_or(StatusCode::BAD_REQUEST)?;
    if outer.len() != 1
        || inner.is_empty()
        || inner.keys().any(|key| {
            !matches!(
                key.as_str(),
                "name" | "email" | "date_of_birth" | "person_type" | "has_capacity"
            )
        })
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let name = match inner.get("name") {
        Some(value) => {
            let text = value.as_str().ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
            if text.trim().is_empty() {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            Some(text.to_owned())
        }
        None => None,
    };
    let email = match inner.get("email") {
        Some(Value::Null) => Some(None),
        Some(Value::String(value)) => {
            let normalized = value.trim().to_lowercase();
            if !normalized.is_empty() && !email_valid(&normalized) {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            Some((!normalized.is_empty()).then_some(normalized))
        }
        Some(_) => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        None => None,
    };
    let date_of_birth = match inner.get("date_of_birth") {
        Some(Value::String(value)) => Some(
            NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?,
        ),
        Some(_) => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        None => None,
    };
    let person_type = match inner.get("person_type") {
        Some(Value::String(value)) => Some(match value.as_str() {
            "adult" => 0,
            "minor" => 1,
            "dependent_adult" => 2,
            _ => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        }),
        Some(_) => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        None => None,
    };
    let has_capacity = match inner.get("has_capacity") {
        Some(Value::Bool(value)) => Some(*value),
        Some(_) => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        None => None,
    };
    if create && (name.is_none() || date_of_birth.is_none()) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    Ok(Attributes {
        name,
        email,
        date_of_birth,
        person_type,
        has_capacity,
    })
}

pub(super) fn name_errors(body: &Value, create: bool) -> Option<Value> {
    match body.get("person")?.get("name") {
        Some(Value::String(value)) if value.trim().is_empty() => {
            Some(json!({"name": ["can't be blank"]}))
        }
        Some(Value::String(_)) => None,
        Some(_) => Some(json!({"name": ["is invalid"]})),
        None if create => Some(json!({"name": ["can't be blank"]})),
        None => None,
    }
}

pub(super) fn person_valid(record: &person::Model, has_carer: bool) -> bool {
    if record.name.trim().is_empty() || record.date_of_birth.is_none() {
        return false;
    }
    if record
        .email
        .as_deref()
        .is_some_and(|value| !email_valid(value))
    {
        return false;
    }
    let Some(years) = age(record.date_of_birth, today()) else {
        return false;
    };
    if (years < 18 && record.person_type == 2) || (years >= 18 && record.person_type == 1) {
        return false;
    }
    if !record.has_capacity && !has_carer {
        return false;
    }
    true
}
