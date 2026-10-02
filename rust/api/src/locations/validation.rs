use serde_json::json;
use serde_json::Value;

pub(super) fn valid_identifier(value: &str) -> bool {
    valid_numeric_id(value)
        || (value.len() == 36
            && value.bytes().enumerate().all(|(index, byte)| match index {
                8 | 13 | 18 | 23 => byte == b'-',
                19 => matches!(byte, b'8' | b'9' | b'a' | b'b' | b'A' | b'B'),
                _ => byte.is_ascii_hexdigit(),
            }))
}

pub(super) fn valid_numeric_id(value: &str) -> bool {
    value
        .as_bytes()
        .first()
        .is_some_and(|byte| (b'1'..=b'9').contains(byte))
        && value.bytes().all(|byte| byte.is_ascii_digit())
}

pub(super) fn attributes(
    body: &Value,
    create: bool,
) -> Option<(Option<String>, Option<Option<String>>)> {
    let outer = body.as_object()?;
    if outer.len() != 1 {
        return None;
    }
    let inner = outer.get("location")?.as_object()?;
    if inner
        .keys()
        .any(|key| key != "name" && key != "description")
        || inner.is_empty()
        || (create && !inner.contains_key("name"))
    {
        return None;
    }
    let name = match inner.get("name") {
        Some(value) => {
            let value = value.as_str()?;
            if value.trim().is_empty() {
                return None;
            }
            Some(value.to_owned())
        }
        None => None,
    };
    let description = match inner.get("description") {
        Some(Value::Null) => Some(None),
        Some(Value::String(value)) => Some(Some(value.clone())),
        Some(_) => return None,
        None => None,
    };
    Some((name, description))
}

pub(super) fn attribute_errors(body: &Value, create: bool) -> Value {
    match body.get("location").and_then(|value| value.get("name")) {
        Some(Value::String(value)) if value.trim().is_empty() => {
            json!({"name": ["can't be blank"]})
        }
        Some(Value::String(_)) => json!({"location": ["is invalid"]}),
        Some(_) => json!({"name": ["is invalid"]}),
        None if create => json!({"name": ["can't be blank"]}),
        None => json!({"location": ["is invalid"]}),
    }
}
