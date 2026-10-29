use super::*;
use loco_rs::model::TenantQueryExt;

#[derive(Clone)]
pub(super) struct Source {
    pub(super) kind: &'static str,
    pub(super) zone: Option<chrono_tz::Tz>,
    pub(super) person_schedule_id: Option<String>,
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
        zone: None,
        person_schedule_id: None,
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
        zone: value
            .origin_time_zone
            .as_ref()
            .map(|_| crate::models::care::person_schedules::execution_zone(&value, chrono_tz::UTC)),
        person_schedule_id: value.person_schedule_id,
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

pub(crate) fn app_zone(settings: Option<&Value>) -> chrono_tz::Tz {
    crate::models::settings::text(settings, &["time_zone"])
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
    source.dose_amount = config_decimal(effective, &["amount", "dose_amount"])
        .or_else(|| config_decimal(config, &["amount", "dose_amount"]))
        .or(source.dose_amount);
    source.dose_unit = config_value(effective, &["unit", "dose_unit"])
        .or_else(|| config_value(config, &["unit", "dose_unit"]))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| source.dose_unit.clone());
    source.max_daily_doses = config_decimal(effective, &["max_daily_doses", "max_doses", "max"])
        .or_else(|| config_decimal(config, &["max_daily_doses", "max_doses", "max"]))
        .and_then(|value| value.trunc().to_string().parse::<i32>().ok())
        .or(source.max_daily_doses);
    source.min_hours_between_doses = config_decimal(
        effective,
        &["min_hours_between_doses", "min_hours", "minimum_hours"],
    )
    .or_else(|| {
        config_decimal(
            config,
            &["min_hours_between_doses", "min_hours", "minimum_hours"],
        )
    })
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
    source: &DoseSource,
) -> Result<Source, ApiError> {
    match source {
        DoseSource::Assignment(AssignmentId(id)) => {
            let mut query = person_medication::Entity::find().in_tenant(household_id);
            query = match id {
                SourceId::Database(id) => query.filter(person_medication::Column::Id.eq(*id)),
                SourceId::Portable(id) => {
                    query.filter(person_medication::Column::PortableId.eq(id))
                }
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
        DoseSource::Schedule(ScheduleId(id)) => {
            let mut query = schedule::Entity::find()
                .in_tenant(household_id)
                .filter(schedule::Column::ExecutionEndedAt.is_null());
            query = match id {
                SourceId::Database(id) => query.filter(schedule::Column::Id.eq(*id)),
                SourceId::Portable(id) => query.filter(schedule::Column::PortableId.eq(id)),
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
    }
}

#[cfg(test)]
mod tests {
    use super::super::timing::applies_on;
    use super::*;

    fn test_source() -> Source {
        Source {
            kind: "schedule",
            zone: None,
            person_schedule_id: None,
            id: 1,
            active: true,
            retired: false,
            person_id: 1,
            medication_id: 1,
            dose_amount: None,
            dose_unit: None,
            max_daily_doses: None,
            min_hours_between_doses: None,
            dose_cycle: None,
            source_dosage_option_id: None,
            schedule_type: None,
            schedule_config: None,
            start_date: None,
            end_date: None,
        }
    }

    #[test]
    fn taper_step_boundary_behavior_and_gap_dates() {
        let mut source = test_source();
        source.schedule_type = Some(5);
        source.start_date = Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        source.end_date = Some(NaiveDate::from_ymd_opt(2026, 1, 14).unwrap());
        source.schedule_config = Some(json!({
            "taper_steps": [
                {
                    "start_date": "2026-01-01",
                    "end_date": "2026-01-05",
                    "amount": "10",
                    "unit": "mg"
                },
                {
                    "start_date": "2026-01-08",
                    "end_date": "2026-01-12",
                    "amount": "5",
                    "unit": "mg"
                }
            ]
        }));

        let mut s_start1 = source.clone();
        effective_source(&mut s_start1, NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        assert_eq!(s_start1.dose_amount, Some(Decimal::from(10)));
        assert_eq!(s_start1.dose_unit, Some("mg".to_string()));
        assert!(applies_on(
            &source,
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
        ));

        let mut s_end1 = source.clone();
        effective_source(&mut s_end1, NaiveDate::from_ymd_opt(2026, 1, 5).unwrap());
        assert_eq!(s_end1.dose_amount, Some(Decimal::from(10)));
        assert_eq!(s_end1.dose_unit, Some("mg".to_string()));
        assert!(applies_on(
            &source,
            NaiveDate::from_ymd_opt(2026, 1, 5).unwrap()
        ));

        assert!(!applies_on(
            &source,
            NaiveDate::from_ymd_opt(2026, 1, 6).unwrap()
        ));
        assert!(!applies_on(
            &source,
            NaiveDate::from_ymd_opt(2026, 1, 7).unwrap()
        ));

        let mut s_start2 = source.clone();
        effective_source(&mut s_start2, NaiveDate::from_ymd_opt(2026, 1, 8).unwrap());
        assert_eq!(s_start2.dose_amount, Some(Decimal::from(5)));
        assert_eq!(s_start2.dose_unit, Some("mg".to_string()));
        assert!(applies_on(
            &source,
            NaiveDate::from_ymd_opt(2026, 1, 8).unwrap()
        ));

        let mut s_end2 = source.clone();
        effective_source(&mut s_end2, NaiveDate::from_ymd_opt(2026, 1, 12).unwrap());
        assert_eq!(s_end2.dose_amount, Some(Decimal::from(5)));
        assert_eq!(s_end2.dose_unit, Some("mg".to_string()));
        assert!(applies_on(
            &source,
            NaiveDate::from_ymd_opt(2026, 1, 12).unwrap()
        ));

        assert!(!applies_on(
            &source,
            NaiveDate::from_ymd_opt(2026, 1, 13).unwrap()
        ));
    }

    #[test]
    fn taper_step_inheritance_from_top_level_schedule_config() {
        let mut source = test_source();
        source.schedule_type = Some(5);
        source.start_date = Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        source.end_date = Some(NaiveDate::from_ymd_opt(2026, 1, 31).unwrap());
        source.max_daily_doses = None;
        source.min_hours_between_doses = None;
        source.dose_unit = None;
        source.dose_amount = None;
        source.schedule_config = Some(json!({
            "max_daily_doses": 2,
            "min_hours_between_doses": 6,
            "unit": "mg",
            "taper_steps": [
                {
                    "start_date": "2026-01-05",
                    "end_date": "2026-01-07",
                    "amount": "5"
                }
            ]
        }));

        let active_date = NaiveDate::from_ymd_opt(2026, 1, 6).unwrap();
        effective_source(&mut source, active_date);

        assert_eq!(source.dose_amount, Some(Decimal::from(5)));
        assert_eq!(source.dose_unit, Some("mg".to_string()));
        assert_eq!(source.max_daily_doses, Some(2));
        assert_eq!(source.min_hours_between_doses, Some(Decimal::from(6)));
    }

    #[test]
    fn taper_step_string_and_decimal_integer_parsing_for_max_daily_doses() {
        let mut source_str = test_source();
        source_str.schedule_type = Some(5);
        source_str.start_date = Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        source_str.end_date = Some(NaiveDate::from_ymd_opt(2026, 1, 31).unwrap());
        source_str.schedule_config = Some(json!({
            "taper_steps": [
                {
                    "start_date": "2026-01-05",
                    "end_date": "2026-01-07",
                    "amount": "5",
                    "max_daily_doses": "2"
                }
            ]
        }));
        effective_source(
            &mut source_str,
            NaiveDate::from_ymd_opt(2026, 1, 6).unwrap(),
        );
        assert_eq!(source_str.max_daily_doses, Some(2));

        let mut source_int = test_source();
        source_int.schedule_type = Some(5);
        source_int.start_date = Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        source_int.end_date = Some(NaiveDate::from_ymd_opt(2026, 1, 31).unwrap());
        source_int.schedule_config = Some(json!({
            "taper_steps": [
                {
                    "start_date": "2026-01-05",
                    "end_date": "2026-01-07",
                    "amount": "5",
                    "max_daily_doses": 2
                }
            ]
        }));
        effective_source(
            &mut source_int,
            NaiveDate::from_ymd_opt(2026, 1, 6).unwrap(),
        );
        assert_eq!(source_int.max_daily_doses, Some(2));

        let mut source_top_str = test_source();
        source_top_str.schedule_type = Some(0);
        source_top_str.schedule_config = Some(json!({
            "max_daily_doses": "2"
        }));
        effective_source(
            &mut source_top_str,
            NaiveDate::from_ymd_opt(2026, 1, 6).unwrap(),
        );
        assert_eq!(source_top_str.max_daily_doses, Some(2));

        let mut source_top_int = test_source();
        source_top_int.schedule_type = Some(0);
        source_top_int.schedule_config = Some(json!({
            "max_daily_doses": 2
        }));
        effective_source(
            &mut source_top_int,
            NaiveDate::from_ymd_opt(2026, 1, 6).unwrap(),
        );
        assert_eq!(source_top_int.max_daily_doses, Some(2));
    }
}
