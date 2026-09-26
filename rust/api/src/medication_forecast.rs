use crate::entities::{medication, person_medication, schedule};
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
) -> Result<HashMap<i64, Forecast>, DbErr> {
    if medications.is_empty() {
        return Ok(HashMap::new());
    }
    let ids: Vec<i64> = medications.iter().map(|medication| medication.id).collect();
    let mut daily_rates = HashMap::<i64, Decimal>::new();
    let today =
        date_in_zone(Utc::now(), std::env::var("TZ").ok().as_deref()).map_err(DbErr::Custom)?;
    let mut last_schedule_id = 0;
    loop {
        let rows = schedule::Entity::find()
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
    if schedule.schedule_type == 5 {
        if let Some(steps) = schedule
            .schedule_config
            .get("taper_steps")
            .and_then(Value::as_array)
        {
            if let Some(step) = steps.iter().find(|step| {
                let start = step
                    .get("start_date")
                    .and_then(Value::as_str)
                    .and_then(parse_date);
                let end = step
                    .get("end_date")
                    .and_then(Value::as_str)
                    .and_then(parse_date);
                matches!((start, end), (Some(start), Some(end)) if start <= today && today <= end)
            }) {
                return step;
            }
        }
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
    use super::date_in_zone;
    use chrono::{TimeZone, Utc};

    #[test]
    fn forecast_date_defaults_to_utc_and_honours_configured_iana_zone() {
        let utc = Utc.with_ymd_and_hms(2026, 9, 25, 0, 30, 0).unwrap();
        assert_eq!(date_in_zone(utc, None).unwrap().to_string(), "2026-09-25");
        assert_eq!(
            date_in_zone(utc, Some("America/Los_Angeles"))
                .unwrap()
                .to_string(),
            "2026-09-24"
        );
        assert!(date_in_zone(utc, Some("invalid-zone")).is_err());
    }
}
