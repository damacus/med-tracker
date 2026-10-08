use super::*;

pub(super) fn take(body: Value) -> std::result::Result<doses::Take, response::Failure> {
    let attributes = body
        .get("medication_take")
        .and_then(Value::as_object)
        .ok_or_else(|| response::Failure::bad_request("medication_take is required"))?;
    if body.as_object().is_none_or(|outer| outer.len() != 1) {
        return Err(response::Failure::validation("unknown request field"));
    }
    let allowed = [
        "client_uuid",
        "source_type",
        "source_id",
        "taken_at",
        "dose_amount",
        "dose_unit",
        "taken_from_medication_id",
    ];
    if attributes
        .keys()
        .any(|key| !allowed.contains(&key.as_str()))
    {
        return Err(response::Failure::validation(
            "unknown medication_take field",
        ));
    }
    let source_type = attributes
        .get("source_type")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| response::Failure::validation("invalid medication source"))?;
    let source_id = attributes
        .get("source_id")
        .and_then(Value::as_str)
        .ok_or_else(|| response::Failure::validation("invalid medication source"))?;
    let taken_at = attributes
        .get("taken_at")
        .and_then(Value::as_str)
        .ok_or_else(|| response::Failure::validation("taken_at is invalid"))?;
    let client_uuid = optional_string(attributes, "client_uuid", "invalid client_uuid")?;
    let dose_unit = optional_string(attributes, "dose_unit", "invalid dose_unit")?;
    let dose_amount = match attributes.get("dose_amount") {
        None => None,
        Some(value) => Some(
            value
                .as_str()
                .ok_or_else(response::Failure::numeric_dose)?
                .into(),
        ),
    };
    let taken_from_medication_id =
        match attributes.get("taken_from_medication_id") {
            None => None,
            Some(value) => Some(value.as_i64().filter(|id| *id > 0).ok_or_else(|| {
                response::Failure::validation("invalid taken_from_medication_id")
            })?),
        };
    Ok(doses::Take {
        client_uuid,
        source_type: source_type.into(),
        source_id: source_id.into(),
        taken_at: taken_at.into(),
        dose_amount,
        dose_unit,
        taken_from_medication_id,
        expected_effective_amount: None,
        expected_effective_unit: None,
    })
}

fn optional_string(
    attributes: &serde_json::Map<String, Value>,
    key: &str,
    message: &str,
) -> std::result::Result<Option<String>, response::Failure> {
    attributes
        .get(key)
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| response::Failure::validation(message))
        })
        .transpose()
}

pub(super) fn stock(
    body: Value,
    medication_id: String,
) -> std::result::Result<medications::AdjustStock, response::Failure> {
    let attributes = body
        .get("adjustment")
        .and_then(Value::as_object)
        .ok_or_else(|| response::Failure::bad_request("Invalid request body"))?;
    if body.as_object().is_none_or(|outer| outer.len() != 1)
        || attributes
            .keys()
            .any(|key| !matches!(key.as_str(), "new_quantity" | "reason"))
        || attributes
            .get("reason")
            .is_some_and(|value| !value.is_string())
    {
        return Err(response::Failure::field(
            "adjustment",
            "contains an unsupported field",
        ));
    }
    let new_quantity = match attributes.get("new_quantity") {
        Some(Value::String(value)) => value.clone(),
        Some(Value::Null) | None => {
            return Err(response::Failure::validation(
                "Quantity must be a valid nonnegative number",
            ));
        }
        _ => return Err(response::Failure::field("new_quantity", "must be a string")),
    };
    Ok(medications::AdjustStock {
        medication_id,
        new_quantity,
        reason: attributes
            .get("reason")
            .and_then(Value::as_str)
            .map(str::to_owned),
    })
}
