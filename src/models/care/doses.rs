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
        api_change_event, dosage, household, medication, medication_take, membership,
        person_medication, schedule,
    },
    errors::OperationError,
};
use audit::{StockVersionChange, record_domain_audit, stock_version};
use changes::{SyncRecord, record_change};
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc};
use input::{ProposedTake, prepare};
use locking::{lock_dosage, lock_household, lock_medication};
use persistence::insert_take;
use replay::{existing_take, lock_client_uuid, replay_matches};
use sea_orm::prelude::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DatabaseTransaction, DbBackend,
    EntityTrait, PrimaryKeyTrait, QueryFilter, QuerySelect, Set, Statement,
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
pub(crate) use stock::{same_stock_signature, stock_candidate_condition};
pub(crate) use timing::dashboard_timing;
use timing::{applies_on, timing_allowed};
use uuid::Uuid;
use validation::parse_decimal_str;
pub(crate) use validation::valid_identifier;

type ApiError = OperationError;

#[derive(Clone, Debug)]
pub struct TakeInput {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceId {
    Database(i64),
    Portable(String),
}

impl SourceId {
    fn parse(value: &str) -> Result<Self, OperationError> {
        if !valid_identifier(value) {
            return Err(error(ErrorKind::Validation, "invalid medication source"));
        }
        Ok(value
            .parse::<i64>()
            .map_or_else(|_| Self::Portable(value.to_owned()), Self::Database))
    }

    fn matches(&self, id: i64, portable_id: &str) -> bool {
        match self {
            Self::Database(value) => *value == id,
            Self::Portable(value) => value == portable_id,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduleId(SourceId);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssignmentId(SourceId);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DoseSource {
    Schedule(ScheduleId),
    Assignment(AssignmentId),
}

impl DoseSource {
    fn parse(kind: &str, id: &str) -> Result<Self, OperationError> {
        if kind.trim().is_empty() {
            return Err(error(ErrorKind::Validation, "invalid medication source"));
        }
        let id = SourceId::parse(id)?;
        match kind {
            "schedule" => Ok(Self::Schedule(ScheduleId(id))),
            "person_medication" => Ok(Self::Assignment(AssignmentId(id))),
            _ => Err(OperationError::NotFound),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MedicationId(i64);

#[derive(Clone, Debug)]
pub struct Take {
    client_uuid: Option<Uuid>,
    source: DoseSource,
    taken_at: NaiveDateTime,
    dose_amount: Option<Decimal>,
    dose_unit: Option<String>,
    taken_from_medication_id: Option<MedicationId>,
    expected_effective_amount: Option<Decimal>,
    expected_effective_unit: Option<String>,
}

pub enum TakeAttempt {
    Ready(Take),
    MalformedReplay(MalformedReplay),
}

pub struct MalformedReplay {
    client_uuid: Uuid,
    original_error: OperationError,
}

impl TakeInput {
    pub fn attempt(self) -> Result<TakeAttempt, OperationError> {
        let source = DoseSource::parse(&self.source_type, &self.source_id)?;
        let taken_at = input::parse_input_time(&self.taken_at);
        let client_uuid = self
            .client_uuid
            .as_deref()
            .map(Uuid::parse_str)
            .transpose()
            .map_err(|_| error(ErrorKind::Validation, "invalid client_uuid"));
        let dose_amount = self
            .dose_amount
            .as_deref()
            .map(|value| {
                let amount = parse_decimal_str(value)
                    .ok_or_else(|| error(ErrorKind::Validation, "Invalid dose configured"))?;
                if !input::valid_numeric_10_2(amount) {
                    return Err(error(ErrorKind::Validation, "Invalid dose configured"));
                }
                Ok(amount)
            })
            .transpose();
        let expected_effective_amount = self
            .expected_effective_amount
            .as_deref()
            .map(Decimal::from_str)
            .transpose()
            .map_err(|_| error(ErrorKind::Validation, "Invalid dose configured"));
        let malformed_amount = dose_amount.is_err();
        let expected_valid = expected_effective_amount.is_ok()
            || (malformed_amount
                && self.expected_effective_amount.as_deref() == self.dose_amount.as_deref());
        if (taken_at.is_err() || malformed_amount)
            && let Ok(Some(client_uuid)) = client_uuid.as_ref()
            && expected_valid
            && self
                .dose_unit
                .as_deref()
                .is_none_or(|value| !value.is_empty())
            && self.taken_from_medication_id.is_none_or(|id| id > 0)
        {
            let original_error = match taken_at {
                Err(error) => error,
                Ok(_) => dose_amount.expect_err("malformed amount was checked"),
            };
            return Ok(TakeAttempt::MalformedReplay(MalformedReplay {
                client_uuid: *client_uuid,
                original_error,
            }));
        }
        let taken_at = taken_at?;
        let client_uuid = client_uuid?;
        let dose_amount = dose_amount?;
        if self.dose_unit.as_deref().is_some_and(str::is_empty) {
            return Err(error(ErrorKind::Validation, "invalid dose_unit"));
        }
        let taken_from_medication_id = self
            .taken_from_medication_id
            .map(|id| {
                (id > 0)
                    .then_some(MedicationId(id))
                    .ok_or_else(|| error(ErrorKind::Validation, "invalid taken_from_medication_id"))
            })
            .transpose()?;
        let expected_effective_amount = expected_effective_amount?;
        Ok(TakeAttempt::Ready(Take {
            client_uuid,
            source,
            taken_at,
            dose_amount,
            dose_unit: self.dose_unit,
            taken_from_medication_id,
            expected_effective_amount,
            expected_effective_unit: self.expected_effective_unit,
        }))
    }

    pub fn validate(self) -> Result<Take, OperationError> {
        match self.attempt()? {
            TakeAttempt::Ready(take) => Ok(take),
            TakeAttempt::MalformedReplay(malformed) => Err(malformed.original_error),
        }
    }
}

#[cfg(test)]
mod typed_take_tests {
    use super::*;

    fn input() -> TakeInput {
        TakeInput {
            client_uuid: Some("006B49D9-1DA2-42F1-800B-8C80867AEE1C".into()),
            source_type: "schedule".into(),
            source_id: "0199a4b5-c6d7-8e9f-a012-3456789abcde".into(),
            taken_at: "2026-10-05T11:00:00.123456+01:00".into(),
            dose_amount: Some("2.30".into()),
            dose_unit: Some("tablet".into()),
            taken_from_medication_id: Some(80001),
            expected_effective_amount: None,
            expected_effective_unit: None,
        }
    }

    #[test]
    fn take_validation_preserves_portable_source_microseconds_and_exact_amount() {
        let command = input().validate().unwrap();
        assert_eq!(
            command.client_uuid.unwrap().hyphenated().to_string(),
            "006b49d9-1da2-42f1-800b-8c80867aee1c"
        );
        assert!(matches!(
            command.source,
            DoseSource::Schedule(ScheduleId(SourceId::Portable(_)))
        ));
        assert_eq!(command.taken_at.to_string(), "2026-10-05 10:00:00.123456");
        assert_eq!(
            command.dose_amount.unwrap(),
            Decimal::from_str_exact("2.30").unwrap()
        );
    }

    #[test]
    fn take_validation_rejects_invalid_source_time_uuid_and_exact_amount() {
        let mut malformed = input();
        malformed.source_id = "../other-household".into();
        assert!(matches!(
            malformed.validate(),
            Err(OperationError::Validation { .. })
        ));
        let mut malformed = input();
        malformed.source_type = "other".into();
        assert!(matches!(
            malformed.validate(),
            Err(OperationError::NotFound)
        ));
        let mut malformed = input();
        malformed.taken_at = "2026-10-05 11:00".into();
        assert!(matches!(
            malformed.validate(),
            Err(OperationError::Validation { .. })
        ));
        let mut malformed = input();
        malformed.client_uuid = Some("invalid".into());
        assert!(matches!(
            malformed.validate(),
            Err(OperationError::Validation { .. })
        ));
        for amount in ["-1", "1.234", "100000000", "1e2", "+1"] {
            let mut malformed = input();
            malformed.dose_amount = Some(amount.into());
            assert!(
                matches!(malformed.validate(), Err(OperationError::Validation { .. })),
                "accepted {amount}"
            );
        }
        let mut malformed = input();
        malformed.dose_unit = Some(String::new());
        assert!(matches!(
            malformed.validate(),
            Err(OperationError::Validation { .. })
        ));
        let mut malformed = input();
        malformed.taken_from_medication_id = Some(0);
        assert!(matches!(
            malformed.validate(),
            Err(OperationError::Validation { .. })
        ));
    }

    #[test]
    fn browser_confirmation_amount_can_be_a_malformed_retry_attempt() {
        let mut confirmed = input();
        confirmed.dose_amount = Some("not-a-decimal".into());
        confirmed.expected_effective_amount = confirmed.dose_amount.clone();
        assert!(matches!(
            confirmed.attempt(),
            Ok(TakeAttempt::MalformedReplay(_))
        ));

        let mut unrelated = input();
        unrelated.expected_effective_amount = Some("not-a-decimal".into());
        assert!(matches!(
            unrelated.attempt(),
            Err(OperationError::Validation { .. })
        ));
    }
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
    let requested_source = DoseSource::parse(kind, id).map_err(|_| OperationError::NotFound)?;
    let source = source(
        tenant.transaction(),
        &context,
        tenant.scope().household_id,
        &requested_source,
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
    let context = DoseContext {
        tenant,
        provenance,
        zone,
    };
    let (replayed, record) = writing::create_with_failure(
        tenant.transaction(),
        &context,
        tenant.scope().household_id,
        &take,
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

pub async fn execute_attempt_in_timezone(
    tenant: &TenantTransaction,
    attempt: TakeAttempt,
    zone: chrono_tz::Tz,
    provenance: Option<&CredentialProvenance>,
) -> Result<Outcome, OperationError> {
    match attempt {
        TakeAttempt::Ready(take) => {
            execute_in_timezone(tenant, Command::Take(take), zone, provenance).await
        }
        TakeAttempt::MalformedReplay(malformed) => {
            let context = DoseContext {
                tenant,
                provenance,
                zone,
            };
            let db = tenant.transaction();
            let household_id = tenant.scope().household_id;
            lock_household(db, household_id).await?;
            access::recheck(tenant).await?;
            lock_client_uuid(db, malformed.client_uuid).await?;
            let existing = match existing_take(db, household_id, malformed.client_uuid).await {
                Ok(existing) => existing,
                Err(OperationError::Conflict { .. }) => return Err(malformed.original_error),
                Err(problem) => return Err(problem),
            };
            if let Some(existing) = existing {
                match replay::authorize_existing_take(db, &context, household_id, &existing).await {
                    Ok(_) => {
                        return Err(error(
                            ErrorKind::Conflict,
                            "client_uuid was already used for a different dose",
                        ));
                    }
                    Err(OperationError::Forbidden | OperationError::NotFound) => {
                        return Err(malformed.original_error);
                    }
                    Err(problem) => return Err(problem),
                }
            }
            Err(malformed.original_error)
        }
    }
}

pub async fn execute_with_provenance(
    tenant: &TenantTransaction,
    command: Command,
    provenance: Option<&CredentialProvenance>,
) -> Result<Outcome, OperationError> {
    execute_in_timezone(tenant, command, app_zone(tenant.settings()), provenance).await
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
