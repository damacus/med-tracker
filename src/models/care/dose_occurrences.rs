use loco_rs::model::TenantQueryExt;
mod configuration;
mod dashboard_projection;
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
use crate::models::identity::credentials::CredentialProvenance;
use crate::models::{
    access::{self, PersonAccess, TenantTransaction},
    care::{administration, doses, valid_identifier},
    entities::{
        api_change_event, dose_occurrence, household, medication_take, pause_period,
        person_medication, schedule,
    },
    errors::OperationError,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use calendar::{config_times, cycle_bounds, date, local_midnight};
pub(crate) use calendar::{scheduled_time_in_zone, with_dashboard_timezone};
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc};
use hmac::{Hmac, KeyInit, Mac};
use identity::{Kind, Source};
use input::ActionInput;
pub use input::ChangeInput;
use keys::{decode_key, key};
use persistence::{actionable, find_row, link_take, reopen_decision, save_decision};
use projection::{Occurrence, projected};
pub(crate) use representation::record_etag;
use representation::{row_value, snapshot};
pub(super) use scheduling::configured_count;
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
pub(crate) use writing::browser_list;
pub(crate) use writing::browser_take_target;
pub(crate) use writing::key_for_row;
pub(crate) use writing::list;
pub use writing::{RangeQuery, authorize, change};

pub struct OccurrenceWritten {
    pub row: dose_occurrence::Model,
    pub(crate) body: crate::views::DataResponse<crate::views::dose_occurrences::DoseOccurrence>,
    pub etag: String,
}

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

struct RangeOccurrence {
    report: ReportOccurrence,
    dashboard: Option<dashboard_projection::DashboardOccurrence>,
}

pub(crate) fn dashboard_capacity(count: usize) -> Result<(), OperationError> {
    projection::check_capacity(count)
}

pub(crate) fn expected_schedule_doses_for_report(
    source: &schedule::Model,
    day: NaiveDate,
    settings: Option<&serde_json::Value>,
) -> i32 {
    let source_settings = Source::Schedule(source.clone()).calendar_settings(settings);
    let settings = source_settings.as_ref();
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
    let times = if config.get("times").is_none() {
        config_times(&source.schedule_config, day, settings)
    } else {
        config_times(config, day, settings)
    };
    if times.is_empty() {
        let fallback =
            effective_count(&source.schedule_config, source.max_daily_doses.unwrap_or(1));
        effective_count(config, fallback)
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
    start: NaiveDateTime,
    end: NaiveDateTime,
) -> Select<pause_period::Entity> {
    pause_period::Entity::find()
        .in_tenant(household_id)
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
                .add(pause_period::Column::EndedAt.gt(start)),
        )
}

fn report_linked_scope(household_id: i64, take_ids: &[i64]) -> Select<dose_occurrence::Entity> {
    dose_occurrence::Entity::find()
        .in_tenant(household_id)
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
    Ok(projected_for_range(tenant, person_ids, start, end, false)
        .await?
        .into_iter()
        .map(|row| row.report)
        .collect())
}

pub(crate) async fn projected_for_dashboard(
    tenant: &TenantTransaction,
    person_ids: &[i64],
    today: NaiveDate,
) -> Result<Vec<dashboard_projection::DashboardOccurrence>, OperationError> {
    projected_for_range(tenant, person_ids, today, today, true)
        .await?
        .into_iter()
        .map(|row| row.dashboard.ok_or(OperationError::Unavailable))
        .collect()
}

async fn projected_for_range(
    tenant: &TenantTransaction,
    person_ids: &[i64],
    start: NaiveDate,
    end: NaiveDate,
    dashboard: bool,
) -> Result<Vec<RangeOccurrence>, OperationError> {
    if person_ids.is_empty() {
        return Ok(Vec::new());
    }
    let db = tenant.transaction();
    let household_id = tenant.scope().household_id;
    let secret = dashboard
        .then(|| configured_signing_key(tenant.settings()))
        .transpose()?;
    let query_end = if dashboard {
        end + Duration::days(2)
    } else {
        end
    };
    let schedules = schedule::Entity::find()
        .in_tenant(household_id)
        .filter(schedule::Column::PersonId.is_in(person_ids.to_vec()))
        .filter(
            Condition::any()
                .add(schedule::Column::StartDate.is_null())
                .add(schedule::Column::StartDate.lte(query_end)),
        )
        .all(db)
        .await?;
    let assignments = person_medication::Entity::find()
        .in_tenant(household_id)
        .filter(person_medication::Column::PersonId.is_in(person_ids.to_vec()))
        .filter(
            person_medication::Column::CreatedAt
                .lte(local_midnight(end + Duration::days(1), tenant.settings())),
        )
        .all(db)
        .await?;
    let sources: Vec<Source> = schedules
        .into_iter()
        .map(Source::Schedule)
        .chain(assignments.into_iter().map(Source::Assignment))
        .collect();
    if sources.is_empty() {
        return Ok(Vec::new());
    }
    if dashboard {
        projection::check_capacity(sources.len())?;
    }
    let mut projection_context = projection::ProjectionContext::new(tenant.settings(), dashboard);
    let scan_start = sources
        .iter()
        .map(|source| {
            dashboard_projection::scan_bounds(source, start, end, dashboard, tenant.settings()).0
        })
        .min()
        .unwrap_or(start);
    let scan_end = sources
        .iter()
        .map(|source| {
            dashboard_projection::scan_bounds(source, start, end, dashboard, tenant.settings()).1
        })
        .max()
        .unwrap_or(end);
    let schedule_ids: Vec<_> = sources
        .iter()
        .filter_map(|source| match source {
            Source::Schedule(row) => Some(row.id),
            Source::Assignment(_) => None,
        })
        .collect();
    let scan_start_time = sources
        .iter()
        .map(|source| {
            let settings = source.calendar_settings(tenant.settings());
            local_midnight(scan_start, settings.as_ref())
        })
        .min()
        .unwrap_or_else(|| local_midnight(scan_start, tenant.settings()));
    let scan_end_time = sources
        .iter()
        .map(|source| {
            let settings = source.calendar_settings(tenant.settings());
            local_midnight(scan_end + Duration::days(1), settings.as_ref())
        })
        .max()
        .unwrap_or_else(|| local_midnight(scan_end + Duration::days(1), tenant.settings()));
    let assignment_ids: Vec<_> = sources
        .iter()
        .filter_map(|source| match source {
            Source::Schedule(_) => None,
            Source::Assignment(row) => Some(row.id),
        })
        .collect();
    let persisted = dose_occurrence::Entity::find()
        .in_tenant(household_id)
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
        scan_start_time,
        scan_end_time,
    )
    .all(db)
    .await?;
    let takes: Vec<medication_take::Model> = medication_take::Entity::find()
        .in_tenant(household_id)
        .filter(
            Condition::any()
                .add(medication_take::Column::ScheduleId.is_in(schedule_ids))
                .add(medication_take::Column::PersonMedicationId.is_in(assignment_ids)),
        )
        .filter(medication_take::Column::TakenAt.gte(scan_start_time))
        .filter(medication_take::Column::TakenAt.lt(scan_end_time))
        .order_by_asc(medication_take::Column::TakenAt)
        .order_by_asc(medication_take::Column::Id)
        .all(db)
        .await?;
    if dashboard {
        projection::check_capacity(persisted.len())?;
        projection::check_capacity(pauses.len())?;
        projection::check_capacity(takes.len())?;
    }
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
    let take_by_id: std::collections::HashMap<_, _> = if dashboard {
        takes.iter().map(|take| (take.id, take)).collect()
    } else {
        std::collections::HashMap::new()
    };
    let mut result = Vec::new();
    for source in sources {
        let (source_start, source_end) =
            dashboard_projection::scan_bounds(&source, start, end, dashboard, tenant.settings());
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
        let mut dose_previews = dashboard_projection::DosePreviews::default();
        for occurrence in projection::projected_loaded_in_mode(
            &source,
            (source_start, source_end),
            &source_persisted,
            &source_pauses,
            &source_takes,
            &mut projection_context,
        )? {
            if if dashboard {
                !dashboard_projection::includes(&source, &occurrence, start, end, tenant.settings())
            } else {
                occurrence.window_start > end || occurrence.window_end < start
            } {
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
            let report = ReportOccurrence {
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
            };
            let dashboard = secret.as_ref().map(|secret| {
                dashboard_projection::project(
                    &report,
                    &source,
                    &occurrence,
                    &take_by_id,
                    secret,
                    tenant.settings(),
                    dose_previews.for_source(&source, occurrence.window_start),
                )
            });
            result.push(RangeOccurrence { report, dashboard });
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
fn database_error(error: sea_orm::DbErr) -> OperationError {
    OperationError::from(error)
}

#[cfg(test)]
mod database_diagnostic_tests {
    #[test]
    fn database_failure_keeps_safe_diagnostic() -> Result<(), String> {
        let sentinel = "synthetic-private-database-detail";
        let (error, events) = crate::models::errors::tests::capture_operation(|| {
            super::database_error(sea_orm::DbErr::Exec(sea_orm::RuntimeErr::Internal(
                sentinel.to_owned(),
            )))
        })?;
        assert_eq!(error, super::OperationError::Unavailable);
        assert!(!error.to_string().contains(sentinel));
        assert_eq!(events.len(), 1);
        let fields = &events[0];
        assert!(
            fields
                .iter()
                .any(|(name, value)| name == "error.type" && value == "execution")
        );
        assert!(fields.iter().any(|(name, value)| {
            name == "medtracker.error_reference" && uuid::Uuid::parse_str(value).is_ok()
        }));
        assert!(!fields.iter().any(|(_, value)| value.contains(sentinel)));
        Ok(())
    }
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
fn local_date(value: NaiveDateTime, settings: Option<&serde_json::Value>) -> NaiveDate {
    calendar::local_date(value, settings)
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
        let pauses = report_pause_scope(
            72001,
            &[83999],
            &[81001],
            local_midnight(first, None),
            local_midnight(last + Duration::days(1), None),
        )
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
