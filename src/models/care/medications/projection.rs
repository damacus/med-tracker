use super::{database_error, forecast as medication_forecast};
use crate::models::entities::{location, medication};
use crate::models::errors::OperationError as ApiError;
use sea_orm::{ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter};
use serde_json::{Value, json};
use std::collections::HashMap;

pub(crate) async fn serialize_many(
    db: &DatabaseTransaction,
    records: Vec<medication::Model>,
) -> Result<Vec<Value>, ApiError> {
    let location_ids: Vec<i64> = records.iter().map(|record| record.location_id).collect();
    let locations: HashMap<i64, String> = if location_ids.is_empty() {
        HashMap::new()
    } else {
        location::Entity::find()
            .filter(location::Column::Id.is_in(location_ids))
            .all(db)
            .await
            .map_err(database_error)?
            .into_iter()
            .map(|location| (location.id, location.portable_id))
            .collect()
    };
    let forecasts = medication_forecast::for_medications(db, &records)
        .await
        .map_err(database_error)?;
    Ok(records.into_iter().map(|record| {
        let forecast = forecasts.get(&record.id).copied().unwrap_or_default();
        let display_name = record.friendly_name.as_ref().filter(|name| !name.is_empty()).or(record.name.as_ref());
        let low_stock = record.current_supply.is_some_and(|supply| supply <= record.reorder_threshold);
        let out_of_stock = record.current_supply.is_some_and(|supply| supply <= 0.into());
        json!({
            "id": record.id,
            "portable_id": record.portable_id,
            "name": record.name,
            "friendly_name": record.friendly_name,
            "barcode": record.barcode,
            "warnings": record.warnings,
            "display_name": display_name,
            "category": record.category,
            "description": record.description,
            "dose_amount": record.dose_amount.map(|amount| decimal_string(amount.to_string())),
            "dose_unit": record.dose_unit,
            "current_supply": record.current_supply.map(|value| decimal_string(value.to_string())),
            "reorder_threshold": decimal_string(record.reorder_threshold.to_string()),
            "reorder_status": record.reorder_status.map(|value| if value == 1 { "ordered" } else { "received" }),
            "location_id": record.location_id,
            "location_portable_id": locations.get(&record.location_id),
            "updated_at": medication_timestamp(record.updated_at),
            "low_stock": low_stock,
            "out_of_stock": out_of_stock,
            "days_until_low_stock": forecast.days_until_low_stock,
            "days_until_out_of_stock": forecast.days_until_out_of_stock
        })
    }).collect())
}

fn medication_timestamp(value: chrono::NaiveDateTime) -> String {
    value
        .and_utc()
        .to_rfc3339_opts(chrono::SecondsFormat::Micros, true)
}

pub(super) fn decimal_string(value: String) -> String {
    if let Some((whole, fraction)) = value.split_once('.') {
        let fraction = fraction.trim_end_matches('0');
        if fraction.is_empty() {
            format!("{whole}.0")
        } else {
            format!("{whole}.{fraction}")
        }
    } else {
        format!("{value}.0")
    }
}
