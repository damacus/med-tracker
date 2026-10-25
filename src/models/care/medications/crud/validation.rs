use super::*;
use crate::models::entities::location;
use loco_rs::model::TenantQueryExt;

pub(super) const CATEGORIES: &[&str] = &[
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
];

pub(super) async fn valid_location(
    db: &DatabaseTransaction,
    household_id: i64,
    location_id: Option<i64>,
) -> Result<bool, ApiError> {
    let Some(id) = location_id else {
        return Ok(true);
    };
    Ok(location::Entity::find_by_id(id)
        .in_tenant(household_id)
        .one(db)
        .await
        .map_err(database_error)?
        .is_some_and(|location| location.household_id == household_id))
}

pub(crate) fn valid_stock_decimal(value: Decimal) -> bool {
    value >= Decimal::ZERO && value.normalize().scale() <= 2 && value < Decimal::from(100_000_000)
}

pub(super) async fn barcode_conflict(
    db: &DatabaseTransaction,
    barcode: Option<&str>,
    existing_id: Option<i64>,
) -> Result<bool, ApiError> {
    let Some(barcode) = barcode.filter(|value| !value.is_empty()) else {
        return Ok(false);
    };
    let row = medication::Entity::find()
        .filter(medication::Column::Barcode.eq(barcode))
        .one(db)
        .await
        .map_err(database_error)?;
    Ok(row.is_some_and(|row| Some(row.id) != existing_id))
}
