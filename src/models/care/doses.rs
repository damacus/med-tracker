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

use crate::models::ids::{AssignmentId, HouseholdId, MedicationId, PersonId, ScheduleId};
use crate::models::{
    access::{self, HouseholdScope, TenantTransaction},
    entities::{
        api_change_event, dosage, medication, medication_take, membership, person_medication,
        schedule,
    },
    errors::OperationError,
};
use audit::{StockVersionChange, record_domain_audit, stock_version};
use changes::{SyncRecord, record_change};
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc};
pub use input::{PreparedRequest, prepare_request};
use input::{TakeCandidate, ValidatedTake, prepare};
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
pub use source::DoseSource;
pub(crate) use source::app_zone;
use source::{
    DosePrescription, Source, SourceReference, effective_source, local_date_in_zone, source,
    source_from_assignment, source_from_schedule,
};
pub(crate) use source::{config_decimal, config_value};
use stock::decrement_stock;
pub(crate) use stock::same_stock_signature;
pub(crate) use timing::cycle_bounds_in_zone;
pub(crate) use timing::dashboard_timing;
use timing::{applies_on, timing_allowed};
use uuid::Uuid;
use validation::parse_decimal;
pub(crate) use validation::valid_identifier;

type ApiError = OperationError;

#[derive(Clone, Debug)]
pub struct TakeRequest {
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
        available: available && !source.invalid_configured_amount,
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
        HouseholdId::from_database(tenant.scope().household_id),
        &SourceReference::parse(kind, id)?,
    )
    .await?;
    if source.medication_id.get() != medication_id {
        return Err(OperationError::NotFound);
    }
    access::require_participation(tenant, source.person_id.get()).await?;
    let mut preview = preview_source(source, taken_at.date_naive());
    preview.available &= taken_at.with_timezone(&Utc) <= Utc::now() + Duration::hours(1);
    Ok(preview)
}

#[derive(Clone, Debug)]
pub enum Request {
    Take(PreparedRequest),
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
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::PersonalApiKey => "personal_api_key",
            Self::ApiSession => "api_session",
            Self::ApiAppToken => "api_app_token",
            Self::OauthGrant => "oauth",
            Self::BrowserSession => "browser_session",
        }
    }

    pub(crate) fn reference_prefix(self) -> &'static str {
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
}

enum DoseConflict {
    PayloadMismatch,
    DoseChanged,
    IdempotencyKeyUnavailable,
}

impl DoseConflict {
    fn into_error(self) -> OperationError {
        let (code, details) = match self {
            Self::PayloadMismatch => (
                "conflict",
                json!({"error":"client_uuid was already used for a different dose"}),
            ),
            Self::DoseChanged => (
                "dose_changed",
                json!({"error":"Dose changed. Review the updated dose before recording."}),
            ),
            Self::IdempotencyKeyUnavailable => (
                "idempotency_key_unavailable",
                json!({"message":"Medication take idempotency key is unavailable"}),
            ),
        };
        OperationError::Conflict {
            code: code.to_owned(),
            details,
        }
    }
}
fn error(kind: ErrorKind, message: &str) -> OperationError {
    match kind {
        ErrorKind::Validation => OperationError::Validation {
            details: json!({"error": message}),
        },
    }
}
fn database_error(error: sea_orm::DbErr) -> OperationError {
    error.into()
}

enum TakeFailureCause {
    Paused,
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
                TakeFailureCause::OutOfStock => "out_of_stock",
            });
        }
        error
    }
}

pub async fn execute(
    tenant: &TenantTransaction,
    request: Request,
) -> Result<Outcome, OperationError> {
    execute_with_provenance(tenant, request, None).await
}

pub async fn execute_with_provenance(
    tenant: &TenantTransaction,
    request: Request,
    provenance: Option<&CredentialProvenance>,
) -> Result<Outcome, OperationError> {
    let Request::Take(request) = request;
    let context = DoseContext {
        tenant,
        provenance,
        zone: request.zone,
    };
    let (replayed, record) = writing::create_with_failure(
        tenant.transaction(),
        &context,
        HouseholdId::from_database(tenant.scope().household_id),
        &request,
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

async fn allowed_person(
    _: &DatabaseTransaction,
    context: &DoseContext<'_>,
    person_id: PersonId,
) -> Result<bool, OperationError> {
    match access::require_participation(context.tenant, person_id.get()).await {
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
