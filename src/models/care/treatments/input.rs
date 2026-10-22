use super::*;

pub(super) fn identifier<'a>(
    value: &'a Value,
    field: &'static str,
) -> Result<&'a str, InputFailure> {
    let id = value
        .as_str()
        .ok_or(InputFailure::Invalid(field, "must be a string"))?;
    if !valid_identifier(id) {
        return Err(InputFailure::Invalid(field, "is invalid"));
    }
    Ok(id)
}

pub(super) fn attributes(body: &Value) -> Result<&Map<String, Value>, InputFailure> {
    let outer = body.as_object().ok_or(InputFailure::Malformed)?;
    let Some(attributes) = body.get("schedule").and_then(Value::as_object) else {
        return Err(InputFailure::Malformed);
    };
    if outer.len() != 1 {
        return Err(InputFailure::Invalid(
            "schedule",
            "contains an unsupported field",
        ));
    }
    if attributes.is_empty() {
        return Err(InputFailure::Invalid(
            "schedule",
            "must have at least one field",
        ));
    }
    if let Some((field, _)) = attributes
        .iter()
        .find(|(field, _)| !FIELDS.contains(&field.as_str()))
    {
        return Err(InputFailure::Invalid(
            "schedule",
            if field.is_empty() {
                "is invalid"
            } else {
                "contains an unsupported field"
            },
        ));
    }
    Ok(attributes)
}

pub(super) fn parse_amount(value: &Value) -> Result<Decimal, InputFailure> {
    if value.is_number() {
        return Err(InputFailure::Invalid("dose_amount", "must be a string"));
    }
    let amount = parse_decimal(value).ok_or(InputFailure::Invalid(
        "dose_amount",
        "must be a decimal string",
    ))?;
    if amount <= Decimal::ZERO || storage_decimal(amount, 8, 2).is_none() {
        return Err(InputFailure::Invalid(
            "dose_amount",
            "must fit two decimal places",
        ));
    }
    Ok(amount)
}

pub(super) fn parse_interval(value: &Value) -> Result<Option<i32>, InputFailure> {
    if value.is_null() {
        return Ok(None);
    }
    let decimal = parse_decimal(value).ok_or(InputFailure::Invalid(
        "min_hours_between_doses",
        "must be a decimal string",
    ))?;
    if decimal <= Decimal::ZERO || storage_decimal(decimal, 10, 0).is_none() {
        return Err(InputFailure::Invalid(
            "min_hours_between_doses",
            "must be a whole positive number",
        ));
    }
    decimal
        .normalize()
        .to_string()
        .parse::<i32>()
        .map(Some)
        .map_err(|_| InputFailure::Invalid("min_hours_between_doses", "is out of range"))
}

pub(super) fn parse_date(value: &Value, field: &'static str) -> Result<NaiveDate, InputFailure> {
    let raw = value
        .as_str()
        .ok_or(InputFailure::Invalid(field, "must be a date"))?;
    if raw.len() != 10 {
        return Err(InputFailure::Invalid(field, "must be a date"));
    }
    NaiveDate::parse_from_str(raw, "%Y-%m-%d")
        .map_err(|_| InputFailure::Invalid(field, "must be a date"))
}

pub(super) fn valid_time(value: &Value) -> bool {
    let Some(raw) = value.as_str() else {
        return false;
    };
    let bytes = raw.as_bytes();
    bytes.len() == 5
        && bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[2] == b':'
        && bytes[3].is_ascii_digit()
        && bytes[4].is_ascii_digit()
        && raw[0..2].parse::<u8>().is_ok_and(|hour| hour <= 23)
        && raw[3..5].parse::<u8>().is_ok_and(|minute| minute <= 59)
}

pub(super) fn valid_times(value: &Value) -> bool {
    value
        .as_array()
        .is_some_and(|times| times.iter().all(valid_time))
}

pub(super) fn valid_config(config: &Value) -> bool {
    let Some(map) = config.as_object() else {
        return false;
    };
    for (key, value) in map {
        let valid = match key.as_str() {
            "times" => valid_times(value),
            "weekdays" => value.as_array().is_some_and(|days| {
                days.iter().all(|day| {
                    day.as_str().is_some_and(|day| {
                        matches!(
                            day.trim().to_ascii_lowercase().as_str(),
                            "sunday"
                                | "sun"
                                | "monday"
                                | "mon"
                                | "tuesday"
                                | "tue"
                                | "wednesday"
                                | "wed"
                                | "thursday"
                                | "thu"
                                | "friday"
                                | "fri"
                                | "saturday"
                                | "sat"
                                | "0"
                                | "1"
                                | "2"
                                | "3"
                                | "4"
                                | "5"
                                | "6"
                                | "7"
                        )
                    })
                })
            }),
            "dates" => value.as_array().is_some_and(|dates| {
                dates
                    .iter()
                    .all(|date| parse_date(date, "schedule_config").is_ok())
            }),
            "as_needed" => value.is_boolean(),
            "tapering_plan" => value
                .as_str()
                .is_some_and(|instruction| !instruction.trim().is_empty()),
            "taper_steps" => value
                .as_array()
                .is_some_and(|steps| steps.iter().all(valid_taper_step)),
            _ => false,
        };
        if !valid {
            return false;
        }
    }
    true
}

pub(super) fn valid_taper_step(value: &Value) -> bool {
    let Some(map) = value.as_object() else {
        return false;
    };
    let (Some(start), Some(end)) = (map.get("start_date"), map.get("end_date")) else {
        return false;
    };
    let (Ok(start), Ok(end)) = (
        parse_date(start, "schedule_config"),
        parse_date(end, "schedule_config"),
    ) else {
        return false;
    };
    if end < start {
        return false;
    }
    map.iter().all(|(key, value)| match key.as_str() {
        "start_date" | "end_date" => true,
        "amount" | "dose_amount" => {
            parse_decimal(value).is_some_and(|amount| amount > Decimal::ZERO)
        }
        "unit" | "dose_unit" => value.as_str().is_some_and(|value| !value.is_empty()),
        "max_daily_doses" => value
            .as_i64()
            .is_some_and(|value| value > 0 && value <= i32::MAX as i64),
        "min_hours_between_doses" => {
            parse_decimal(value).is_some_and(|hours| hours > Decimal::ZERO)
        }
        "times" => valid_times(value),
        _ => false,
    })
}

pub(super) fn parse_input(attributes: &Map<String, Value>) -> Result<ScheduleInput, InputFailure> {
    let parse_id = |field: &'static str| {
        attributes
            .get(field)
            .map(|value| identifier(value, field))
            .transpose()
    };
    let person_id = parse_id("person_id")?
        .map(|value| {
            value
                .parse::<PersonId>()
                .map_err(|_| InputFailure::Invalid("person_id", "is invalid"))
        })
        .transpose()?;
    let medication_id = parse_id("medication_id")?
        .map(|value| {
            value
                .parse::<MedicationId>()
                .map_err(|_| InputFailure::Invalid("medication_id", "is invalid"))
        })
        .transpose()?;
    let source_dosage_option_id = parse_id("source_dosage_option_id")?
        .map(|value| {
            value
                .parse::<DosageOptionId>()
                .map_err(|_| InputFailure::Invalid("source_dosage_option_id", "is invalid"))
        })
        .transpose()?;
    let dose_amount = attributes
        .get("dose_amount")
        .map(parse_amount)
        .transpose()?;
    let dose_unit = attributes
        .get("dose_unit")
        .map(|value| {
            let unit = value
                .as_str()
                .ok_or(InputFailure::Invalid("dose_unit", "must be a string"))?;
            if !UNITS.contains(&unit) {
                return Err(InputFailure::Invalid("dose_unit", "is invalid"));
            }
            Ok(unit.to_owned())
        })
        .transpose()?;
    let parse_text = |field| {
        attributes
            .get(field)
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .ok_or(InputFailure::Invalid(
                        "schedule",
                        "contains an invalid string",
                    ))
            })
            .transpose()
    };
    let frequency = parse_text("frequency")?;
    let notes = parse_text("notes")?;
    let start_date = attributes
        .get("start_date")
        .map(|value| parse_date(value, "start_date"))
        .transpose()?;
    let end_date = attributes
        .get("end_date")
        .map(|value| parse_date(value, "end_date"))
        .transpose()?;
    let max_daily_doses = attributes
        .get("max_daily_doses")
        .map(|value| {
            value
                .as_i64()
                .filter(|amount| *amount > 0 && *amount <= i32::MAX as i64)
                .map(|amount| amount as i32)
                .ok_or(InputFailure::Invalid("max_daily_doses", "must be positive"))
        })
        .transpose()?;
    let min_hours_between_doses = attributes
        .get("min_hours_between_doses")
        .map(parse_interval)
        .transpose()?;
    let dose_cycle = attributes
        .get("dose_cycle")
        .map(|value| {
            value
                .as_str()
                .and_then(DoseCycle::parse)
                .ok_or(InputFailure::Invalid("dose_cycle", "is invalid"))
        })
        .transpose()?;
    let schedule_type = attributes
        .get("schedule_type")
        .map(|value| {
            value
                .as_str()
                .and_then(ScheduleType::parse)
                .ok_or(InputFailure::Invalid("schedule_type", "is invalid"))
        })
        .transpose()?;
    let schedule_config = attributes
        .get("schedule_config")
        .map(|value| {
            valid_config(value)
                .then(|| value.clone())
                .ok_or(InputFailure::Invalid("schedule_config", "is invalid"))
        })
        .transpose()?;
    Ok(ScheduleInput {
        person_id,
        medication_id,
        source_dosage_option_id,
        dose_amount,
        dose_unit,
        frequency,
        start_date,
        end_date,
        notes,
        max_daily_doses,
        min_hours_between_doses,
        dose_cycle,
        schedule_type,
        schedule_config,
    })
}
