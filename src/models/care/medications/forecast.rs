use crate::models::entities::{medication, person_medication, schedule};
use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::prelude::Decimal;
use sea_orm::{
    ColumnTrait, DatabaseTransaction, DbErr, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Clone, Copy, Default)]
pub struct Forecast {
    pub days_until_low_stock: Option<i64>,
    pub days_until_out_of_stock: Option<i64>,
}

pub async fn for_medications(
    db: &DatabaseTransaction,
    medications: &[medication::Model],
    settings: Option<&serde_json::Value>,
) -> Result<HashMap<i64, Forecast>, DbErr> {
    if medications.is_empty() {
        return Ok(HashMap::new());
    }
    let ids: Vec<i64> = medications.iter().map(|medication| medication.id).collect();
    let mut daily_rates = HashMap::<i64, Decimal>::new();
    let today = date_in_zone(
        Utc::now(),
        crate::models::settings::text(settings, &["time_zone"]),
    )
    .map_err(DbErr::Custom)?;
    let mut last_schedule_id = 0;
    loop {
        let rows = schedule::Entity::find()
            .filter(
                schedule::Column::HouseholdId.is_in(medications.iter().map(|row| row.household_id)),
            )
            .filter(schedule::Column::MedicationId.is_in(ids.clone()))
            .filter(schedule::Column::Id.gt(last_schedule_id))
            .filter(schedule::Column::Active.eq(true))
            .filter(schedule::Column::StartDate.lte(today))
            .filter(schedule::Column::EndDate.gte(today))
            .order_by_asc(schedule::Column::Id)
            .limit(256)
            .all(db)
            .await?;
        if rows.is_empty() {
            break;
        }
        for row in &rows {
            if let Some(rate) = schedule_rate(row, today) {
                *daily_rates.entry(row.medication_id).or_default() += rate;
            }
        }
        last_schedule_id = rows.last().map(|row| row.id).unwrap_or(last_schedule_id);
        if rows.len() < 256 {
            break;
        }
    }
    let mut last_assignment_id = 0;
    loop {
        let rows = person_medication::Entity::find()
            .filter(
                person_medication::Column::HouseholdId
                    .is_in(medications.iter().map(|row| row.household_id)),
            )
            .filter(person_medication::Column::MedicationId.is_in(ids.clone()))
            .filter(person_medication::Column::Id.gt(last_assignment_id))
            .order_by_asc(person_medication::Column::Id)
            .limit(256)
            .all(db)
            .await?;
        if rows.is_empty() {
            break;
        }
        for row in &rows {
            if let (Some(max), Some(quantity)) = (
                row.max_daily_doses,
                stock_quantity(row.dose_amount, row.dose_unit.as_deref()),
            ) {
                *daily_rates.entry(row.medication_id).or_default() += Decimal::from(max) * quantity;
            }
        }
        last_assignment_id = rows.last().map(|row| row.id).unwrap_or(last_assignment_id);
        if rows.len() < 256 {
            break;
        }
    }
    Ok(medications
        .iter()
        .map(|medication| {
            let rate = daily_rates.get(&medication.id).copied().unwrap_or_default();
            (medication.id, forecast(medication, rate))
        })
        .collect())
}

fn date_in_zone(now: DateTime<Utc>, configured_zone: Option<&str>) -> Result<NaiveDate, String> {
    match configured_zone {
        None => Ok(now.date_naive()),
        Some(name) => name
            .parse::<chrono_tz::Tz>()
            .map(|zone| now.with_timezone(&zone).date_naive())
            .map_err(|_| format!("invalid TZ time zone: {name}")),
    }
}

fn schedule_rate(schedule: &schedule::Model, today: NaiveDate) -> Option<Decimal> {
    let max = schedule.max_daily_doses?;
    let config = effective_config(schedule, today);
    let amount = config_decimal(config, &["amount", "dose_amount"]).or(schedule.dose_amount);
    let unit = config_string(config, &["unit", "dose_unit"]).or(schedule.dose_unit.as_deref());
    let quantity = stock_quantity(amount, unit)?;
    let cycle_days = match schedule.dose_cycle {
        Some(1) => Decimal::from(7),
        Some(2) => Decimal::from_str("30.436875").ok()?,
        _ => Decimal::ONE,
    };
    Some(Decimal::from(max) / cycle_days * quantity)
}

fn effective_config(schedule: &schedule::Model, today: NaiveDate) -> &Value {
    if schedule.schedule_type == 5
        && let Some(steps) = schedule
            .schedule_config
            .get("taper_steps")
            .and_then(Value::as_array)
        && let Some(step) = steps.iter().find(|step| {
            let start = step
                .get("start_date")
                .and_then(Value::as_str)
                .and_then(parse_date);
            let end = step
                .get("end_date")
                .and_then(Value::as_str)
                .and_then(parse_date);
            matches!((start, end), (Some(start), Some(end)) if start <= today && today <= end)
        })
    {
        return step;
    }
    &schedule.schedule_config
}

fn parse_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

fn config_decimal(config: &Value, names: &[&str]) -> Option<Decimal> {
    names.iter().find_map(|name| {
        config.get(*name).and_then(|value| {
            let value = value
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| value.to_string());
            Decimal::from_str(&value).ok()
        })
    })
}

fn config_string<'a>(config: &'a Value, names: &[&str]) -> Option<&'a str> {
    names
        .iter()
        .find_map(|name| config.get(*name).and_then(Value::as_str))
        .filter(|value| !value.is_empty())
}

fn stock_quantity(amount: Option<Decimal>, unit: Option<&str>) -> Option<Decimal> {
    let amount = amount?;
    if matches!(
        unit,
        Some("tablet" | "capsule" | "gummy" | "sachet" | "spray" | "drop" | "pad" | "ml")
    ) {
        Some(amount)
    } else {
        Some(Decimal::ONE)
    }
}

fn forecast(medication: &medication::Model, daily_rate: Decimal) -> Forecast {
    let Some(current) = medication.current_supply else {
        return Forecast::default();
    };
    if daily_rate <= Decimal::ZERO {
        return Forecast::default();
    }
    Forecast {
        days_until_low_stock: days(current - medication.reorder_threshold, daily_rate),
        days_until_out_of_stock: days(current, daily_rate),
    }
}

fn days(remaining: Decimal, daily_rate: Decimal) -> Option<i64> {
    if remaining <= Decimal::ZERO {
        return Some(0);
    }
    (remaining / daily_rate).ceil().to_string().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn day(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn schedule_model(config: Value) -> schedule::Model {
        schedule::Model {
            id: 1,
            household_id: 1,
            portable_id: "s1".to_string(),
            person_id: 1,
            medication_id: 1,
            active: true,
            retired_at: None,
            start_date: Some(day(2026, 1, 1)),
            end_date: Some(day(2026, 12, 31)),
            max_daily_doses: None,
            min_hours_between_doses: None,
            source_dosage_option_id: None,
            dose_cycle: None,
            dose_amount: None,
            dose_unit: None,
            schedule_type: 1,
            schedule_config: config,
            frequency: None,
            notes: None,
            created_at: day(2026, 1, 1).and_hms_opt(0, 0, 0).unwrap(),
            updated_at: day(2026, 1, 1).and_hms_opt(0, 0, 0).unwrap(),
        }
    }

    fn medication_model(supply: Option<Decimal>, threshold: Decimal) -> medication::Model {
        medication::Model {
            id: 1,
            household_id: 1,
            created_at: day(2026, 1, 1).and_hms_opt(0, 0, 0).unwrap(),
            created_by_membership_id: None,
            portable_id: "m1".to_string(),
            name: None,
            friendly_name: None,
            category: None,
            description: None,
            barcode: None,
            dmd_code: None,
            dmd_system: None,
            dmd_concept_class: None,
            dose_amount: None,
            dose_unit: None,
            current_supply: supply,
            supply_at_last_restock: None,
            reorder_threshold: threshold,
            reorder_status: None,
            order_supplier: None,
            order_quantity: None,
            expiry_date: None,
            expected_arrival_on: None,
            ordered_at: None,
            reordered_at: None,
            warnings: None,
            default_schedule_type: 1,
            default_schedule_config: json!({}),
            location_id: Some(1),
            updated_at: day(2026, 1, 1).and_hms_opt(0, 0, 0).unwrap(),
        }
    }

    #[test]
    fn days_rounds_up_and_floors_at_zero() {
        assert_eq!(days(Decimal::from(10), Decimal::from(3)), Some(4));
        assert_eq!(days(Decimal::from(10), Decimal::from(2)), Some(5));
        assert_eq!(days(Decimal::from(0), Decimal::from(3)), Some(0));
        assert_eq!(days(Decimal::from(-5), Decimal::from(3)), Some(0));
    }

    #[test]
    fn stock_quantity_counts_discrete_units_only() {
        let two = Decimal::from(2);
        for unit in [
            "tablet", "capsule", "gummy", "sachet", "spray", "drop", "pad", "ml",
        ] {
            assert_eq!(stock_quantity(Some(two), Some(unit)), Some(two), "{unit}");
        }
        assert_eq!(stock_quantity(Some(two), Some("puff")), Some(Decimal::ONE));
        assert_eq!(stock_quantity(Some(two), None), Some(Decimal::ONE));
        assert_eq!(stock_quantity(None, Some("tablet")), None);
    }

    #[test]
    fn forecast_is_empty_without_supply_or_consumption() {
        let no_supply = medication_model(None, Decimal::from(10));
        assert_eq!(
            forecast(&no_supply, Decimal::from(2)).days_until_out_of_stock,
            None
        );
        let stocked = medication_model(Some(Decimal::from(30)), Decimal::from(10));
        assert_eq!(forecast(&stocked, Decimal::ZERO).days_until_low_stock, None);
    }

    #[test]
    fn forecast_projects_low_and_out_of_stock_days() {
        let stocked = medication_model(Some(Decimal::from(30)), Decimal::from(10));
        let projected = forecast(&stocked, Decimal::from(2));
        assert_eq!(projected.days_until_low_stock, Some(10));
        assert_eq!(projected.days_until_out_of_stock, Some(15));
        let below_threshold = medication_model(Some(Decimal::from(5)), Decimal::from(10));
        let projected = forecast(&below_threshold, Decimal::from(2));
        assert_eq!(projected.days_until_low_stock, Some(0));
        assert_eq!(projected.days_until_out_of_stock, Some(3));
    }

    #[test]
    fn schedule_rate_uses_cycle_length_and_config_amount() {
        let mut schedule = schedule_model(json!({"amount": "2", "unit": "tablet"}));
        schedule.max_daily_doses = Some(3);
        schedule.dose_cycle = Some(1);
        assert_eq!(
            schedule_rate(&schedule, day(2026, 6, 1)),
            Some(Decimal::from(3) / Decimal::from(7) * Decimal::from(2))
        );
        schedule.dose_cycle = Some(2);
        assert_eq!(
            schedule_rate(&schedule, day(2026, 6, 1)),
            Some(Decimal::from(3) / Decimal::from_str("30.436875").unwrap() * Decimal::from(2))
        );
        schedule.dose_cycle = None;
        assert_eq!(
            schedule_rate(&schedule, day(2026, 6, 1)),
            Some(Decimal::from(6))
        );
    }

    #[test]
    fn schedule_rate_falls_back_to_schedule_columns_and_skips_missing_data() {
        let mut schedule = schedule_model(json!({}));
        schedule.dose_amount = Some(Decimal::from(1));
        schedule.dose_unit = Some("tablet".to_string());
        assert_eq!(schedule_rate(&schedule, day(2026, 6, 1)), None);
        schedule.max_daily_doses = Some(2);
        assert_eq!(
            schedule_rate(&schedule, day(2026, 6, 1)),
            Some(Decimal::from(2))
        );
        schedule.dose_amount = None;
        assert_eq!(schedule_rate(&schedule, day(2026, 6, 1)), None);
    }

    #[test]
    fn effective_config_prefers_the_covering_taper_step() {
        let step = json!({"start_date": "2026-06-01", "end_date": "2026-06-07", "amount": "1"});
        let mut schedule = schedule_model(json!({"amount": "4", "taper_steps": [step.clone()]}));
        assert_eq!(
            effective_config(&schedule, day(2026, 6, 3)),
            &schedule.schedule_config
        );
        schedule.schedule_type = 5;
        assert_eq!(effective_config(&schedule, day(2026, 6, 3)), &step);
        assert_eq!(
            effective_config(&schedule, day(2026, 6, 10)),
            &schedule.schedule_config
        );
    }

    #[test]
    fn config_decimal_and_string_pick_first_named_value() {
        let config = json!({"amount": "2.5", "unit": "tablet", "empty": ""});
        assert_eq!(
            config_decimal(&config, &["dose_amount", "amount"]),
            Decimal::from_str("2.5").ok()
        );
        assert_eq!(config_decimal(&config, &["missing"]), None);
        assert_eq!(
            config_string(&config, &["dose_unit", "unit"]),
            Some("tablet")
        );
        assert_eq!(config_string(&config, &["empty", "missing"]), None);
        assert_eq!(config_string(&json!({"unit": 5}), &["unit"]), None);
    }

    #[test]
    fn date_in_zone_rejects_unknown_zones() {
        let now = "2026-06-01T14:00:00Z".parse::<DateTime<Utc>>().unwrap();
        assert_eq!(date_in_zone(now, None), Ok(day(2026, 6, 1)));
        assert_eq!(
            date_in_zone(now, Some("Pacific/Auckland")),
            Ok(day(2026, 6, 2))
        );
        assert_eq!(
            date_in_zone(now, Some("Europe/London")),
            Ok(day(2026, 6, 1))
        );
        assert!(date_in_zone(now, Some("Not/AZone")).is_err());
    }
}
