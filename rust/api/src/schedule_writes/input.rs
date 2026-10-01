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

pub(super) async fn apply_attributes(
    db: &DatabaseTransaction,
    context: &AuthContext,
    attributes: &Map<String, Value>,
    fields: &mut ScheduleFields,
    existing: Option<&schedule::Model>,
) -> Result<Result<(), InputFailure>, ApiError> {
    if let Some(value) = attributes.get("person_id") {
        let id = match identifier(value, "person_id") {
            Ok(id) => id,
            Err(error) => return Ok(Err(error)),
        };
        let Some(person) = find_person(db, context, id).await? else {
            return Ok(Err(InputFailure::NotFound));
        };
        if !person_access(db, context, person.id, false).await? {
            return Ok(Err(InputFailure::NotFound));
        }
        if existing.is_some_and(|existing| existing.person_id != person.id) {
            return Ok(Err(InputFailure::Invalid("person_id", "cannot be changed")));
        }
        fields.person_id = Some(person.id);
    }
    if let Some(value) = attributes.get("medication_id") {
        let id = match identifier(value, "medication_id") {
            Ok(id) => id,
            Err(error) => return Ok(Err(error)),
        };
        let Some(medication) = visible_medication(db, context, id).await? else {
            return Ok(Err(InputFailure::NotFound));
        };
        fields.medication_id = Some(medication.id);
    }
    if let Some(value) = attributes.get("source_dosage_option_id") {
        if value.is_null() {
            return Ok(Err(InputFailure::Invalid(
                "source_dosage_option_id",
                "must be a string",
            )));
        } else {
            let id = match identifier(value, "source_dosage_option_id") {
                Ok(id) => id,
                Err(error) => return Ok(Err(error)),
            };
            let Some(option) = find_dosage(db, context, id).await? else {
                return Ok(Err(InputFailure::NotFound));
            };
            fields.source_dosage_option_id = Some(option.id);
        }
    }
    if let Some(value) = attributes.get("dose_amount") {
        match parse_amount(value) {
            Ok(amount) => fields.dose_amount = Some(amount),
            Err(error) => return Ok(Err(error)),
        }
    }
    if let Some(value) = attributes.get("dose_unit") {
        let Some(unit) = value.as_str() else {
            return Ok(Err(InputFailure::Invalid("dose_unit", "must be a string")));
        };
        if !UNITS.contains(&unit) {
            return Ok(Err(InputFailure::Invalid("dose_unit", "is invalid")));
        }
        fields.dose_unit = Some(unit.to_owned());
    }
    for (key, destination) in [
        ("frequency", &mut fields.frequency),
        ("notes", &mut fields.notes),
    ] {
        if let Some(value) = attributes.get(key) {
            if let Some(value) = value.as_str() {
                *destination = Some(value.to_owned());
            } else {
                return Ok(Err(InputFailure::Invalid(
                    "schedule",
                    "contains an invalid string",
                )));
            }
        }
    }
    if let Some(value) = attributes.get("start_date") {
        match parse_date(value, "start_date") {
            Ok(date) => fields.start_date = Some(date),
            Err(error) => return Ok(Err(error)),
        }
    }
    if let Some(value) = attributes.get("end_date") {
        match parse_date(value, "end_date") {
            Ok(date) => fields.end_date = Some(date),
            Err(error) => return Ok(Err(error)),
        }
    }
    if let Some(value) = attributes.get("max_daily_doses") {
        let Some(amount) = value
            .as_i64()
            .filter(|amount| *amount > 0 && *amount <= i32::MAX as i64)
        else {
            return Ok(Err(InputFailure::Invalid(
                "max_daily_doses",
                "must be positive",
            )));
        };
        fields.max_daily_doses = Some(amount as i32);
    }
    if let Some(value) = attributes.get("min_hours_between_doses") {
        match parse_interval(value) {
            Ok(interval) => fields.min_hours_between_doses = interval,
            Err(error) => return Ok(Err(error)),
        }
    }
    if let Some(value) = attributes.get("dose_cycle") {
        fields.dose_cycle = match value.as_str() {
            Some("daily") => Some(0),
            Some("weekly") => Some(1),
            Some("monthly") => Some(2),
            _ => return Ok(Err(InputFailure::Invalid("dose_cycle", "is invalid"))),
        };
    }
    if let Some(value) = attributes.get("schedule_type") {
        fields.schedule_type = match value.as_str() {
            Some("daily") => 0,
            Some("multiple_daily") => 1,
            Some("weekly") => 2,
            Some("specific_dates") => 3,
            Some("prn") => 4,
            Some("tapering") => 5,
            Some("every_other_day") => 6,
            _ => return Ok(Err(InputFailure::Invalid("schedule_type", "is invalid"))),
        };
    }
    if let Some(value) = attributes.get("schedule_config") {
        if !valid_config(value) {
            return Ok(Err(InputFailure::Invalid("schedule_config", "is invalid")));
        }
        fields.schedule_config = value.clone();
    }
    if let Some(option_id) = fields.source_dosage_option_id {
        let Some(option) = dosage::Entity::find_by_id(option_id)
            .one(db)
            .await
            .map_err(database_error)?
        else {
            return Ok(Err(InputFailure::NotFound));
        };
        if Some(option.medication_id) != fields.medication_id
            || Some(option.amount) != fields.dose_amount
            || fields.dose_unit.as_deref() != Some(option.unit.as_str())
        {
            return Ok(Err(InputFailure::Invalid(
                "source_dosage_option",
                "must match the selected medication and dose",
            )));
        }
    } else if existing.is_none() {
        if let (Some(medication_id), Some(amount), Some(unit)) = (
            fields.medication_id,
            fields.dose_amount,
            fields.dose_unit.as_deref(),
        ) {
            let matches = dosage::Entity::find()
                .filter(dosage::Column::HouseholdId.eq(context.membership.household_id))
                .filter(dosage::Column::MedicationId.eq(medication_id))
                .filter(dosage::Column::Amount.eq(amount))
                .filter(dosage::Column::Unit.eq(unit))
                .all(db)
                .await
                .map_err(database_error)?;
            if matches.len() == 1 {
                fields.source_dosage_option_id = Some(matches[0].id);
            }
        }
    }
    Ok(Ok(()))
}
