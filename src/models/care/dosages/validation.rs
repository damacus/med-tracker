use super::*;

#[derive(Clone)]
pub(super) struct MedicationId(String);

impl MedicationId {
    fn parse(value: String) -> Option<Self> {
        valid_identifier(&value).then_some(Self(value))
    }

    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoseCycle {
    Daily,
    Weekly,
    Monthly,
}

impl DoseCycle {
    pub(super) fn code(self) -> i32 {
        self as i32
    }

    fn from_text(value: &str) -> Option<Self> {
        Some(match value {
            "daily" => Self::Daily,
            "weekly" => Self::Weekly,
            "monthly" => Self::Monthly,
            _ => return None,
        })
    }
}

pub struct DosageInput(Option<Attributes>);

impl DosageInput {
    pub fn stock_quantity(quantity: Decimal) -> Self {
        Self(
            storage_decimal(quantity, 8, 2)
                .filter(|value| *value >= Decimal::ZERO)
                .map(|value| Attributes {
                    current_supply: Some(Some(value)),
                    ..Attributes::default()
                }),
        )
    }

    pub fn from_json(body: Value, create: bool) -> Self {
        let parsed = body.as_object().and_then(|outer| {
            (outer.len() == 1)
                .then(|| outer.get("dosage_option")?.as_object())
                .flatten()
                .and_then(|inner| attributes(inner, create, None))
        });
        Self(parsed)
    }

    pub fn from_fields(fields: &serde_json::Map<String, Value>, create: bool) -> Self {
        Self(attributes(fields, create, None))
    }

    pub fn from_wizard_extra(option: &Value, medication_id: i64) -> Self {
        let parsed = MedicationId::parse(medication_id.to_string()).and_then(|id| {
            option
                .as_object()
                .and_then(|fields| attributes(fields, true, Some(id)))
        });
        Self(parsed)
    }

    pub fn from_form(form: FormDosageInput, create: bool) -> Self {
        let FormDosageInput {
            medication_id,
            amount,
            unit,
            frequency,
            description,
            default_for_adults,
            default_for_children,
            max_daily_doses,
            min_hours_between_doses,
            dose_cycle,
            current_supply,
            reorder_threshold,
        } = form;
        let medication_id = medication_id.and_then(MedicationId::parse);
        let amount = parse_decimal_text(&amount);
        let unit = (!unit.trim().is_empty()).then_some(unit);
        let frequency = (!frequency.trim().is_empty()).then_some(frequency);
        let default_max_daily_doses = max_daily_doses
            .parse::<i32>()
            .ok()
            .filter(|value| *value >= 1);
        let default_min_hours_between_doses = parse_decimal_text(&min_hours_between_doses)
            .and_then(|value| storage_decimal(value, 3, 1));
        let default_dose_cycle = DoseCycle::from_text(&dose_cycle);
        let parsed = (|| {
            let current_supply = parse_optional_stock(current_supply)?;
            let reorder_threshold = parse_optional_stock(reorder_threshold)?;
            if amount.is_some_and(|value| value <= Decimal::ZERO)
                || default_min_hours_between_doses.is_some_and(|value| value < Decimal::ZERO)
                || current_supply
                    .flatten()
                    .is_some_and(|value| value < Decimal::ZERO)
                || reorder_threshold
                    .flatten()
                    .is_some_and(|value| value < Decimal::ZERO)
                || (create && medication_id.is_none())
                || amount.is_none()
                || unit.is_none()
                || frequency.is_none()
                || default_max_daily_doses.is_none()
                || default_min_hours_between_doses.is_none()
                || default_dose_cycle.is_none()
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
        })();
        Self(parsed)
    }

    pub(super) fn into_attributes(self) -> Result<Attributes, OperationError> {
        self.0.ok_or_else(invalid)
    }
}

pub struct FormDosageInput {
    pub medication_id: Option<String>,
    pub amount: String,
    pub unit: String,
    pub frequency: String,
    pub description: Option<String>,
    pub default_for_adults: Option<bool>,
    pub default_for_children: Option<bool>,
    pub max_daily_doses: String,
    pub min_hours_between_doses: String,
    pub dose_cycle: String,
    pub current_supply: Option<Option<String>>,
    pub reorder_threshold: Option<Option<String>>,
}

fn parse_optional_stock(raw: Option<Option<String>>) -> Option<Option<Option<Decimal>>> {
    match raw {
        None => Some(None),
        Some(None) => Some(Some(None)),
        Some(Some(value)) => {
            storage_decimal(parse_decimal_text(&value)?, 8, 2).map(|value| Some(Some(value)))
        }
    }
}

fn parse_decimal_text(text: &str) -> Option<Decimal> {
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    let mut parts = unsigned.split('.');
    let whole = parts.next()?;
    if whole.is_empty() || !whole.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    if let Some(fraction) = parts.next()
        && (fraction.is_empty() || !fraction.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return None;
    }
    if parts.next().is_some() {
        return None;
    }
    Decimal::from_str_exact(text).ok()
}

#[derive(Default)]
pub(super) struct Attributes {
    pub(super) medication_id: Option<MedicationId>,
    pub(super) amount: Option<Decimal>,
    pub(super) unit: Option<String>,
    pub(super) frequency: Option<String>,
    pub(super) description: Option<String>,
    pub(super) default_for_adults: Option<bool>,
    pub(super) default_for_children: Option<bool>,
    pub(super) default_max_daily_doses: Option<i32>,
    pub(super) default_min_hours_between_doses: Option<Decimal>,
    pub(super) default_dose_cycle: Option<DoseCycle>,
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
    parse_decimal_text(value.as_str()?)
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

fn attributes(
    inner: &serde_json::Map<String, Value>,
    create: bool,
    wizard_medication_id: Option<MedicationId>,
) -> Option<Attributes> {
    let wizard = wizard_medication_id.is_some();
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
    let medication_id = match wizard_medication_id {
        Some(id) => Some(id),
        None => inner
            .get("medication_id")
            .map(required_string)
            .transpose_option()?
            .map(MedicationId::parse)
            .transpose_option()?,
    };
    let amount = inner
        .get("amount")
        .map(|value| parse_field_decimal(value, wizard))
        .transpose_option()?;
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
        .map(|value| storage_decimal(parse_field_decimal(value, wizard)?, 3, 1))
        .transpose_option()?;
    let default_dose_cycle = inner
        .get("default_dose_cycle")
        .map(|value| DoseCycle::from_text(value.as_str()?))
        .transpose_option()?;
    let current_supply = inner
        .get("current_supply")
        .map(|value| {
            if value.is_null() || (wizard && value.as_str() == Some("")) {
                Some(None)
            } else {
                storage_decimal(parse_field_decimal(value, wizard)?, 8, 2).map(Some)
            }
        })
        .transpose_option()?;
    let reorder_threshold = inner
        .get("reorder_threshold")
        .map(|value| {
            if value.is_null() || (wizard && value.as_str() == Some("")) {
                Some(None)
            } else {
                storage_decimal(parse_field_decimal(value, wizard)?, 8, 2).map(Some)
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

fn parse_field_decimal(value: &Value, wizard: bool) -> Option<Decimal> {
    if wizard && let Value::Number(number) = value {
        return parse_decimal_text(&number.to_string());
    }
    parse_decimal(value)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wizard_extra_option_preserves_numeric_and_empty_stock_coercion() {
        let option = json!({
            "medication_id": "ignored",
            "amount": 1.25,
            "unit": "tablet",
            "frequency": "daily",
            "default_max_daily_doses": 2,
            "default_min_hours_between_doses": 4,
            "default_dose_cycle": "daily",
            "current_supply": "",
            "reorder_threshold": 2.5
        });
        let attrs = DosageInput::from_wizard_extra(&option, 90001)
            .into_attributes()
            .unwrap();
        assert_eq!(attrs.medication_id.unwrap().as_str(), "90001");
        assert_eq!(attrs.amount.unwrap().normalize().to_string(), "1.25");
        assert_eq!(attrs.current_supply, Some(None));
        assert_eq!(
            attrs
                .reorder_threshold
                .unwrap()
                .unwrap()
                .normalize()
                .to_string(),
            "2.5"
        );
    }
}
