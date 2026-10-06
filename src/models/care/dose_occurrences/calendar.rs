use super::*;

pub(super) fn date(value: &str) -> Option<NaiveDate> {
    if value.len() != 10
        || !value.as_bytes().iter().enumerate().all(|(index, byte)| {
            if matches!(index, 4 | 7) {
                *byte == b'-'
            } else {
                byte.is_ascii_digit()
            }
        })
    {
        return None;
    }
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

tokio::task_local! {
    static DASHBOARD_TIMEZONE: chrono_tz::Tz;
}

pub(crate) async fn with_dashboard_timezone<F: std::future::Future>(
    timezone: chrono_tz::Tz,
    future: F,
) -> F::Output {
    DASHBOARD_TIMEZONE.scope(timezone, future).await
}

fn zone() -> chrono_tz::Tz {
    if let Ok(timezone) = DASHBOARD_TIMEZONE.try_with(|timezone| *timezone) {
        return timezone;
    }
    std::env::var("TZ")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(chrono_tz::UTC)
}

pub(super) fn local_date(value: NaiveDateTime) -> NaiveDate {
    value.and_utc().with_timezone(&zone()).date_naive()
}
pub(super) fn current_zone() -> chrono_tz::Tz {
    zone()
}

pub(super) fn cycle_bounds(value: NaiveDate, cycle: i32) -> (NaiveDate, NaiveDate) {
    match cycle {
        1 => {
            let start = value - Duration::days(i64::from(value.weekday().num_days_from_monday()));
            (start, start + Duration::days(6))
        }
        2 => {
            let start = value.with_day(1).unwrap();
            let next = if value.month() == 12 {
                NaiveDate::from_ymd_opt(value.year() + 1, 1, 1).unwrap()
            } else {
                NaiveDate::from_ymd_opt(value.year(), value.month() + 1, 1).unwrap()
            };
            (start, next - Duration::days(1))
        }
        _ => (value, value),
    }
}

fn scheduled_time(date: NaiveDate, value: &str) -> Option<NaiveDateTime> {
    scheduled_time_in_zone(date, value, zone())
}

pub(super) fn scheduled_time_in_zone(
    date: NaiveDate,
    value: &str,
    time_zone: chrono_tz::Tz,
) -> Option<NaiveDateTime> {
    let (hour, minute) = value.split_once(':')?;
    let hour: u32 = hour.parse().ok()?;
    let minute: u32 = minute.parse().ok()?;
    let local = date.and_hms_opt(hour, minute, 0)?;
    if let Some(value) = time_zone.from_local_datetime(&local).earliest() {
        return Some(value.with_timezone(&Utc).naive_utc());
    }
    let before = (1..=180).find_map(|minutes| {
        time_zone
            .from_local_datetime(&(local - Duration::minutes(minutes)))
            .earliest()
    })?;
    let after = (1..=180).find_map(|minutes| {
        time_zone
            .from_local_datetime(&(local + Duration::minutes(minutes)))
            .earliest()
    })?;
    let before_offset = before.naive_local() - before.naive_utc();
    let after_offset = after.naive_local() - after.naive_utc();
    let shifted = local + (after_offset - before_offset);
    time_zone
        .from_local_datetime(&shifted)
        .earliest()
        .map(|value| value.with_timezone(&Utc).naive_utc())
}

pub(super) fn local_midnight(date: NaiveDate) -> NaiveDateTime {
    zone()
        .from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
        .earliest()
        .map(|value| value.with_timezone(&Utc).naive_utc())
        .unwrap_or_else(|| date.and_hms_opt(0, 0, 0).unwrap())
}

pub(super) fn config_times(config: &Value, date: NaiveDate) -> Vec<NaiveDateTime> {
    config
        .get("times")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().and_then(|value| scheduled_time(date, value)))
        .collect()
}
