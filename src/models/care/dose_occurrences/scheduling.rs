use super::*;

pub(super) fn schedule_config_on(schedule: &schedule::Model, day: NaiveDate) -> Option<&Value> {
    if schedule.schedule_type != 5 {
        return Some(&schedule.schedule_config);
    }
    schedule
        .schedule_config
        .get("taper_steps")
        .and_then(Value::as_array)?
        .iter()
        .find(|step| {
            let Some(start) = step
                .get("start_date")
                .and_then(Value::as_str)
                .and_then(date)
            else {
                return false;
            };
            let Some(end) = step.get("end_date").and_then(Value::as_str).and_then(date) else {
                return false;
            };
            (start..=end).contains(&day)
        })
}

pub(super) fn schedule_applies(schedule: &schedule::Model, day: NaiveDate) -> bool {
    let (Some(start), Some(end)) = (schedule.start_date, schedule.end_date) else {
        return false;
    };
    if day < start || day > end {
        return false;
    }
    match schedule.schedule_type {
        2 => schedule
            .schedule_config
            .get("weekdays")
            .and_then(Value::as_array)
            .is_some_and(|days| {
                days.iter().any(|value| {
                    let index = match value {
                        Value::Number(value) => {
                            value.as_u64().and_then(|value| u32::try_from(value).ok())
                        }
                        Value::String(value) => {
                            let lower = value.trim().to_ascii_lowercase();
                            lower.parse::<u32>().ok().or_else(|| {
                                [
                                    "sunday",
                                    "monday",
                                    "tuesday",
                                    "wednesday",
                                    "thursday",
                                    "friday",
                                    "saturday",
                                ]
                                .iter()
                                .position(|name| {
                                    *name == lower || name.starts_with(&lower) && lower.len() == 3
                                })
                                .map(|value| value as u32)
                            })
                        }
                        _ => None,
                    };
                    index.is_some_and(|index| {
                        index == day.weekday().num_days_from_sunday()
                            || index == day.weekday().number_from_monday()
                    })
                })
            }),
        3 => schedule
            .schedule_config
            .get("dates")
            .and_then(Value::as_array)
            .is_some_and(|days| {
                days.iter()
                    .any(|value| value.as_str().and_then(date) == Some(day))
            }),
        5 => schedule_config_on(schedule, day).is_some(),
        6 => (day - start).num_days() % 2 == 0,
        _ => true,
    }
}

pub(super) fn schedule_as_needed(schedule: &schedule::Model) -> bool {
    schedule.schedule_type == 4
        || schedule
            .frequency
            .as_deref()
            .is_some_and(|value| value.eq_ignore_ascii_case("as needed"))
        || schedule.schedule_config.get("as_needed") == Some(&Value::Bool(true))
}

pub(super) fn effective_count(config: &Value, fallback: i32) -> i32 {
    ["max_daily_doses", "max_doses", "max"]
        .iter()
        .find_map(|name| {
            config.get(*name).and_then(|value| {
                value
                    .as_i64()
                    .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
            })
        })
        .and_then(|value| i32::try_from(value).ok())
        .unwrap_or(fallback)
        .max(1)
}
