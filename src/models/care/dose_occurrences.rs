mod configuration;
pub(crate) use configuration::configured_signing_key;
mod calendar;
mod identity;
mod input;
mod keys;
mod persistence;
mod projection;
mod representation;
mod scheduling;
mod writing;
use crate::models::{
    access::{self, PersonAccess, TenantTransaction},
    care::{administration, doses},
    entities::{
        api_change_event, dose_occurrence, household, medication_take, pause_period, person,
        person_medication, schedule,
    },
    errors::OperationError,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
pub(crate) use calendar::with_dashboard_timezone;
use calendar::{config_times, cycle_bounds, date, local_midnight};
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc};
use doses::CredentialProvenance;
use hmac::{Hmac, KeyInit, Mac};
use identity::{Kind, Source};
use input::{attributes, parse_not_taken, parse_take};
use keys::{decode_key, key};
use persistence::{actionable, find_row, link_take, reopen_decision, save_decision};
use projection::{Occurrence, projected};
pub(crate) use representation::record_etag;
use representation::{row_value, snapshot};
use scheduling::{effective_count, schedule_applies};
pub(crate) use scheduling::{schedule_as_needed, schedule_config_on, weekday_matches};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseTransaction, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, QueryTrait, Select, Set,
};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, str::FromStr, sync::Arc};
use uuid::Uuid;
pub use writing::{RangeQuery, authorize, change, list};

pub(crate) struct ReportOccurrence {
    pub person_id: i64,
    pub medication_id: i64,
    pub source_id: i64,
    pub source_type: &'static str,
    pub window_start: NaiveDate,
    pub window_end: NaiveDate,
    pub scheduled_at: Option<NaiveDateTime>,
    pub expected: bool,
    pub outcome: String,
    pub reason: Option<String>,
    pub note: Option<String>,
}

pub(crate) fn expected_schedule_doses_for_report(source: &schedule::Model, day: NaiveDate) -> i32 {
    if !source.active
        || source.retired_at.is_some()
        || schedule_as_needed(source)
        || !schedule_applies(source, day)
    {
        return 0;
    }
    let Some(config) = schedule_config_on(source, day) else {
        return 0;
    };
    let times = config_times(config, day);
    if times.is_empty() {
        effective_count(config, source.max_daily_doses.unwrap_or(1))
    } else {
        i32::try_from(times.len()).unwrap_or(i32::MAX)
    }
}

fn report_source_scope(schedule_ids: &[i64], assignment_ids: &[i64]) -> Condition {
    Condition::any()
        .add(dose_occurrence::Column::ScheduleId.is_in(schedule_ids.to_vec()))
        .add(dose_occurrence::Column::PersonMedicationId.is_in(assignment_ids.to_vec()))
}

fn report_pause_scope(
    household_id: i64,
    schedule_ids: &[i64],
    assignment_ids: &[i64],
    scan_start: NaiveDate,
    scan_end: NaiveDate,
) -> Select<pause_period::Entity> {
    let end = local_midnight(scan_end + Duration::days(1));
    pause_period::Entity::find()
        .filter(pause_period::Column::HouseholdId.eq(household_id))
        .filter(
            Condition::any()
                .add(pause_period::Column::ScheduleId.is_in(schedule_ids.to_vec()))
                .add(pause_period::Column::PersonMedicationId.is_in(assignment_ids.to_vec())),
        )
        .filter(
            Condition::any()
                .add(pause_period::Column::StartedAt.lt(end))
                .add(
                    Condition::all()
                        .add(pause_period::Column::StartedAt.is_null())
                        .add(pause_period::Column::CreatedAt.lt(end)),
                ),
        )
        .filter(
            Condition::any()
                .add(pause_period::Column::EndedAt.is_null())
                .add(pause_period::Column::EndedAt.gt(local_midnight(scan_start))),
        )
}

fn report_linked_scope(household_id: i64, take_ids: &[i64]) -> Select<dose_occurrence::Entity> {
    dose_occurrence::Entity::find()
        .filter(dose_occurrence::Column::HouseholdId.eq(household_id))
        .filter(dose_occurrence::Column::MedicationTakeId.is_in(take_ids.to_vec()))
        .select_only()
        .column(dose_occurrence::Column::ScheduleId)
        .column(dose_occurrence::Column::PersonMedicationId)
        .column(dose_occurrence::Column::MedicationTakeId)
}

pub(crate) async fn projected_for_report(
    tenant: &TenantTransaction,
    person_ids: &[i64],
    start: NaiveDate,
    end: NaiveDate,
) -> Result<Vec<ReportOccurrence>, OperationError> {
    projected_for_range(tenant, person_ids, start, end, false).await
}

pub(crate) async fn projected_for_dashboard(
    tenant: &TenantTransaction,
    person_ids: &[i64],
    today: NaiveDate,
) -> Result<Vec<ReportOccurrence>, OperationError> {
    projected_for_range(tenant, person_ids, today, today, true).await
}

async fn projected_for_range(
    tenant: &TenantTransaction,
    person_ids: &[i64],
    start: NaiveDate,
    end: NaiveDate,
    dashboard: bool,
) -> Result<Vec<ReportOccurrence>, OperationError> {
    if person_ids.is_empty() {
        return Ok(Vec::new());
    }
    let db = tenant.transaction();
    let household_id = tenant.scope().household_id;
    let schedules = schedule::Entity::find()
        .filter(schedule::Column::HouseholdId.eq(household_id))
        .filter(schedule::Column::PersonId.is_in(person_ids.to_vec()))
        .filter(
            Condition::any()
                .add(schedule::Column::StartDate.is_null())
                .add(schedule::Column::StartDate.lte(end)),
        )
        .all(db)
        .await?;
    let assignments = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(household_id))
        .filter(person_medication::Column::PersonId.is_in(person_ids.to_vec()))
        .filter(person_medication::Column::CreatedAt.lte(local_midnight(end + Duration::days(1))))
        .all(db)
        .await?;
    let sources: Vec<Source> = schedules
        .into_iter()
        .map(|mut source| {
            if dashboard
                && source.max_daily_doses.is_none()
                && !schedule_as_needed(&source)
                && let Some(config) = schedule_config_on(&source, start)
                && config_times(config, start).is_empty()
                && let Some(hours) =
                    doses::config_decimal(config, &["min_hours_between_doses", "min_hours"]).or(
                        source
                            .min_hours_between_doses
                            .map(sea_orm::prelude::Decimal::from),
                    )
                && hours > sea_orm::prelude::Decimal::ZERO
            {
                use sea_orm::prelude::Decimal;
                source.max_daily_doses =
                    (Decimal::from(24) / hours).ceil().to_string().parse().ok();
            }
            Source::Schedule(source)
        })
        .chain(assignments.into_iter().map(Source::Assignment))
        .collect();
    if sources.is_empty() {
        return Ok(Vec::new());
    }
    let scan_start = sources
        .iter()
        .map(|source| projection::scan_bounds(source, start, end).0)
        .min()
        .unwrap_or(start);
    let scan_end = sources
        .iter()
        .map(|source| projection::scan_bounds(source, start, end).1)
        .max()
        .unwrap_or(end);
    let schedule_ids: Vec<_> = sources
        .iter()
        .filter_map(|source| match source {
            Source::Schedule(row) => Some(row.id),
            Source::Assignment(_) => None,
        })
        .collect();
    let assignment_ids: Vec<_> = sources
        .iter()
        .filter_map(|source| match source {
            Source::Schedule(_) => None,
            Source::Assignment(row) => Some(row.id),
        })
        .collect();
    let persisted = dose_occurrence::Entity::find()
        .filter(dose_occurrence::Column::HouseholdId.eq(household_id))
        .filter(report_source_scope(&schedule_ids, &assignment_ids))
        .filter(dose_occurrence::Column::WindowStartsOn.lte(scan_end))
        .filter(
            Condition::any()
                .add(dose_occurrence::Column::WindowEndsOn.gte(scan_start))
                .add(
                    Condition::all()
                        .add(dose_occurrence::Column::WindowEndsOn.is_null())
                        .add(dose_occurrence::Column::WindowStartsOn.gte(scan_start)),
                ),
        )
        .all(db)
        .await?;
    let pauses = report_pause_scope(
        household_id,
        &schedule_ids,
        &assignment_ids,
        scan_start,
        scan_end,
    )
    .all(db)
    .await?;
    let takes: Vec<medication_take::Model> = medication_take::Entity::find()
        .filter(medication_take::Column::HouseholdId.eq(household_id))
        .filter(
            Condition::any()
                .add(medication_take::Column::ScheduleId.is_in(schedule_ids))
                .add(medication_take::Column::PersonMedicationId.is_in(assignment_ids)),
        )
        .filter(medication_take::Column::TakenAt.gte(local_midnight(scan_start)))
        .filter(medication_take::Column::TakenAt.lt(local_midnight(scan_end + Duration::days(1))))
        .order_by_asc(medication_take::Column::TakenAt)
        .order_by_asc(medication_take::Column::Id)
        .all(db)
        .await?;
    let take_ids: Vec<_> = takes.iter().map(|row| row.id).collect();
    let linked_ids: std::collections::HashSet<(Option<i64>, Option<i64>, i64)> =
        if take_ids.is_empty() {
            std::collections::HashSet::new()
        } else {
            report_linked_scope(household_id, &take_ids)
                .into_tuple::<(Option<i64>, Option<i64>, Option<i64>)>()
                .all(db)
                .await?
                .into_iter()
                .filter_map(|(schedule_id, assignment_id, take_id)| {
                    take_id.map(|take_id| (schedule_id, assignment_id, take_id))
                })
                .collect()
        };
    let mut result = Vec::new();
    for source in sources {
        let (source_start, source_end) = projection::scan_bounds(&source, start, end);
        let source_persisted: Vec<_> = persisted
            .iter()
            .filter(|row| match &source {
                Source::Schedule(source) => row.schedule_id == Some(source.id),
                Source::Assignment(source) => row.person_medication_id == Some(source.id),
            })
            .cloned()
            .collect();
        let source_pauses: Vec<_> = pauses
            .iter()
            .filter(|row| match &source {
                Source::Schedule(source) => row.schedule_id == Some(source.id),
                Source::Assignment(source) => row.person_medication_id == Some(source.id),
            })
            .cloned()
            .collect();
        let source_takes: Vec<_> = takes
            .iter()
            .filter(|row| match &source {
                Source::Schedule(source) => {
                    row.schedule_id == Some(source.id)
                        && !linked_ids.contains(&(Some(source.id), None, row.id))
                }
                Source::Assignment(source) => {
                    row.person_medication_id == Some(source.id)
                        && !linked_ids.contains(&(None, Some(source.id), row.id))
                }
            })
            .cloned()
            .collect();
        let medication_id = match &source {
            Source::Schedule(row) => row.medication_id,
            Source::Assignment(row) => row.medication_id,
        };
        for occurrence in projection::projected_loaded(
            &source,
            source_start,
            source_end,
            &source_persisted,
            &source_pauses,
            &source_takes,
        ) {
            if occurrence.window_start > end || occurrence.window_end < start {
                continue;
            }
            let record = occurrence.record.as_ref();
            let outcome = if record.is_some_and(|row| row.outcome == "taken")
                || occurrence.legacy_take_id.is_some()
            {
                "taken"
            } else {
                record.map_or("open", |row| row.outcome.as_str())
            };
            result.push(ReportOccurrence {
                person_id: source.person_id(),
                medication_id,
                source_id: source.id(),
                source_type: source.kind().name(),
                window_start: occurrence.window_start,
                window_end: occurrence.window_end,
                scheduled_at: occurrence.scheduled_at,
                expected: occurrence.expected,
                outcome: outcome.to_owned(),
                reason: record.and_then(|row| row.reason.clone()),
                note: record.and_then(|row| row.note.clone()),
            });
        }
    }
    Ok(result)
}
type ApiError = OperationError;
struct AuthContext<'a> {
    tenant: &'a TenantTransaction,
    provenance: Option<&'a CredentialProvenance>,
}
enum Failure {
    Malformed,
    Invalid(&'static str, &'static str),
}
impl From<Failure> for OperationError {
    fn from(error: Failure) -> Self {
        match error {
            Failure::Malformed => OperationError::Validation {
                details: json!({"code":"bad_request","message":"Invalid request body"}),
            },
            Failure::Invalid(field, message) => invalid(field, message),
        }
    }
}
fn invalid(field: &str, message: &str) -> OperationError {
    OperationError::Validation {
        details: json!({"errors":{field:[message]}}),
    }
}
fn invalid_occurrence() -> OperationError {
    OperationError::Validation {
        details: json!({"code":"invalid_occurrence","message":"Occurrence is unavailable"}),
    }
}
fn conflict(code: &str) -> OperationError {
    OperationError::Conflict {
        code: code.into(),
        details: json!({"error":if code == "sync_conflict" { "Occurrence has changed" } else { "Occurrence is already resolved" }}),
    }
}
fn not_found() -> OperationError {
    OperationError::NotFound
}
fn database_error(_: sea_orm::DbErr) -> OperationError {
    OperationError::Unavailable
}
fn valid_identifier(id: &str) -> bool {
    id.parse::<i64>().is_ok_and(|id| id > 0) || Uuid::parse_str(id).is_ok()
}
fn parse_decimal(value: &Value) -> Option<sea_orm::prelude::Decimal> {
    sea_orm::prelude::Decimal::from_str(value.as_str()?).ok()
}
fn representation_etag(value: &Value) -> String {
    let mut value = value.clone();
    value.sort_all_objects();
    format!(
        "\"{}\"",
        hex::encode(Sha256::digest(value.to_string().as_bytes()))
    )
}
fn local_date(value: NaiveDateTime) -> NaiveDate {
    calendar::local_date(value)
}
async fn record_version(
    context: &AuthContext<'_>,
    kind: &str,
    id: i64,
    event: &str,
    before: Option<Value>,
    after: Option<Value>,
) -> Result<(), OperationError> {
    administration::persistence::record_version_as(
        context.tenant,
        kind,
        id,
        event,
        before,
        after.ok_or(OperationError::Unavailable)?,
        context.provenance,
    )
    .await
}
struct SyncRecord<'a> {
    record_type: &'a str,
    record_id: i64,
    portable_id: &'a str,
    action: &'a str,
    person_portable_id: Option<&'a str>,
}
async fn record_change(
    db: &DatabaseTransaction,
    context: &AuthContext<'_>,
    request_id: &str,
    row: SyncRecord<'_>,
) -> Result<(), OperationError> {
    let now = Utc::now().naive_utc();
    api_change_event::ActiveModel{household_id:Set(context.tenant.scope().household_id),household_membership_id:Set(Some(context.tenant.membership().id)),account_id:Set(Some(context.tenant.scope().actor.account_id)),action:Set(row.action.into()),record_type:Set(row.record_type.into()),record_id:Set(row.record_id),record_portable_id:Set(Some(row.portable_id.into())),request_id:Set(Some(request_id.into())),metadata:Set(json!({"record_type":row.record_type,"record_id":row.record_id,"portable_id":row.portable_id,"person_portable_id":row.person_portable_id})),occurred_at:Set(now),created_at:Set(now),updated_at:Set(now),..Default::default()}.insert(db).await?;
    Ok(())
}

#[cfg(test)]
mod report_scope_tests {
    use super::*;
    use sea_orm::DbBackend;

    #[test]
    fn report_projection_queries_bound_pause_sources_and_linked_take_ids() {
        let first = NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();
        let last = NaiveDate::from_ymd_opt(2026, 7, 7).unwrap();
        let pauses = report_pause_scope(72001, &[83999], &[81001], first, last)
            .build(DbBackend::Postgres)
            .to_string();
        let pause_where = pauses.split_once(" WHERE ").unwrap().1;
        assert!(pause_where.contains("\"schedule_id\" IN"), "{pause_where}");
        assert!(
            pause_where.contains("\"person_medication_id\" IN"),
            "{pause_where}"
        );
        assert!(pause_where.contains("\"started_at\" <"), "{pause_where}");
        assert!(pause_where.contains("\"ended_at\" >"), "{pause_where}");
        assert!(pause_where.contains("2026-07-08"), "{pause_where}");
        let linked = report_linked_scope(72001, &[84999])
            .build(DbBackend::Postgres)
            .to_string();
        let (selected, linked_where) = linked.split_once(" WHERE ").unwrap();
        assert!(linked_where.contains("\"medication_take_id\" IN"));
        assert!(selected.contains("\"medication_take_id\""));
        assert!(selected.contains("\"schedule_id\""));
        assert!(selected.contains("\"person_medication_id\""));
        assert!(!selected.contains("\"id\""));
    }
}
