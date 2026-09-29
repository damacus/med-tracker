use crate::audit;
use crate::dosage_options::{parse_decimal, valid_identifier};
use crate::dose;
use crate::entities::{
    dose_occurrence, grant, medication_take, pause_period, person, person_medication, schedule,
};
use crate::medication_management::{
    error_response, finish_with_request_id, record_version, request_context,
};
use crate::mutation_idempotency::{self, Lookup, StoredResponse};
use crate::sync_batch::{SyncOperation, SyncResult};
use crate::sync_events::{record_change, SyncRecord};
use crate::{database_error, representation_etag, ApiError, AppState, AuthContext};
use axum::extract::{
    rejection::{JsonRejection, QueryRejection},
    Path, Query, State,
};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc};
use hmac::{Hmac, Mac};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseTransaction, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, QueryTrait, Set,
};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use sha2::Sha256;
use std::collections::BTreeMap;
use std::sync::Arc;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;
const REASONS: &[&str] = &[
    "refused",
    "unwell",
    "asleep",
    "medicine_unavailable",
    "clinician_advice",
    "other",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Schedule,
    Assignment,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Self::Schedule => "schedule",
            Self::Assignment => "person_medication",
        }
    }

    fn controller(self) -> &'static str {
        match self {
            Self::Schedule => "api/v1/schedules",
            Self::Assignment => "api/v1/person_medications",
        }
    }

    fn policy(self) -> &'static str {
        match self {
            Self::Schedule => "SchedulePolicy",
            Self::Assignment => "PersonMedicationPolicy",
        }
    }

    fn path_segment(self) -> &'static str {
        match self {
            Self::Schedule => "schedules",
            Self::Assignment => "person_medications",
        }
    }
}

enum Source {
    Schedule(schedule::Model),
    Assignment(person_medication::Model),
}

impl Source {
    fn kind(&self) -> Kind {
        match self {
            Self::Schedule(_) => Kind::Schedule,
            Self::Assignment(_) => Kind::Assignment,
        }
    }

    fn id(&self) -> i64 {
        match self {
            Self::Schedule(row) => row.id,
            Self::Assignment(row) => row.id,
        }
    }

    fn person_id(&self) -> i64 {
        match self {
            Self::Schedule(row) => row.person_id,
            Self::Assignment(row) => row.person_id,
        }
    }

    fn portable_id(&self) -> &str {
        match self {
            Self::Schedule(row) => &row.portable_id,
            Self::Assignment(row) => &row.portable_id,
        }
    }

    fn active(&self) -> bool {
        match self {
            Self::Schedule(row) => row.active,
            Self::Assignment(row) => row.active,
        }
    }

    fn created_at(&self) -> NaiveDateTime {
        match self {
            Self::Schedule(row) => row.created_at,
            Self::Assignment(row) => row.created_at,
        }
    }

    fn max_daily_doses(&self) -> i32 {
        match self {
            Self::Schedule(row) => row.max_daily_doses.unwrap_or(1),
            Self::Assignment(row) => row.max_daily_doses.unwrap_or(1),
        }
    }

    fn dose_cycle(&self) -> i32 {
        match self {
            Self::Schedule(row) => row.dose_cycle.unwrap_or(0),
            Self::Assignment(row) => row.dose_cycle.unwrap_or(0),
        }
    }
}

#[derive(Clone)]
struct Occurrence {
    window_start: NaiveDate,
    window_end: NaiveDate,
    position: i32,
    scheduled_at: Option<NaiveDateTime>,
    expected: bool,
    record: Option<dose_occurrence::Model>,
    legacy_take_id: Option<i64>,
}

#[derive(Deserialize)]
pub(super) struct RangeQuery {
    start_date: Option<String>,
    end_date: Option<String>,
}

#[derive(Clone, Copy)]
enum Failure {
    Malformed,
    Invalid(&'static str, &'static str),
    NotFound,
    Forbidden,
    AlreadyResolved,
    InvalidOccurrence,
    PreconditionRequired,
    SyncConflict,
    KeyConflict,
}

fn date(value: &str) -> Option<NaiveDate> {
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

fn timestamp(value: NaiveDateTime) -> String {
    value
        .and_utc()
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn zone() -> chrono_tz::Tz {
    std::env::var("TZ")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(chrono_tz::UTC)
}

async fn person_access(
    db: &DatabaseTransaction,
    context: &AuthContext,
    person_id: i64,
    action: &str,
) -> Result<bool, ApiError> {
    let levels: &[&str] = match action {
        "reopen" => &["manage"],
        "not_taken" | "take" => &["record", "manage"],
        _ => &["view", "record", "manage"],
    };
    Ok(grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(context.membership.household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(context.membership.id))
        .filter(grant::Column::PersonId.eq(person_id))
        .filter(grant::Column::AccessLevel.is_in(levels.to_vec()))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(
            Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(grant::Column::ExpiresAt.gt(Utc::now().naive_utc())),
        )
        .one(db)
        .await
        .map_err(database_error)?
        .is_some())
}

async fn find_source(
    db: &DatabaseTransaction,
    context: &AuthContext,
    kind: Kind,
    id: &str,
) -> Result<Option<Source>, ApiError> {
    if !valid_identifier(id) {
        return Ok(None);
    }
    let household_id = context.membership.household_id;
    let source = match kind {
        Kind::Schedule => {
            let query = schedule::Entity::find()
                .filter(schedule::Column::HouseholdId.eq(household_id))
                .filter(schedule::Column::RetiredAt.is_null());
            let query = match id.parse::<i64>() {
                Ok(id) => query.filter(schedule::Column::Id.eq(id)),
                Err(_) => query.filter(schedule::Column::PortableId.eq(id)),
            };
            query
                .one(db)
                .await
                .map_err(database_error)?
                .map(Source::Schedule)
        }
        Kind::Assignment => {
            let query = person_medication::Entity::find()
                .filter(person_medication::Column::HouseholdId.eq(household_id))
                .filter(person_medication::Column::RetiredAt.is_null());
            let query = match id.parse::<i64>() {
                Ok(id) => query.filter(person_medication::Column::Id.eq(id)),
                Err(_) => query.filter(person_medication::Column::PortableId.eq(id)),
            };
            query
                .one(db)
                .await
                .map_err(database_error)?
                .map(Source::Assignment)
        }
    };
    match source {
        Some(source) if person_access(db, context, source.person_id(), "index").await? => {
            Ok(Some(source))
        }
        _ => Ok(None),
    }
}

fn key(secret: &Arc<[u8]>, source: &Source, window_start: NaiveDate, position: i32) -> String {
    let payload = json!([
        source.kind().name(),
        source.portable_id(),
        window_start.to_string(),
        position
    ]);
    let encoded = URL_SAFE_NO_PAD.encode(payload.to_string());
    let mut mac = HmacSha256::new_from_slice(secret).expect("validated HMAC secret");
    mac.update(b"medtracker-dose-occurrence-v1\0");
    mac.update(encoded.as_bytes());
    let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
    format!("{encoded}.{signature}")
}

fn decode_key(secret: &Arc<[u8]>, source: &Source, value: &str) -> Option<(NaiveDate, i32)> {
    if value.len() > 1024 {
        return None;
    }
    let (encoded, signature) = value.split_once('.')?;
    let signature = URL_SAFE_NO_PAD.decode(signature).ok()?;
    let mut mac = HmacSha256::new_from_slice(secret).ok()?;
    mac.update(b"medtracker-dose-occurrence-v1\0");
    mac.update(encoded.as_bytes());
    mac.verify_slice(&signature).ok()?;
    let payload: Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(encoded).ok()?).ok()?;
    let values = payload.as_array()?;
    if values.len() != 4 || values[0] != source.kind().name() || values[1] != source.portable_id() {
        return None;
    }
    let date = date(values[2].as_str()?)?;
    let position = values[3]
        .as_i64()
        .and_then(|value| i32::try_from(value).ok())?;
    (position > 0).then_some((date, position))
}

fn cycle_bounds(value: NaiveDate, cycle: i32) -> (NaiveDate, NaiveDate) {
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

fn scheduled_time_in_zone(
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

fn local_midnight(date: NaiveDate) -> NaiveDateTime {
    zone()
        .from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
        .earliest()
        .map(|value| value.with_timezone(&Utc).naive_utc())
        .unwrap_or_else(|| date.and_hms_opt(0, 0, 0).unwrap())
}

fn config_times(config: &Value, date: NaiveDate) -> Vec<NaiveDateTime> {
    config
        .get("times")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().and_then(|value| scheduled_time(date, value)))
        .collect()
}

fn schedule_config_on(schedule: &schedule::Model, day: NaiveDate) -> Option<&Value> {
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

fn schedule_applies(schedule: &schedule::Model, day: NaiveDate) -> bool {
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

fn schedule_as_needed(schedule: &schedule::Model) -> bool {
    schedule.schedule_type == 4
        || schedule
            .frequency
            .as_deref()
            .is_some_and(|value| value.eq_ignore_ascii_case("as needed"))
        || schedule.schedule_config.get("as_needed") == Some(&Value::Bool(true))
}

fn effective_count(config: &Value, fallback: i32) -> i32 {
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

async fn projected(
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
                {
                    if let Some(config) = schedule_config_on(schedule, day) {
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
        let day = dose::local_date(taken_at);
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

pub(super) fn record_etag(record: &dose_occurrence::Model) -> String {
    representation_etag(&json!([
        "MedicationDoseOccurrence",
        record.id,
        record.updated_at.and_utc().timestamp_micros(),
    ]))
}

fn row_value(secret: &Arc<[u8]>, source: &Source, row: &Occurrence) -> Value {
    let (outcome, reason, note, resolved_at, medication_take_id, etag) = match &row.record {
        Some(record) if row.legacy_take_id.is_some() => (
            "taken",
            record.reason.as_deref(),
            record.note.as_deref(),
            record.resolved_at.map(timestamp),
            row.legacy_take_id,
            Some(record_etag(record)),
        ),
        Some(record) => (
            record.outcome.as_str(),
            record.reason.as_deref(),
            record.note.as_deref(),
            record.resolved_at.map(timestamp),
            record.medication_take_id.or(row.legacy_take_id),
            Some(record_etag(record)),
        ),
        None if row.legacy_take_id.is_some() => {
            ("taken", None, None, None, row.legacy_take_id, None)
        }
        None => ("open", None, None, None, None, None),
    };
    let due_time = row
        .scheduled_at
        .unwrap_or_else(|| local_midnight(row.window_start));
    let due = due_time <= Utc::now().naive_utc()
        && (source.kind() == Kind::Schedule || source.created_at() <= Utc::now().naive_utc());
    json!({
        "key": key(secret, source, row.window_start, row.position),
        "source_type": source.kind().name(),
        "source_id": source.id(),
        "source_portable_id": source.portable_id(),
        "window_starts_on": row.window_start.to_string(),
        "window_ends_on": row.window_end.to_string(),
        "position": row.position,
        "scheduled_at": row.scheduled_at.map(timestamp),
        "outcome": outcome,
        "expected": row.expected,
        "due": due,
        "reason": reason,
        "note": note,
        "resolved_at": resolved_at,
        "medication_take_id": medication_take_id,
        "etag": etag,
    })
}

fn snapshot(record: &dose_occurrence::Model) -> Value {
    json!({
        "household_id": record.household_id,
        "portable_id": record.portable_id,
        "schedule_id": record.schedule_id,
        "person_medication_id": record.person_medication_id,
        "medication_take_id": record.medication_take_id,
        "resolved_by_membership_id": record.resolved_by_membership_id,
        "window_starts_on": record.window_starts_on.to_string(),
        "window_ends_on": record.window_ends_on.map(|value| value.to_string()),
        "position": record.position,
        "scheduled_at": record.scheduled_at.map(timestamp),
        "outcome": record.outcome,
        "reason": record.reason,
        "note": record.note,
        "resolved_at": record.resolved_at.map(timestamp),
    })
}

async fn record_transition(
    db: &DatabaseTransaction,
    context: &AuthContext,
    source: &Source,
    record: &dose_occurrence::Model,
    before: Option<Value>,
    request_id: &str,
) -> Result<(), ApiError> {
    let action = if before.is_some() { "update" } else { "create" };
    record_version(
        db,
        context,
        request_id,
        "MedicationDoseOccurrence",
        record.id,
        action,
        before,
        Some(snapshot(record)),
    )
    .await?;
    let person_portable_id = person::Entity::find_by_id(source.person_id())
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?
        .portable_id;
    record_change(
        db,
        context,
        request_id,
        SyncRecord {
            record_type: "MedicationDoseOccurrence",
            record_id: record.id,
            portable_id: &record.portable_id,
            action,
            person_portable_id: Some(&person_portable_id),
        },
    )
    .await?;
    Ok(())
}

async fn fail(
    db: DatabaseTransaction,
    context: &AuthContext,
    kind: Kind,
    method: &str,
    action: &str,
    failure: Failure,
) -> Result<Response, ApiError> {
    let (status, code, message, errors) = match failure {
        Failure::Malformed => (
            StatusCode::BAD_REQUEST,
            "bad_request",
            "Invalid request body",
            None,
        ),
        Failure::Invalid(field, message) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            "Validation failed",
            Some(json!({field: [message]})),
        ),
        Failure::NotFound => (StatusCode::NOT_FOUND, "not_found", "Record not found", None),
        Failure::Forbidden => (
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
            None,
        ),
        Failure::AlreadyResolved => (
            StatusCode::CONFLICT,
            "already_resolved",
            "Occurrence is already resolved",
            None,
        ),
        Failure::InvalidOccurrence => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_occurrence",
            "Occurrence is unavailable",
            None,
        ),
        Failure::PreconditionRequired => (
            StatusCode::PRECONDITION_REQUIRED,
            "precondition_required",
            "A current version is required",
            None,
        ),
        Failure::SyncConflict => (
            StatusCode::CONFLICT,
            "sync_conflict",
            "Occurrence has changed",
            None,
        ),
        Failure::KeyConflict => (
            StatusCode::CONFLICT,
            "idempotency_key_reused",
            "Idempotency key has already been used for a different request",
            None,
        ),
    };
    error_response(
        db,
        context,
        method,
        kind.controller(),
        kind.policy(),
        action,
        status,
        code,
        message,
        errors,
    )
    .await
}

async fn fail_api(
    db: DatabaseTransaction,
    context: &AuthContext,
    kind: Kind,
    method: &str,
    action: &str,
    error: ApiError,
) -> Result<Response, ApiError> {
    error_response(
        db,
        context,
        method,
        kind.controller(),
        kind.policy(),
        action,
        error.status,
        error.code,
        error.message,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn finish(
    db: DatabaseTransaction,
    context: &AuthContext,
    kind: Kind,
    method: &str,
    action: &str,
    body: Value,
    etag: Option<&str>,
    request_id: &str,
) -> Result<Response, ApiError> {
    finish_with_request_id(
        db,
        context,
        request_id,
        method,
        kind.controller(),
        kind.policy(),
        action,
        StatusCode::OK,
        true,
        body,
        etag,
    )
    .await
}

enum Replay {
    New,
    Saved(Response),
    Conflict,
}

#[allow(clippy::too_many_arguments)]
async fn keyed_replay(
    db: &DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    kind: Kind,
    method: &str,
    action: &str,
    path: &str,
    body: &Value,
) -> Result<Replay, ApiError> {
    let Some(key) = mutation_idempotency::key(headers) else {
        return Ok(Replay::New);
    };
    let digest = mutation_idempotency::digest(method, path, body);
    match mutation_idempotency::lookup(db, context, key, method, path, &digest).await? {
        Lookup::New => Ok(Replay::New),
        Lookup::Conflict => Ok(Replay::Conflict),
        Lookup::Replay(saved) => {
            let request_id = Uuid::new_v4().to_string();
            let status = StatusCode::from_u16(saved.response_status as u16)
                .map_err(|_| ApiError::internal())?;
            audit::record_resource_request_with_id(
                db,
                context,
                &request_id,
                method,
                kind.controller(),
                kind.policy(),
                action,
                status,
                status.is_success(),
            )
            .await
            .map_err(database_error)?;
            let mut response = mutation_idempotency::replay(*saved)?;
            response.headers_mut().insert(
                "x-request-id",
                request_id.parse().map_err(|_| ApiError::internal())?,
            );
            Ok(Replay::Saved(response))
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn store_key(
    db: &DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    method: &str,
    path: &str,
    request: &Value,
    body: &Value,
    request_id: &str,
    etag: &str,
) -> Result<(), ApiError> {
    let Some(key) = mutation_idempotency::key(headers) else {
        return Ok(());
    };
    let digest = mutation_idempotency::digest(method, path, request);
    mutation_idempotency::store(
        db,
        context,
        StoredResponse {
            key,
            method,
            path,
            digest: &digest,
            status: StatusCode::OK,
            body: body.clone(),
            request_id,
            etag: Some(etag),
        },
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn fail_mutation(
    db: DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    kind: Kind,
    method: &str,
    action: &str,
    path: &str,
    request: &Value,
    failure: Failure,
) -> Result<Response, ApiError> {
    let (status, code, message, errors) = match failure {
        Failure::Malformed => (
            StatusCode::BAD_REQUEST,
            "bad_request",
            "Invalid request body",
            None,
        ),
        Failure::Invalid(field, message) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            "Validation failed",
            Some(json!({field: [message]})),
        ),
        Failure::InvalidOccurrence => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_occurrence",
            "Occurrence is unavailable",
            None,
        ),
        Failure::PreconditionRequired => (
            StatusCode::PRECONDITION_REQUIRED,
            "precondition_required",
            "A current version is required",
            None,
        ),
        _ => return fail(db, context, kind, method, action, failure).await,
    };
    finish_cached_error(
        db, context, headers, kind, method, action, path, request, status, code, message, errors,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn finish_cached_error(
    db: DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    kind: Kind,
    method: &str,
    action: &str,
    path: &str,
    request: &Value,
    status: StatusCode,
    code: &str,
    message: &str,
    errors: Option<Value>,
) -> Result<Response, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let mut body = json!({"error": {"code": code, "message": message}});
    if let Some(errors) = errors {
        body["error"]["errors"] = errors;
    }
    let mut stored = body.clone();
    stored["error"]["request_id"] = json!(request_id);
    if let Some(key) = mutation_idempotency::key(headers) {
        let digest = mutation_idempotency::digest(method, path, request);
        mutation_idempotency::store(
            &db,
            context,
            StoredResponse {
                key,
                method,
                path,
                digest: &digest,
                status,
                body: stored,
                request_id: &request_id,
                etag: None,
            },
        )
        .await?;
    }
    finish_with_request_id(
        db,
        context,
        &request_id,
        method,
        kind.controller(),
        kind.policy(),
        action,
        status,
        false,
        body,
        None,
    )
    .await
}

fn attributes<'a>(body: &'a Value, action: &str) -> Result<&'a Map<String, Value>, Failure> {
    let outer = body.as_object().ok_or(Failure::Malformed)?;
    let inner = outer
        .get("dose_occurrence")
        .and_then(Value::as_object)
        .ok_or(Failure::Malformed)?;
    if outer.len() != 1 {
        return Err(Failure::Invalid(
            "dose_occurrence",
            "contains an unsupported field",
        ));
    }
    let fields: &[&str] = match action {
        "not_taken" => &["key", "reason", "note"],
        "reopen" => &["key"],
        _ => &[
            "key",
            "taken_at",
            "client_uuid",
            "dose_amount",
            "taken_from_medication_id",
        ],
    };
    if inner.keys().any(|key| !fields.contains(&key.as_str())) {
        return Err(Failure::Invalid(
            "dose_occurrence",
            "contains an unsupported field",
        ));
    }
    if inner
        .get("key")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
    {
        return Err(Failure::Invalid("key", "must be a string"));
    }
    Ok(inner)
}

fn parse_not_taken(
    attributes: &Map<String, Value>,
) -> Result<(Option<String>, Option<String>), Failure> {
    let reason = match attributes.get("reason") {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) if REASONS.contains(&value.as_str()) => Some(value.clone()),
        _ => return Err(Failure::Invalid("reason", "is invalid")),
    };
    let note = match attributes.get("note") {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) if value.chars().count() <= 2000 => Some(value.clone()),
        _ => return Err(Failure::Invalid("note", "is invalid")),
    };
    Ok((reason, note))
}

fn parse_take(attributes: &Map<String, Value>) -> Result<NaiveDateTime, Failure> {
    let value = attributes
        .get("taken_at")
        .and_then(Value::as_str)
        .ok_or(Failure::Invalid("taken_at", "must be ISO8601"))?;
    let taken_at = chrono::DateTime::parse_from_rfc3339(value)
        .map_err(|_| Failure::Invalid("taken_at", "must be ISO8601"))?
        .naive_utc();
    if let Some(value) = attributes.get("client_uuid") {
        if value
            .as_str()
            .is_none_or(|value| Uuid::parse_str(value).is_err())
        {
            return Err(Failure::Invalid("client_uuid", "must be a UUID"));
        }
    }
    if let Some(value) = attributes.get("dose_amount") {
        if parse_decimal(value).is_none_or(|amount| amount <= sea_orm::prelude::Decimal::ZERO) {
            return Err(Failure::Invalid("dose_amount", "must be a decimal string"));
        }
    }
    if let Some(value) = attributes.get("taken_from_medication_id") {
        if value.as_i64().is_none_or(|value| value <= 0) {
            return Err(Failure::Invalid(
                "taken_from_medication_id",
                "must be a positive integer",
            ));
        }
    }
    Ok(taken_at)
}

async fn list(
    state: AppState,
    household_id: i64,
    id: String,
    headers: HeaderMap,
    query: Result<Query<RangeQuery>, QueryRejection>,
    kind: Kind,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let Some(source) = find_source(&db, &context, kind, &id).await? else {
        return fail(db, &context, kind, "GET", "index", Failure::NotFound).await;
    };
    let Query(query) = match query {
        Ok(query) => query,
        Err(_) => {
            return fail(
                db,
                &context,
                kind,
                "GET",
                "index",
                Failure::Invalid("date_range", "is invalid"),
            )
            .await
        }
    };
    let dates = query
        .start_date
        .as_deref()
        .and_then(date)
        .zip(query.end_date.as_deref().and_then(date));
    let Some((start, end)) =
        dates.filter(|(start, end)| end >= start && (*end - *start).num_days() <= 30)
    else {
        return fail(
            db,
            &context,
            kind,
            "GET",
            "index",
            Failure::Invalid("date_range", "must contain at most 31 inclusive days"),
        )
        .await;
    };
    let secret = state.oauth.occurrence_key_secret();
    let rows = projected(&db, &source, start, end).await?;
    let data: Vec<Value> = rows
        .iter()
        .map(|row| row_value(&secret, &source, row))
        .collect();
    let request_id = Uuid::new_v4().to_string();
    finish(
        db,
        &context,
        kind,
        "GET",
        "index",
        json!({"data": data}),
        None,
        &request_id,
    )
    .await
}

async fn find_row(
    db: &DatabaseTransaction,
    secret: &Arc<[u8]>,
    source: &Source,
    key_value: &str,
) -> Result<Option<Occurrence>, ApiError> {
    let Some((date, position)) = decode_key(secret, source, key_value) else {
        return Ok(None);
    };
    Ok(projected(db, source, date, date)
        .await?
        .into_iter()
        .find(|row| row.window_start == date && row.position == position))
}

fn actionable(source: &Source, row: &Occurrence) -> bool {
    let due_time = row
        .scheduled_at
        .unwrap_or_else(|| local_midnight(row.window_start));
    row.expected
        && row.legacy_take_id.is_none()
        && due_time <= Utc::now().naive_utc()
        && (source.kind() == Kind::Schedule || source.created_at() <= Utc::now().naive_utc())
}

async fn save_decision(
    db: &DatabaseTransaction,
    context: &AuthContext,
    source: &Source,
    row: &Occurrence,
    reason: Option<String>,
    note: Option<String>,
    request_id: &str,
) -> Result<dose_occurrence::Model, ApiError> {
    let now = Utc::now().naive_utc();
    let before = row.record.as_ref().map(snapshot);
    let model = if let Some(record) = &row.record {
        let mut model: dose_occurrence::ActiveModel = record.clone().into();
        model.outcome = Set("not_taken".to_owned());
        model.reason = Set(reason);
        model.note = Set(note);
        model.resolved_at = Set(Some(now));
        model.resolved_by_membership_id = Set(Some(context.membership.id));
        model.updated_at = Set(now);
        model.update(db).await.map_err(database_error)?
    } else {
        dose_occurrence::ActiveModel {
            household_id: Set(context.membership.household_id),
            portable_id: Set(Uuid::new_v4().to_string()),
            schedule_id: Set((source.kind() == Kind::Schedule).then_some(source.id())),
            person_medication_id: Set((source.kind() == Kind::Assignment).then_some(source.id())),
            medication_take_id: Set(None),
            resolved_by_membership_id: Set(Some(context.membership.id)),
            window_starts_on: Set(row.window_start),
            window_ends_on: Set(Some(row.window_end)),
            position: Set(row.position),
            scheduled_at: Set(row.scheduled_at),
            outcome: Set("not_taken".to_owned()),
            reason: Set(reason),
            note: Set(note),
            resolved_at: Set(Some(now)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(database_error)?
    };
    record_transition(db, context, source, &model, before, request_id).await?;
    Ok(model)
}

async fn reopen_decision(
    db: &DatabaseTransaction,
    context: &AuthContext,
    source: &Source,
    record: &dose_occurrence::Model,
    request_id: &str,
) -> Result<dose_occurrence::Model, ApiError> {
    let before = snapshot(record);
    let mut model: dose_occurrence::ActiveModel = record.clone().into();
    model.outcome = Set("open".to_owned());
    model.reason = Set(None);
    model.note = Set(None);
    model.resolved_at = Set(None);
    model.resolved_by_membership_id = Set(None);
    model.updated_at = Set(Utc::now().naive_utc());
    let model = model.update(db).await.map_err(database_error)?;
    record_transition(db, context, source, &model, Some(before), request_id).await?;
    Ok(model)
}

async fn link_take(
    db: &DatabaseTransaction,
    context: &AuthContext,
    source: &Source,
    row: &Occurrence,
    take: &medication_take::Model,
    request_id: &str,
) -> Result<dose_occurrence::Model, ApiError> {
    let now = Utc::now().naive_utc();
    let before = row.record.as_ref().map(snapshot);
    let model = if let Some(record) = &row.record {
        let mut model: dose_occurrence::ActiveModel = record.clone().into();
        model.outcome = Set("taken".to_owned());
        model.medication_take_id = Set(Some(take.id));
        model.reason = Set(None);
        model.note = Set(None);
        model.resolved_at = Set(Some(now));
        model.resolved_by_membership_id = Set(Some(context.membership.id));
        model.updated_at = Set(now);
        model.update(db).await.map_err(database_error)?
    } else {
        dose_occurrence::ActiveModel {
            household_id: Set(context.membership.household_id),
            portable_id: Set(Uuid::new_v4().to_string()),
            schedule_id: Set((source.kind() == Kind::Schedule).then_some(source.id())),
            person_medication_id: Set((source.kind() == Kind::Assignment).then_some(source.id())),
            medication_take_id: Set(Some(take.id)),
            resolved_by_membership_id: Set(Some(context.membership.id)),
            window_starts_on: Set(row.window_start),
            window_ends_on: Set(Some(row.window_end)),
            position: Set(row.position),
            scheduled_at: Set(row.scheduled_at),
            outcome: Set("taken".to_owned()),
            reason: Set(None),
            note: Set(None),
            resolved_at: Set(Some(now)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(database_error)?
    };
    record_transition(db, context, source, &model, before, request_id).await?;
    Ok(model)
}

#[allow(clippy::too_many_arguments)]
async fn mutate(
    state: AppState,
    household_id: i64,
    id: String,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
    kind: Kind,
    action: &'static str,
    method: &'static str,
) -> Result<Response, ApiError> {
    let (db, _) = request_context(&state, &headers, household_id).await?;
    let (_, context) = mutation_idempotency::lock_household_and_reauthenticate(
        &state,
        &db,
        &headers,
        household_id,
    )
    .await?;
    let Some(source) = find_source(&db, &context, kind, &id).await? else {
        return fail(db, &context, kind, method, action, Failure::NotFound).await;
    };
    if !person_access(&db, &context, source.person_id(), action).await? {
        return fail(db, &context, kind, method, action, Failure::Forbidden).await;
    }
    let Json(body) = match payload {
        Ok(body) => body,
        Err(_) => return fail(db, &context, kind, method, action, Failure::Malformed).await,
    };
    let path = format!(
        "/api/v1/households/{household_id}/{}/{id}/dose_occurrences/{action}",
        kind.path_segment(),
    );
    match keyed_replay(&db, &context, &headers, kind, method, action, &path, &body).await? {
        Replay::New => {}
        Replay::Saved(response) => {
            db.commit().await.map_err(database_error)?;
            return Ok(response);
        }
        Replay::Conflict => {
            return fail(db, &context, kind, method, action, Failure::KeyConflict).await
        }
    }
    let attributes = match attributes(&body, action) {
        Ok(attributes) => attributes,
        Err(failure) => {
            return fail_mutation(
                db, &context, &headers, kind, method, action, &path, &body, failure,
            )
            .await
        }
    };
    let secret = state.oauth.occurrence_key_secret();
    let Some(mut row) =
        find_row(&db, &secret, &source, attributes["key"].as_str().unwrap()).await?
    else {
        return fail_mutation(
            db,
            &context,
            &headers,
            kind,
            method,
            action,
            &path,
            &body,
            Failure::InvalidOccurrence,
        )
        .await;
    };
    let request_id = Uuid::new_v4().to_string();
    match action {
        "not_taken" => {
            let (reason, note) = match parse_not_taken(attributes) {
                Ok(values) => values,
                Err(failure) => {
                    return fail_mutation(
                        db, &context, &headers, kind, method, action, &path, &body, failure,
                    )
                    .await
                }
            };
            if let Some(record) = row.record.as_ref() {
                if record.outcome != "open" {
                    if record.outcome == "not_taken"
                        && record.reason == reason
                        && record.note == note
                    {
                        let data = row_value(&secret, &source, &row);
                        let etag = record_etag(record);
                        let response_body = json!({"data": data});
                        store_key(
                            &db,
                            &context,
                            &headers,
                            method,
                            &path,
                            &body,
                            &response_body,
                            &request_id,
                            &etag,
                        )
                        .await?;
                        return finish(
                            db,
                            &context,
                            kind,
                            method,
                            action,
                            response_body,
                            Some(&etag),
                            &request_id,
                        )
                        .await;
                    }
                    return fail(db, &context, kind, method, action, Failure::AlreadyResolved)
                        .await;
                }
            }
            if row.legacy_take_id.is_some() {
                return fail(db, &context, kind, method, action, Failure::AlreadyResolved).await;
            }
            if !actionable(&source, &row) {
                return fail_mutation(
                    db,
                    &context,
                    &headers,
                    kind,
                    method,
                    action,
                    &path,
                    &body,
                    Failure::InvalidOccurrence,
                )
                .await;
            }
            row.record =
                Some(save_decision(&db, &context, &source, &row, reason, note, &request_id).await?);
        }
        "reopen" => {
            let Some(record) = row.record.as_ref() else {
                return fail_mutation(
                    db,
                    &context,
                    &headers,
                    kind,
                    method,
                    action,
                    &path,
                    &body,
                    Failure::InvalidOccurrence,
                )
                .await;
            };
            if record.outcome != "not_taken" {
                return fail_mutation(
                    db,
                    &context,
                    &headers,
                    kind,
                    method,
                    action,
                    &path,
                    &body,
                    Failure::InvalidOccurrence,
                )
                .await;
            }
            let Some(if_match) = headers
                .get(header::IF_MATCH)
                .and_then(|value| value.to_str().ok())
                .filter(|value| !value.is_empty())
            else {
                return fail_mutation(
                    db,
                    &context,
                    &headers,
                    kind,
                    method,
                    action,
                    &path,
                    &body,
                    Failure::PreconditionRequired,
                )
                .await;
            };
            if if_match != record_etag(record) {
                return fail(db, &context, kind, method, action, Failure::SyncConflict).await;
            }
            row.record = Some(reopen_decision(&db, &context, &source, record, &request_id).await?);
        }
        "take" => {
            let taken_at = match parse_take(attributes) {
                Ok(value) => value,
                Err(failure) => {
                    return fail_mutation(
                        db, &context, &headers, kind, method, action, &path, &body, failure,
                    )
                    .await
                }
            };
            if row.legacy_take_id.is_some() {
                return fail(db, &context, kind, method, action, Failure::AlreadyResolved).await;
            }
            if let Some(record) = row.record.as_ref() {
                if record.outcome == "not_taken" {
                    let Some(if_match) = headers
                        .get(header::IF_MATCH)
                        .and_then(|value| value.to_str().ok())
                        .filter(|value| !value.is_empty())
                    else {
                        return fail_mutation(
                            db,
                            &context,
                            &headers,
                            kind,
                            method,
                            action,
                            &path,
                            &body,
                            Failure::PreconditionRequired,
                        )
                        .await;
                    };
                    if if_match != record_etag(record) {
                        return fail(db, &context, kind, method, action, Failure::SyncConflict)
                            .await;
                    }
                } else if record.outcome == "taken" {
                    let supplied_uuid = attributes.get("client_uuid").and_then(Value::as_str);
                    let linked_uuid = match record.medication_take_id {
                        Some(id) => medication_take::Entity::find_by_id(id)
                            .one(&db)
                            .await
                            .map_err(database_error)?
                            .and_then(|take| take.client_uuid),
                        None => None,
                    };
                    if supplied_uuid.is_none() || supplied_uuid != linked_uuid.as_deref() {
                        return fail(db, &context, kind, method, action, Failure::AlreadyResolved)
                            .await;
                    }
                }
            }
            if row
                .record
                .as_ref()
                .is_none_or(|record| record.outcome != "taken")
            {
                if !actionable(&source, &row) {
                    return fail_mutation(
                        db,
                        &context,
                        &headers,
                        kind,
                        method,
                        action,
                        &path,
                        &body,
                        Failure::InvalidOccurrence,
                    )
                    .await;
                }
                let taken_day = dose::local_date(taken_at);
                let in_window = if kind == Kind::Schedule {
                    taken_day == row.window_start
                } else {
                    taken_day >= row.window_start
                        && taken_day <= row.window_end
                        && taken_at >= source.created_at()
                };
                if !in_window {
                    return fail_mutation(
                        db,
                        &context,
                        &headers,
                        kind,
                        method,
                        action,
                        &path,
                        &body,
                        Failure::Invalid("taken_at", "does not match the occurrence window"),
                    )
                    .await;
                }
            }
            let mut take_attributes = Map::new();
            take_attributes.insert("source_type".to_owned(), json!(kind.name()));
            take_attributes.insert("source_id".to_owned(), json!(source.portable_id()));
            for field in [
                "taken_at",
                "client_uuid",
                "dose_amount",
                "taken_from_medication_id",
            ] {
                if let Some(value) = attributes.get(field) {
                    take_attributes.insert(field.to_owned(), value.clone());
                }
            }
            let take_body = json!({"medication_take": take_attributes});
            let take = match dose::create_in_transaction(
                &db,
                &context,
                household_id,
                &take_body,
                &request_id,
            )
            .await
            {
                Ok((_, take)) => take,
                Err(error) => {
                    db.rollback().await.map_err(database_error)?;
                    let (audit_db, _) = request_context(&state, &headers, household_id).await?;
                    let (_, audit_context) =
                        mutation_idempotency::lock_household_and_reauthenticate(
                            &state,
                            &audit_db,
                            &headers,
                            household_id,
                        )
                        .await?;
                    if error.status.is_server_error() || error.status == StatusCode::CONFLICT {
                        return fail_api(audit_db, &audit_context, kind, method, action, error)
                            .await;
                    }
                    return finish_cached_error(
                        audit_db,
                        &audit_context,
                        &headers,
                        kind,
                        method,
                        action,
                        &path,
                        &body,
                        error.status,
                        error.code,
                        error.message,
                        None,
                    )
                    .await;
                }
            };
            if let Some(record) = row
                .record
                .as_ref()
                .filter(|record| record.outcome == "taken")
            {
                if record.medication_take_id != Some(take.id) {
                    return fail(db, &context, kind, method, action, Failure::AlreadyResolved)
                        .await;
                }
            } else {
                row.record =
                    Some(link_take(&db, &context, &source, &row, &take, &request_id).await?);
            }
        }
        _ => return Err(ApiError::internal()),
    }
    let data = row_value(&secret, &source, &row);
    let etag = row
        .record
        .as_ref()
        .map(record_etag)
        .ok_or_else(ApiError::internal)?;
    let response_body = json!({"data": data});
    store_key(
        &db,
        &context,
        &headers,
        method,
        &path,
        &body,
        &response_body,
        &request_id,
        &etag,
    )
    .await?;
    finish(
        db,
        &context,
        kind,
        method,
        action,
        response_body,
        Some(&etag),
        &request_id,
    )
    .await
}

pub(super) async fn list_schedule(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    query: Result<Query<RangeQuery>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    list(state, household_id, id, headers, query, Kind::Schedule).await
}

pub(super) async fn list_assignment(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    query: Result<Query<RangeQuery>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    list(state, household_id, id, headers, query, Kind::Assignment).await
}

pub(super) async fn not_taken_schedule(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    mutate(
        state,
        household_id,
        id,
        headers,
        payload,
        Kind::Schedule,
        "not_taken",
        "POST",
    )
    .await
}

pub(super) async fn not_taken_assignment(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    mutate(
        state,
        household_id,
        id,
        headers,
        payload,
        Kind::Assignment,
        "not_taken",
        "POST",
    )
    .await
}

pub(super) async fn reopen_schedule(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    mutate(
        state,
        household_id,
        id,
        headers,
        payload,
        Kind::Schedule,
        "reopen",
        "PATCH",
    )
    .await
}

pub(super) async fn reopen_assignment(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    mutate(
        state,
        household_id,
        id,
        headers,
        payload,
        Kind::Assignment,
        "reopen",
        "PATCH",
    )
    .await
}

pub(super) async fn take_schedule(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    mutate(
        state,
        household_id,
        id,
        headers,
        payload,
        Kind::Schedule,
        "take",
        "POST",
    )
    .await
}

pub(super) async fn take_assignment(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    mutate(
        state,
        household_id,
        id,
        headers,
        payload,
        Kind::Assignment,
        "take",
        "POST",
    )
    .await
}

fn sync_error(failure: Failure) -> ApiError {
    let (status, code, message) = match failure {
        Failure::Malformed => (
            StatusCode::BAD_REQUEST,
            "bad_request",
            "Invalid request body",
        ),
        Failure::Invalid(_, _) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_occurrence",
            "Outcome is invalid",
        ),
        Failure::NotFound => (StatusCode::NOT_FOUND, "not_found", "Record not found"),
        Failure::Forbidden => (
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
        ),
        Failure::AlreadyResolved => (
            StatusCode::CONFLICT,
            "already_resolved",
            "Occurrence is already resolved",
        ),
        Failure::InvalidOccurrence => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_occurrence",
            "Occurrence is unavailable",
        ),
        Failure::PreconditionRequired => (
            StatusCode::PRECONDITION_REQUIRED,
            "precondition_required",
            "A current version is required",
        ),
        Failure::SyncConflict => (
            StatusCode::CONFLICT,
            "sync_conflict",
            "Occurrence has changed",
        ),
        Failure::KeyConflict => (
            StatusCode::CONFLICT,
            "idempotency_key_reused",
            "Idempotency key has already been used for a different request",
        ),
    };
    ApiError {
        status,
        code,
        message,
        preserve_activity: false,
    }
}

pub(super) async fn apply_sync_operation(
    db: &DatabaseTransaction,
    context: &AuthContext,
    operation: &SyncOperation,
    request_id: &str,
    secret: &Arc<[u8]>,
) -> Result<SyncResult, ApiError> {
    let (source, mut row) = if operation.action == "create" {
        let attrs = &operation.attributes;
        if attrs.keys().any(|key| {
            !matches!(
                key.as_str(),
                "source_type" | "source_id" | "occurrence_key" | "outcome" | "reason" | "note"
            )
        }) {
            return Err(sync_error(Failure::Invalid(
                "attributes",
                "contains an unsupported field",
            )));
        }
        if attrs.get("outcome").and_then(Value::as_str) != Some("not_taken") {
            return Err(sync_error(Failure::InvalidOccurrence));
        }
        let kind = match attrs.get("source_type").and_then(Value::as_str) {
            Some("schedule") => Kind::Schedule,
            Some("person_medication") => Kind::Assignment,
            _ => return Err(sync_error(Failure::InvalidOccurrence)),
        };
        let source_id = attrs
            .get("source_id")
            .and_then(Value::as_str)
            .ok_or_else(|| sync_error(Failure::InvalidOccurrence))?;
        let source = find_source(db, context, kind, source_id)
            .await?
            .ok_or_else(|| sync_error(Failure::InvalidOccurrence))?;
        if !person_access(db, context, source.person_id(), "not_taken").await? {
            return Err(sync_error(Failure::Forbidden));
        }
        let occurrence_key = attrs
            .get("occurrence_key")
            .and_then(Value::as_str)
            .ok_or_else(|| sync_error(Failure::InvalidOccurrence))?;
        let row = find_row(db, secret, &source, occurrence_key)
            .await?
            .ok_or_else(|| sync_error(Failure::InvalidOccurrence))?;
        (source, row)
    } else if operation.action == "update" {
        if operation.attributes.get("outcome").and_then(Value::as_str) != Some("open")
            || operation.attributes.len() != 1
        {
            return Err(sync_error(Failure::InvalidOccurrence));
        }
        let id = operation
            .id
            .as_deref()
            .ok_or_else(|| sync_error(Failure::NotFound))?;
        let mut query = dose_occurrence::Entity::find()
            .filter(dose_occurrence::Column::HouseholdId.eq(context.membership.household_id));
        query = match id.parse::<i64>() {
            Ok(id) => query.filter(dose_occurrence::Column::Id.eq(id)),
            Err(_) => query.filter(dose_occurrence::Column::PortableId.eq(id)),
        };
        let record = query
            .one(db)
            .await
            .map_err(database_error)?
            .ok_or_else(|| sync_error(Failure::NotFound))?;
        let (kind, source_id) = if let Some(id) = record.schedule_id {
            (Kind::Schedule, id)
        } else if let Some(id) = record.person_medication_id {
            (Kind::Assignment, id)
        } else {
            return Err(sync_error(Failure::InvalidOccurrence));
        };
        let source = find_source(db, context, kind, &source_id.to_string())
            .await?
            .ok_or_else(|| sync_error(Failure::InvalidOccurrence))?;
        if !person_access(db, context, source.person_id(), "reopen").await? {
            return Err(sync_error(Failure::Forbidden));
        }
        let occurrence_key = key(secret, &source, record.window_starts_on, record.position);
        let row = find_row(db, secret, &source, &occurrence_key)
            .await?
            .ok_or_else(|| sync_error(Failure::InvalidOccurrence))?;
        if row
            .record
            .as_ref()
            .is_none_or(|row_record| row_record.id != record.id)
        {
            return Err(sync_error(Failure::InvalidOccurrence));
        }
        (source, row)
    } else {
        return Err(sync_error(Failure::InvalidOccurrence));
    };
    let replayed = if operation.action == "create" {
        let (reason, note) = parse_not_taken(&operation.attributes).map_err(sync_error)?;
        if let Some(record) = row.record.as_ref() {
            if record.outcome != "open" {
                if record.outcome == "not_taken" && record.reason == reason && record.note == note {
                    true
                } else {
                    return Err(sync_error(Failure::AlreadyResolved));
                }
            } else {
                if !actionable(&source, &row) {
                    return Err(sync_error(Failure::InvalidOccurrence));
                }
                row.record = Some(
                    save_decision(db, context, &source, &row, reason, note, request_id).await?,
                );
                false
            }
        } else {
            if !actionable(&source, &row) {
                return Err(sync_error(Failure::InvalidOccurrence));
            }
            row.record =
                Some(save_decision(db, context, &source, &row, reason, note, request_id).await?);
            false
        }
    } else {
        let record = row
            .record
            .as_ref()
            .ok_or_else(|| sync_error(Failure::InvalidOccurrence))?;
        if record.outcome != "not_taken" {
            return Err(sync_error(Failure::InvalidOccurrence));
        }
        let expected = operation
            .if_match
            .as_deref()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| sync_error(Failure::PreconditionRequired))?;
        if expected != record_etag(record) {
            return Err(sync_error(Failure::SyncConflict));
        }
        row.record = Some(reopen_decision(db, context, &source, record, request_id).await?);
        false
    };
    let record = row.record.ok_or_else(ApiError::internal)?;
    Ok(SyncResult {
        record_type: "MedicationDoseOccurrence",
        record_id: Some(record.id),
        record_portable_id: Some(record.portable_id.clone()),
        etag: Some(record_etag(&record)),
        replayed: Some(replayed),
    })
}

pub(super) async fn authorize_sync_replay(
    db: &DatabaseTransaction,
    context: &AuthContext,
    operation: &SyncOperation,
    saved: &Value,
) -> Result<(), ApiError> {
    if saved.get("action").and_then(Value::as_str) != Some(operation.action.as_str())
        || saved.get("record_type").and_then(Value::as_str) != Some("MedicationDoseOccurrence")
    {
        return Err(ApiError::forbidden());
    }
    let portable_id = saved
        .get("record_portable_id")
        .and_then(Value::as_str)
        .ok_or_else(ApiError::forbidden)?;
    let record = dose_occurrence::Entity::find()
        .filter(dose_occurrence::Column::HouseholdId.eq(context.membership.household_id))
        .filter(dose_occurrence::Column::PortableId.eq(portable_id))
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::forbidden)?;
    if saved.get("record_id").and_then(Value::as_str) != Some(record.id.to_string().as_str())
        || operation
            .id
            .as_deref()
            .is_some_and(|id| id != record.portable_id && id != record.id.to_string())
    {
        return Err(ApiError::forbidden());
    }
    let source = if let Some(id) = record.schedule_id {
        schedule::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(database_error)?
            .filter(|source| source.household_id == context.membership.household_id)
            .map(Source::Schedule)
    } else if let Some(id) = record.person_medication_id {
        person_medication::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(database_error)?
            .filter(|source| source.household_id == context.membership.household_id)
            .map(Source::Assignment)
    } else {
        None
    }
    .ok_or_else(ApiError::forbidden)?;
    let action = if operation.action == "create" {
        "not_taken"
    } else {
        "reopen"
    };
    if !person_access(db, context, source.person_id(), action).await? {
        return Err(ApiError::forbidden());
    }
    if operation.action == "create"
        && (operation
            .attributes
            .get("source_type")
            .and_then(Value::as_str)
            != Some(source.kind().name())
            || operation
                .attributes
                .get("source_id")
                .and_then(Value::as_str)
                .is_none_or(|id| id != source.portable_id() && id != source.id().to_string()))
    {
        return Err(ApiError::forbidden());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduled_time_preserves_minutes_across_daylight_saving_gap() {
        let date = NaiveDate::from_ymd_opt(2026, 3, 29).unwrap();
        let actual = scheduled_time_in_zone(date, "01:30", chrono_tz::Europe::London).unwrap();
        let expected = date.and_hms_opt(1, 30, 0).unwrap();
        assert_eq!(actual, expected);
    }
}
