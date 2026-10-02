use super::*;

pub(super) fn required_string(
    attributes: &Map<String, Value>,
    name: &str,
) -> Option<Option<String>> {
    attributes
        .get(name)
        .map(|value| {
            value
                .as_str()
                .filter(|text| !text.trim().is_empty())
                .map(str::to_owned)
        })
        .map_or(Some(None), |value| value.map(Some))
}

pub(super) fn identifier(attributes: &Map<String, Value>, name: &str) -> Option<Option<String>> {
    let value = required_string(attributes, name)?;
    if value
        .as_deref()
        .is_some_and(|value| !valid_identifier(value))
    {
        return None;
    }
    Some(value)
}

pub(super) fn optional_string(
    attributes: &Map<String, Value>,
    name: &str,
) -> Option<Option<String>> {
    attributes
        .get(name)
        .map(|value| value.as_str().map(str::to_owned))
        .map_or(Some(None), |value| value.map(Some))
}

pub(super) fn numeric_10_2(value: Decimal) -> bool {
    value > Decimal::ZERO && storage_decimal(value, 8, 2).is_some()
}

pub(super) fn decimal(attributes: &Map<String, Value>, name: &str) -> Option<Option<Decimal>> {
    attributes
        .get(name)
        .map(|value| {
            let amount = parse_decimal(value)?;
            numeric_10_2(amount).then_some(amount)
        })
        .map_or(Some(None), |value| value.map(Some))
}

pub(super) fn positive_integer(attributes: &Map<String, Value>, name: &str) -> Option<Option<i32>> {
    attributes
        .get(name)
        .map(|value| {
            i32::try_from(value.as_i64()?)
                .ok()
                .filter(|number| *number >= 1)
        })
        .map_or(Some(None), |value| value.map(Some))
}

pub(super) fn whole_hours(attributes: &Map<String, Value>) -> Option<Option<Option<i32>>> {
    let Some(value) = attributes.get("min_hours_between_doses") else {
        return Some(None);
    };
    if value.is_null() {
        return Some(Some(None));
    }
    let amount = parse_decimal(value)?;
    if amount.fract() != Decimal::ZERO || amount <= Decimal::ZERO {
        return None;
    }
    let hours = amount.trunc().to_string().parse::<i32>().ok()?;
    Some(Some(Some(hours)))
}
