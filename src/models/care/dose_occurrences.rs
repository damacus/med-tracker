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
use scheduling::{effective_count, schedule_applies, schedule_as_needed, schedule_config_on};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseTransaction, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, QueryTrait, Set,
};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, str::FromStr, sync::Arc};
use uuid::Uuid;
pub use writing::{RangeQuery, authorize, change, list};
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
