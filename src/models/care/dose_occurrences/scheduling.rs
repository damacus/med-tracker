use super::*;

pub(crate) fn schedule_config_on(schedule: &schedule::Model, day: NaiveDate) -> Option<&Value> {
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
            .is_some_and(|days| days.iter().any(|value| weekday_matches(value, day))),
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

pub(crate) fn schedule_as_needed(schedule: &schedule::Model) -> bool {
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

pub(crate) fn weekday_matches(value: &Value, day: NaiveDate) -> bool {
    let index = match value {
        Value::Number(value) => value.as_u64().and_then(|value| u32::try_from(value).ok()),
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
                .position(|name| *name == lower || name.starts_with(&lower) && lower.len() == 3)
                .map(|value| value as u32)
            })
        }
        _ => None,
    };
    index.is_some_and(|index| {
        index == day.weekday().num_days_from_sunday() || index == day.weekday().number_from_monday()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn schedule_model(
        schedule_type: i32,
        config: Value,
        start: NaiveDate,
        end: NaiveDate,
    ) -> schedule::Model {
        schedule::Model {
            person_schedule_id: None,
            sharing_approval_id: None,
            canonical_origin: false,
            clinical_revision: None,
            origin_time_zone: None,
            execution_ended_at: None,
            id: 1,
            household_id: 1,
            portable_id: "s1".to_string(),
            person_id: 1,
            medication_id: 1,
            active: true,
            retired_at: None,
            start_date: Some(start),
            end_date: Some(end),
            max_daily_doses: None,
            min_hours_between_doses: None,
            source_dosage_option_id: None,
            dose_cycle: None,
            dose_amount: None,
            dose_unit: None,
            schedule_type,
            schedule_config: config,
            frequency: None,
            notes: None,
            created_at: day(2026, 1, 1).and_hms_opt(0, 0, 0).unwrap(),
            updated_at: day(2026, 1, 1).and_hms_opt(0, 0, 0).unwrap(),
        }
    }

    // 2026-01-05 is a Monday; 2026-01-11 is a Sunday.
    #[test]
    fn weekday_matches_accepts_sunday_and_monday_numbering() {
        let monday = day(2026, 1, 5);
        let sunday = day(2026, 1, 11);
        for index in 1..=6 {
            assert!(
                !weekday_matches(&json!(index), sunday),
                "{index} should not match Sunday"
            );
        }
        assert!(weekday_matches(&json!(1), monday));
        assert!(weekday_matches(&json!(0), sunday));
        assert!(weekday_matches(&json!(7), sunday));
        assert!(!weekday_matches(&json!(0), monday));
        assert!(!weekday_matches(&json!(7), monday));
    }

    #[test]
    fn weekday_matches_names_abbreviations_and_numeric_strings() {
        let monday = day(2026, 1, 5);
        for value in [
            json!("monday"),
            json!("Mon"),
            json!("MONDAY"),
            json!("  monday  "),
            json!("1"),
        ] {
            assert!(weekday_matches(&value, monday), "{value}");
        }
        for value in [
            json!("tue"),
            json!("funday"),
            json!("mondayy"),
            json!(""),
            json!(-1),
            json!(null),
            json!([1]),
        ] {
            assert!(!weekday_matches(&value, monday), "{value}");
        }
    }

    #[test]
    fn schedule_applies_respects_start_and_end_inclusively() {
        let scheduled = schedule_model(1, json!({}), day(2026, 1, 5), day(2026, 1, 11));
        assert!(schedule_applies(&scheduled, day(2026, 1, 5)));
        assert!(schedule_applies(&scheduled, day(2026, 1, 11)));
        assert!(!schedule_applies(&scheduled, day(2026, 1, 4)));
        assert!(!schedule_applies(&scheduled, day(2026, 1, 12)));
        let mut open = scheduled.clone();
        open.end_date = None;
        assert!(!schedule_applies(&open, day(2026, 1, 6)));
    }

    #[test]
    fn schedule_applies_type_2_filters_to_listed_weekdays() {
        let scheduled = schedule_model(
            2,
            json!({"weekdays": [1, "wed"]}),
            day(2026, 1, 5),
            day(2026, 1, 11),
        );
        assert!(schedule_applies(&scheduled, day(2026, 1, 5))); // Monday
        assert!(!schedule_applies(&scheduled, day(2026, 1, 6))); // Tuesday
        assert!(schedule_applies(&scheduled, day(2026, 1, 7))); // Wednesday
        assert!(!schedule_applies(&scheduled, day(2026, 1, 11))); // Sunday
    }

    #[test]
    fn schedule_applies_type_3_matches_only_listed_dates() {
        let scheduled = schedule_model(
            3,
            json!({"dates": ["2026-01-06", "not-a-date"]}),
            day(2026, 1, 5),
            day(2026, 1, 11),
        );
        assert!(schedule_applies(&scheduled, day(2026, 1, 6)));
        assert!(!schedule_applies(&scheduled, day(2026, 1, 5)));
        assert!(!schedule_applies(&scheduled, day(2026, 1, 7)));
    }

    #[test]
    fn schedule_applies_type_6_alternates_days_from_start() {
        let scheduled = schedule_model(6, json!({}), day(2026, 1, 5), day(2026, 1, 11));
        assert!(schedule_applies(&scheduled, day(2026, 1, 5)));
        assert!(!schedule_applies(&scheduled, day(2026, 1, 6)));
        assert!(schedule_applies(&scheduled, day(2026, 1, 7)));
    }

    #[test]
    fn schedule_applies_type_5_needs_a_covering_taper_step() {
        let scheduled = schedule_model(
            5,
            json!({"taper_steps": [
                {"start_date": "2026-01-05", "end_date": "2026-01-07", "amount": "2"},
                {"start_date": "2026-01-10", "end_date": "2026-01-11", "amount": "1"},
            ]}),
            day(2026, 1, 1),
            day(2026, 1, 31),
        );
        assert!(schedule_applies(&scheduled, day(2026, 1, 6)));
        assert!(!schedule_applies(&scheduled, day(2026, 1, 8)));
        assert!(schedule_applies(&scheduled, day(2026, 1, 10)));
    }

    #[test]
    fn schedule_config_on_returns_the_covering_taper_step() {
        let step = json!({"start_date": "2026-01-05", "end_date": "2026-01-07", "amount": "2"});
        let scheduled = schedule_model(
            5,
            json!({"taper_steps": [step.clone(), {"start_date": "bad", "end_date": "dates"}]}),
            day(2026, 1, 1),
            day(2026, 1, 31),
        );
        assert_eq!(schedule_config_on(&scheduled, day(2026, 1, 6)), Some(&step));
        assert_eq!(schedule_config_on(&scheduled, day(2026, 1, 20)), None);
        let daily = schedule_model(1, json!({"amount": "3"}), day(2026, 1, 1), day(2026, 1, 31));
        assert_eq!(
            schedule_config_on(&daily, day(2026, 1, 6)),
            Some(&daily.schedule_config)
        );
    }

    #[test]
    fn schedule_as_needed_detects_type_frequency_and_flag() {
        let start = day(2026, 1, 1);
        let end = day(2026, 1, 31);
        assert!(schedule_as_needed(&schedule_model(
            4,
            json!({}),
            start,
            end
        )));
        let mut by_frequency = schedule_model(1, json!({}), start, end);
        by_frequency.frequency = Some("As Needed".to_string());
        assert!(schedule_as_needed(&by_frequency));
        assert!(schedule_as_needed(&schedule_model(
            1,
            json!({"as_needed": true}),
            start,
            end
        )));
        assert!(!schedule_as_needed(&schedule_model(
            1,
            json!({"as_needed": false}),
            start,
            end
        )));
    }

    #[test]
    fn effective_count_prefers_named_keys_and_clamps_positive() {
        assert_eq!(effective_count(&json!({"max_daily_doses": 3}), 9), 3);
        assert_eq!(effective_count(&json!({"max_doses": "2"}), 9), 2);
        assert_eq!(effective_count(&json!({"max": 4}), 9), 4);
        assert_eq!(
            effective_count(&json!({"max_doses": 7, "max_daily_doses": 3}), 9),
            3
        );
        assert_eq!(effective_count(&json!({"max_daily_doses": 0}), 9), 1);
        assert_eq!(effective_count(&json!({"max_daily_doses": -2}), 9), 1);
        assert_eq!(effective_count(&json!({"max_daily_doses": "nope"}), 9), 9);
        assert_eq!(effective_count(&json!({}), 9), 9);
    }
}
