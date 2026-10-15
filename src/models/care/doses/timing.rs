use super::*;
use crate::models::care::take_history::{self, TimingFacts, TimingWindow};
use loco_rs::model::TenantQueryExt;
use std::collections::{BTreeMap, BTreeSet, HashMap};

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
    let midnight = date
        .and_hms_opt(0, 0, 0)
        .ok_or(OperationError::Unavailable)?;
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

/// # Errors
/// Returns an error when local calendar boundaries cannot resolve to UTC.
pub(crate) fn cycle_bounds_in_zone(
    time: NaiveDateTime,
    cycle: Option<i32>,
    zone: chrono_tz::Tz,
) -> Result<(NaiveDateTime, NaiveDateTime), ApiError> {
    let date = local_date_in_zone(time, zone);
    let start = match cycle.unwrap_or(0) {
        1 => date - Duration::days(i64::from(date.weekday().num_days_from_monday())),
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
    proposed: &TakeCandidate,
) -> Result<bool, ApiError> {
    let assignments = person_medication::Entity::find()
        .in_tenant(proposed.selected.household_id)
        .filter(person_medication::Column::PersonId.eq(proposed.source.person_id.get()))
        .filter(person_medication::Column::MedicationId.eq(proposed.source.medication_id.get()))
        .filter(person_medication::Column::Active.eq(true))
        .filter(person_medication::Column::RetiredAt.is_null())
        .all(db)
        .await
        .map_err(database_error)?;
    let schedules = schedule::Entity::find()
        .in_tenant(proposed.selected.household_id)
        .filter(schedule::Column::PersonId.eq(proposed.source.person_id.get()))
        .filter(schedule::Column::MedicationId.eq(proposed.source.medication_id.get()))
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
                    applies_on(source, local_date_in_zone(proposed.taken_at, proposed.zone))
                }),
        )
        .collect();
    let rules = timing_rules(related, proposed.taken_at, proposed.zone)?;
    let windows = unique_windows(rules.iter());
    let facts = take_history::timing_facts(
        db,
        HouseholdId::from_database(proposed.selected.household_id),
        &windows,
        proposed.taken_at,
    )
    .await?;
    Ok(timing_block(&rules, &facts, proposed.taken_at)?.is_none())
}

struct TimingRule {
    window: TimingWindow,
    maximum: Option<u64>,
    minimum_hours: Option<Decimal>,
}

fn timing_rules(
    sources: Vec<Source>,
    taken_at: NaiveDateTime,
    zone: chrono_tz::Tz,
) -> Result<Vec<TimingRule>, ApiError> {
    let mut rules = Vec::new();
    for mut source in sources {
        effective_source(&mut source, local_date_in_zone(taken_at, zone));
        let minimum_hours = source
            .min_hours_between_doses
            .filter(|hours| *hours > Decimal::ZERO);
        if source.max_daily_doses.is_none() && minimum_hours.is_none() {
            continue;
        }
        let (start, end) = cycle_bounds(taken_at, source.dose_cycle, zone)?;
        rules.push(TimingRule {
            window: TimingWindow {
                person_id: source.person_id,
                medication_id: source.medication_id,
                start,
                end,
            },
            maximum: source
                .max_daily_doses
                .map(|maximum| u64::try_from(maximum.max(0)))
                .transpose()
                .map_err(|_| OperationError::Unavailable)?,
            minimum_hours,
        });
    }
    Ok(rules)
}

fn unique_windows<'a>(rules: impl Iterator<Item = &'a TimingRule>) -> Vec<TimingWindow> {
    rules
        .map(|rule| rule.window)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn timing_block(
    rules: &[TimingRule],
    facts: &HashMap<TimingWindow, TimingFacts>,
    taken_at: NaiveDateTime,
) -> Result<Option<&'static str>, ApiError> {
    for rule in rules {
        let facts = facts.get(&rule.window).ok_or(OperationError::Unavailable)?;
        if rule
            .maximum
            .is_some_and(|maximum| facts.cycle_count >= maximum)
        {
            return Ok(Some("max_reached"));
        }
        if let (Some(hours), Some(recent)) = (rule.minimum_hours, facts.latest_at) {
            let elapsed = Decimal::from((taken_at - recent).num_seconds());
            if elapsed < hours * Decimal::from(3600) {
                return Ok(Some("cooldown"));
            }
        }
    }
    Ok(None)
}

pub(crate) async fn dashboard_timing(
    tenant: &TenantTransaction,
    schedules: &[schedule::Model],
    assignments: &[person_medication::Model],
    now: DateTime<Utc>,
    zone: chrono_tz::Tz,
) -> Result<HashMap<(i64, i64), &'static str>, ApiError> {
    let sources = assignments
        .iter()
        .cloned()
        .map(source_from_assignment)
        .chain(schedules.iter().cloned().map(source_from_schedule))
        .filter(|source| {
            source.active
                && !source.retired
                && applies_on(source, now.with_timezone(&zone).date_naive())
        });
    let mut groups = BTreeMap::<_, Vec<Source>>::new();
    for source in sources {
        groups
            .entry((source.person_id, source.medication_id))
            .or_default()
            .push(source);
    }
    let mut rules = BTreeMap::new();
    for (key, sources) in groups {
        rules.insert(key, timing_rules(sources, now.naive_utc(), zone)?);
    }
    let windows = unique_windows(rules.values().flat_map(|rules| rules.iter()));
    let facts = take_history::timing_facts(
        tenant.transaction(),
        HouseholdId::from_database(tenant.scope().household_id),
        &windows,
        now.naive_utc(),
    )
    .await?;
    let mut result = HashMap::new();
    for ((person, medication), rules) in rules {
        if let Some(reason) = timing_block(&rules, &facts, now.naive_utc())? {
            result.insert((person.get(), medication.get()), reason);
        }
    }
    Ok(result)
}
