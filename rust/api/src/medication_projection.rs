use crate::entities::{location, medication};
use crate::{database_error, medication_forecast, ApiError};
use sea_orm::{ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter};
use serde_json::{json, Value};
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

#[cfg(test)]
mod tests {
    use super::decimal_string;
    use crate::representation_etag;
    use serde_json::json;

    #[test]
    fn medication_versions_distinguish_writes_within_one_second() {
        let before = chrono::NaiveDateTime::parse_from_str(
            "2026-10-01T09:00:00.123456",
            "%Y-%m-%dT%H:%M:%S%.f",
        )
        .unwrap();
        let after = before + chrono::Duration::microseconds(1);
        let initial = json!({"data": {"updated_at": super::medication_timestamp(before)}});
        let changed = json!({"data": {"updated_at": super::medication_timestamp(after)}});
        assert_ne!(representation_etag(&initial), representation_etag(&changed));
        assert_eq!(initial["data"]["updated_at"], "2026-10-01T09:00:00.123456Z");
    }

    #[test]
    fn etag_changes_with_forecast_and_location_in_the_response() {
        let initial =
            json!({"data": {"id": 1, "location_portable_id": "A", "days_until_low_stock": null}});
        let changed_forecast =
            json!({"data": {"id": 1, "location_portable_id": "A", "days_until_low_stock": 3}});
        let changed_location =
            json!({"data": {"id": 1, "location_portable_id": "B", "days_until_low_stock": null}});
        assert_ne!(
            representation_etag(&initial),
            representation_etag(&changed_forecast)
        );
        assert_ne!(
            representation_etag(&initial),
            representation_etag(&changed_location)
        );
        assert_eq!(representation_etag(&initial), representation_etag(&initial));
    }

    #[test]
    fn decimal_strings_keep_integer_digits_and_one_fraction_digit() {
        assert_eq!(decimal_string("80.00".to_owned()), "80.0");
        assert_eq!(decimal_string("10.00".to_owned()), "10.0");
        assert_eq!(decimal_string("0.00".to_owned()), "0.0");
        assert_eq!(decimal_string("80".to_owned()), "80.0");
        assert_eq!(decimal_string("2.125".to_owned()), "2.125");
    }
}
