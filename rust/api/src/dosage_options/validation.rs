use super::*;

pub(super) struct Attributes {
    pub(super) medication_id: Option<String>,
    pub(super) amount: Option<Decimal>,
    pub(super) unit: Option<String>,
    pub(super) frequency: Option<String>,
    pub(super) description: Option<String>,
    pub(super) default_for_adults: Option<bool>,
    pub(super) default_for_children: Option<bool>,
    pub(super) default_max_daily_doses: Option<i32>,
    pub(super) default_min_hours_between_doses: Option<Decimal>,
    pub(super) default_dose_cycle: Option<i32>,
    pub(super) current_supply: Option<Option<Decimal>>,
    pub(super) reorder_threshold: Option<Option<Decimal>>,
}

pub(crate) fn valid_identifier(value: &str) -> bool {
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

pub(crate) fn parse_decimal(value: &Value) -> Option<Decimal> {
    let text = value.as_str()?;
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    let mut parts = unsigned.split('.');
    let whole = parts.next()?;
    if whole.is_empty() || !whole.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    if let Some(fraction) = parts.next() {
        if fraction.is_empty() || !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
    }
    if parts.next().is_some() {
        return None;
    }
    Decimal::from_str_exact(text).ok()
}

pub(crate) fn storage_decimal(value: Decimal, integer_digits: i64, scale: u32) -> Option<Decimal> {
    let bound = Decimal::from(10_i64.pow(integer_digits as u32));
    (value > -bound && value < bound && value.normalize().scale() <= scale).then_some(value)
}

fn required_string(value: &Value) -> Option<String> {
    value
        .as_str()
        .filter(|text| !text.trim().is_empty())
        .map(str::to_owned)
}

pub(super) fn attributes(body: &Value, create: bool) -> Option<Attributes> {
    let outer = body.as_object()?;
    if outer.len() != 1 {
        return None;
    }
    let inner = outer.get("dosage_option")?.as_object()?;
    let allowed = [
        "amount",
        "unit",
        "frequency",
        "description",
        "default_for_adults",
        "default_for_children",
        "default_max_daily_doses",
        "default_min_hours_between_doses",
        "default_dose_cycle",
        "current_supply",
        "reorder_threshold",
    ];
    if inner.is_empty()
        || inner
            .keys()
            .any(|key| !allowed.contains(&key.as_str()) && !(create && key == "medication_id"))
    {
        return None;
    }
    let medication_id = inner
        .get("medication_id")
        .map(required_string)
        .transpose_option()?;
    if medication_id
        .as_deref()
        .is_some_and(|value| !valid_identifier(value))
    {
        return None;
    }
    let amount = inner.get("amount").map(parse_decimal).transpose_option()?;
    let unit = inner.get("unit").map(required_string).transpose_option()?;
    let frequency = inner
        .get("frequency")
        .map(required_string)
        .transpose_option()?;
    let description = inner
        .get("description")
        .map(|value| value.as_str().map(str::to_owned))
        .transpose_option()?;
    let default_for_adults = inner
        .get("default_for_adults")
        .map(Value::as_bool)
        .transpose_option()?;
    let default_for_children = inner
        .get("default_for_children")
        .map(Value::as_bool)
        .transpose_option()?;
    let default_max_daily_doses = inner
        .get("default_max_daily_doses")
        .map(|value| {
            i32::try_from(value.as_i64()?)
                .ok()
                .filter(|number| *number >= 1)
        })
        .transpose_option()?;
    let default_min_hours_between_doses = inner
        .get("default_min_hours_between_doses")
        .map(|value| storage_decimal(parse_decimal(value)?, 3, 1))
        .transpose_option()?;
    let default_dose_cycle = inner
        .get("default_dose_cycle")
        .map(|value| match value.as_str()? {
            "daily" => Some(0),
            "weekly" => Some(1),
            "monthly" => Some(2),
            _ => None,
        })
        .transpose_option()?;
    let current_supply = inner
        .get("current_supply")
        .map(|value| {
            if value.is_null() {
                Some(None)
            } else {
                storage_decimal(parse_decimal(value)?, 8, 2).map(Some)
            }
        })
        .transpose_option()?;
    let reorder_threshold = inner
        .get("reorder_threshold")
        .map(|value| {
            if value.is_null() {
                Some(None)
            } else {
                storage_decimal(parse_decimal(value)?, 8, 2).map(Some)
            }
        })
        .transpose_option()?;
    if amount.is_some_and(|value| value <= Decimal::ZERO)
        || default_min_hours_between_doses.is_some_and(|value| value < Decimal::ZERO)
        || current_supply
            .flatten()
            .is_some_and(|value| value < Decimal::ZERO)
        || reorder_threshold
            .flatten()
            .is_some_and(|value| value < Decimal::ZERO)
    {
        return None;
    }
    if create
        && (medication_id.is_none()
            || amount.is_none()
            || unit.is_none()
            || frequency.is_none()
            || default_max_daily_doses.is_none()
            || default_min_hours_between_doses.is_none()
            || default_dose_cycle.is_none())
    {
        return None;
    }
    Some(Attributes {
        medication_id,
        amount,
        unit,
        frequency,
        description,
        default_for_adults,
        default_for_children,
        default_max_daily_doses,
        default_min_hours_between_doses,
        default_dose_cycle,
        current_supply,
        reorder_threshold,
    })
}

trait TransposeOption<T> {
    fn transpose_option(self) -> Option<Option<T>>;
}

impl<T> TransposeOption<T> for Option<Option<T>> {
    fn transpose_option(self) -> Option<Option<T>> {
        match self {
            Some(Some(value)) => Some(Some(value)),
            Some(None) => None,
            None => Some(None),
        }
    }
}

pub(super) fn valid_persisted_dosage(record: &dosage::Model) -> bool {
    record.amount > Decimal::ZERO
        && !record.unit.trim().is_empty()
        && !record.frequency.trim().is_empty()
        && record.default_max_daily_doses > 0
        && record.default_min_hours_between_doses >= Decimal::ZERO
        && matches!(record.default_dose_cycle, 0..=2)
        && record
            .current_supply
            .is_none_or(|value| value >= Decimal::ZERO)
        && record
            .reorder_threshold
            .is_none_or(|value| value >= Decimal::ZERO)
}
