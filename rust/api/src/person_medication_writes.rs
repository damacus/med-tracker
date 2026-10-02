mod access;
mod dosing;
mod input;
mod replay;
mod responses;
mod sync;
mod writing;

use crate::dosage_options::{parse_decimal, storage_decimal, valid_identifier};
use crate::entities::{api_tombstone, dosage, grant, medication, person_medication};
use crate::medication_management::{
    error_response, finish_with_request_id, record_version, request_context,
};
use crate::mutation_idempotency::{self, Lookup, StoredResponse};
use crate::read_entities::{person, person_medication as read_assignment};
use crate::read_resources::serialize_assignments;
use crate::sync_batch::{SyncOperation, SyncResult};
use crate::sync_events::{record_change, SyncRecord};
use crate::{
    audit, database_error, granted_people, representation_etag, scope, ApiError, AppState,
    AuthContext,
};
use axum::extract::{rejection::JsonRejection, Path, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;
use sea_orm::prelude::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseTransaction, EntityTrait, IntoActiveModel,
    QueryFilter, QueryOrder, Set,
};
use serde_json::{json, Map, Value};
use uuid::Uuid;

const CONTROLLER: &str = "api/v1/person_medications";
const POLICY: &str = "PersonMedicationPolicy";
const DOSE_UNITS: &[&str] = &[
    "tablet", "capsule", "gummy", "mg", "ml", "g", "mcg", "IU", "spray", "drop", "sachet", "pad",
];

#[derive(Default)]
struct Attributes {
    person_id: Option<String>,
    medication_id: Option<String>,
    source_dosage_option_id: Option<String>,
    dose_amount: Option<Decimal>,
    dose_unit: Option<String>,
    administration_kind: Option<i32>,
    notes: Option<String>,
    max_daily_doses: Option<i32>,
    min_hours_between_doses: Option<Option<i32>>,
    dose_cycle: Option<i32>,
}

impl Attributes {
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
        let person_id = identifier(inner, "person_id").ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
        let medication_id =
            identifier(inner, "medication_id").ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
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
            .map(|value| match value.as_str()? {
                "routine" => Some(0),
                "as_needed" => Some(1),
                _ => None,
            })
            .map_or(Some(None), |value| value.map(Some))
            .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
        let dose_cycle = inner
            .get("dose_cycle")
            .map(|value| match value.as_str()? {
                "daily" => Some(0),
                "weekly" => Some(1),
                "monthly" => Some(2),
                _ => None,
            })
            .map_or(Some(None), |value| value.map(Some))
            .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
        Ok(Self {
            person_id,
            medication_id,
            source_dosage_option_id: identifier(inner, "source_dosage_option_id")
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

struct WriteCompletion<'a> {
    context: &'a AuthContext,
    headers: &'a HeaderMap,
    method: &'a str,
    action: &'a str,
    request_path: &'a str,
    request_digest: &'a str,
    request_id: &'a str,
    status: StatusCode,
    body: Value,
    etag: &'a str,
}

use access::{
    can_manage_person, find_assignment, find_medication, find_person, find_visible_option,
};
use dosing::resolved_dose;
use input::{
    decimal, identifier, numeric_10_2, optional_string, positive_integer, required_string,
    whole_hours,
};
use replay::replay_or_conflict;
use responses::{failure, finish_write, path, representation, validation_failure};
pub(super) use sync::{apply_sync_operation, authorize_sync_replay};
pub(super) use writing::{create, patch, put};
