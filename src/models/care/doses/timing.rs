use super::*;
use loco_rs::model::TenantQueryExt;

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
                days.iter()
                    .any(|day| crate::models::care::dose_occurrences::weekday_matches(day, date))
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

fn local_midnight_utc(date: NaiveDate, zone: chrono_tz::Tz) -> Result<NaiveDateTime, ApiError> {
    let midnight = date.and_hms_opt(0, 0, 0).expect("midnight is valid");
    zone.from_local_datetime(&midnight)
        .earliest()
        .or_else(|| chrono_tz::GapInfo::new(&midnight, &zone).and_then(|gap| gap.end))
        .map(|time| time.with_timezone(&Utc).naive_utc())
        .ok_or_else(|| error(ErrorKind::Validation, "taken_at is invalid"))
}

fn cycle_bounds(
    time: NaiveDateTime,
    cycle: Option<i32>,
    zone: chrono_tz::Tz,
) -> Result<(NaiveDateTime, NaiveDateTime), ApiError> {
    cycle_bounds_in_zone(time, cycle, zone)
}

pub(super) fn cycle_bounds_in_zone(
    time: NaiveDateTime,
    cycle: Option<i32>,
    zone: chrono_tz::Tz,
) -> Result<(NaiveDateTime, NaiveDateTime), ApiError> {
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
    Ok((
        local_midnight_utc(start, zone)?,
        local_midnight_utc(end, zone)?,
    ))
}

pub(super) async fn timing_allowed(
    db: &DatabaseTransaction,
    proposed: &ProposedTake,
) -> Result<bool, ApiError> {
    let assignments = person_medication::Entity::find()
        .in_tenant(proposed.selected.household_id)
        .filter(person_medication::Column::PersonId.eq(proposed.source.person_id))
        .filter(person_medication::Column::MedicationId.eq(proposed.source.medication_id))
        .filter(person_medication::Column::Active.eq(true))
        .filter(person_medication::Column::RetiredAt.is_null())
        .all(db)
        .await
        .map_err(database_error)?;
    let schedules = schedule::Entity::find()
        .in_tenant(proposed.selected.household_id)
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
                .filter(|source| {
                    applies_on(
                        source,
                        local_date_in_zone(proposed.taken_at, source.zone.unwrap_or(proposed.zone)),
                    )
                }),
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
        .in_tenant(proposed.selected.household_id)
        .filter(condition)
        .all(db)
        .await
        .map_err(database_error)?;
    Ok(timing_block(&related, &takes, proposed.taken_at, proposed.zone)?.is_none())
}

fn timing_block(
    related: &[Source],
    takes: &[medication_take::Model],
    taken_at: NaiveDateTime,
    zone: chrono_tz::Tz,
) -> Result<Option<&'static str>, ApiError> {
    for mut source in related.iter().cloned() {
        let zone = source.zone.unwrap_or(zone);
        effective_source(&mut source, local_date_in_zone(taken_at, zone));
        if let Some(max_doses) = source.max_daily_doses {
            let (start, end) = cycle_bounds(taken_at, source.dose_cycle, zone)?;
            let count = takes
                .iter()
                .filter(|take| {
                    take.taken_at
                        .is_some_and(|time| time >= start && time < end)
                })
                .count();
            if count >= max_doses.max(0) as usize {
                return Ok(Some("max_reached"));
            }
        }
        if let Some(hours) = source.min_hours_between_doses
            && hours > Decimal::ZERO
        {
            let recent = takes
                .iter()
                .filter_map(|take| take.taken_at)
                .filter(|time| *time <= taken_at)
                .max();
            if let Some(recent) = recent {
                let elapsed = Decimal::from((taken_at - recent).num_seconds());
                if elapsed < hours * Decimal::from(3600) {
                    return Ok(Some("cooldown"));
                }
            }
        }
    }
    Ok(None)
}

fn requires_timing_history(sources: &[Source], date: NaiveDate) -> bool {
    sources.iter().cloned().any(|mut source| {
        effective_source(&mut source, date);
        source.max_daily_doses.is_some()
            || source
                .min_hours_between_doses
                .is_some_and(|hours| hours > Decimal::ZERO)
    })
}

pub(crate) async fn dashboard_timing(
    tenant: &TenantTransaction,
    schedules: &[schedule::Model],
    assignments: &[person_medication::Model],
    now: DateTime<Utc>,
    zone: chrono_tz::Tz,
) -> Result<std::collections::HashMap<(i64, i64), &'static str>, ApiError> {
    let sources: Vec<_> = assignments
        .iter()
        .cloned()
        .map(source_from_assignment)
        .chain(schedules.iter().cloned().map(source_from_schedule))
        .filter(|source| {
            source.active
                && !source.retired
                && applies_on(
                    source,
                    now.with_timezone(&source.zone.unwrap_or(zone)).date_naive(),
                )
        })
        .collect();
    if !requires_timing_history(&sources, now.with_timezone(&zone).date_naive()) {
        return Ok(std::collections::HashMap::new());
    }
    let schedule_ids: Vec<_> = sources
        .iter()
        .filter(|source| source.kind == "schedule")
        .map(|source| source.id)
        .collect();
    let assignment_ids: Vec<_> = sources
        .iter()
        .filter(|source| source.kind == "person_medication")
        .map(|source| source.id)
        .collect();
    let takes = medication_take::Entity::find()
        .filter(medication_take::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(
            Condition::any()
                .add(medication_take::Column::ScheduleId.is_in(schedule_ids))
                .add(medication_take::Column::PersonMedicationId.is_in(assignment_ids)),
        )
        .all(tenant.transaction())
        .await?;
    let mut groups = std::collections::HashMap::<_, Vec<Source>>::new();
    for source in sources {
        groups
            .entry((source.person_id, source.medication_id))
            .or_default()
            .push(source);
    }
    let mut result = std::collections::HashMap::new();
    for (key, related) in groups {
        let related_takes: Vec<_> = takes
            .iter()
            .filter(|take| {
                related.iter().any(|source| match source.kind {
                    "schedule" => take.schedule_id == Some(source.id),
                    _ => take.person_medication_id == Some(source.id),
                })
            })
            .cloned()
            .collect();
        if let Some(reason) = timing_block(&related, &related_takes, now.naive_utc(), zone)? {
            result.insert(key, reason);
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

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
            dose_amount: Some(Decimal::ONE),
            dose_unit: Some("tablet".to_string()),
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
    fn timing_history_requires_effective_config_and_taper_limits() {
        let date = NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
        let mut source = test_source();
        assert!(!requires_timing_history(&[], date));
        assert!(!requires_timing_history(&[source.clone()], date));
        source.schedule_config = Some(json!({"max_daily_doses": 0}));
        assert!(requires_timing_history(&[source.clone()], date));
        assert_eq!(source.max_daily_doses, None);
        source.schedule_type = Some(5);
        source.schedule_config = Some(json!({"taper_steps": [
            {"start_date": "2026-06-15", "end_date": "2026-06-15", "max_daily_doses": 0},
            {"start_date": "2026-06-16", "end_date": "2026-06-16", "min_hours": 4}
        ]}));
        assert!(requires_timing_history(&[source.clone()], date));
        assert!(requires_timing_history(
            &[source.clone()],
            date + Duration::days(1)
        ));
        assert!(!requires_timing_history(
            &[source],
            date + Duration::days(2)
        ));
    }

    #[test]
    fn timing_history_preserves_zero_limits_and_cooldown_overrides() {
        let date = NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
        let mut source = test_source();
        for limit in [0, -1, 1] {
            source.max_daily_doses = Some(limit);
            assert!(requires_timing_history(&[source.clone()], date));
        }
        source.max_daily_doses = None;
        for hours in [Decimal::ZERO, -Decimal::ONE] {
            source.min_hours_between_doses = Some(hours);
            assert!(!requires_timing_history(&[source.clone()], date));
        }
        source.min_hours_between_doses = Some(Decimal::new(1, 2));
        assert!(requires_timing_history(&[source.clone()], date));
        source.schedule_type = Some(0);
        source.schedule_config = Some(json!({"times": ["08:00"]}));
        assert!(!requires_timing_history(&[source.clone()], date));
        source.max_daily_doses = Some(0);
        assert!(requires_timing_history(&[source], date));
    }

    fn test_take(time: NaiveDateTime) -> medication_take::Model {
        medication_take::Model {
            id: 1,
            client_uuid: None,
            created_at: time,
            dose_amount: Some(Decimal::ONE),
            dose_unit: Some("tablet".to_string()),
            household_id: 1,
            person_medication_id: None,
            portable_id: "take-1".to_string(),
            schedule_id: Some(1),
            taken_at: Some(time),
            taken_from_location_id: None,
            taken_from_medication_id: None,
            updated_at: time,
        }
    }

    fn local_to_utc_naive(local: NaiveDateTime, zone: chrono_tz::Tz) -> NaiveDateTime {
        zone.from_local_datetime(&local)
            .earliest()
            .or_else(|| chrono_tz::GapInfo::new(&local, &zone).and_then(|g| g.end))
            .expect("local datetime resolves")
            .with_timezone(&Utc)
            .naive_utc()
    }

    fn independent_local_cycle_bounds(
        taken_at: NaiveDateTime,
        zone: chrono_tz::Tz,
    ) -> (NaiveDateTime, NaiveDateTime) {
        let local_dt = Utc.from_utc_datetime(&taken_at).with_timezone(&zone);
        let local_date = local_dt.date_naive();
        let midnight_start = local_date.and_hms_opt(0, 0, 0).unwrap();
        let midnight_end = (local_date + Duration::days(1))
            .and_hms_opt(0, 0, 0)
            .unwrap();

        let start_utc = zone
            .from_local_datetime(&midnight_start)
            .earliest()
            .or_else(|| chrono_tz::GapInfo::new(&midnight_start, &zone).and_then(|g| g.end))
            .expect("start midnight resolves")
            .with_timezone(&Utc)
            .naive_utc();

        let end_utc = zone
            .from_local_datetime(&midnight_end)
            .earliest()
            .or_else(|| chrono_tz::GapInfo::new(&midnight_end, &zone).and_then(|g| g.end))
            .expect("end midnight resolves")
            .with_timezone(&Utc)
            .naive_utc();

        (start_utc, end_utc)
    }

    #[test]
    fn cooldown_crosses_local_midnight_across_zones() {
        let zones = [
            chrono_tz::Europe::London,
            chrono_tz::America::New_York,
            chrono_tz::Australia::Lord_Howe,
            chrono_tz::America::Santiago,
        ];

        for zone in zones {
            let mut source = test_source();
            source.max_daily_doses = Some(2);
            source.min_hours_between_doses = Some(Decimal::from(6));

            let day1_2200_local = NaiveDate::from_ymd_opt(2026, 6, 15)
                .unwrap()
                .and_hms_opt(22, 0, 0)
                .unwrap();
            let take1_utc = local_to_utc_naive(day1_2200_local, zone);
            let takes = vec![test_take(take1_utc)];

            let day2_0200_local = NaiveDate::from_ymd_opt(2026, 6, 16)
                .unwrap()
                .and_hms_opt(2, 0, 0)
                .unwrap();
            let candidate1_utc = local_to_utc_naive(day2_0200_local, zone);
            let blocked = timing_block(&[source.clone()], &takes, candidate1_utc, zone).unwrap();
            assert_eq!(
                blocked,
                Some("cooldown"),
                "At 02:00 Day 2 in {zone:?}, cooldown of 6h must block dose even though Day 2 dose count is 0"
            );

            let day2_0400_local = NaiveDate::from_ymd_opt(2026, 6, 16)
                .unwrap()
                .and_hms_opt(4, 0, 0)
                .unwrap();
            let candidate2_utc = local_to_utc_naive(day2_0400_local, zone);
            let allowed = timing_block(&[source], &takes, candidate2_utc, zone).unwrap();
            assert_eq!(
                allowed, None,
                "At 04:00 Day 2 in {zone:?}, 6h cooldown expired and Day 2 count is 0, so dose is allowed"
            );
        }
    }

    #[test]
    fn local_calendar_day_limit_blocks_after_cooldown_and_resets_at_midnight() {
        let zones = [
            chrono_tz::Europe::London,
            chrono_tz::America::New_York,
            chrono_tz::Australia::Lord_Howe,
            chrono_tz::America::Santiago,
        ];

        for zone in zones {
            let mut source = test_source();
            source.max_daily_doses = Some(1);
            source.min_hours_between_doses = Some(Decimal::from(4));

            let day1_1000_local = NaiveDate::from_ymd_opt(2026, 6, 15)
                .unwrap()
                .and_hms_opt(10, 0, 0)
                .unwrap();
            let take1_utc = local_to_utc_naive(day1_1000_local, zone);
            let takes = vec![test_take(take1_utc)];

            let day1_1600_local = NaiveDate::from_ymd_opt(2026, 6, 15)
                .unwrap()
                .and_hms_opt(16, 0, 0)
                .unwrap();
            let candidate1_utc = local_to_utc_naive(day1_1600_local, zone);
            let blocked = timing_block(&[source.clone()], &takes, candidate1_utc, zone).unwrap();
            assert_eq!(
                blocked,
                Some("max_reached"),
                "At 16:00 Day 1 in {zone:?}, 6h > 4h cooldown has elapsed but max_daily_doses=1 blocks"
            );

            let day2_midnight_utc =
                local_midnight_utc(NaiveDate::from_ymd_opt(2026, 6, 16).unwrap(), zone).unwrap();
            let allowed = timing_block(&[source], &takes, day2_midnight_utc, zone).unwrap();
            assert_eq!(
                allowed, None,
                "At local midnight of Day 2 in {zone:?}, local calendar-day limit resets"
            );
        }
    }

    #[test]
    fn midnight_dst_transition_in_america_santiago() {
        let zone = chrono_tz::America::Santiago;

        let take_time_utc = NaiveDate::from_ymd_opt(2026, 9, 6)
            .unwrap()
            .and_hms_opt(2, 30, 0)
            .unwrap();

        let (start_utc, end_utc) = cycle_bounds_in_zone(take_time_utc, None, zone).unwrap();
        let expected_start_utc = NaiveDate::from_ymd_opt(2026, 9, 5)
            .unwrap()
            .and_hms_opt(4, 0, 0)
            .unwrap();
        let expected_end_utc = NaiveDate::from_ymd_opt(2026, 9, 6)
            .unwrap()
            .and_hms_opt(4, 0, 0)
            .unwrap();
        assert_eq!(
            (start_utc, end_utc),
            (expected_start_utc, expected_end_utc),
            "cycle_bounds_in_zone for 2026-09-05 in Santiago must end at 2026-09-06T04:00:00 UTC"
        );

        let mut source = test_source();
        source.max_daily_doses = Some(1);
        source.min_hours_between_doses = Some(Decimal::from(1));

        let takes = vec![test_take(take_time_utc)];

        let candidate_utc = NaiveDate::from_ymd_opt(2026, 9, 6)
            .unwrap()
            .and_hms_opt(3, 45, 0)
            .unwrap();
        let blocked = timing_block(&[source.clone()], &takes, candidate_utc, zone).unwrap();
        assert_eq!(
            blocked,
            Some("max_reached"),
            "Dose at 02:30 UTC counts towards 2026-09-05 daily limit until 04:00:00 UTC"
        );

        let reset_utc = NaiveDate::from_ymd_opt(2026, 9, 6)
            .unwrap()
            .and_hms_opt(4, 0, 0)
            .unwrap();
        let allowed = timing_block(&[source], &takes, reset_utc, zone).unwrap();
        assert_eq!(
            allowed, None,
            "Daily limit must reset at 2026-09-06T04:00:00 UTC (local midnight transition)"
        );
    }

    #[test]
    fn cooldown_across_dst_transitions_uses_elapsed_utc_seconds() {
        {
            let zone = chrono_tz::Europe::London;

            let take_utc = NaiveDate::from_ymd_opt(2026, 3, 29)
                .unwrap()
                .and_hms_opt(0, 30, 0)
                .unwrap();
            let mut source = test_source();
            source.max_daily_doses = Some(5);
            source.min_hours_between_doses = Some(Decimal::from(2));
            let takes = vec![test_take(take_utc)];

            let candidate_1h_utc = NaiveDate::from_ymd_opt(2026, 3, 29)
                .unwrap()
                .and_hms_opt(1, 30, 0)
                .unwrap();
            assert_eq!(
                timing_block(&[source.clone()], &takes, candidate_1h_utc, zone).unwrap(),
                Some("cooldown")
            );

            let candidate_2h_utc = NaiveDate::from_ymd_opt(2026, 3, 29)
                .unwrap()
                .and_hms_opt(2, 30, 0)
                .unwrap();
            assert_eq!(
                timing_block(&[source], &takes, candidate_2h_utc, zone).unwrap(),
                None
            );

            let take_fall_utc = NaiveDate::from_ymd_opt(2026, 10, 25)
                .unwrap()
                .and_hms_opt(0, 30, 0)
                .unwrap();
            let mut source_fall = test_source();
            source_fall.max_daily_doses = Some(5);
            source_fall.min_hours_between_doses = Some(Decimal::from(1));
            let takes_fall = vec![test_take(take_fall_utc)];

            let cand_45m_utc = NaiveDate::from_ymd_opt(2026, 10, 25)
                .unwrap()
                .and_hms_opt(1, 15, 0)
                .unwrap();
            assert_eq!(
                timing_block(&[source_fall.clone()], &takes_fall, cand_45m_utc, zone).unwrap(),
                Some("cooldown")
            );

            let cand_60m_utc = NaiveDate::from_ymd_opt(2026, 10, 25)
                .unwrap()
                .and_hms_opt(1, 30, 0)
                .unwrap();
            assert_eq!(
                timing_block(&[source_fall], &takes_fall, cand_60m_utc, zone).unwrap(),
                None
            );
        }

        {
            let zone = chrono_tz::America::New_York;

            let take_utc = NaiveDate::from_ymd_opt(2026, 3, 8)
                .unwrap()
                .and_hms_opt(6, 30, 0)
                .unwrap();
            let mut source = test_source();
            source.max_daily_doses = Some(5);
            source.min_hours_between_doses = Some(Decimal::from(2));
            let takes = vec![test_take(take_utc)];

            let cand_1h_utc = NaiveDate::from_ymd_opt(2026, 3, 8)
                .unwrap()
                .and_hms_opt(7, 30, 0)
                .unwrap();
            assert_eq!(
                timing_block(&[source.clone()], &takes, cand_1h_utc, zone).unwrap(),
                Some("cooldown")
            );

            let cand_2h_utc = NaiveDate::from_ymd_opt(2026, 3, 8)
                .unwrap()
                .and_hms_opt(8, 30, 0)
                .unwrap();
            assert_eq!(
                timing_block(&[source], &takes, cand_2h_utc, zone).unwrap(),
                None
            );

            let take_fall_utc = NaiveDate::from_ymd_opt(2026, 11, 1)
                .unwrap()
                .and_hms_opt(5, 30, 0)
                .unwrap();
            let mut source_fall = test_source();
            source_fall.max_daily_doses = Some(5);
            source_fall.min_hours_between_doses = Some(Decimal::from(1));
            let takes_fall = vec![test_take(take_fall_utc)];

            let cand_45m_utc = NaiveDate::from_ymd_opt(2026, 11, 1)
                .unwrap()
                .and_hms_opt(6, 15, 0)
                .unwrap();
            assert_eq!(
                timing_block(&[source_fall.clone()], &takes_fall, cand_45m_utc, zone).unwrap(),
                Some("cooldown")
            );

            let cand_60m_utc = NaiveDate::from_ymd_opt(2026, 11, 1)
                .unwrap()
                .and_hms_opt(6, 30, 0)
                .unwrap();
            assert_eq!(
                timing_block(&[source_fall], &takes_fall, cand_60m_utc, zone).unwrap(),
                None
            );
        }

        {
            let zone = chrono_tz::Australia::Lord_Howe;

            let take_utc = NaiveDate::from_ymd_opt(2026, 10, 3)
                .unwrap()
                .and_hms_opt(15, 15, 0)
                .unwrap();
            let mut source = test_source();
            source.max_daily_doses = Some(5);
            source.min_hours_between_doses = Some(Decimal::from(1));
            let takes = vec![test_take(take_utc)];

            let cand_30m_utc = NaiveDate::from_ymd_opt(2026, 10, 3)
                .unwrap()
                .and_hms_opt(15, 45, 0)
                .unwrap();
            assert_eq!(
                timing_block(&[source.clone()], &takes, cand_30m_utc, zone).unwrap(),
                Some("cooldown")
            );

            let cand_60m_utc = NaiveDate::from_ymd_opt(2026, 10, 3)
                .unwrap()
                .and_hms_opt(16, 15, 0)
                .unwrap();
            assert_eq!(
                timing_block(&[source], &takes, cand_60m_utc, zone).unwrap(),
                None
            );

            let take_fall_utc = NaiveDate::from_ymd_opt(2026, 4, 4)
                .unwrap()
                .and_hms_opt(14, 15, 0)
                .unwrap();
            let mut source_fall = test_source();
            source_fall.max_daily_doses = Some(5);
            source_fall.min_hours_between_doses = Some(Decimal::from(1));
            let takes_fall = vec![test_take(take_fall_utc)];

            let cand_45m_utc = NaiveDate::from_ymd_opt(2026, 4, 4)
                .unwrap()
                .and_hms_opt(15, 0, 0)
                .unwrap();
            assert_eq!(
                timing_block(&[source_fall.clone()], &takes_fall, cand_45m_utc, zone).unwrap(),
                Some("cooldown")
            );

            let cand_60m_utc = NaiveDate::from_ymd_opt(2026, 4, 4)
                .unwrap()
                .and_hms_opt(15, 15, 0)
                .unwrap();
            assert_eq!(
                timing_block(&[source_fall], &takes_fall, cand_60m_utc, zone).unwrap(),
                None
            );
        }
    }

    #[test]
    fn weekly_and_monthly_dose_cycle_across_leap_day_and_year_boundary() {
        let zone = chrono_tz::UTC;

        {
            let mut source = test_source();
            source.dose_cycle = Some(1);
            source.max_daily_doses = Some(1);

            let take_leap = NaiveDate::from_ymd_opt(2024, 2, 29)
                .unwrap()
                .and_hms_opt(12, 0, 0)
                .unwrap();
            let takes = vec![test_take(take_leap)];

            let next_day = NaiveDate::from_ymd_opt(2024, 3, 1)
                .unwrap()
                .and_hms_opt(12, 0, 0)
                .unwrap();
            assert_eq!(
                timing_block(&[source.clone()], &takes, next_day, zone).unwrap(),
                Some("max_reached"),
                "Weekly cycle must group 2024-02-29 and 2024-03-01 in the same week"
            );

            let next_week = NaiveDate::from_ymd_opt(2024, 3, 4)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap();
            assert_eq!(
                timing_block(&[source], &takes, next_week, zone).unwrap(),
                None
            );
        }

        {
            let mut source = test_source();
            source.dose_cycle = Some(2);
            source.max_daily_doses = Some(1);

            let take_leap = NaiveDate::from_ymd_opt(2024, 2, 29)
                .unwrap()
                .and_hms_opt(23, 0, 0)
                .unwrap();
            let takes = vec![test_take(take_leap)];

            let same_month = NaiveDate::from_ymd_opt(2024, 2, 29)
                .unwrap()
                .and_hms_opt(23, 30, 0)
                .unwrap();
            assert_eq!(
                timing_block(&[source.clone()], &takes, same_month, zone).unwrap(),
                Some("max_reached")
            );

            let new_month = NaiveDate::from_ymd_opt(2024, 3, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap();
            assert_eq!(
                timing_block(&[source], &takes, new_month, zone).unwrap(),
                None,
                "Monthly cycle must reset at 2024-03-01 00:00:00 across leap day"
            );
        }

        {
            let mut source = test_source();
            source.dose_cycle = Some(1);
            source.max_daily_doses = Some(1);

            let take_nye = NaiveDate::from_ymd_opt(2026, 12, 31)
                .unwrap()
                .and_hms_opt(18, 0, 0)
                .unwrap();
            let takes = vec![test_take(take_nye)];

            let new_years_day = NaiveDate::from_ymd_opt(2027, 1, 1)
                .unwrap()
                .and_hms_opt(9, 0, 0)
                .unwrap();
            assert_eq!(
                timing_block(&[source.clone()], &takes, new_years_day, zone).unwrap(),
                Some("max_reached"),
                "Weekly cycle must span the year boundary for the same ISO week"
            );

            let next_week = NaiveDate::from_ymd_opt(2027, 1, 4)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap();
            assert_eq!(
                timing_block(&[source], &takes, next_week, zone).unwrap(),
                None
            );
        }

        {
            let mut source = test_source();
            source.dose_cycle = Some(2);
            source.max_daily_doses = Some(1);

            let take_nye = NaiveDate::from_ymd_opt(2026, 12, 31)
                .unwrap()
                .and_hms_opt(23, 0, 0)
                .unwrap();
            let takes = vec![test_take(take_nye)];

            let same_month = NaiveDate::from_ymd_opt(2026, 12, 31)
                .unwrap()
                .and_hms_opt(23, 45, 0)
                .unwrap();
            assert_eq!(
                timing_block(&[source.clone()], &takes, same_month, zone).unwrap(),
                Some("max_reached")
            );

            let new_month = NaiveDate::from_ymd_opt(2027, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap();
            assert_eq!(
                timing_block(&[source], &takes, new_month, zone).unwrap(),
                None,
                "Monthly cycle must reset at 2027-01-01 00:00:00 across year boundary"
            );
        }
    }

    #[test]
    fn applies_on_schedule_variants() {
        let mut source = test_source();
        source.start_date = Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        source.end_date = Some(NaiveDate::from_ymd_opt(2026, 1, 31).unwrap());

        source.schedule_type = Some(0);
        assert!(!applies_on(
            &source,
            NaiveDate::from_ymd_opt(2025, 12, 31).unwrap()
        ));
        assert!(!applies_on(
            &source,
            NaiveDate::from_ymd_opt(2026, 2, 1).unwrap()
        ));
        assert!(applies_on(
            &source,
            NaiveDate::from_ymd_opt(2026, 1, 15).unwrap()
        ));

        source.schedule_type = Some(2);
        source.schedule_config = Some(json!({ "weekdays": ["monday", "wednesday"] }));
        assert!(applies_on(
            &source,
            NaiveDate::from_ymd_opt(2026, 1, 5).unwrap()
        ));
        assert!(!applies_on(
            &source,
            NaiveDate::from_ymd_opt(2026, 1, 6).unwrap()
        ));

        source.schedule_type = Some(3);
        source.schedule_config = Some(json!({ "dates": ["2026-01-10", "2026-01-20"] }));
        assert!(applies_on(
            &source,
            NaiveDate::from_ymd_opt(2026, 1, 10).unwrap()
        ));
        assert!(!applies_on(
            &source,
            NaiveDate::from_ymd_opt(2026, 1, 11).unwrap()
        ));

        source.schedule_type = Some(5);
        source.schedule_config = Some(json!({
            "taper_steps": [
                {
                    "start_date": "2026-01-05",
                    "end_date": "2026-01-07",
                    "amount": "5"
                }
            ]
        }));
        assert!(!applies_on(
            &source,
            NaiveDate::from_ymd_opt(2026, 1, 4).unwrap()
        ));
        assert!(applies_on(
            &source,
            NaiveDate::from_ymd_opt(2026, 1, 5).unwrap()
        ));
        assert!(applies_on(
            &source,
            NaiveDate::from_ymd_opt(2026, 1, 7).unwrap()
        ));
        assert!(!applies_on(
            &source,
            NaiveDate::from_ymd_opt(2026, 1, 8).unwrap()
        ));

        source.schedule_type = Some(6);
        source.schedule_config = None;
        assert!(applies_on(
            &source,
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
        ));
        assert!(!applies_on(
            &source,
            NaiveDate::from_ymd_opt(2026, 1, 2).unwrap()
        ));
        assert!(applies_on(
            &source,
            NaiveDate::from_ymd_opt(2026, 1, 3).unwrap()
        ));
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        #[test]
        fn prop_timing_block_separation(
            zone_idx in 0..4usize,
            min_hours in 1..=12i64,
            max_doses in 1..=4i32,
            day_of_year in 0..365i64,
            minute_of_day in 0..1440i64,
            take_minute_offsets in proptest::collection::vec(-2880..=120i64, 0..=8),
        ) {
            let zones = [
                chrono_tz::Europe::London,
                chrono_tz::America::New_York,
                chrono_tz::Australia::Lord_Howe,
                chrono_tz::America::Santiago,
            ];
            let zone = zones[zone_idx];

            let mut source = test_source();
            source.max_daily_doses = Some(max_doses);
            source.min_hours_between_doses = Some(Decimal::from(min_hours));
            source.dose_cycle = None;

            let base = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap();
            let taken_at = base + Duration::days(day_of_year) + Duration::minutes(minute_of_day);

            let takes: Vec<medication_take::Model> = take_minute_offsets
                .into_iter()
                .map(|offset| test_take(taken_at + Duration::minutes(offset)))
                .collect();

            let (cycle_start_utc, cycle_end_utc) = independent_local_cycle_bounds(taken_at, zone);

            let takes_in_cycle = takes
                .iter()
                .filter_map(|t| t.taken_at)
                .filter(|&t| t >= cycle_start_utc && t < cycle_end_utc)
                .count();

            let latest_prior_take = takes
                .iter()
                .filter_map(|t| t.taken_at)
                .filter(|&t| t <= taken_at)
                .max();

            let expected = if takes_in_cycle >= max_doses as usize {
                Some("max_reached")
            } else if let Some(prior) = latest_prior_take {
                let diff_secs = (taken_at - prior).num_seconds();
                if (0..min_hours * 3600).contains(&diff_secs) {
                    Some("cooldown")
                } else {
                    None
                }
            } else {
                None
            };

            let actual = timing_block(&[source], &takes, taken_at, zone)
                .expect("timing_block should succeed");

            prop_assert_eq!(actual, expected);
        }
    }
}
