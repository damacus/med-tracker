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

fn zone(settings: Option<&serde_json::Value>) -> chrono_tz::Tz {
    if let Ok(timezone) = DASHBOARD_TIMEZONE.try_with(|timezone| *timezone) {
        return timezone;
    }
    crate::models::care::doses::app_zone(settings)
}

pub(super) fn local_date(value: NaiveDateTime, settings: Option<&serde_json::Value>) -> NaiveDate {
    value.and_utc().with_timezone(&zone(settings)).date_naive()
}
pub(super) fn current_zone(settings: Option<&serde_json::Value>) -> chrono_tz::Tz {
    zone(settings)
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

fn scheduled_time(
    date: NaiveDate,
    value: &str,
    settings: Option<&serde_json::Value>,
) -> Option<NaiveDateTime> {
    scheduled_time_in_zone(date, value, zone(settings))
}

pub(crate) fn scheduled_time_in_zone(
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

pub(super) fn local_midnight(
    date: NaiveDate,
    settings: Option<&serde_json::Value>,
) -> NaiveDateTime {
    let tz = zone(settings);
    let midnight = date.and_hms_opt(0, 0, 0).unwrap();
    tz.from_local_datetime(&midnight)
        .earliest()
        .or_else(|| chrono_tz::GapInfo::new(&midnight, &tz).and_then(|gap| gap.end))
        .map(|value| value.with_timezone(&Utc).naive_utc())
        .unwrap_or_else(|| scheduled_time_in_zone(date, "00:00", tz).unwrap_or(midnight))
}

pub(super) fn config_times(
    config: &Value,
    date: NaiveDate,
    settings: Option<&serde_json::Value>,
) -> Vec<NaiveDateTime> {
    config
        .get("times")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|value| {
            value
                .as_str()
                .and_then(|value| scheduled_time(date, value, settings))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn ymd(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn ymd_hms(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32) -> NaiveDateTime {
        ymd(year, month, day).and_hms_opt(hour, min, sec).unwrap()
    }

    #[test]
    fn scheduled_time_and_local_midnight_europe_london() {
        let london = chrono_tz::Europe::London;
        let settings = json!({ "time_zone": "Europe/London" });

        let spring = scheduled_time_in_zone(ymd(2026, 3, 29), "01:30", london);
        assert_eq!(spring, Some(ymd_hms(2026, 3, 29, 1, 30, 0)));

        let autumn = scheduled_time_in_zone(ymd(2026, 10, 25), "01:30", london);
        assert_eq!(autumn, Some(ymd_hms(2026, 10, 25, 0, 30, 0)));

        assert_eq!(
            local_midnight(ymd(2026, 3, 29), Some(&settings)),
            ymd_hms(2026, 3, 29, 0, 0, 0)
        );
        assert_eq!(
            local_midnight(ymd(2026, 10, 25), Some(&settings)),
            ymd_hms(2026, 10, 24, 23, 0, 0)
        );
    }

    #[test]
    fn scheduled_time_and_local_midnight_america_new_york() {
        let new_york = chrono_tz::America::New_York;
        let settings = json!({ "time_zone": "America/New_York" });

        let spring = scheduled_time_in_zone(ymd(2026, 3, 8), "02:30", new_york);
        assert_eq!(spring, Some(ymd_hms(2026, 3, 8, 7, 30, 0)));

        let autumn = scheduled_time_in_zone(ymd(2026, 11, 1), "01:30", new_york);
        assert_eq!(autumn, Some(ymd_hms(2026, 11, 1, 5, 30, 0)));

        assert_eq!(
            local_midnight(ymd(2026, 3, 8), Some(&settings)),
            ymd_hms(2026, 3, 8, 5, 0, 0)
        );
        assert_eq!(
            local_midnight(ymd(2026, 11, 1), Some(&settings)),
            ymd_hms(2026, 11, 1, 4, 0, 0)
        );
    }

    #[test]
    fn scheduled_time_and_local_midnight_australia_lord_howe() {
        let lord_howe = chrono_tz::Australia::Lord_Howe;
        let settings = json!({ "time_zone": "Australia/Lord_Howe" });

        let spring = scheduled_time_in_zone(ymd(2026, 10, 4), "02:15", lord_howe);
        assert_eq!(spring, Some(ymd_hms(2026, 10, 3, 15, 45, 0)));

        let autumn = scheduled_time_in_zone(ymd(2026, 4, 5), "01:45", lord_howe);
        assert_eq!(autumn, Some(ymd_hms(2026, 4, 4, 14, 45, 0)));

        assert_eq!(
            local_midnight(ymd(2026, 10, 4), Some(&settings)),
            ymd_hms(2026, 10, 3, 13, 30, 0)
        );
        assert_eq!(
            local_midnight(ymd(2026, 4, 5), Some(&settings)),
            ymd_hms(2026, 4, 4, 13, 0, 0)
        );
    }

    #[test]
    fn scheduled_time_and_local_midnight_america_santiago() {
        let santiago = chrono_tz::America::Santiago;
        let settings = json!({ "time_zone": "America/Santiago" });

        let spring = scheduled_time_in_zone(ymd(2026, 9, 6), "00:30", santiago);
        assert_eq!(spring, Some(ymd_hms(2026, 9, 6, 4, 30, 0)));

        assert_eq!(
            local_midnight(ymd(2026, 9, 6), Some(&settings)),
            ymd_hms(2026, 9, 6, 4, 0, 0)
        );

        let autumn = scheduled_time_in_zone(ymd(2026, 4, 4), "23:30", santiago);
        assert_eq!(autumn, Some(ymd_hms(2026, 4, 5, 2, 30, 0)));

        assert_eq!(
            local_midnight(ymd(2026, 4, 5), Some(&settings)),
            ymd_hms(2026, 4, 5, 4, 0, 0)
        );
    }

    #[test]
    fn cycle_bounds_daily_weekly_and_monthly() {
        let d = ymd(2026, 6, 15);
        assert_eq!(cycle_bounds(d, 0), (d, d));

        assert_eq!(
            cycle_bounds(ymd(2026, 1, 7), 1),
            (ymd(2026, 1, 5), ymd(2026, 1, 11))
        );
        assert_eq!(
            cycle_bounds(ymd(2026, 1, 5), 1),
            (ymd(2026, 1, 5), ymd(2026, 1, 11))
        );
        assert_eq!(
            cycle_bounds(ymd(2026, 1, 11), 1),
            (ymd(2026, 1, 5), ymd(2026, 1, 11))
        );

        assert_eq!(
            cycle_bounds(ymd(2026, 12, 31), 1),
            (ymd(2026, 12, 28), ymd(2027, 1, 3))
        );

        assert_eq!(
            cycle_bounds(ymd(2024, 2, 15), 2),
            (ymd(2024, 2, 1), ymd(2024, 2, 29))
        );

        assert_eq!(
            cycle_bounds(ymd(2026, 2, 15), 2),
            (ymd(2026, 2, 1), ymd(2026, 2, 28))
        );

        assert_eq!(
            cycle_bounds(ymd(2026, 12, 31), 2),
            (ymd(2026, 12, 1), ymd(2026, 12, 31))
        );
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn local_midnight_monotonic_and_roundtrips_local_date(
            tz_name in prop_oneof![
                Just("Europe/London"),
                Just("America/New_York"),
                Just("Australia/Lord_Howe"),
                Just("America/Santiago"),
            ],
            day_offset in prop_oneof![
                Just((ymd(2026, 3, 29) - ymd(2024, 1, 1)).num_days()),
                Just((ymd(2026, 10, 25) - ymd(2024, 1, 1)).num_days()),
                Just((ymd(2026, 3, 8) - ymd(2024, 1, 1)).num_days()),
                Just((ymd(2026, 11, 1) - ymd(2024, 1, 1)).num_days()),
                Just((ymd(2026, 10, 4) - ymd(2024, 1, 1)).num_days()),
                Just((ymd(2026, 4, 5) - ymd(2024, 1, 1)).num_days()),
                Just((ymd(2026, 9, 6) - ymd(2024, 1, 1)).num_days()),
                0..1461i64,
            ],
        ) {
            let base_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
            let d = base_date + Duration::days(day_offset);
            let settings = json!({ "time_zone": tz_name });
            let midnight = local_midnight(d, Some(&settings));
            let next_midnight = local_midnight(d + Duration::days(1), Some(&settings));
            prop_assert!(
                midnight < next_midnight,
                "local_midnight({d:?}, {tz_name}) = {midnight:?} not < local_midnight({:?}) = {next_midnight:?}",
                d + Duration::days(1)
            );
            prop_assert_eq!(
                local_date(midnight, Some(&settings)),
                d,
                "local_date mismatch for tz {} on date {:?}",
                tz_name,
                d
            );
        }
    }
}
