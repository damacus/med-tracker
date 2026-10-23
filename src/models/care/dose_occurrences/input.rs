use super::*;

const REASONS: &[&str] = &[
    "refused",
    "unwell",
    "asleep",
    "medicine_unavailable",
    "clinician_advice",
    "other",
];

pub struct ChangeInput {
    fields: Result<ChangeFields, Failure>,
}

pub(super) struct ChangeFields {
    pub key: String,
    pub action: ActionInput,
}

pub(super) enum ActionInput {
    NotTaken(Result<(Option<String>, Option<String>), Failure>),
    Reopen,
    Take {
        taken_at: Result<NaiveDateTime, Failure>,
        attempt: Option<Result<doses::TakeAttempt, OperationError>>,
        client_uuid: Option<String>,
    },
    Invalid,
}

impl ChangeInput {
    pub fn not_taken(key: &str, reason: Option<&Value>, note: Option<&Value>) -> Self {
        Self {
            fields: Ok(ChangeFields {
                key: key.to_owned(),
                action: ActionInput::NotTaken(parse_not_taken_fields(reason, note)),
            }),
        }
    }

    pub fn not_taken_strings(key: &str, reason: Option<&str>, note: Option<&str>) -> Self {
        let reason = match reason {
            None => Ok(None),
            Some(value) if REASONS.contains(&value) => Ok(Some(value.to_owned())),
            Some(_) => Err(Failure::Invalid("reason", "is invalid")),
        };
        let values = reason.and_then(|reason| match note {
            None => Ok((reason, None)),
            Some(value) if value.chars().count() <= 2000 => Ok((reason, Some(value.to_owned()))),
            Some(_) => Err(Failure::Invalid("note", "is invalid")),
        });
        Self {
            fields: Ok(ChangeFields {
                key: key.to_owned(),
                action: ActionInput::NotTaken(values),
            }),
        }
    }

    pub fn reopen(key: &str) -> Self {
        Self {
            fields: Ok(ChangeFields {
                key: key.to_owned(),
                action: ActionInput::Reopen,
            }),
        }
    }

    pub fn take(
        source_path: (&str, &str),
        key: &str,
        taken_at: &str,
        client_uuid: Option<&str>,
        dose_amount: Option<&str>,
        stock_id: Option<i64>,
    ) -> Self {
        let timestamp = chrono::DateTime::parse_from_rfc3339(taken_at)
            .map(|value| value.naive_utc())
            .map_err(|_| Failure::Invalid("taken_at", "must be ISO8601"));
        let parsed = timestamp.and_then(|value| {
            if client_uuid.is_some_and(|uuid| Uuid::parse_str(uuid).is_err()) {
                return Err(Failure::Invalid("client_uuid", "must be a UUID"));
            }
            if dose_amount.is_some_and(|amount| {
                sea_orm::prelude::Decimal::from_str(amount)
                    .ok()
                    .is_none_or(|amount| amount <= sea_orm::prelude::Decimal::ZERO)
            }) {
                return Err(Failure::Invalid("dose_amount", "must be a decimal string"));
            }
            if stock_id.is_some_and(|id| id <= 0) {
                return Err(Failure::Invalid(
                    "taken_from_medication_id",
                    "must be a positive integer",
                ));
            }
            Ok(value)
        });
        let attempt = parsed.as_ref().ok().map(|_| {
            doses::TakeInput {
                source_type: source_path.0.to_owned(),
                source_id: source_path.1.to_owned(),
                taken_at: taken_at.to_owned(),
                client_uuid: client_uuid.map(str::to_owned),
                dose_amount: dose_amount.map(str::to_owned),
                dose_unit: None,
                taken_from_medication_id: stock_id,
                expected_effective_amount: None,
                expected_effective_unit: None,
            }
            .attempt()
        });
        Self {
            fields: Ok(ChangeFields {
                key: key.to_owned(),
                action: ActionInput::Take {
                    taken_at: parsed,
                    attempt,
                    client_uuid: client_uuid.map(str::to_owned),
                },
            }),
        }
    }

    pub fn from_body(source_path: (&str, &str), action: &str, body: &Value) -> Self {
        let fields = attributes(body, action).map(|attrs| {
            let key = attrs
                .get("key")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let action = match action {
                "not_taken" => ActionInput::NotTaken(parse_not_taken(attrs)),
                "reopen" => ActionInput::Reopen,
                "take" => {
                    let taken_at = parse_take(attrs);
                    let attempt = taken_at.as_ref().ok().map(|_| {
                        doses::TakeInput {
                            source_type: source_path.0.to_owned(),
                            source_id: source_path.1.to_owned(),
                            taken_at: attrs
                                .get("taken_at")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_owned(),
                            client_uuid: attrs
                                .get("client_uuid")
                                .and_then(Value::as_str)
                                .map(str::to_owned),
                            dose_amount: attrs
                                .get("dose_amount")
                                .and_then(Value::as_str)
                                .map(str::to_owned),
                            dose_unit: None,
                            taken_from_medication_id: attrs
                                .get("taken_from_medication_id")
                                .and_then(Value::as_i64),
                            expected_effective_amount: None,
                            expected_effective_unit: None,
                        }
                        .attempt()
                    });
                    ActionInput::Take {
                        taken_at,
                        attempt,
                        client_uuid: attrs
                            .get("client_uuid")
                            .and_then(Value::as_str)
                            .map(str::to_owned),
                    }
                }
                _ => ActionInput::Invalid,
            };
            ChangeFields { key, action }
        });
        Self { fields }
    }

    pub(super) fn fields(self) -> Result<ChangeFields, OperationError> {
        let fields = self.fields.map_err(OperationError::from)?;
        if fields.key.is_empty() {
            return Err(OperationError::from(Failure::Invalid(
                "key",
                "must be a string",
            )));
        }
        Ok(fields)
    }
}

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
    parse_not_taken_fields(attributes.get("reason"), attributes.get("note"))
}

fn parse_not_taken_fields(
    reason: Option<&Value>,
    note: Option<&Value>,
) -> Result<(Option<String>, Option<String>), Failure> {
    let reason = match reason {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) if REASONS.contains(&value.as_str()) => Some(value.clone()),
        _ => return Err(Failure::Invalid("reason", "is invalid")),
    };
    let note = match note {
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
    if let Some(value) = attributes.get("client_uuid")
        && value
            .as_str()
            .is_none_or(|value| Uuid::parse_str(value).is_err())
    {
        return Err(Failure::Invalid("client_uuid", "must be a UUID"));
    }
    if let Some(value) = attributes.get("dose_amount")
        && parse_decimal(value).is_none_or(|amount| amount <= sea_orm::prelude::Decimal::ZERO)
    {
        return Err(Failure::Invalid("dose_amount", "must be a decimal string"));
    }
    if let Some(value) = attributes.get("taken_from_medication_id")
        && value.as_i64().is_none_or(|value| value <= 0)
    {
        return Err(Failure::Invalid(
            "taken_from_medication_id",
            "must be a positive integer",
        ));
    }
    Ok(taken_at)
}
