use super::*;

pub(super) fn create_attributes(
    body: &Value,
) -> Result<(Kind, &str, &str, Option<String>), Failure> {
    let outer = body.as_object().ok_or(Failure::Malformed)?;
    let attributes = outer
        .get("medication_pause_period")
        .and_then(Value::as_object)
        .ok_or(Failure::Malformed)?;
    if outer.len() != 1
        || attributes.keys().any(|key| {
            !matches!(
                key.as_str(),
                "source_type" | "source_id" | "reason" | "note"
            )
        })
    {
        return Err(Failure::Invalid(
            "medication_pause_period",
            "contains an unsupported field",
        ));
    }
    let kind = attributes
        .get("source_type")
        .and_then(Value::as_str)
        .and_then(Kind::parse)
        .ok_or(Failure::Invalid("source_type", "is invalid"))?;
    let source_id = attributes
        .get("source_id")
        .and_then(Value::as_str)
        .filter(|id| Uuid::parse_str(id).is_ok())
        .ok_or(Failure::Invalid(
            "source_id",
            "must be a portable identifier",
        ))?;
    let reason = attributes
        .get("reason")
        .and_then(Value::as_str)
        .filter(|reason| REASONS.contains(reason))
        .ok_or(Failure::Invalid("reason", "is invalid"))?;
    let note = match attributes.get("note") {
        None | Some(Value::Null) => None,
        Some(Value::String(note)) => Some(note.clone()),
        _ => return Err(Failure::Invalid("note", "must be a string")),
    };
    Ok((kind, source_id, reason, note))
}

pub(super) fn source_identity(body: &Value) -> Result<(Kind, &str), Failure> {
    let outer = body.as_object().ok_or(Failure::Malformed)?;
    let attributes = outer
        .get("medication_pause_period")
        .and_then(Value::as_object)
        .ok_or(Failure::Malformed)?;
    let kind = attributes
        .get("source_type")
        .and_then(Value::as_str)
        .and_then(Kind::parse)
        .ok_or(Failure::Invalid("source_type", "is invalid"))?;
    let source_id = attributes
        .get("source_id")
        .and_then(Value::as_str)
        .filter(|id| Uuid::parse_str(id).is_ok())
        .ok_or(Failure::Invalid(
            "source_id",
            "must be a portable identifier",
        ))?;
    Ok((kind, source_id))
}

pub(super) fn empty_request(payload: &Bytes) -> Result<Value, Failure> {
    if payload.is_empty() {
        return Ok(json!({}));
    }
    let body: Value = serde_json::from_slice(payload).map_err(|_| Failure::Malformed)?;
    if body.as_object().is_none_or(|body| !body.is_empty()) {
        return Err(Failure::Invalid("body", "must be empty"));
    }
    Ok(body)
}
