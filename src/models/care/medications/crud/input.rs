use super::*;

#[derive(Clone, Debug, Default)]
pub enum Field<T> {
    #[default]
    Missing,
    Null,
    Value(T),
    InvalidText,
    WrongNumber,
    Invalid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocationId(pub(super) i64);

impl TryFrom<i64> for LocationId {
    type Error = ();

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        (value > 0).then_some(Self(value)).ok_or(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScheduleType {
    Daily,
    MultipleDaily,
    Weekly,
    SpecificDates,
    Prn,
    Tapering,
    EveryOtherDay,
}

impl ScheduleType {
    pub fn from_name(value: &str) -> Option<Self> {
        Some(match value {
            "daily" => Self::Daily,
            "multiple_daily" => Self::MultipleDaily,
            "weekly" => Self::Weekly,
            "specific_dates" => Self::SpecificDates,
            "prn" => Self::Prn,
            "tapering" => Self::Tapering,
            "every_other_day" => Self::EveryOtherDay,
            _ => return None,
        })
    }

    fn from_code(value: i64) -> Option<Self> {
        Some(match value {
            0 => Self::Daily,
            1 => Self::MultipleDaily,
            2 => Self::Weekly,
            3 => Self::SpecificDates,
            4 => Self::Prn,
            5 => Self::Tapering,
            6 => Self::EveryOtherDay,
            _ => return None,
        })
    }

    fn code(self) -> i32 {
        self as i32
    }
}

#[derive(Clone, Debug, Default)]
pub struct MedicationInput {
    pub name: Field<String>,
    pub friendly_name: Field<String>,
    pub barcode: Field<String>,
    pub dmd_code: Field<String>,
    pub dmd_system: Field<String>,
    pub dmd_concept_class: Field<String>,
    pub category: Field<String>,
    pub description: Field<String>,
    pub dose_amount: Field<Decimal>,
    pub dose_unit: Field<String>,
    pub current_supply: Field<Decimal>,
    pub reorder_threshold: Field<Decimal>,
    pub warnings: Field<String>,
    pub location_id: Field<i64>,
    pub default_schedule_type: Field<ScheduleType>,
    invalid_shape: bool,
    unknown_field: bool,
}

impl MedicationInput {
    pub fn from_json(value: Value) -> Self {
        let Value::Object(fields) = value else {
            return Self {
                invalid_shape: true,
                ..Self::default()
            };
        };
        Self::from_fields(fields)
    }

    pub fn from_fields(mut fields: serde_json::Map<String, Value>) -> Self {
        let mut input = Self {
            name: take_text(&mut fields, "name"),
            friendly_name: take_text(&mut fields, "friendly_name"),
            barcode: take_text(&mut fields, "barcode"),
            dmd_code: take_text(&mut fields, "dmd_code"),
            dmd_system: take_text(&mut fields, "dmd_system"),
            dmd_concept_class: take_text(&mut fields, "dmd_concept_class"),
            category: take_text(&mut fields, "category"),
            description: take_text(&mut fields, "description"),
            dose_amount: take_decimal(&mut fields, "dose_amount"),
            dose_unit: take_text(&mut fields, "dose_unit"),
            current_supply: take_decimal(&mut fields, "current_supply"),
            reorder_threshold: take_decimal(&mut fields, "reorder_threshold"),
            warnings: take_text(&mut fields, "warnings"),
            location_id: take_integer(&mut fields, "location_id"),
            default_schedule_type: take_schedule(&mut fields, "default_schedule_type"),
            ..Self::default()
        };
        input.unknown_field = !fields.is_empty();
        input
    }

    fn is_empty(&self) -> bool {
        matches!(self.name, Field::Missing)
            && matches!(self.friendly_name, Field::Missing)
            && matches!(self.barcode, Field::Missing)
            && matches!(self.dmd_code, Field::Missing)
            && matches!(self.dmd_system, Field::Missing)
            && matches!(self.dmd_concept_class, Field::Missing)
            && matches!(self.category, Field::Missing)
            && matches!(self.description, Field::Missing)
            && matches!(self.dose_amount, Field::Missing)
            && matches!(self.dose_unit, Field::Missing)
            && matches!(self.current_supply, Field::Missing)
            && matches!(self.reorder_threshold, Field::Missing)
            && matches!(self.warnings, Field::Missing)
            && matches!(self.location_id, Field::Missing)
            && matches!(self.default_schedule_type, Field::Missing)
    }

    pub(super) fn only_null_dose(&self) -> bool {
        if !matches!(self.dose_amount, Field::Null) {
            return false;
        }
        let mut other = self.clone();
        other.dose_amount = Field::Missing;
        other.is_empty() && !other.unknown_field && !other.invalid_shape
    }

    pub(super) fn location_for_scope(&self) -> Option<i64> {
        match self.location_id {
            Field::Value(value) => Some(value),
            _ => None,
        }
    }

    pub(super) fn validate(
        &self,
        existing: Option<&medication::Model>,
    ) -> Result<MedicationChanges, (&'static str, &'static str)> {
        if self.invalid_shape {
            return Err(("medication", "must be an object"));
        }
        if existing.is_some() && self.is_empty() && !self.unknown_field {
            return Err(("medication", "must include an attribute"));
        }
        if self.unknown_field {
            return Err(("medication", "contains an unknown attribute"));
        }
        for (field, present) in [
            ("friendly_name", matches!(self.friendly_name, Field::Null)),
            ("barcode", matches!(self.barcode, Field::Null)),
            ("dmd_code", matches!(self.dmd_code, Field::Null)),
            ("dmd_system", matches!(self.dmd_system, Field::Null)),
            (
                "dmd_concept_class",
                matches!(self.dmd_concept_class, Field::Null),
            ),
            ("category", matches!(self.category, Field::Null)),
            ("description", matches!(self.description, Field::Null)),
            ("dose_unit", matches!(self.dose_unit, Field::Null)),
            ("warnings", matches!(self.warnings, Field::Null)),
        ] {
            if present {
                return Err((field, "must be a string"));
            }
        }
        if existing.is_none() || !matches!(self.name, Field::Missing) {
            let name = text(&self.name, "name")?;
            if name.is_none_or(|value| value.trim().is_empty()) {
                return Err(("name", "can't be blank"));
            }
        }
        if existing.is_none() && self.location_for_scope().is_none() {
            return Err(("location_id", "can't be blank"));
        }
        if !matches!(self.location_id, Field::Missing)
            && !self.location_for_scope().is_some_and(|value| value > 0)
        {
            return Err(("location_id", "must be a positive integer"));
        }
        if existing.is_none() && matches!(self.reorder_threshold, Field::Missing) {
            return Err(("reorder_threshold", "can't be blank"));
        }
        for (field, value) in [
            ("dose_amount", &self.dose_amount),
            ("current_supply", &self.current_supply),
            ("reorder_threshold", &self.reorder_threshold),
        ] {
            match value {
                Field::Missing => continue,
                Field::WrongNumber | Field::Invalid => return Err((field, "must be a string")),
                Field::InvalidText => return Err((field, "is not a valid decimal")),
                Field::Null if field == "reorder_threshold" => {
                    return Err((field, "can't be blank"));
                }
                Field::Value(number)
                    if *number < Decimal::ZERO
                        || (field == "dose_amount" && *number == Decimal::ZERO) =>
                {
                    return Err((field, "must be greater than 0"));
                }
                Field::Value(number)
                    if field != "dose_amount" && !validation::valid_stock_decimal(*number) =>
                {
                    return Err((field, "is outside stock precision"));
                }
                _ => {}
            }
        }
        if let Some(unit) = text(&self.dose_unit, "dose_unit")?
            && ![
                "tablet", "capsule", "gummy", "mg", "ml", "g", "mcg", "IU", "spray", "drop",
                "sachet", "pad",
            ]
            .contains(&unit)
        {
            return Err(("dose_unit", "is not included in the list"));
        }
        if let Some(barcode) = text(&self.barcode, "barcode")?
            && !barcode.is_empty()
            && (!matches!(barcode.len(), 13 | 14)
                || !barcode.bytes().all(|byte| byte.is_ascii_digit()))
        {
            return Err(("barcode", "is invalid"));
        }
        if let Some(category) = text(&self.category, "category")?
            && !category.is_empty()
            && !validation::CATEGORIES.contains(&category)
        {
            return Err(("category", "is not included in the list"));
        }
        let code = text(&self.dmd_code, "dmd_code")?
            .map(str::to_owned)
            .or_else(|| existing.and_then(|record| record.dmd_code.clone()));
        if code.as_deref().is_some_and(|value| !value.is_empty()) {
            let system = text(&self.dmd_system, "dmd_system")?
                .map(str::to_owned)
                .or_else(|| existing.and_then(|record| record.dmd_system.clone()));
            if system.as_deref().is_none_or(str::is_empty) {
                return Err(("dmd_system", "can't be blank"));
            }
        }
        if matches!(
            self.default_schedule_type,
            Field::Null | Field::Invalid | Field::InvalidText | Field::WrongNumber
        ) {
            return Err(("default_schedule_type", "is invalid"));
        }
        Ok(MedicationChanges {
            name: cloned_text(&self.name),
            friendly_name: cloned_text(&self.friendly_name),
            barcode: cloned_text(&self.barcode),
            dmd_code: cloned_text(&self.dmd_code),
            dmd_system: cloned_text(&self.dmd_system),
            dmd_concept_class: cloned_text(&self.dmd_concept_class),
            category: cloned_text(&self.category),
            description: cloned_text(&self.description),
            dose_amount: nullable_decimal(&self.dose_amount),
            dose_unit: cloned_text(&self.dose_unit),
            current_supply: nullable_decimal(&self.current_supply),
            reorder_threshold: match self.reorder_threshold {
                Field::Value(value) => Some(value),
                _ => None,
            },
            warnings: cloned_text(&self.warnings),
            location_id: self.location_for_scope().map(LocationId),
            default_schedule_type: match self.default_schedule_type {
                Field::Value(value) => Some(value),
                _ => None,
            },
        })
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct MedicationChanges {
    pub(super) name: Option<String>,
    pub(super) friendly_name: Option<String>,
    pub(super) barcode: Option<String>,
    pub(super) dmd_code: Option<String>,
    pub(super) dmd_system: Option<String>,
    pub(super) dmd_concept_class: Option<String>,
    pub(super) category: Option<String>,
    pub(super) description: Option<String>,
    pub(super) dose_amount: Option<Option<Decimal>>,
    pub(super) dose_unit: Option<String>,
    pub(super) current_supply: Option<Option<Decimal>>,
    pub(super) reorder_threshold: Option<Decimal>,
    pub(super) warnings: Option<String>,
    pub(super) location_id: Option<LocationId>,
    pub(super) default_schedule_type: Option<ScheduleType>,
}

impl MedicationChanges {
    pub(super) fn assign(self, active: &mut medication::ActiveModel) {
        if let Some(value) = self.name {
            active.name = Set(Some(value));
        }
        if let Some(value) = self.friendly_name {
            active.friendly_name = Set(Some(value));
        }
        if let Some(value) = self.barcode {
            active.barcode = Set(Some(value));
        }
        if let Some(value) = self.dmd_code {
            active.dmd_code = Set(Some(value));
        }
        if let Some(value) = self.dmd_system {
            active.dmd_system = Set(Some(value));
        }
        if let Some(value) = self.dmd_concept_class {
            active.dmd_concept_class = Set(Some(value));
        }
        if let Some(value) = self.category {
            active.category = Set(Some(value));
        }
        if let Some(value) = self.description {
            active.description = Set(Some(value));
        }
        if let Some(value) = self.dose_amount {
            active.dose_amount =
                Set(value.map(|amount| amount.to_string().parse::<f64>().unwrap_or(0.0)));
        }
        if let Some(value) = self.dose_unit {
            active.dose_unit = Set(Some(value));
        }
        if let Some(value) = self.current_supply {
            active.current_supply = Set(value);
        }
        if let Some(value) = self.reorder_threshold {
            active.reorder_threshold = Set(value);
        }
        if let Some(value) = self.warnings {
            active.warnings = Set(Some(value));
        }
        if let Some(value) = self.location_id {
            active.location_id = Set(value.0);
        }
        if let Some(value) = self.default_schedule_type {
            active.default_schedule_type = Set(value.code());
        }
    }
}

pub struct MedicationCreate(pub(super) MedicationChanges);

impl MedicationCreate {
    pub fn new(name: String, location: LocationId, reorder_threshold: Decimal) -> Self {
        Self(MedicationChanges {
            name: Some(name),
            location_id: Some(location),
            reorder_threshold: Some(reorder_threshold),
            ..MedicationChanges::default()
        })
    }

    pub fn with_stock(mut self, stock: Decimal) -> Self {
        self.0.current_supply = Some(Some(stock));
        self
    }

    pub(super) fn validate(&self) -> Result<(), (&'static str, &'static str)> {
        if self
            .0
            .name
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        {
            return Err(("name", "can't be blank"));
        }
        if self
            .0
            .reorder_threshold
            .is_none_or(|value| value < Decimal::ZERO)
        {
            return Err(("reorder_threshold", "must be greater than 0"));
        }
        if self
            .0
            .reorder_threshold
            .is_some_and(|value| !validation::valid_stock_decimal(value))
        {
            return Err(("reorder_threshold", "is outside stock precision"));
        }
        if self
            .0
            .current_supply
            .flatten()
            .is_some_and(|value| !validation::valid_stock_decimal(value))
        {
            return Err(("current_supply", "is outside stock precision"));
        }
        Ok(())
    }
}

pub enum CreateInput {
    Draft(MedicationInput),
    Validated(MedicationCreate),
}

impl From<MedicationInput> for CreateInput {
    fn from(value: MedicationInput) -> Self {
        Self::Draft(value)
    }
}

impl From<MedicationCreate> for CreateInput {
    fn from(value: MedicationCreate) -> Self {
        Self::Validated(value)
    }
}

fn take_text(fields: &mut serde_json::Map<String, Value>, key: &str) -> Field<String> {
    match fields.remove(key) {
        None => Field::Missing,
        Some(Value::Null) => Field::Null,
        Some(Value::String(value)) => Field::Value(value),
        Some(Value::Number(_)) => Field::WrongNumber,
        Some(_) => Field::Invalid,
    }
}

fn take_decimal(fields: &mut serde_json::Map<String, Value>, key: &str) -> Field<Decimal> {
    match take_text(fields, key) {
        Field::Value(value) => parse_decimal_text(&value).map_or(Field::InvalidText, Field::Value),
        Field::WrongNumber => Field::WrongNumber,
        Field::Invalid => Field::Invalid,
        Field::Missing => Field::Missing,
        Field::Null => Field::Null,
        Field::InvalidText => Field::InvalidText,
    }
}

fn take_integer(fields: &mut serde_json::Map<String, Value>, key: &str) -> Field<i64> {
    match fields.remove(key) {
        None => Field::Missing,
        Some(Value::Null) => Field::Null,
        Some(Value::Number(value)) => value.as_i64().map_or(Field::Invalid, Field::Value),
        Some(_) => Field::Invalid,
    }
}

fn take_schedule(fields: &mut serde_json::Map<String, Value>, key: &str) -> Field<ScheduleType> {
    match fields.remove(key) {
        None => Field::Missing,
        Some(Value::String(value)) => {
            ScheduleType::from_name(&value).map_or(Field::Invalid, Field::Value)
        }
        Some(Value::Number(value)) => value
            .as_i64()
            .and_then(ScheduleType::from_code)
            .map_or(Field::Invalid, Field::Value),
        Some(_) => Field::Invalid,
    }
}

fn text<'a>(
    field: &'a Field<String>,
    name: &'static str,
) -> Result<Option<&'a str>, (&'static str, &'static str)> {
    match field {
        Field::Missing | Field::Null => Ok(None),
        Field::Value(value) => Ok(Some(value)),
        Field::WrongNumber => Err((name, "must be a string")),
        Field::Invalid | Field::InvalidText => Err((name, "is invalid")),
    }
}

fn cloned_text(field: &Field<String>) -> Option<String> {
    match field {
        Field::Value(value) => Some(value.clone()),
        _ => None,
    }
}

fn nullable_decimal(field: &Field<Decimal>) -> Option<Option<Decimal>> {
    match field {
        Field::Missing => None,
        Field::Null => Some(None),
        Field::Value(value) => Some(Some(*value)),
        _ => None,
    }
}

pub fn parse_decimal_text(text: &str) -> Option<Decimal> {
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
