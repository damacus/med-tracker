mod dosing;
mod input;
mod reading;
mod retirement;
mod writing;
use crate::models::{
    access::{self, PersonAccess, TenantTransaction},
    care::{
        administration,
        doses::CredentialProvenance,
        treatment_command::{
            AdministrationKind, DosageOptionId, DoseCycle, MedicationId, PersonId,
        },
        valid_identifier,
    },
    entities::{api_change_event, dosage, household, medication, person, person_medication},
    errors::OperationError,
};
use axum::http::StatusCode;
use chrono::Utc;
pub use reading::{Pagination, list, read};
pub use retirement::unassign;
use sea_orm::prelude::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde_json::{Map, Value, json};
use std::str::FromStr;
use uuid::Uuid;
pub(crate) use writing::clinical_snapshot;
pub use writing::{authorize_create, authorize_update, create, update};
pub(crate) use writing::{create_input, update_input};
type AuthContext = TenantTransaction;
type ApiError = OperationError;
const DOSE_UNITS: &[&str] = &[
    "tablet", "capsule", "gummy", "mg", "ml", "g", "mcg", "IU", "spray", "drop", "sachet", "pad",
];
use input::{
    decimal, identifier, numeric_10_2, optional_string, positive_integer, required_string,
    whole_hours,
};
fn database_error(_: sea_orm::DbErr) -> OperationError {
    OperationError::Unavailable
}
fn parse_decimal(value: &Value) -> Option<Decimal> {
    value
        .as_str()
        .and_then(|value| Decimal::from_str(value).ok())
}
fn storage_decimal(amount: Decimal, integer: u32, places: u32) -> Option<Decimal> {
    (amount.abs() < Decimal::from(10_i64.pow(integer)) && amount.normalize().scale() <= places)
        .then_some(amount)
}
fn invalid() -> OperationError {
    OperationError::Validation {
        details: json!({"errors":{"person_medication":["is invalid"]}}),
    }
}
pub fn validate_body(body: &Value, create: bool) -> Result<(), StatusCode> {
    Attributes::parse(body, create).map(|_| ())
}
#[derive(Default)]
pub(crate) struct Attributes {
    pub(crate) person_id: Option<PersonId>,
    pub(crate) medication_id: Option<MedicationId>,
    pub(crate) source_dosage_option_id: Option<DosageOptionId>,
    pub(crate) dose_amount: Option<Decimal>,
    pub(crate) dose_unit: Option<String>,
    pub(crate) administration_kind: Option<AdministrationKind>,
    pub(crate) notes: Option<String>,
    pub(crate) max_daily_doses: Option<i32>,
    pub(crate) min_hours_between_doses: Option<Option<i32>>,
    pub(crate) dose_cycle: Option<DoseCycle>,
}
pub(crate) struct ValidatedAssignmentInput(Attributes);

impl ValidatedAssignmentInput {
    fn into_inner(self) -> Attributes {
        self.0
    }
}

impl Attributes {
    pub(crate) fn validated(
        self,
        create: bool,
    ) -> Result<ValidatedAssignmentInput, OperationError> {
        if create && (self.person_id.is_none() || self.medication_id.is_none()) {
            return Err(invalid());
        }
        if self.dose_amount.is_some_and(|amount| !numeric_10_2(amount))
            || self
                .dose_unit
                .as_ref()
                .is_some_and(|unit| !DOSE_UNITS.contains(&unit.as_str()))
            || self.max_daily_doses.is_some_and(|value| value <= 0)
            || self
                .min_hours_between_doses
                .is_some_and(|value| value.is_some_and(|hours| hours <= 0))
        {
            return Err(invalid());
        }
        Ok(ValidatedAssignmentInput(self))
    }

    fn parse(body: &Value, create: bool) -> Result<Self, StatusCode> {
        let outer = body.as_object().ok_or(StatusCode::BAD_REQUEST)?;
        let inner = outer
            .get("person_medication")
            .ok_or(StatusCode::BAD_REQUEST)?
            .as_object()
            .ok_or(StatusCode::BAD_REQUEST)?;
        if outer.len() != 1
            || inner.is_empty()
            || inner.keys().any(|key| {
                !matches!(
                    key.as_str(),
                    "person_id"
                        | "medication_id"
                        | "source_dosage_option_id"
                        | "dose_amount"
                        | "dose_unit"
                        | "administration_kind"
                        | "notes"
                        | "max_daily_doses"
                        | "min_hours_between_doses"
                        | "dose_cycle"
                )
            })
        {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let person_id =
            identifier::<PersonId>(inner, "person_id").ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
        let medication_id = identifier::<MedicationId>(inner, "medication_id")
            .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
        if create && (person_id.is_none() || medication_id.is_none()) {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let dose_unit =
            required_string(inner, "dose_unit").ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
        if dose_unit
            .as_deref()
            .is_some_and(|unit| !DOSE_UNITS.contains(&unit))
        {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let administration_kind = inner
            .get("administration_kind")
            .map(|value| AdministrationKind::parse(value.as_str()?))
            .map_or(Some(None), |value| value.map(Some))
            .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
        let dose_cycle = inner
            .get("dose_cycle")
            .map(|value| DoseCycle::parse(value.as_str()?))
            .map_or(Some(None), |value| value.map(Some))
            .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
        Ok(Self {
            person_id,
            medication_id,
            source_dosage_option_id: identifier::<DosageOptionId>(inner, "source_dosage_option_id")
                .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?,
            dose_amount: decimal(inner, "dose_amount").ok_or(StatusCode::UNPROCESSABLE_ENTITY)?,
            dose_unit,
            administration_kind,
            notes: optional_string(inner, "notes").ok_or(StatusCode::UNPROCESSABLE_ENTITY)?,
            max_daily_doses: positive_integer(inner, "max_daily_doses")
                .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?,
            min_hours_between_doses: whole_hours(inner).ok_or(StatusCode::UNPROCESSABLE_ENTITY)?,
            dose_cycle,
        })
    }
}
