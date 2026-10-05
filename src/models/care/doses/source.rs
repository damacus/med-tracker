use super::*;

pub(super) struct Source {
    pub(super) kind: &'static str,
    pub(super) id: i64,
    pub(super) active: bool,
    pub(super) retired: bool,
    pub(super) person_id: i64,
    pub(super) medication_id: i64,
    pub(super) dose_amount: Option<Decimal>,
    pub(super) dose_unit: Option<String>,
    pub(super) max_daily_doses: Option<i32>,
    pub(super) min_hours_between_doses: Option<Decimal>,
    pub(super) dose_cycle: Option<i32>,
    pub(super) source_dosage_option_id: Option<i64>,
    pub(super) schedule_type: Option<i32>,
    pub(super) schedule_config: Option<Value>,
    pub(super) start_date: Option<NaiveDate>,
    pub(super) end_date: Option<NaiveDate>,
}

pub(super) fn source_from_assignment(value: person_medication::Model) -> Source {
    Source {
        kind: "person_medication",
        id: value.id,
        active: value.active,
        retired: value.retired_at.is_some(),
        person_id: value.person_id,
        medication_id: value.medication_id,
        dose_amount: value.dose_amount,
        dose_unit: value.dose_unit,
        max_daily_doses: value.max_daily_doses,
        min_hours_between_doses: value.min_hours_between_doses.map(Decimal::from),
        dose_cycle: value.dose_cycle,
        source_dosage_option_id: value.source_dosage_option_id,
        schedule_type: None,
        schedule_config: None,
        start_date: None,
        end_date: None,
    }
}

pub(super) fn source_from_schedule(value: schedule::Model) -> Source {
    Source {
        kind: "schedule",
        id: value.id,
        active: value.active,
        retired: value.retired_at.is_some(),
        person_id: value.person_id,
        medication_id: value.medication_id,
        dose_amount: value.dose_amount,
        dose_unit: value.dose_unit,
        max_daily_doses: value.max_daily_doses,
        min_hours_between_doses: value.min_hours_between_doses.map(Decimal::from),
        dose_cycle: value.dose_cycle,
        source_dosage_option_id: value.source_dosage_option_id,
        schedule_type: Some(value.schedule_type),
        schedule_config: Some(value.schedule_config),
        start_date: value.start_date,
        end_date: value.end_date,
    }
}

pub(super) fn app_zone() -> chrono_tz::Tz {
    std::env::var("TZ")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(chrono_tz::UTC)
}

pub(super) fn local_date_in_zone(time: NaiveDateTime, zone: chrono_tz::Tz) -> NaiveDate {
    Utc.from_utc_datetime(&time)
        .with_timezone(&zone)
        .date_naive()
}

pub(crate) fn config_value<'a>(config: &'a Value, names: &[&str]) -> Option<&'a Value> {
    names.iter().find_map(|name| config.get(*name))
}

pub(crate) fn config_decimal(config: &Value, names: &[&str]) -> Option<Decimal> {
    config_value(config, names)
        .and_then(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .or_else(|| value.as_f64().map(|value| value.to_string()))
        })
        .and_then(|value| Decimal::from_str(&value).ok())
}

pub(super) fn effective_source(source: &mut Source, date: NaiveDate) {
    let Some(config) = source.schedule_config.as_ref() else {
        return;
    };
    let effective = if source.schedule_type == Some(5) {
        config
            .get("taper_steps")
            .and_then(Value::as_array)
            .and_then(|steps| {
                steps.iter().find(|step| {
                    let start = step
                        .get("start_date")
                        .and_then(Value::as_str)
                        .and_then(|v| NaiveDate::parse_from_str(v, "%Y-%m-%d").ok());
                    let end = step
                        .get("end_date")
                        .and_then(Value::as_str)
                        .and_then(|v| NaiveDate::parse_from_str(v, "%Y-%m-%d").ok());
                    start
                        .zip(end)
                        .is_some_and(|(start, end)| date >= start && date <= end)
                })
            })
            .unwrap_or(config)
    } else {
        config
    };
    source.dose_amount =
        config_decimal(effective, &["amount", "dose_amount"]).or(source.dose_amount);
    source.dose_unit = config_value(effective, &["unit", "dose_unit"])
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| source.dose_unit.clone());
    source.max_daily_doses = config_decimal(effective, &["max_daily_doses", "max_doses", "max"])
        .and_then(|value| value.to_string().parse::<i32>().ok())
        .or(source.max_daily_doses);
    source.min_hours_between_doses = config_decimal(
        effective,
        &["min_hours_between_doses", "min_hours", "minimum_hours"],
    )
    .or(source.min_hours_between_doses);
    if source.schedule_type == Some(0)
        && effective
            .get("times")
            .and_then(Value::as_array)
            .is_some_and(|times| {
                times
                    .iter()
                    .filter(|value| !value.is_null() && value.as_str() != Some(""))
                    .count()
                    == 1
            })
    {
        source.min_hours_between_doses = None;
    }
}

pub(super) async fn source(
    db: &DatabaseTransaction,
    context: &DoseContext<'_>,
    household_id: i64,
    kind: &str,
    id: &str,
) -> Result<Source, ApiError> {
    match kind {
        "person_medication" => {
            let mut query = person_medication::Entity::find()
                .filter(person_medication::Column::HouseholdId.eq(household_id));
            query = match id.parse::<i64>() {
                Ok(id) => query.filter(person_medication::Column::Id.eq(id)),
                Err(_) => query.filter(person_medication::Column::PortableId.eq(id)),
            };
            let value = query
                .one(db)
                .await
                .map_err(database_error)?
                .ok_or(OperationError::NotFound)?;
            if !allowed_person(db, context, value.person_id, false).await? {
                return Err(OperationError::NotFound);
            }
            Ok(source_from_assignment(value))
        }
        "schedule" => {
            let mut query =
                schedule::Entity::find().filter(schedule::Column::HouseholdId.eq(household_id));
            query = match id.parse::<i64>() {
                Ok(id) => query.filter(schedule::Column::Id.eq(id)),
                Err(_) => query.filter(schedule::Column::PortableId.eq(id)),
            };
            let value = query
                .one(db)
                .await
                .map_err(database_error)?
                .ok_or(OperationError::NotFound)?;
            if !allowed_person(db, context, value.person_id, false).await? {
                return Err(OperationError::NotFound);
            }
            Ok(source_from_schedule(value))
        }
        _ => Err(OperationError::NotFound),
    }
}
