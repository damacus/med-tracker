use super::*;
use loco_rs::model::TenantQueryExt;

#[derive(Clone)]
pub(super) struct Occurrence {
    pub(super) window_start: NaiveDate,
    pub(super) window_end: NaiveDate,
    pub(super) position: i32,
    pub(super) scheduled_at: Option<NaiveDateTime>,
    pub(super) expected: bool,
    pub(super) record: Option<dose_occurrence::Model>,
    pub(super) legacy_take_id: Option<i64>,
    pub(super) take: Option<medication_take::Model>,
}

fn pause_covers(period: &pause_period::Model, time: NaiveDateTime) -> bool {
    let start = period.started_at.unwrap_or(period.created_at);
    start <= time && period.ended_at.is_none_or(|end| time < end)
}

fn fully_paused(
    source: &Source,
    start: NaiveDate,
    end: NaiveDate,
    pauses: &[pause_period::Model],
    settings: Option<&serde_json::Value>,
) -> bool {
    let mut cursor = local_midnight(start, settings);
    let finish = local_midnight(end + Duration::days(1), settings);
    if source.kind() == Kind::Assignment {
        cursor = cursor.max(source.created_at());
    }
    let mut periods = pauses.to_vec();
    periods.sort_by_key(|period| period.started_at.unwrap_or(period.created_at));
    for period in periods {
        let begun = period.started_at.unwrap_or(period.created_at);
        if begun <= cursor {
            cursor = cursor.max(period.ended_at.unwrap_or(finish));
            if cursor >= finish {
                return true;
            }
        }
    }
    false
}

pub(super) async fn projected(
    db: &DatabaseTransaction,
    source: &Source,
    start: NaiveDate,
    end: NaiveDate,
    settings: Option<&serde_json::Value>,
) -> Result<Vec<Occurrence>, ApiError> {
    let source_settings = source.calendar_settings(settings);
    let settings = source_settings.as_ref();
    let (scan_start, scan_end) = scan_bounds(source, start, end);
    let mut query = dose_occurrence::Entity::find()
        .in_tenant(match source {
            Source::Schedule(row) => row.household_id,
            Source::Assignment(row) => row.household_id,
        })
        .filter(dose_occurrence::Column::WindowStartsOn.lte(scan_end))
        .filter(
            Condition::any()
                .add(dose_occurrence::Column::WindowEndsOn.gte(scan_start))
                .add(
                    Condition::all()
                        .add(dose_occurrence::Column::WindowEndsOn.is_null())
                        .add(dose_occurrence::Column::WindowStartsOn.gte(scan_start)),
                ),
        );
    query = match source {
        Source::Schedule(row) => query.filter(dose_occurrence::Column::ScheduleId.eq(row.id)),
        Source::Assignment(row) => {
            query.filter(dose_occurrence::Column::PersonMedicationId.eq(row.id))
        }
    };
    let persisted = query.all(db).await.map_err(database_error)?;
    let pauses = match source {
        Source::Schedule(row) => {
            pause_period::Entity::find()
                .filter(pause_period::Column::ScheduleId.eq(row.id))
                .all(db)
                .await
        }
        Source::Assignment(row) => {
            pause_period::Entity::find()
                .filter(pause_period::Column::PersonMedicationId.eq(row.id))
                .all(db)
                .await
        }
    }
    .map_err(database_error)?;
    let linked = match source {
        Source::Schedule(row) => {
            dose_occurrence::Entity::find().filter(dose_occurrence::Column::ScheduleId.eq(row.id))
        }
        Source::Assignment(row) => dose_occurrence::Entity::find()
            .filter(dose_occurrence::Column::PersonMedicationId.eq(row.id)),
    }
    .filter(dose_occurrence::Column::MedicationTakeId.is_not_null())
    .select_only()
    .column(dose_occurrence::Column::MedicationTakeId)
    .into_query();
    let mut takes = medication_take::Entity::find()
        .filter(medication_take::Column::TakenAt.gte(local_midnight(scan_start, settings)))
        .filter(
            medication_take::Column::TakenAt
                .lt(local_midnight(scan_end + Duration::days(1), settings)),
        )
        .filter(medication_take::Column::Id.not_in_subquery(linked))
        .order_by_asc(medication_take::Column::TakenAt)
        .order_by_asc(medication_take::Column::Id);
    takes = match source {
        Source::Schedule(row) => takes.filter(medication_take::Column::ScheduleId.eq(row.id)),
        Source::Assignment(row) => {
            takes.filter(medication_take::Column::PersonMedicationId.eq(row.id))
        }
    };
    let mut takes = takes.all(db).await.map_err(database_error)?;
    let linked_ids: Vec<_> = persisted
        .iter()
        .filter_map(|row| row.medication_take_id)
        .collect();
    if !linked_ids.is_empty() {
        takes.extend(
            medication_take::Entity::find()
                .in_tenant(match source {
                    Source::Schedule(row) => row.household_id,
                    Source::Assignment(row) => row.household_id,
                })
                .filter(medication_take::Column::Id.is_in(linked_ids))
                .all(db)
                .await
                .map_err(database_error)?,
        );
    }
    Ok(projected_loaded(
        source, scan_start, scan_end, &persisted, &pauses, &takes, settings,
    ))
}

pub(super) fn scan_bounds(
    source: &Source,
    start: NaiveDate,
    end: NaiveDate,
) -> (NaiveDate, NaiveDate) {
    if source.kind() == Kind::Assignment {
        let (start, _) = cycle_bounds(start, source.dose_cycle());
        let (_, end) = cycle_bounds(end, source.dose_cycle());
        (start, end)
    } else {
        (start, end)
    }
}

pub(super) fn projected_loaded(
    source: &Source,
    scan_start: NaiveDate,
    scan_end: NaiveDate,
    persisted: &[dose_occurrence::Model],
    pauses: &[pause_period::Model],
    takes: &[medication_take::Model],
    settings: Option<&serde_json::Value>,
) -> Vec<Occurrence> {
    let source_settings = source.calendar_settings(settings);
    let settings = source_settings.as_ref();
    let mut rows: BTreeMap<(NaiveDate, i32), Occurrence> = BTreeMap::new();
    let mut day = scan_start;
    while day <= scan_end {
        match source {
            Source::Schedule(schedule) => {
                if schedule_applies(schedule, day)
                    && !schedule_as_needed(schedule)
                    && (source.active() || !pauses.is_empty())
                    && let Some(config) = schedule_config_on(schedule, day)
                {
                    let times = if config.get("times").is_none() {
                        config_times(&schedule.schedule_config, day, settings)
                    } else {
                        config_times(config, day, settings)
                    };
                    if times.is_empty() {
                        if !fully_paused(source, day, day, pauses, settings) {
                            let fallback = effective_count(
                                &schedule.schedule_config,
                                source.max_daily_doses(),
                            );
                            let count = effective_count(config, fallback);
                            for position in 1..=count {
                                rows.insert(
                                    (day, position),
                                    Occurrence {
                                        window_start: day,
                                        window_end: day,
                                        position,
                                        scheduled_at: None,
                                        expected: true,
                                        record: None,
                                        legacy_take_id: None,
                                        take: None,
                                    },
                                );
                            }
                        }
                    } else {
                        for (index, time) in times.into_iter().enumerate() {
                            if !pauses.iter().any(|period| pause_covers(period, time)) {
                                let position = i32::try_from(index + 1).unwrap_or(i32::MAX);
                                rows.insert(
                                    (day, position),
                                    Occurrence {
                                        window_start: day,
                                        window_end: day,
                                        position,
                                        scheduled_at: Some(time),
                                        expected: true,
                                        record: None,
                                        legacy_take_id: None,
                                        take: None,
                                    },
                                );
                            }
                        }
                    }
                }
            }
            Source::Assignment(assignment) => {
                if assignment.administration_kind == 0
                    && (source.active() || !pauses.is_empty())
                    && cycle_bounds(day, source.dose_cycle()).0 == day
                {
                    let (_, finish) = cycle_bounds(day, source.dose_cycle());
                    if finish >= local_date(source.created_at(), settings)
                        && !fully_paused(source, day, finish, pauses, settings)
                    {
                        for position in 1..=source.max_daily_doses().max(1) {
                            rows.insert(
                                (day, position),
                                Occurrence {
                                    window_start: day,
                                    window_end: finish,
                                    position,
                                    scheduled_at: None,
                                    expected: true,
                                    record: None,
                                    legacy_take_id: None,
                                    take: None,
                                },
                            );
                        }
                    }
                }
            }
        }
        day += Duration::days(1);
    }
    for record in persisted {
        let finish = record.window_ends_on.unwrap_or(record.window_starts_on);
        if finish < scan_start || record.window_starts_on > scan_end {
            continue;
        }
        let identity = (record.window_starts_on, record.position);
        let expected = rows.contains_key(&identity);
        rows.insert(
            identity,
            Occurrence {
                window_start: record.window_starts_on,
                window_end: finish,
                position: record.position,
                scheduled_at: record.scheduled_at,
                expected,
                record: Some(record.clone()),
                legacy_take_id: None,
                take: record
                    .medication_take_id
                    .and_then(|id| takes.iter().find(|take| take.id == id))
                    .cloned(),
            },
        );
    }
    let mut historical = Vec::new();
    let mut seen: std::collections::BTreeSet<_> = persisted
        .iter()
        .filter_map(|row| row.medication_take_id)
        .collect();
    let mut ordered_takes: Vec<_> = takes.iter().collect();
    ordered_takes.sort_by_key(|take| (take.taken_at, take.id));
    for take in ordered_takes {
        if !seen.insert(take.id) {
            continue;
        }
        let Some(taken_at) = take.taken_at else {
            continue;
        };
        let day = local_date(taken_at, settings);
        if day < scan_start || day > scan_end {
            continue;
        }
        let window = if source.kind() == Kind::Assignment {
            cycle_bounds(day, source.dose_cycle()).0
        } else {
            day
        };
        if let Some((_, row)) = rows.iter_mut().find(|((date, _), row)| {
            *date == window
                && row.expected
                && row.record.as_ref().is_none_or(|record| {
                    record.outcome == "open" && record.medication_take_id.is_none()
                })
                && row.legacy_take_id.is_none()
        }) {
            row.legacy_take_id = Some(take.id);
            row.take = Some(take.clone());
        } else {
            historical.push(Occurrence {
                window_start: day,
                window_end: day,
                position: 0,
                scheduled_at: None,
                expected: false,
                record: None,
                legacy_take_id: Some(take.id),
                take: Some(take.clone()),
            });
        }
    }
    let mut last_position: BTreeMap<NaiveDate, i32> = BTreeMap::new();
    for row in rows.values() {
        let position = last_position.entry(row.window_start).or_default();
        *position = (*position).max(row.position);
    }
    for row in &mut historical {
        let position = last_position.entry(row.window_start).or_default();
        *position = position.saturating_add(1);
        row.position = *position;
    }
    let mut result: Vec<_> = rows.into_values().chain(historical).collect();
    result.sort_by_key(|row| {
        (
            row.window_start,
            row.position,
            row.take.as_ref().and_then(|take| take.taken_at),
            row.legacy_take_id,
        )
    });
    result
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

    fn make_schedule(
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
            created_at: ymd(2024, 1, 1).and_hms_opt(0, 0, 0).unwrap(),
            updated_at: ymd(2024, 1, 1).and_hms_opt(0, 0, 0).unwrap(),
        }
    }

    fn make_assignment(
        created_at: NaiveDateTime,
        dose_cycle: Option<i32>,
        max_daily_doses: Option<i32>,
    ) -> person_medication::Model {
        person_medication::Model {
            id: 1,
            active: true,
            administration_kind: 0,
            created_at,
            dose_amount: None,
            dose_cycle,
            dose_unit: None,
            household_id: 1,
            max_daily_doses,
            medication_id: 1,
            min_hours_between_doses: None,
            notes: None,
            person_id: 1,
            portable_id: "pm1".to_string(),
            position: 1,
            retired_at: None,
            source_dosage_option_id: None,
            updated_at: created_at,
        }
    }

    fn direct_take(id: i64, taken_at: NaiveDateTime) -> medication_take::Model {
        medication_take::Model {
            id,
            client_uuid: Some(format!("direct-{id}")),
            created_at: taken_at,
            dose_amount: Some("2.5".parse().unwrap()),
            dose_unit: Some("ml".into()),
            household_id: 1,
            person_medication_id: Some(1),
            portable_id: format!("take-{id}"),
            schedule_id: None,
            taken_at: Some(taken_at),
            taken_from_location_id: Some(7),
            taken_from_medication_id: Some(1),
            updated_at: taken_at,
        }
    }

    #[test]
    fn projected_loaded_direct_prn_assignment_takes_appear_once_without_expected_doses() {
        let day = ymd(2026, 11, 14);
        let mut assignment = make_assignment(ymd_hms(2026, 11, 1, 0, 0, 0), Some(0), Some(4));
        assignment.administration_kind = 1;
        let source = Source::Assignment(assignment);
        assert!(projected_loaded(&source, day, day, &[], &[], &[], None).is_empty());
        let takes = [
            direct_take(41, ymd_hms(2026, 11, 14, 10, 0, 0)),
            direct_take(42, ymd_hms(2026, 11, 14, 15, 36, 0)),
        ];
        let original = takes.clone();
        let rows = projected_loaded(&source, day, day, &[], &[], &takes, None);
        assert_eq!(
            rows.len(),
            2,
            "both direct PRN ledger entries must be visible"
        );
        assert!(rows.iter().all(|row| !row.expected && row.record.is_none()));
        assert_eq!(
            rows.iter()
                .filter_map(|row| row.legacy_take_id)
                .collect::<Vec<_>>(),
            vec![41, 42]
        );
        assert!(
            rows.iter()
                .all(|row| row.window_start == day && row.window_end == day)
        );
        let secret: Arc<[u8]> = Arc::from(&b"medtracker-test-secret-32-bytes-long!"[..]);
        let values: Vec<_> = rows
            .iter()
            .map(|row| row_value(&secret, &source, row, None))
            .collect();
        assert_ne!(values[0]["key"], values[1]["key"]);
        assert_ne!(rows[0].position, rows[1].position);
        for ((value, row), take) in values.iter().zip(&rows).zip(&takes) {
            assert!(row.position > 0);
            assert_eq!(row.take.as_ref(), Some(take));
            assert_eq!(
                value["resolved_at"],
                take.taken_at
                    .unwrap()
                    .and_utc()
                    .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
            );
            assert!(!super::super::persistence::actionable(
                &source, &rows[0], None
            ));
            assert!(decode_key(&secret, &source, value["key"].as_str().unwrap()).is_none());
        }
        assert_eq!(
            takes, original,
            "projection must preserve ledger attribution"
        );
    }

    #[test]
    fn projected_loaded_direct_prn_schedule_take_has_no_expected_dose() {
        let day = ymd(2026, 11, 14);
        let source = Source::Schedule(make_schedule(0, json!({"as_needed": true}), day, day));
        let mut take = direct_take(41, ymd_hms(2026, 11, 14, 10, 0, 0));
        take.person_medication_id = None;
        take.schedule_id = Some(1);
        let rows = projected_loaded(&source, day, day, &[], &[], &[take], None);
        assert_eq!(rows.len(), 1);
        assert!(!rows[0].expected);
        assert_eq!(rows[0].legacy_take_id, Some(41));
    }

    #[test]
    fn projected_loaded_regular_excess_direct_take_is_not_lost() {
        let day = ymd(2026, 11, 14);
        let source = Source::Assignment(make_assignment(
            ymd_hms(2026, 11, 1, 0, 0, 0),
            Some(0),
            Some(1),
        ));
        let takes = [
            direct_take(41, ymd_hms(2026, 11, 14, 10, 0, 0)),
            direct_take(42, ymd_hms(2026, 11, 14, 15, 36, 0)),
        ];
        let rows = projected_loaded(&source, day, day, &[], &[], &takes, None);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows.iter().filter(|row| row.expected).count(), 1);
        assert_eq!(
            rows.iter()
                .filter_map(|row| row.legacy_take_id)
                .collect::<Vec<_>>(),
            vec![41, 42]
        );
    }

    #[test]
    fn projected_loaded_historical_keys_remain_non_actionable_after_treatment_edits() {
        let day = ymd(2026, 11, 14);
        let mut assignment = make_assignment(ymd_hms(2026, 11, 1, 0, 0, 0), Some(0), Some(1));
        assignment.administration_kind = 1;
        let source = Source::Assignment(assignment.clone());
        let take = direct_take(41, ymd_hms(2026, 11, 14, 10, 0, 0));
        let secret: Arc<[u8]> = Arc::from(&b"medtracker-test-secret-32-bytes-long!"[..]);
        let rows = projected_loaded(
            &source,
            day,
            day,
            &[],
            &[],
            std::slice::from_ref(&take),
            None,
        );
        let value = row_value(&secret, &source, &rows[0], None);
        let historical_key = value["key"].as_str().unwrap();
        assignment.administration_kind = 0;
        assignment.max_daily_doses = Some(4);
        assignment.dose_cycle = Some(1);
        let edited = Source::Assignment(assignment);
        assert!(decode_key(&secret, &edited, historical_key).is_none());
        assert_ne!(historical_key, key(&secret, &edited, day, 1));
        assert_eq!(
            historical_key,
            keys::historical_key(&secret, &edited, &take)
        );
    }

    #[test]
    fn projected_loaded_prn_cycle_uses_taken_local_day_and_filters_range_edges() {
        let mut assignment = make_assignment(ymd_hms(2026, 10, 1, 0, 0, 0), Some(1), Some(4));
        assignment.administration_kind = 1;
        let source = Source::Assignment(assignment);
        let settings = json!({"time_zone": "Europe/London"});
        let day = ymd(2026, 10, 24);
        let takes = [
            direct_take(40, ymd_hms(2026, 10, 23, 22, 59, 59)),
            direct_take(41, ymd_hms(2026, 10, 23, 23, 0, 0)),
            direct_take(42, ymd_hms(2026, 10, 24, 22, 59, 59)),
            direct_take(43, ymd_hms(2026, 10, 24, 23, 0, 0)),
        ];
        let duplicated = [
            takes[2].clone(),
            takes[1].clone(),
            takes[1].clone(),
            takes[0].clone(),
            takes[3].clone(),
        ];
        let rows = projected_loaded(&source, day, day, &[], &[], &duplicated, Some(&settings));
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows.iter()
                .filter_map(|row| row.legacy_take_id)
                .collect::<Vec<_>>(),
            vec![41, 42]
        );
        assert!(
            rows.iter()
                .all(|row| row.window_start == day && row.window_end == day && !row.expected)
        );
        let secret: Arc<[u8]> = Arc::from(&b"medtracker-test-secret-32-bytes-long!"[..]);
        let (start, end) = scan_bounds(&source, day, day);
        let wide = projected_loaded(&source, start, end, &[], &[], &takes, Some(&settings));
        for row in rows {
            let full = wide
                .iter()
                .find(|candidate| candidate.legacy_take_id == row.legacy_take_id)
                .unwrap();
            assert_eq!(
                row_value(&secret, &source, &row, Some(&settings))["key"],
                row_value(&secret, &source, full, Some(&settings))["key"]
            );
        }
    }

    #[test]
    fn projected_loaded_linked_prn_take_and_direct_take_each_appear_once() {
        let day = ymd(2026, 11, 14);
        let mut assignment = make_assignment(ymd_hms(2026, 11, 1, 0, 0, 0), Some(0), Some(4));
        assignment.administration_kind = 1;
        let source = Source::Assignment(assignment);
        let time = ymd_hms(2026, 11, 14, 10, 0, 0);
        let record = dose_occurrence::Model {
            id: 9,
            household_id: 1,
            schedule_id: None,
            person_medication_id: Some(1),
            medication_take_id: Some(41),
            resolved_by_membership_id: Some(3),
            portable_id: "linked-prn".into(),
            window_starts_on: day,
            window_ends_on: Some(day),
            position: 1,
            scheduled_at: None,
            outcome: "taken".into(),
            reason: None,
            note: Some("Original attribution".into()),
            resolved_at: Some(time),
            created_at: time,
            updated_at: time,
        };
        let direct = direct_take(42, ymd_hms(2026, 11, 14, 15, 36, 0));
        let rows = projected_loaded(
            &source,
            day,
            day,
            std::slice::from_ref(&record),
            &[],
            &[direct, direct_take(41, time)],
            None,
        );
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows.iter()
                .filter(|row| row.record.as_ref() == Some(&record))
                .count(),
            1
        );
        assert_eq!(
            rows.iter()
                .filter(|row| row.legacy_take_id == Some(42))
                .count(),
            1
        );
        assert!(rows.iter().all(|row| !row.expected));
        assert_ne!(rows[0].position, rows[1].position);
    }

    #[test]
    fn projected_loaded_alternate_day_schedule_leap_day_and_year_boundary() {
        let leap_sched = make_schedule(6, json!({}), ymd(2024, 2, 28), ymd(2024, 3, 1));
        let source_leap = Source::Schedule(leap_sched);
        let (scan_start, scan_end) = scan_bounds(&source_leap, ymd(2024, 2, 28), ymd(2024, 3, 1));
        let occs = projected_loaded(&source_leap, scan_start, scan_end, &[], &[], &[], None);
        assert_eq!(occs.len(), 2);
        assert_eq!(occs[0].window_start, ymd(2024, 2, 28));
        assert_eq!(occs[1].window_start, ymd(2024, 3, 1));
        assert!(!occs.iter().any(|o| o.window_start == ymd(2024, 2, 29)));

        let year_sched = make_schedule(6, json!({}), ymd(2026, 12, 30), ymd(2027, 1, 1));
        let source_year = Source::Schedule(year_sched);
        let (scan_start, scan_end) = scan_bounds(&source_year, ymd(2026, 12, 30), ymd(2027, 1, 1));
        let occs = projected_loaded(&source_year, scan_start, scan_end, &[], &[], &[], None);
        assert_eq!(occs.len(), 2);
        assert_eq!(occs[0].window_start, ymd(2026, 12, 30));
        assert_eq!(occs[1].window_start, ymd(2027, 1, 1));
        assert!(!occs.iter().any(|o| o.window_start == ymd(2026, 12, 31)));
    }

    #[test]
    fn projected_loaded_taper_schedule_inherits_top_level_times_and_handles_gaps() {
        let config = json!({
            "times": ["08:00", "20:00"],
            "taper_steps": [
                {
                    "start_date": "2026-01-05",
                    "end_date": "2026-01-07",
                    "amount": "1"
                },
                {
                    "start_date": "2026-01-09",
                    "end_date": "2026-01-10",
                    "amount": "0.5",
                    "times": ["12:00"]
                }
            ]
        });
        let sched = make_schedule(5, config, ymd(2026, 1, 1), ymd(2026, 1, 31));
        let source = Source::Schedule(sched);
        let occs = projected_loaded(
            &source,
            ymd(2026, 1, 5),
            ymd(2026, 1, 10),
            &[],
            &[],
            &[],
            None,
        );

        assert_eq!(occs.len(), 8);

        let day1_occs: Vec<_> = occs
            .iter()
            .filter(|o| o.window_start == ymd(2026, 1, 5))
            .collect();
        assert_eq!(day1_occs.len(), 2);
        assert_eq!(day1_occs[0].position, 1);
        assert_eq!(
            day1_occs[0].scheduled_at,
            Some(ymd_hms(2026, 1, 5, 8, 0, 0))
        );
        assert_eq!(day1_occs[1].position, 2);
        assert_eq!(
            day1_occs[1].scheduled_at,
            Some(ymd_hms(2026, 1, 5, 20, 0, 0))
        );

        assert!(!occs.iter().any(|o| o.window_start == ymd(2026, 1, 8)));

        let day5_occs: Vec<_> = occs
            .iter()
            .filter(|o| o.window_start == ymd(2026, 1, 9))
            .collect();
        assert_eq!(day5_occs.len(), 1);
        assert_eq!(
            day5_occs[0].scheduled_at,
            Some(ymd_hms(2026, 1, 9, 12, 0, 0))
        );
    }

    #[test]
    fn projected_loaded_taper_schedule_inherits_top_level_max_daily_doses() {
        let config = json!({
            "max_daily_doses": 3,
            "taper_steps": [
                {
                    "start_date": "2026-01-05",
                    "end_date": "2026-01-07",
                    "amount": "1"
                }
            ]
        });
        let sched = make_schedule(5, config, ymd(2026, 1, 1), ymd(2026, 1, 31));
        let source = Source::Schedule(sched);
        let occs = projected_loaded(
            &source,
            ymd(2026, 1, 5),
            ymd(2026, 1, 5),
            &[],
            &[],
            &[],
            None,
        );

        assert_eq!(occs.len(), 3);
        assert_eq!(occs[0].position, 1);
        assert_eq!(occs[1].position, 2);
        assert_eq!(occs[2].position, 3);
        assert_eq!(occs[0].scheduled_at, None);
    }

    #[test]
    fn projected_loaded_taper_schedule_explicit_empty_times_generates_untimed_occurrences() {
        let config = json!({
            "times": ["08:00", "20:00"],
            "taper_steps": [
                {
                    "start_date": "2026-01-05",
                    "end_date": "2026-01-07",
                    "amount": "1",
                    "times": [],
                    "max_daily_doses": 1
                }
            ]
        });
        let sched = make_schedule(5, config, ymd(2026, 1, 1), ymd(2026, 1, 31));
        let source = Source::Schedule(sched);
        let occs = projected_loaded(
            &source,
            ymd(2026, 1, 5),
            ymd(2026, 1, 5),
            &[],
            &[],
            &[],
            None,
        );

        assert_eq!(occs.len(), 1);
        assert_eq!(occs[0].position, 1);
        assert_eq!(occs[0].scheduled_at, None);
    }

    #[test]
    fn projected_loaded_assignment_created_in_evening_local_time_included_on_local_date() {
        let assignment = make_assignment(ymd_hms(2026, 10, 6, 1, 0, 0), Some(0), Some(1));
        let source = Source::Assignment(assignment);
        let settings = json!({ "time_zone": "America/New_York" });
        let target = ymd(2026, 10, 5);
        let occs = projected_loaded(&source, target, target, &[], &[], &[], Some(&settings));

        assert_eq!(
            occs.len(),
            1,
            "assignment created 2026-10-05 21:00 America/New_York must project on 2026-10-05"
        );
        assert_eq!(occs[0].window_start, target);
        assert_eq!(occs[0].window_end, target);
        assert_eq!(occs[0].position, 1);
    }

    #[test]
    fn projected_loaded_assignment_created_after_midnight_local_time_excluded_on_previous_local_date()
     {
        let assignment = make_assignment(ymd_hms(2026, 7, 1, 23, 30, 0), Some(0), Some(1));
        let source = Source::Assignment(assignment);
        let settings = json!({ "time_zone": "Europe/London" });
        let occs = projected_loaded(
            &source,
            ymd(2026, 7, 1),
            ymd(2026, 7, 2),
            &[],
            &[],
            &[],
            Some(&settings),
        );

        assert_eq!(
            occs.len(),
            1,
            "assignment created 2026-07-02 00:30 BST must only project on 2026-07-02, not 2026-07-01"
        );
        assert_eq!(occs[0].window_start, ymd(2026, 7, 2));
        assert_eq!(occs[0].window_end, ymd(2026, 7, 2));
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn adjacent_date_range_partition_invariant(
            tz_name in prop_oneof![
                Just("Europe/London"),
                Just("America/New_York"),
                Just("Australia/Lord_Howe"),
                Just("America/Santiago"),
            ],
            base_choice in prop_oneof![
                Just(NaiveDate::from_ymd_opt(2026, 3, 27).unwrap()),
                Just(NaiveDate::from_ymd_opt(2026, 10, 23).unwrap()),
                Just(NaiveDate::from_ymd_opt(2026, 3, 6).unwrap()),
                Just(NaiveDate::from_ymd_opt(2026, 10, 30).unwrap()),
                Just(NaiveDate::from_ymd_opt(2026, 10, 2).unwrap()),
                Just(NaiveDate::from_ymd_opt(2026, 4, 3).unwrap()),
                Just(NaiveDate::from_ymd_opt(2026, 9, 4).unwrap()),
                Just(NaiveDate::from_ymd_opt(2026, 4, 3).unwrap()),
                Just(NaiveDate::from_ymd_opt(2024, 2, 27).unwrap()),
                Just(NaiveDate::from_ymd_opt(2026, 2, 26).unwrap()),
                Just(NaiveDate::from_ymd_opt(2026, 12, 28).unwrap()),
            ],
            day_shift in 0..3i64,
            arbitrary_offset in 0..1400i64,
            use_arbitrary in any::<bool>(),
            range_len in 1..=10usize,
            split_offset in 0..10usize,
            source_kind in 0..=5i32,
        ) {
            let d0 = if use_arbitrary {
                NaiveDate::from_ymd_opt(2024, 1, 1).unwrap() + Duration::days(arbitrary_offset)
            } else {
                base_choice + Duration::days(day_shift)
            };
            let dn = d0 + Duration::days(range_len as i64 - 1);
            let settings = json!({ "time_zone": tz_name });
            let secret: Arc<[u8]> = Arc::from(&b"medtracker-test-secret-32-bytes-long!"[..]);

            let source = match source_kind {
                0 => Source::Schedule(make_schedule(
                    1,
                    json!({ "times": ["08:00", "20:00"] }),
                    ymd(2024, 1, 1),
                    ymd(2029, 1, 1),
                )),
                1 => {
                    let mut s = make_schedule(
                        1,
                        json!({ "max_daily_doses": 2 }),
                        ymd(2024, 1, 1),
                        ymd(2029, 1, 1),
                    );
                    s.max_daily_doses = Some(2);
                    Source::Schedule(s)
                }
                2 => Source::Schedule(make_schedule(
                    6,
                    json!({}),
                    d0,
                    ymd(2029, 1, 1),
                )),
                3 => Source::Schedule(make_schedule(
                    2,
                    json!({ "weekdays": ["monday", "wednesday", "friday"] }),
                    ymd(2024, 1, 1),
                    ymd(2029, 1, 1),
                )),
                4 => Source::Schedule(make_schedule(
                    3,
                    json!({ "dates": [d0.to_string(), dn.to_string()] }),
                    ymd(2024, 1, 1),
                    ymd(2029, 1, 1),
                )),
                _ => Source::Assignment(make_assignment(
                    ymd_hms(2024, 1, 1, 0, 0, 0),
                    Some(0),
                    Some(1),
                )),
            };

            let (scan_start, scan_end) = scan_bounds(&source, d0, dn);
            let full_raw = projected_loaded(&source, scan_start, scan_end, &[], &[], &[], Some(&settings));
            let full: Vec<(String, Option<NaiveDateTime>)> = full_raw
                .into_iter()
                .filter(|occ| occ.window_start <= dn && occ.window_end >= d0)
                .map(|occ| (key(&secret, &source, occ.window_start, occ.position), occ.scheduled_at))
                .collect();

            let mut day_by_day = Vec::new();
            let mut cur = d0;
            while cur <= dn {
                let (s, e) = scan_bounds(&source, cur, cur);
                let day_occs = projected_loaded(&source, s, e, &[], &[], &[], Some(&settings));
                for occ in day_occs {
                    if occ.window_start <= cur && occ.window_end >= cur {
                        day_by_day.push((
                            key(&secret, &source, occ.window_start, occ.position),
                            occ.scheduled_at,
                        ));
                    }
                }
                cur += Duration::days(1);
            }

            let actual_split_offset = split_offset % range_len;
            let split = d0 + Duration::days(actual_split_offset as i64);
            let subranges: Vec<(String, Option<NaiveDateTime>)> = if split >= dn {
                full.clone()
            } else {
                let (s1, e1) = scan_bounds(&source, d0, split);
                let left_occs = projected_loaded(&source, s1, e1, &[], &[], &[], Some(&settings));
                let left_part = left_occs
                    .into_iter()
                    .filter(|occ| occ.window_start <= split && occ.window_end >= d0)
                    .map(|occ| (key(&secret, &source, occ.window_start, occ.position), occ.scheduled_at));

                let (s2, e2) = scan_bounds(&source, split + Duration::days(1), dn);
                let right_occs = projected_loaded(&source, s2, e2, &[], &[], &[], Some(&settings));
                let right_part = right_occs
                    .into_iter()
                    .filter(|occ| occ.window_start <= dn && occ.window_end >= split + Duration::days(1))
                    .map(|occ| (key(&secret, &source, occ.window_start, occ.position), occ.scheduled_at));

                left_part.chain(right_part).collect()
            };

            let (scan_start2, scan_end2) = scan_bounds(&source, d0, dn);
            let full_repeat: Vec<(String, Option<NaiveDateTime>)> = projected_loaded(
                &source,
                scan_start2,
                scan_end2,
                &[],
                &[],
                &[],
                Some(&settings),
            )
            .into_iter()
            .filter(|occ| occ.window_start <= dn && occ.window_end >= d0)
            .map(|occ| (key(&secret, &source, occ.window_start, occ.position), occ.scheduled_at))
            .collect();

            prop_assert_eq!(&day_by_day, &full, "day-by-day projection mismatch with full range");
            prop_assert_eq!(&subranges, &full, "sub-range partition mismatch with full range");

            let mut seen = std::collections::HashSet::new();
            for item in &full {
                prop_assert!(seen.insert(item.0.clone()), "duplicate occurrence key found: {}", item.0);
            }

            prop_assert_eq!(&full_repeat, &full, "repeated run produced non-deterministic sequence");
        }
    }
}
