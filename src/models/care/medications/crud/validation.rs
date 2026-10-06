use super::*;
use crate::models::entities::location;
pub(super) async fn valid_location(
    db: &DatabaseTransaction,
    household_id: i64,
    attributes: &Value,
) -> Result<bool, ApiError> {
    let Some(value) = attributes.get("location_id") else {
        return Ok(true);
    };
    let id = value.as_i64();
    let Some(id) = id else {
        return Ok(true);
    };
    Ok(location::Entity::find_by_id(id)
        .filter(location::Column::HouseholdId.eq(household_id))
        .one(db)
        .await
        .map_err(database_error)?
        .is_some_and(|location| location.household_id == household_id))
}

fn scalar_string(value: &Value, field: &str) -> Result<Option<String>, &'static str> {
    match value.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(Value::Number(_)) => Err("must be a string"),
        _ => Err("is invalid"),
    }
}

pub(super) fn decimal_field(value: &Value, field: &str) -> Result<Option<Decimal>, &'static str> {
    let Some(raw) = scalar_string(value, field)? else {
        return Ok(None);
    };
    Decimal::from_str(&raw)
        .map(Some)
        .map_err(|_| "is not a number")
}

pub(crate) fn valid_stock_decimal(value: Decimal) -> bool {
    value >= Decimal::ZERO && value.normalize().scale() <= 2 && value < Decimal::from(100_000_000)
}

fn location_id(value: &Value) -> Option<i64> {
    value.get("location_id").and_then(|id| {
        id.as_i64()
            .or_else(|| id.as_str().and_then(|id| id.parse::<i64>().ok()))
    })
}

pub(super) fn validate_attributes(
    attributes: &Value,
    existing: Option<&medication::Model>,
) -> Result<(), (&'static str, &'static str)> {
    const FIELDS: &[&str] = &[
        "name",
        "friendly_name",
        "barcode",
        "dmd_code",
        "dmd_system",
        "dmd_concept_class",
        "category",
        "description",
        "dose_amount",
        "dose_unit",
        "current_supply",
        "reorder_threshold",
        "warnings",
        "location_id",
        "default_schedule_type",
    ];
    let Some(object) = attributes.as_object() else {
        return Err(("medication", "must be an object"));
    };
    if object.is_empty() && existing.is_some() {
        return Err(("medication", "must include an attribute"));
    }
    if object.keys().any(|key| !FIELDS.contains(&key.as_str())) {
        return Err(("medication", "contains an unknown attribute"));
    }
    for field in [
        "friendly_name",
        "barcode",
        "dmd_code",
        "dmd_system",
        "dmd_concept_class",
        "category",
        "description",
        "dose_unit",
        "warnings",
    ] {
        if attributes.get(field).is_some_and(Value::is_null) {
            return Err((field, "must be a string"));
        }
    }
    if existing.is_none() || attributes.get("name").is_some() {
        let name = scalar_string(attributes, "name").map_err(|error| ("name", error))?;
        if name.as_deref().is_none_or(|name| name.trim().is_empty()) {
            return Err(("name", "can't be blank"));
        }
    }
    if existing.is_none() && location_id(attributes).is_none() {
        return Err(("location_id", "can't be blank"));
    }
    if attributes.get("location_id").is_some()
        && !attributes
            .get("location_id")
            .and_then(Value::as_i64)
            .is_some_and(|id| id > 0)
    {
        return Err(("location_id", "must be a positive integer"));
    }
    if existing.is_none() && attributes.get("reorder_threshold").is_none() {
        return Err(("reorder_threshold", "can't be blank"));
    }
    for field in ["dose_amount", "current_supply", "reorder_threshold"] {
        if attributes.get(field).is_none() {
            continue;
        }
        if let Some(value) = attributes.get(field).filter(|value| !value.is_null()) {
            if !value.is_string() {
                return Err((field, "must be a string"));
            }
            if parse_decimal(value).is_none() {
                return Err((field, "is not a valid decimal"));
            }
        }
        let value = decimal_field(attributes, field).map_err(|error| (field, error))?;
        if let Some(value) = value {
            if value < Decimal::ZERO || (field == "dose_amount" && value == Decimal::ZERO) {
                return Err((field, "must be greater than 0"));
            }
            if field != "dose_amount" && !valid_stock_decimal(value) {
                return Err((field, "is outside stock precision"));
            }
        } else if field == "reorder_threshold" {
            return Err((field, "can't be blank"));
        }
    }
    if let Some(unit) =
        scalar_string(attributes, "dose_unit").map_err(|error| ("dose_unit", error))?
        && ![
            "tablet", "capsule", "gummy", "mg", "ml", "g", "mcg", "IU", "spray", "drop", "sachet",
            "pad",
        ]
        .contains(&unit.as_str())
    {
        return Err(("dose_unit", "is not included in the list"));
    }
    if let Some(barcode) =
        scalar_string(attributes, "barcode").map_err(|error| ("barcode", error))?
        && !barcode.is_empty()
        && (!matches!(barcode.len(), 13 | 14) || !barcode.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Err(("barcode", "is invalid"));
    }
    if let Some(category) =
        scalar_string(attributes, "category").map_err(|error| ("category", error))?
        && !category.is_empty()
        && ![
            "Analgesic",
            "Antibiotic",
            "Anticoagulant",
            "Anticonvulsant",
            "Antidepressant",
            "Antidiabetic",
            "Antiemetic",
            "Antifungal",
            "Antihistamine",
            "Antihypertensive",
            "Anti-Inflammatory",
            "Antiparasitic",
            "Antipsychotic",
            "Antiviral",
            "Anxiolytic",
            "Cardiovascular",
            "Cholesterol",
            "Contraceptive",
            "Dermatological",
            "Gastrointestinal",
            "Hormonal",
            "Immunosuppressant",
            "Migraine",
            "Mineral",
            "Muscle Relaxant",
            "Neurological",
            "Oncology",
            "Ophthalmic",
            "Osmotic Laxative",
            "Opioid",
            "Osteoporosis",
            "Respiratory",
            "Sleep Aid",
            "Smoking Cessation",
            "Supplement",
            "Thyroid",
            "Urological",
            "Vitamin",
            "Weight Management",
        ]
        .contains(&category.as_str())
    {
        return Err(("category", "is not included in the list"));
    }
    let code = if attributes.get("dmd_code").is_some() {
        scalar_string(attributes, "dmd_code").map_err(|error| ("dmd_code", error))?
    } else {
        existing.and_then(|record| record.dmd_code.clone())
    };
    if code.as_deref().is_some_and(|value| !value.is_empty()) {
        let system = if attributes.get("dmd_system").is_some() {
            scalar_string(attributes, "dmd_system").map_err(|error| ("dmd_system", error))?
        } else {
            existing.and_then(|record| record.dmd_system.clone())
        };
        if system.as_deref().is_none_or(str::is_empty) {
            return Err(("dmd_system", "can't be blank"));
        }
    }
    if let Some(schedule_type) = attributes.get("default_schedule_type") {
        let valid = schedule_type
            .as_i64()
            .is_some_and(|value| (0..=6).contains(&value))
            || schedule_type.as_str().is_some_and(|value| {
                [
                    "daily",
                    "multiple_daily",
                    "weekly",
                    "specific_dates",
                    "prn",
                    "tapering",
                    "every_other_day",
                ]
                .contains(&value)
            });
        if !valid {
            return Err(("default_schedule_type", "is invalid"));
        }
    }
    Ok(())
}

pub(super) async fn barcode_conflict(
    db: &DatabaseTransaction,
    attributes: &Value,
    existing_id: Option<i64>,
) -> Result<bool, ApiError> {
    let Some(barcode) = attributes.get("barcode").and_then(Value::as_str) else {
        return Ok(false);
    };
    if barcode.is_empty() {
        return Ok(false);
    }
    let row = medication::Entity::find()
        .filter(medication::Column::Barcode.eq(barcode))
        .one(db)
        .await
        .map_err(database_error)?;
    Ok(row.is_some_and(|row| Some(row.id) != existing_id))
}

pub(super) fn assign_attributes(
    active: &mut medication::ActiveModel,
    attributes: &Value,
) -> Result<(), (&'static str, &'static str)> {
    for field in [
        "name",
        "friendly_name",
        "category",
        "description",
        "dose_unit",
        "barcode",
        "dmd_code",
        "dmd_system",
        "dmd_concept_class",
        "warnings",
    ] {
        if attributes.get(field).is_none() {
            continue;
        }
        let value = scalar_string(attributes, field).map_err(|error| (field, error))?;
        match field {
            "name" => active.name = Set(value),
            "friendly_name" => active.friendly_name = Set(value),
            "category" => active.category = Set(value),
            "description" => active.description = Set(value),
            "dose_unit" => active.dose_unit = Set(value),
            "barcode" => active.barcode = Set(value),
            "dmd_code" => active.dmd_code = Set(value),
            "dmd_system" => active.dmd_system = Set(value),
            "dmd_concept_class" => active.dmd_concept_class = Set(value),
            "warnings" => active.warnings = Set(value),
            _ => {}
        }
    }
    if let Some(id) = location_id(attributes) {
        active.location_id = Set(id);
    }
    if attributes.get("dose_amount").is_some() {
        let value =
            decimal_field(attributes, "dose_amount").map_err(|error| ("dose_amount", error))?;
        active.dose_amount =
            Set(value.map(|value| value.to_string().parse::<f64>().unwrap_or(0.0)));
    }
    if attributes.get("current_supply").is_some() {
        active.current_supply = Set(decimal_field(attributes, "current_supply")
            .map_err(|error| ("current_supply", error))?);
    }
    if attributes.get("reorder_threshold").is_some() {
        active.reorder_threshold = Set(decimal_field(attributes, "reorder_threshold")
            .map_err(|error| ("reorder_threshold", error))?
            .unwrap_or(Decimal::ZERO));
    }
    if let Some(value) = attributes.get("default_schedule_type") {
        let names = [
            "daily",
            "multiple_daily",
            "weekly",
            "specific_dates",
            "prn",
            "tapering",
            "every_other_day",
        ];
        let number = value
            .as_i64()
            .or_else(|| {
                value.as_str().and_then(|value| {
                    names
                        .iter()
                        .position(|name| *name == value)
                        .map(|index| index as i64)
                })
            })
            .ok_or(("default_schedule_type", "is invalid"))?;
        active.default_schedule_type = Set(number as i32);
    }
    Ok(())
}

pub(crate) fn parse_decimal(value: &Value) -> Option<Decimal> {
    let text = value.as_str()?;
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
