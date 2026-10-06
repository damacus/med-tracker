use super::*;

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
) -> bool {
    let mut cursor = local_midnight(start);
    let finish = local_midnight(end + Duration::days(1));
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
) -> Result<Vec<Occurrence>, ApiError> {
    let (scan_start, scan_end) = if source.kind() == Kind::Assignment {
        let (start, _) = cycle_bounds(start, source.dose_cycle());
        let (_, end) = cycle_bounds(end, source.dose_cycle());
        (start, end)
    } else {
        (start, end)
    };
    let mut query = dose_occurrence::Entity::find()
        .filter(dose_occurrence::Column::HouseholdId.eq(match source {
            Source::Schedule(row) => row.household_id,
            Source::Assignment(row) => row.household_id,
        }))
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
                    let times = config_times(&schedule.schedule_config, day);
                    if times.is_empty() {
                        if !fully_paused(source, day, day, &pauses) {
                            let count = effective_count(config, source.max_daily_doses());
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
                    if finish >= source.created_at().date()
                        && !fully_paused(source, day, finish, &pauses)
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
                record: Some(record),
                legacy_take_id: None,
            },
        );
    }
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
        .filter(medication_take::Column::TakenAt.gte(local_midnight(scan_start)))
        .filter(medication_take::Column::TakenAt.lt(local_midnight(scan_end + Duration::days(1))))
        .filter(medication_take::Column::Id.not_in_subquery(linked))
        .order_by_asc(medication_take::Column::TakenAt)
        .order_by_asc(medication_take::Column::Id);
    takes = match source {
        Source::Schedule(row) => takes.filter(medication_take::Column::ScheduleId.eq(row.id)),
        Source::Assignment(row) => {
            takes.filter(medication_take::Column::PersonMedicationId.eq(row.id))
        }
    };
    let takes = takes.all(db).await.map_err(database_error)?;
    for take in takes {
        let Some(taken_at) = take.taken_at else {
            continue;
        };
        let day = local_date(taken_at);
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
