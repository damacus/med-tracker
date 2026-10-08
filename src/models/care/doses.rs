mod audit;
mod changes;
mod input;
mod locking;
mod persistence;
mod replay;
mod source;
mod stock;
mod timing;
mod validation;
mod writing;

use crate::models::{
    access::{self, HouseholdScope, PersonAccess, TenantTransaction},
    entities::{
        api_change_event, dosage, medication, medication_take, membership, person,
        person_medication, schedule,
    },
    errors::OperationError,
};
use audit::{StockVersionChange, record_domain_audit, stock_version};
use changes::{SyncRecord, record_change};
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc};
use input::{ProposedTake, decimal_from_json, parse_input_time, prepare};
use locking::lock_row;
use persistence::insert_take;
use replay::{existing_take, lock_client_uuid, replay_matches};
use sea_orm::prelude::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DatabaseTransaction, DbBackend,
    EntityTrait, QueryFilter, Set, Statement,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
pub(crate) use source::app_zone;
use source::{
    Source, effective_source, local_date_in_zone, source, source_from_assignment,
    source_from_schedule,
};
pub(crate) use source::{config_decimal, config_value};
use std::{collections::HashSet, str::FromStr};
use stock::decrement_stock;
pub(crate) use stock::same_stock_signature;
use timing::{applies_on, timing_allowed};
use uuid::Uuid;
use validation::parse_decimal;
pub(crate) use validation::valid_identifier;

type ApiError = OperationError;

#[derive(Clone, Debug)]
pub struct Take {
    pub client_uuid: Option<String>,
    pub source_type: String,
    pub source_id: String,
    pub taken_at: String,
    pub dose_amount: Option<String>,
    pub dose_unit: Option<String>,
    pub taken_from_medication_id: Option<i64>,
    pub expected_effective_amount: Option<String>,
    pub expected_effective_unit: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct BrowserDosePreview {
    pub available: bool,
    pub amount: Option<String>,
    pub unit: Option<String>,
}

fn preview_source(mut source: Source, date: NaiveDate) -> BrowserDosePreview {
    let available = source.active && !source.retired && applies_on(&source, date);
    effective_source(&mut source, date);
    BrowserDosePreview {
        available,
        amount: source
            .dose_amount
            .map(|value| value.normalize().to_string()),
        unit: source.dose_unit,
    }
}

pub fn browser_schedule_preview(record: schedule::Model, date: NaiveDate) -> BrowserDosePreview {
    preview_source(source_from_schedule(record), date)
}

pub async fn browser_preview(
    tenant: &TenantTransaction,
    medication_id: i64,
    kind: &str,
    id: &str,
    local_time: &str,
    zone: chrono_tz::Tz,
) -> Result<BrowserDosePreview, OperationError> {
    let local = NaiveDateTime::parse_from_str(local_time, "%Y-%m-%dT%H:%M")
        .map_err(|_| error(ErrorKind::Validation, "taken_at is invalid"))?;
    let chrono::LocalResult::Single(taken_at) = zone.from_local_datetime(&local) else {
        return Err(error(ErrorKind::Validation, "taken_at is invalid"));
    };
    access::recheck(tenant).await?;
    let context = DoseContext {
        tenant,
        provenance: None,
        zone,
    };
    let source = source(
        tenant.transaction(),
        &context,
        tenant.scope().household_id,
        kind,
        id,
    )
    .await?;
    if source.medication_id != medication_id {
        return Err(OperationError::NotFound);
    }
    access::require_person_access(tenant, source.person_id, PersonAccess::Record).await?;
    let mut preview = preview_source(source, taken_at.date_naive());
    preview.available &= taken_at.with_timezone(&Utc) <= Utc::now() + Duration::hours(1);
    Ok(preview)
}

#[derive(Clone, Debug)]
pub enum Command {
    Take(Take),
}

#[derive(Debug)]
pub enum Outcome {
    Created(medication_take::Model),
    Replayed(medication_take::Model),
}

#[derive(Clone, Copy, Debug)]
pub enum CredentialMethod {
    PersonalApiKey,
    ApiSession,
    ApiAppToken,
    OauthGrant,
    BrowserSession,
}

impl CredentialMethod {
    fn label(self) -> &'static str {
        match self {
            Self::PersonalApiKey => "personal_api_key",
            Self::ApiSession => "api_session",
            Self::ApiAppToken => "api_app_token",
            Self::OauthGrant => "oauth",
            Self::BrowserSession => "browser_session",
        }
    }

    fn reference_prefix(self) -> &'static str {
        match self {
            Self::PersonalApiKey => "personal_api_key",
            Self::ApiSession => "api_session",
            Self::ApiAppToken => "api_app_token",
            Self::OauthGrant => "oauth_grant",
            Self::BrowserSession => "browser_session",
        }
    }
}

#[derive(Clone, Debug)]
pub struct CredentialProvenance {
    pub method: CredentialMethod,
    pub reference: String,
}

struct DoseContext<'a> {
    tenant: &'a TenantTransaction,
    provenance: Option<&'a CredentialProvenance>,
    zone: chrono_tz::Tz,
}
impl DoseContext<'_> {
    fn membership(&self) -> &membership::Model {
        self.tenant.membership()
    }
    fn scope(&self) -> &HouseholdScope {
        self.tenant.scope()
    }
}

enum ErrorKind {
    Validation,
    Conflict,
}
fn error(kind: ErrorKind, message: &str) -> OperationError {
    match kind {
        ErrorKind::Validation => OperationError::Validation {
            details: json!({"error": message}),
        },
        ErrorKind::Conflict => OperationError::Conflict {
            code: "conflict".to_owned(),
            details: json!({"error": message}),
        },
    }
}
fn database_error(_: sea_orm::DbErr) -> OperationError {
    OperationError::Unavailable
}

enum TakeFailureCause {
    Paused,
    NumericDoseAmount,
    OutOfStock,
}
struct TakeFailure {
    error: OperationError,
    cause: Option<TakeFailureCause>,
}
impl From<OperationError> for TakeFailure {
    fn from(error: OperationError) -> Self {
        Self { error, cause: None }
    }
}
impl TakeFailure {
    fn into_error(self) -> OperationError {
        let mut error = self.error;
        if let (Some(cause), OperationError::Validation { details }) = (self.cause, &mut error) {
            details["code"] = json!(match cause {
                TakeFailureCause::Paused => "paused",
                TakeFailureCause::NumericDoseAmount => "numeric_dose_amount",
                TakeFailureCause::OutOfStock => "out_of_stock",
            });
        }
        error
    }
}

pub async fn execute(
    tenant: &TenantTransaction,
    command: Command,
) -> Result<Outcome, OperationError> {
    execute_with_provenance(tenant, command, None).await
}

pub async fn execute_in_timezone(
    tenant: &TenantTransaction,
    command: Command,
    zone: chrono_tz::Tz,
    provenance: Option<&CredentialProvenance>,
) -> Result<Outcome, OperationError> {
    let Command::Take(take) = command;
    let mut attributes = json!({"source_type": take.source_type, "source_id": take.source_id, "taken_at": take.taken_at});
    if let Some(value) = take.client_uuid {
        attributes["client_uuid"] = json!(value);
    }
    if let Some(value) = take.dose_amount {
        attributes["dose_amount"] = json!(value);
    }
    if let Some(value) = take.dose_unit {
        attributes["dose_unit"] = json!(value);
    }
    if let Some(value) = take.taken_from_medication_id {
        attributes["taken_from_medication_id"] = json!(value);
    }
    if let Some(value) = take.expected_effective_amount {
        attributes["expected_effective_amount"] = json!(value);
    }
    if let Some(value) = take.expected_effective_unit {
        attributes["expected_effective_unit"] = json!(value);
    }
    let context = DoseContext {
        tenant,
        provenance,
        zone,
    };
    let (replayed, record) = writing::create_with_failure(
        tenant.transaction(),
        &context,
        tenant.scope().household_id,
        &json!({"medication_take": attributes}),
        &tenant.scope().request_id,
    )
    .await
    .map_err(TakeFailure::into_error)?;
    Ok(if replayed {
        Outcome::Replayed(record)
    } else {
        Outcome::Created(record)
    })
}

pub async fn execute_with_provenance(
    tenant: &TenantTransaction,
    command: Command,
    provenance: Option<&CredentialProvenance>,
) -> Result<Outcome, OperationError> {
    execute_in_timezone(tenant, command, app_zone(), provenance).await
}

async fn allowed_person(
    _: &DatabaseTransaction,
    context: &DoseContext<'_>,
    person_id: i64,
    write: bool,
) -> Result<bool, OperationError> {
    match access::require_person_access(
        context.tenant,
        person_id,
        if write {
            PersonAccess::Record
        } else {
            PersonAccess::View
        },
    )
    .await
    {
        Ok(()) => Ok(true),
        Err(OperationError::Forbidden) => Ok(false),
        Err(error) => Err(error),
    }
}

pub(super) fn quantity(amount: Decimal, unit: &str) -> Decimal {
    if matches!(
        unit,
        "tablet" | "capsule" | "gummy" | "sachet" | "spray" | "drop" | "pad" | "ml"
    ) {
        amount
    } else {
        Decimal::ONE
    }
}

pub(super) fn same_dosage_signature(left: &dosage::Model, right: &dosage::Model) -> bool {
    left.amount == right.amount
        && left.unit == right.unit
        && left.frequency == right.frequency
        && left.description.as_deref().unwrap_or("") == right.description.as_deref().unwrap_or("")
        && left.default_for_adults == right.default_for_adults
        && left.default_for_children == right.default_for_children
        && left.default_max_daily_doses == right.default_max_daily_doses
        && left.default_min_hours_between_doses == right.default_min_hours_between_doses
        && left.default_dose_cycle == right.default_dose_cycle
}

pub(super) fn sufficient_stock(supply: Option<Decimal>, amount: Decimal, unit: &str) -> bool {
    supply.is_none_or(|value| value >= quantity(amount, unit))
}

pub(super) fn selected_tracked_dosage<'a>(
    tracked: &'a [dosage::Model],
    inventory_id: i64,
    source_option: Option<&dosage::Model>,
    source_amount: Option<Decimal>,
    source_unit: Option<&str>,
) -> Option<&'a dosage::Model> {
    let mut matches = tracked.iter().filter(|option| {
        if let Some(source_option) = source_option {
            if source_option.medication_id == inventory_id {
                option.id == source_option.id
            } else {
                same_dosage_signature(option, source_option)
            }
        } else {
            Some(option.amount) == source_amount && Some(option.unit.as_str()) == source_unit
        }
    });
    let selected = matches.next()?;
    matches.next().is_none().then_some(selected)
}

pub(super) fn decimal_string(value: String) -> String {
    if let Some((whole, fraction)) = value.split_once('.') {
        let fraction = fraction.trim_end_matches('0');
        if fraction.is_empty() {
            format!("{whole}.0")
        } else {
            format!("{whole}.{fraction}")
        }
    } else {
        format!("{value}.0")
    }
}
