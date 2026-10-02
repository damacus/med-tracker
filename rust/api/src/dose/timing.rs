use super::*;

pub(super) fn applies_on(source: &Source, date: NaiveDate) -> bool {
    let Some(kind) = source.schedule_type else {
        return true;
    };
    let (Some(start), Some(end)) = (source.start_date, source.end_date) else {
        return false;
    };
    if date < start || date > end {
        return false;
    }
    let config = source.schedule_config.as_ref();
    match kind {
        2 => config
            .and_then(|v| v.get("weekdays"))
            .and_then(Value::as_array)
            .is_some_and(|days| {
                days.iter().any(|day| {
                    let weekday = date.format("%A").to_string().to_lowercase();
                    let short = &weekday[..3];
                    day.as_str().is_some_and(|value| {
                        value.eq_ignore_ascii_case(&weekday) || value.eq_ignore_ascii_case(short)
                    }) || day.as_u64() == Some(date.format("%w").to_string().parse().unwrap_or(7))
                })
            }),
        3 => config
            .and_then(|v| v.get("dates"))
            .and_then(Value::as_array)
            .is_some_and(|dates| {
                dates
                    .iter()
                    .any(|value| value.as_str() == Some(&date.to_string()))
            }),
        5 => config
            .and_then(|v| v.get("taper_steps"))
            .and_then(Value::as_array)
            .is_some_and(|steps| {
                steps.iter().any(|step| {
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
            }),
        6 => (date - start).num_days() % 2 == 0,
        _ => true,
    }
}

fn local_midnight_utc(date: NaiveDate, zone: chrono_tz::Tz) -> NaiveDateTime {
    let midnight = date.and_hms_opt(0, 0, 0).expect("midnight is valid");
    zone.from_local_datetime(&midnight)
        .earliest()
        .map(|time| time.with_timezone(&Utc).naive_utc())
        .unwrap_or(midnight)
}

fn cycle_bounds(time: NaiveDateTime, cycle: Option<i32>) -> (NaiveDateTime, NaiveDateTime) {
    cycle_bounds_in_zone(time, cycle, app_zone())
}

pub(super) fn cycle_bounds_in_zone(
    time: NaiveDateTime,
    cycle: Option<i32>,
    zone: chrono_tz::Tz,
) -> (NaiveDateTime, NaiveDateTime) {
    let date = local_date_in_zone(time, zone);
    let start = match cycle.unwrap_or(0) {
        1 => date - Duration::days(date.weekday().num_days_from_monday() as i64),
        2 => NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap_or(date),
        _ => date,
    };
    let end = match cycle.unwrap_or(0) {
        1 => start + Duration::weeks(1),
        2 => {
            let (year, month) = if start.month() == 12 {
                (start.year() + 1, 1)
            } else {
                (start.year(), start.month() + 1)
            };
            NaiveDate::from_ymd_opt(year, month, 1).unwrap_or(start + Duration::days(31))
        }
        _ => start + Duration::days(1),
    };
    (
        local_midnight_utc(start, zone),
        local_midnight_utc(end, zone),
    )
}

pub(super) async fn timing_allowed(
    db: &DatabaseTransaction,
    proposed: &ProposedTake,
) -> Result<bool, ApiError> {
    let assignments = person_medication::Entity::find()
        .filter(person_medication::Column::PersonId.eq(proposed.source.person_id))
        .filter(person_medication::Column::MedicationId.eq(proposed.source.medication_id))
        .filter(person_medication::Column::Active.eq(true))
        .filter(person_medication::Column::RetiredAt.is_null())
        .all(db)
        .await
        .map_err(database_error)?;
    let schedules = schedule::Entity::find()
        .filter(schedule::Column::PersonId.eq(proposed.source.person_id))
        .filter(schedule::Column::MedicationId.eq(proposed.source.medication_id))
        .filter(schedule::Column::Active.eq(true))
        .filter(schedule::Column::RetiredAt.is_null())
        .all(db)
        .await
        .map_err(database_error)?;
    let related: Vec<Source> = assignments
        .into_iter()
        .map(source_from_assignment)
        .chain(
            schedules
                .into_iter()
                .map(source_from_schedule)
                .filter(|source| applies_on(source, local_date(proposed.taken_at))),
        )
        .collect();
    if related.is_empty() {
        return Ok(true);
    }
    let schedule_ids: Vec<i64> = related
        .iter()
        .filter(|source| source.kind == "schedule")
        .map(|source| source.id)
        .collect();
    let assignment_ids: Vec<i64> = related
        .iter()
        .filter(|source| source.kind == "person_medication")
        .map(|source| source.id)
        .collect();
    let mut condition = Condition::any();
    if !schedule_ids.is_empty() {
        condition = condition.add(medication_take::Column::ScheduleId.is_in(schedule_ids));
    }
    if !assignment_ids.is_empty() {
        condition =
            condition.add(medication_take::Column::PersonMedicationId.is_in(assignment_ids));
    }
    let takes = medication_take::Entity::find()
        .filter(condition)
        .all(db)
        .await
        .map_err(database_error)?;
    for mut source in related {
        effective_source(&mut source, local_date(proposed.taken_at));
        if let Some(max_doses) = source.max_daily_doses {
            let (start, end) = cycle_bounds(proposed.taken_at, source.dose_cycle);
            let count = takes
                .iter()
                .filter(|take| {
                    take.taken_at
                        .is_some_and(|time| time >= start && time < end)
                })
                .count();
            if count >= max_doses.max(0) as usize {
                return Ok(false);
            }
        }
        if let Some(hours) = source.min_hours_between_doses {
            if hours > Decimal::ZERO {
                let recent = takes
                    .iter()
                    .filter_map(|take| take.taken_at)
                    .filter(|time| *time <= proposed.taken_at)
                    .max();
                if let Some(recent) = recent {
                    let elapsed = Decimal::from((proposed.taken_at - recent).num_seconds());
                    if elapsed < hours * Decimal::from(3600) {
                        return Ok(false);
                    }
                }
            }
        }
    }
    Ok(true)
}
