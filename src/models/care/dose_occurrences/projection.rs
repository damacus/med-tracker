use super::*;
use loco_rs::model::TenantQueryExt;
use std::collections::BTreeSet;

pub(super) const MAX_PROJECTION_ROWS: usize = 10_000;
pub(super) const PROJECTION_FETCH_LIMIT: u64 = MAX_PROJECTION_ROWS as u64 + 1;

pub(super) fn capacity_error() -> OperationError {
    OperationError::Validation {
        details: json!({"code":"projection_capacity_exceeded","errors":{"date_range":["This timeline is too large to display. Choose a shorter date range or review the schedule."]}}),
    }
}

pub(super) fn check_capacity(count: usize) -> Result<(), OperationError> {
    if count > MAX_PROJECTION_ROWS {
        Err(capacity_error())
    } else {
        Ok(())
    }
}

pub(super) struct ProjectionContext<'a> {
    settings: Option<&'a Value>,
    execution: bool,
    remaining_rows: usize,
    remaining_days: usize,
    remaining_work: usize,
}

impl<'a> ProjectionContext<'a> {
    pub(super) fn new(settings: Option<&'a Value>, execution: bool) -> Self {
        Self {
            settings,
            execution,
            remaining_rows: MAX_PROJECTION_ROWS,
            remaining_days: MAX_PROJECTION_ROWS,
            remaining_work: MAX_PROJECTION_ROWS,
        }
    }

    fn prepare(
        &mut self,
        source: &Source,
        range: (NaiveDate, NaiveDate),
        persisted: &[dose_occurrence::Model],
        pauses: &[pause_period::Model],
        takes: &[medication_take::Model],
        settings: Option<&Value>,
    ) -> Result<(), OperationError> {
        check_capacity(takes.len())?;
        check_capacity(pauses.len())?;
        let (first, last) = expected_range(source, range);
        let days = usize::try_from(((last - first).num_days() + 1).max(0))
            .map_err(|_| capacity_error())?;
        if days > self.remaining_days {
            return Err(capacity_error());
        }
        check_capacity(persisted.len())?;
        let mut identities = BTreeSet::new();
        for row in persisted.iter().filter(|row| {
            row.window_starts_on <= range.1
                && row.window_ends_on.unwrap_or(row.window_starts_on) >= range.0
        }) {
            reserve_identity(
                &mut identities,
                (row.window_starts_on, row.position),
                self.remaining_rows,
            )?;
        }
        let mut work = self.remaining_work;
        let mut day = first;
        while day <= last {
            match source {
                Source::Schedule(schedule)
                    if schedule_applies(schedule, day)
                        && !schedule_as_needed(schedule)
                        && (source.active() || !pauses.is_empty()) =>
                {
                    if let Some(config) = schedule_config_on(schedule, day) {
                        let times = if config.get("times").is_none() {
                            &schedule.schedule_config
                        } else {
                            config
                        }
                        .get("times")
                        .and_then(Value::as_array);
                        if let Some(times) = times {
                            check_capacity(times.len())?;
                            work = work.checked_sub(times.len()).ok_or_else(capacity_error)?;
                        }
                        let mut valid = false;
                        for (index, time) in times
                            .into_iter()
                            .flatten()
                            .filter_map(|value| {
                                value.as_str().and_then(|time| {
                                    calendar::scheduled_time_in_zone(
                                        day,
                                        time,
                                        calendar::current_zone(settings),
                                    )
                                })
                            })
                            .enumerate()
                        {
                            valid = true;
                            if !pauses.iter().any(|period| pause_covers(period, time)) {
                                let position =
                                    i32::try_from(index + 1).map_err(|_| capacity_error())?;
                                reserve_identity(
                                    &mut identities,
                                    (day, position),
                                    self.remaining_rows,
                                )?;
                            }
                        }
                        if !valid {
                            let count = schedule_count(schedule, config, self.execution)?;
                            work = work
                                .checked_sub(usize::try_from(count).map_err(|_| capacity_error())?)
                                .ok_or_else(capacity_error)?;
                            if !fully_paused(source, day, day, pauses, settings) {
                                for position in 1..=count {
                                    reserve_identity(
                                        &mut identities,
                                        (day, position),
                                        self.remaining_rows,
                                    )?;
                                }
                            }
                        }
                    }
                }
                Source::Assignment(assignment)
                    if assignment.administration_kind == 0
                        && (source.active() || !pauses.is_empty())
                        && cycle_bounds(day, source.dose_cycle()).0 == day =>
                {
                    let (_, finish) = cycle_bounds(day, source.dose_cycle());
                    if finish >= local_date(source.created_at(), settings) {
                        let count = source.max_daily_doses().max(1);
                        work = work
                            .checked_sub(usize::try_from(count).map_err(|_| capacity_error())?)
                            .ok_or_else(capacity_error)?;
                        if !fully_paused(source, day, finish, pauses, settings) {
                            for position in 1..=count {
                                reserve_identity(
                                    &mut identities,
                                    (day, position),
                                    self.remaining_rows,
                                )?;
                            }
                        }
                    }
                }
                _ => {}
            }
            day += Duration::days(1);
        }
        self.remaining_rows -= identities.len();
        self.remaining_days -= days;
        self.remaining_work = work;
        Ok(())
    }
}

fn expected_range(source: &Source, range: (NaiveDate, NaiveDate)) -> (NaiveDate, NaiveDate) {
    if let Source::Schedule(schedule) = source {
        match (schedule.start_date, schedule.end_date) {
            (Some(start), Some(end)) => (range.0.max(start), range.1.min(end)),
            _ => (NaiveDate::MAX, NaiveDate::MIN),
        }
    } else {
        range
    }
}

fn reserve_identity(
    identities: &mut BTreeSet<(NaiveDate, i32)>,
    identity: (NaiveDate, i32),
    limit: usize,
) -> Result<(), OperationError> {
    if identities.contains(&identity) {
        return Ok(());
    }
    if identities.len() >= limit {
        return Err(capacity_error());
    }
    identities.insert(identity);
    Ok(())
}

#[derive(Clone)]
pub(super) struct Occurrence {
    pub(super) window_start: NaiveDate,
    pub(super) window_end: NaiveDate,
    pub(super) position: i32,
    pub(super) scheduled_at: Option<NaiveDateTime>,
    pub(super) expected: bool,
    pub(super) record: Option<dose_occurrence::Model>,
    pub(super) legacy_take_id: Option<i64>,
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
    projected_in_mode(db, source, start, end, settings, false).await
}

pub(super) async fn projected_execution(
    db: &DatabaseTransaction,
    source: &Source,
    start: NaiveDate,
    end: NaiveDate,
    settings: Option<&serde_json::Value>,
) -> Result<Vec<Occurrence>, ApiError> {
    projected_in_mode(db, source, start, end, settings, true).await
}

async fn projected_in_mode(
    db: &DatabaseTransaction,
    source: &Source,
    start: NaiveDate,
    end: NaiveDate,
    settings: Option<&serde_json::Value>,
    execution: bool,
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
    let persisted = query
        .limit(PROJECTION_FETCH_LIMIT)
        .all(db)
        .await
        .map_err(database_error)?;
    let pauses = match source {
        Source::Schedule(row) => {
            pause_period::Entity::find()
                .filter(pause_period::Column::ScheduleId.eq(row.id))
                .limit(PROJECTION_FETCH_LIMIT)
                .all(db)
                .await
        }
        Source::Assignment(row) => {
            pause_period::Entity::find()
                .filter(pause_period::Column::PersonMedicationId.eq(row.id))
                .limit(PROJECTION_FETCH_LIMIT)
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
    let takes = takes
        .limit(PROJECTION_FETCH_LIMIT)
        .all(db)
        .await
        .map_err(database_error)?;
    projected_loaded_in_mode(
        source,
        (scan_start, scan_end),
        &persisted,
        &pauses,
        &takes,
        &mut ProjectionContext::new(settings, execution),
    )
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

#[cfg(test)]
pub(super) fn projected_loaded(
    source: &Source,
    scan_start: NaiveDate,
    scan_end: NaiveDate,
    persisted: &[dose_occurrence::Model],
    pauses: &[pause_period::Model],
    takes: &[medication_take::Model],
    settings: Option<&serde_json::Value>,
) -> Vec<Occurrence> {
    projected_loaded_in_mode(
        source,
        (scan_start, scan_end),
        persisted,
        pauses,
        takes,
        &mut ProjectionContext::new(settings, false),
    )
    .unwrap()
}

pub(super) fn projected_loaded_in_mode(
    source: &Source,
    range: (NaiveDate, NaiveDate),
    persisted: &[dose_occurrence::Model],
    pauses: &[pause_period::Model],
    takes: &[medication_take::Model],
    context: &mut ProjectionContext<'_>,
) -> Result<Vec<Occurrence>, OperationError> {
    let (scan_start, scan_end) = range;
    let source_settings = source.calendar_settings(context.settings);
    let settings = source_settings.as_ref();
    context.prepare(source, range, persisted, pauses, takes, settings)?;
    let mut rows: BTreeMap<(NaiveDate, i32), Occurrence> = BTreeMap::new();
    let (mut day, last) = expected_range(source, range);
    while day <= last {
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
                            let count = schedule_count(schedule, config, context.execution)?;
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
            },
        );
    }
    for take in takes {
        let Some(taken_at) = take.taken_at else {
            continue;
        };
        let day = local_date(taken_at, settings);
        let window = if source.kind() == Kind::Assignment {
            cycle_bounds(day, source.dose_cycle()).0
        } else {
            day
        };
        if let Some((_, row)) = rows.iter_mut().find(|((date, _), row)| {
            *date == window
                && row.expected
                && row
                    .record
                    .as_ref()
                    .is_none_or(|record| record.outcome == "open")
                && row.legacy_take_id.is_none()
        }) {
            row.legacy_take_id = Some(take.id);
        }
    }
    Ok(rows.into_values().collect())
}

fn schedule_count(
    schedule: &schedule::Model,
    config: &Value,
    execution: bool,
) -> Result<i32, OperationError> {
    let fallback = if execution {
        execution_count(schedule, config)?
    } else {
        schedule.max_daily_doses.unwrap_or(1)
    };
    let limit = i64::try_from(MAX_PROJECTION_ROWS).map_err(|_| capacity_error())?;
    for config in [&schedule.schedule_config, config] {
        for name in ["max_daily_doses", "max_doses", "max"] {
            if let Some(value) = config.get(name) {
                let count = value
                    .as_i64()
                    .or_else(|| value.as_str().and_then(|value| value.parse::<i64>().ok()));
                if count.is_some_and(|count| count > limit)
                    || value.is_number() && count.is_none()
                    || value.as_str().is_some_and(|value| {
                        !value.is_empty()
                            && value
                                .trim_start_matches('+')
                                .bytes()
                                .all(|byte| byte.is_ascii_digit())
                            && count.is_none()
                    })
                {
                    return Err(capacity_error());
                }
            }
        }
    }
    Ok(effective_count(
        config,
        effective_count(&schedule.schedule_config, fallback),
    ))
}

fn execution_count(schedule: &schedule::Model, config: &Value) -> Result<i32, OperationError> {
    if let Some(maximum) = schedule.max_daily_doses {
        return Ok(maximum);
    }
    let hours = doses::config_decimal(config, &["min_hours_between_doses", "min_hours"])
        .or_else(|| {
            doses::config_decimal(
                &schedule.schedule_config,
                &["min_hours_between_doses", "min_hours"],
            )
        })
        .or(schedule
            .min_hours_between_doses
            .map(sea_orm::prelude::Decimal::from));
    let Some(hours) = hours.filter(|hours| *hours > sea_orm::prelude::Decimal::ZERO) else {
        return Ok(1);
    };
    let count = sea_orm::prelude::Decimal::from(24)
        .checked_div(hours)
        .ok_or_else(capacity_error)?
        .ceil();
    let limit = sea_orm::prelude::Decimal::from(
        u64::try_from(MAX_PROJECTION_ROWS).map_err(|_| capacity_error())?,
    );
    if count > limit {
        return Err(capacity_error());
    }
    count.to_string().parse().map_err(|_| capacity_error())
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

    #[test]
    fn interval_execution_slots_preserve_report_projection_and_signed_identity() {
        let day = ymd(2026, 11, 16);
        let mut schedule = make_schedule(0, json!({}), day, day);
        schedule.min_hours_between_doses = Some(6);
        schedule.clinical_revision = Some(3);
        let source = Source::Schedule(schedule);
        let rows = projected_loaded(&source, day, day, &[], &[], &[], None);
        let secret: Arc<[u8]> = Arc::from(&b"medtracker-test-secret-32-bytes-long!"[..]);
        let row = super::representation::row_value(&secret, &source, &rows[0], None);
        assert_eq!(row.source_type, "schedule");
        assert_eq!(row.outcome, "open");
        let body = serde_json::to_value(row).unwrap();
        assert_eq!(body["window_starts_on"], "2026-11-16");
        assert!(body["medication_take_id"].is_null());
        assert!(body.get("prescribed_amount").is_none());
        let reports = projected_loaded(&source, day, day, &[], &[], &[], None);
        let execution = projected_loaded_in_mode(
            &source,
            (day, day),
            &[],
            &[],
            &[],
            &mut ProjectionContext::new(None, true),
        )
        .unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(execution.len(), 4);
        let secret = Arc::from("synthetic-interval-key-at-least-32-bytes".as_bytes());
        let second = key(
            &secret,
            &source,
            execution[1].window_start,
            execution[1].position,
        );
        assert_eq!(decode_key(&secret, &source, &second), Some((day, 2)));
        assert_eq!(source.max_daily_doses(), 1);
    }

    #[test]
    fn interval_execution_count_rejects_decimal_overflow_without_panicking() {
        let day = ymd(2026, 11, 16);
        let schedule = make_schedule(5, json!({}), day, day);
        let result = std::panic::catch_unwind(|| {
            execution_count(
                &schedule,
                &json!({"min_hours":"0.0000000000000000000000000001"}),
            )
        });
        assert!(
            result.is_ok(),
            "a valid tiny interval must not panic projection"
        );
        assert!(result.unwrap().is_err());
    }

    #[test]
    fn execution_projection_rejects_unsafe_counts_before_materialising_rows() {
        let day = ymd(2026, 11, 16);
        for config in [
            json!({"min_hours":"0.000001"}),
            json!({"max_daily_doses":10_001}),
            json!({"max_daily_doses":"999999999999999999999999999999"}),
        ] {
            let source = Source::Schedule(make_schedule(0, config, day, day));
            assert!(
                projected_loaded_in_mode(
                    &source,
                    (day, day),
                    &[],
                    &[],
                    &[],
                    &mut ProjectionContext::new(None, true)
                )
                .is_err()
            );
        }
    }

    #[test]
    fn report_budget_accepts_high_frequency_sources_and_rejects_an_incomplete_result() {
        let day = ymd(2026, 11, 24);
        let source = Source::Assignment(make_assignment(
            day.and_hms_opt(0, 0, 0).unwrap(),
            Some(0),
            Some(60),
        ));
        let mut context = ProjectionContext::new(None, false);
        context.remaining_rows = 60;
        let rows =
            projected_loaded_in_mode(&source, (day, day), &[], &[], &[], &mut context).unwrap();
        assert_eq!(rows.len(), 60);
        assert_eq!(context.remaining_rows, 0);
        assert!(
            projected_loaded_in_mode(&source, (day, day), &[], &[], &[], &mut context).is_err()
        );
    }

    #[test]
    fn expired_schedule_preserves_day_budget() {
        let day = ymd(2026, 11, 24);
        let source = Source::Schedule(make_schedule(0, json!({}), day, day));
        let tomorrow = day.succ_opt().unwrap();
        let mut context = ProjectionContext::new(None, false);
        context.remaining_days = 0;
        assert!(
            projected_loaded_in_mode(&source, (tomorrow, tomorrow), &[], &[], &[], &mut context)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn execution_projection_budget_is_shared_and_never_truncates() {
        let day = ymd(2026, 11, 16);
        let source = Source::Schedule(make_schedule(
            0,
            json!({"times":["00:00","12:00"]}),
            day,
            day,
        ));
        let mut context = ProjectionContext::new(None, true);
        context.remaining_rows = 3;
        context.remaining_days = 3;
        let rows =
            projected_loaded_in_mode(&source, (day, day), &[], &[], &[], &mut context).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(context.remaining_rows, 1);
        assert!(
            projected_loaded_in_mode(&source, (day, day), &[], &[], &[], &mut context).is_err()
        );
        assert_eq!(context.remaining_rows, 1);
        let source = Source::Schedule(make_schedule(4, json!({}), day, day + Duration::days(3)));
        assert!(
            projected_loaded_in_mode(
                &source,
                (day, day + Duration::days(3)),
                &[],
                &[],
                &[],
                &mut context
            )
            .is_err()
        );
    }

    #[test]
    fn persisted_decision_replaces_expected_identity_within_projection_budget() {
        let day = ymd(2026, 11, 16);
        let now = day.and_hms_opt(0, 0, 0).unwrap();
        let source = Source::Schedule(make_schedule(0, json!({"times":["00:00"]}), day, day));
        let mut record = dose_occurrence::Model {
            id: 1,
            household_id: 1,
            schedule_id: Some(1),
            person_medication_id: None,
            medication_take_id: None,
            resolved_by_membership_id: Some(1),
            portable_id: "record".into(),
            window_starts_on: day,
            window_ends_on: Some(day),
            position: 1,
            scheduled_at: Some(now),
            outcome: "not_taken".into(),
            reason: None,
            note: None,
            resolved_at: Some(now),
            created_at: now,
            updated_at: now,
        };
        let mut context = ProjectionContext::new(None, true);
        context.remaining_rows = 1;
        let rows = projected_loaded_in_mode(
            &source,
            (day, day),
            std::slice::from_ref(&record),
            &[],
            &[],
            &mut context,
        )
        .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].record.as_ref().unwrap().outcome, "not_taken");
        assert_eq!(context.remaining_rows, 0);
        let yesterday = day.pred_opt().unwrap();
        let expired = Source::Schedule(make_schedule(0, json!({}), yesterday, yesterday));
        let mut expired_context = ProjectionContext::new(None, false);
        expired_context.remaining_days = 0;
        let saved = projected_loaded_in_mode(
            &expired,
            (day, day),
            std::slice::from_ref(&record),
            &[],
            &[],
            &mut expired_context,
        )
        .unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].record.as_ref().unwrap().id, record.id);
        record.position = 2;
        let mut context = ProjectionContext::new(None, true);
        context.remaining_rows = 1;
        assert!(
            projected_loaded_in_mode(&source, (day, day), &[record], &[], &[], &mut context)
                .is_err()
        );
    }

    #[test]
    fn paused_candidates_consume_shared_work_budget_without_removing_doses() {
        let day = ymd(2026, 11, 16);
        let now = day.and_hms_opt(0, 0, 0).unwrap();
        let source = Source::Schedule(make_schedule(0, json!({"max_daily_doses":3}), day, day));
        let pause = pause_period::Model {
            id: 1,
            household_id: 1,
            schedule_id: Some(1),
            person_medication_id: None,
            portable_id: "pause".into(),
            reason: "other".into(),
            note: None,
            recorded_by_membership_id: Some(1),
            resumed_by_membership_id: None,
            started_at: Some(now),
            ended_at: None,
            created_at: now,
            updated_at: now,
            canonical_resume_id: None,
            canonical_person_schedule_id: None,
            canonical_pause_revision: None,
            imported_actor_references: json!({}),
            imported_context: false,
            legacy_context: false,
        };
        let mut context = ProjectionContext::new(None, true);
        context.remaining_work = 3;
        let rows = projected_loaded_in_mode(
            &source,
            (day, day),
            &[],
            std::slice::from_ref(&pause),
            &[],
            &mut context,
        )
        .unwrap();
        assert!(rows.is_empty());
        assert_eq!(context.remaining_work, 0);
        assert_eq!(context.remaining_rows, MAX_PROJECTION_ROWS);
        assert!(
            projected_loaded_in_mode(&source, (day, day), &[], &[pause], &[], &mut context)
                .is_err()
        );
    }

    #[tokio::test]
    async fn dashboard_source_dates_cover_opposite_date_line_timezones() {
        let day = ymd(2026, 11, 16);
        let mut schedule = make_schedule(
            0,
            json!({"times":["00:00","12:00","23:59"]}),
            day - Duration::days(3),
            day + Duration::days(3),
        );
        schedule.origin_time_zone = Some("Pacific/Kiritimati".into());
        let source = Source::Schedule(schedule);
        with_dashboard_timezone("Etc/GMT+12".parse().unwrap(), async {
            let range = dashboard_projection::scan_bounds(&source, day, day, true, None);
            assert_eq!(range, (day + Duration::days(1), day + Duration::days(2)));
            let rows = projected_loaded_in_mode(
                &source,
                range,
                &[],
                &[],
                &[],
                &mut ProjectionContext::new(None, true),
            )
            .unwrap();
            let visible: Vec<_> = rows
                .iter()
                .filter(|row| dashboard_projection::includes(&source, row, day, day, None))
                .collect();
            assert_eq!(visible.len(), 3);
        })
        .await;
    }

    #[tokio::test]
    async fn assignment_execution_key_retains_viewer_date_at_creation_boundary() {
        let created = ymd_hms(2026, 11, 16, 0, 30, 0);
        let day = ymd(2026, 11, 15);
        let source = Source::Assignment(make_assignment(created, Some(0), Some(1)));
        let settings = json!({"time_zone":"UTC"});
        with_dashboard_timezone(chrono_tz::Pacific::Honolulu, async {
            let rows = projected_loaded_in_mode(
                &source,
                (day, day),
                &[],
                &[],
                &[],
                &mut ProjectionContext::new(Some(&settings), true),
            )
            .unwrap();
            assert_eq!(rows.len(), 1);
            let secret = Arc::from("synthetic-boundary-key-at-least-32-bytes".as_bytes());
            let signed = key(&secret, &source, rows[0].window_start, rows[0].position);
            assert_eq!(decode_key(&secret, &source, &signed), Some((day, 1)));
            assert!(actionable(&source, &rows[0], Some(&settings)));
        })
        .await;
        assert!(
            projected_loaded_in_mode(
                &source,
                (day, day),
                &[],
                &[],
                &[],
                &mut ProjectionContext::new(Some(&settings), true)
            )
            .unwrap()
            .is_empty()
        );
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
