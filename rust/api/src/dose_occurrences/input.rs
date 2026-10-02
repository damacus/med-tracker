use super::*;

const REASONS: &[&str] = &[
    "refused",
    "unwell",
    "asleep",
    "medicine_unavailable",
    "clinician_advice",
    "other",
];

pub(super) fn attributes<'a>(
    body: &'a Value,
    action: &str,
) -> Result<&'a Map<String, Value>, Failure> {
    let outer = body.as_object().ok_or(Failure::Malformed)?;
    let inner = outer
        .get("dose_occurrence")
        .and_then(Value::as_object)
        .ok_or(Failure::Malformed)?;
    if outer.len() != 1 {
        return Err(Failure::Invalid(
            "dose_occurrence",
            "contains an unsupported field",
        ));
    }
    let fields: &[&str] = match action {
        "not_taken" => &["key", "reason", "note"],
        "reopen" => &["key"],
        _ => &[
            "key",
            "taken_at",
            "client_uuid",
            "dose_amount",
            "taken_from_medication_id",
        ],
    };
    if inner.keys().any(|key| !fields.contains(&key.as_str())) {
        return Err(Failure::Invalid(
            "dose_occurrence",
            "contains an unsupported field",
        ));
    }
    if inner
        .get("key")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
    {
        return Err(Failure::Invalid("key", "must be a string"));
    }
    Ok(inner)
}

pub(super) fn parse_not_taken(
    attributes: &Map<String, Value>,
) -> Result<(Option<String>, Option<String>), Failure> {
    let reason = match attributes.get("reason") {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) if REASONS.contains(&value.as_str()) => Some(value.clone()),
        _ => return Err(Failure::Invalid("reason", "is invalid")),
    };
    let note = match attributes.get("note") {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) if value.chars().count() <= 2000 => Some(value.clone()),
        _ => return Err(Failure::Invalid("note", "is invalid")),
    };
    Ok((reason, note))
}

pub(super) fn parse_take(attributes: &Map<String, Value>) -> Result<NaiveDateTime, Failure> {
    let value = attributes
        .get("taken_at")
        .and_then(Value::as_str)
        .ok_or(Failure::Invalid("taken_at", "must be ISO8601"))?;
    let taken_at = chrono::DateTime::parse_from_rfc3339(value)
        .map_err(|_| Failure::Invalid("taken_at", "must be ISO8601"))?
        .naive_utc();
    if let Some(value) = attributes.get("client_uuid") {
        if value
            .as_str()
            .is_none_or(|value| Uuid::parse_str(value).is_err())
        {
            return Err(Failure::Invalid("client_uuid", "must be a UUID"));
        }
    }
    if let Some(value) = attributes.get("dose_amount") {
        if parse_decimal(value).is_none_or(|amount| amount <= sea_orm::prelude::Decimal::ZERO) {
            return Err(Failure::Invalid("dose_amount", "must be a decimal string"));
        }
    }
    if let Some(value) = attributes.get("taken_from_medication_id") {
        if value.as_i64().is_none_or(|value| value <= 0) {
            return Err(Failure::Invalid(
                "taken_from_medication_id",
                "must be a positive integer",
            ));
        }
    }
    Ok(taken_at)
}
