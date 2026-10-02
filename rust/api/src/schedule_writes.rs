mod access;
mod input;
mod replay;
mod representation;
mod responses;
mod sync;
mod writing;

use crate::audit;
use crate::dosage_options::{parse_decimal, storage_decimal, valid_identifier};
use crate::entities::{api_tombstone, dosage, grant, person, schedule};
use crate::medication_management::{
    error_response, finish_with_request_id, record_version, request_context, visible_medication,
};
use crate::mutation_idempotency::{self, Lookup, StoredResponse};
use crate::read_entities::schedule as read_schedule;
use crate::read_resources::serialize_schedules;
use crate::sync_batch::{SyncOperation, SyncResult};
use crate::sync_events::{record_change, SyncRecord};
use crate::{database_error, representation_etag, ApiError, AppState, AuthContext};
use axum::extract::{rejection::JsonRejection, Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use chrono::{NaiveDate, Utc};
use sea_orm::prelude::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseTransaction, EntityTrait, QueryFilter, Set,
};
use serde_json::{json, Map, Value};
use uuid::Uuid;

const CONTROLLER: &str = "api/v1/schedules";
const POLICY: &str = "SchedulePolicy";
const UNITS: &[&str] = &[
    "tablet", "capsule", "gummy", "mg", "ml", "g", "mcg", "IU", "spray", "drop", "sachet", "pad",
];
const FIELDS: &[&str] = &[
    "person_id",
    "medication_id",
    "source_dosage_option_id",
    "dose_amount",
    "dose_unit",
    "frequency",
    "start_date",
    "end_date",
    "notes",
    "max_daily_doses",
    "min_hours_between_doses",
    "dose_cycle",
    "schedule_type",
    "schedule_config",
];

enum InputFailure {
    Malformed,
    NotFound,
    Forbidden,
    Invalid(&'static str, &'static str),
    InvalidFields(Value),
}

#[derive(PartialEq)]
struct ScheduleFields {
    person_id: Option<i64>,
    medication_id: Option<i64>,
    source_dosage_option_id: Option<i64>,
    dose_amount: Option<Decimal>,
    dose_unit: Option<String>,
    frequency: Option<String>,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    notes: Option<String>,
    max_daily_doses: Option<i32>,
    min_hours_between_doses: Option<i32>,
    dose_cycle: Option<i32>,
    schedule_type: i32,
    schedule_config: Value,
}

impl ScheduleFields {
    fn new() -> Self {
        Self {
            person_id: None,
            medication_id: None,
            source_dosage_option_id: None,
            dose_amount: None,
            dose_unit: None,
            frequency: None,
            start_date: None,
            end_date: None,
            notes: None,
            max_daily_doses: Some(4),
            min_hours_between_doses: None,
            dose_cycle: None,
            schedule_type: 0,
            schedule_config: json!({}),
        }
    }

    fn from_record(record: &schedule::Model) -> Self {
        Self {
            person_id: Some(record.person_id),
            medication_id: Some(record.medication_id),
            source_dosage_option_id: record.source_dosage_option_id,
            dose_amount: record.dose_amount,
            dose_unit: record.dose_unit.clone(),
            frequency: record.frequency.clone(),
            start_date: record.start_date,
            end_date: record.end_date,
            notes: record.notes.clone(),
            max_daily_doses: record.max_daily_doses,
            min_hours_between_doses: record.min_hours_between_doses,
            dose_cycle: record.dose_cycle,
            schedule_type: record.schedule_type,
            schedule_config: record.schedule_config.clone(),
        }
    }

    fn validate(&self) -> Result<(), InputFailure> {
        let mut errors = Map::new();
        if self.person_id.is_none() {
            errors.insert("person_id".to_owned(), json!(["can't be blank"]));
        }
        if self.medication_id.is_none() {
            errors.insert("medication_id".to_owned(), json!(["can't be blank"]));
        }
        if self.dose_amount.is_none() {
            errors.insert("dose_amount".to_owned(), json!(["can't be blank"]));
        }
        if self.dose_unit.is_none() {
            errors.insert("dose_unit".to_owned(), json!(["can't be blank"]));
        }
        if self.start_date.is_none() {
            errors.insert("start_date".to_owned(), json!(["can't be blank"]));
        }
        if self.end_date.is_none() {
            errors.insert("end_date".to_owned(), json!(["can't be blank"]));
        }
        if let (Some(start), Some(end)) = (self.start_date, self.end_date) {
            if end < start {
                errors.insert(
                    "end_date".to_owned(),
                    json!(["must be after the start date"]),
                );
            }
        }
        if !errors.is_empty() {
            return Err(InputFailure::InvalidFields(Value::Object(errors)));
        }
        Ok(())
    }

    fn assign(self, active: &mut schedule::ActiveModel) {
        active.person_id = Set(self.person_id.unwrap());
        active.medication_id = Set(self.medication_id.unwrap());
        active.source_dosage_option_id = Set(self.source_dosage_option_id);
        active.dose_amount = Set(self.dose_amount);
        active.dose_unit = Set(self.dose_unit);
        active.frequency = Set(self.frequency);
        active.start_date = Set(self.start_date);
        active.end_date = Set(self.end_date);
        active.notes = Set(self.notes);
        active.max_daily_doses = Set(self.max_daily_doses);
        active.min_hours_between_doses = Set(self.min_hours_between_doses);
        active.dose_cycle = Set(self.dose_cycle);
        active.schedule_type = Set(self.schedule_type);
        active.schedule_config = Set(self.schedule_config);
    }
}

enum ReplayDecision {
    New,
    Replay(Response),
    Conflict,
}

use access::{find_dosage, find_person, find_schedule, person_access};
use input::{apply_attributes, attributes, identifier};
use replay::{keyed_replay, store_key};
use representation::{schedule_body, snapshot};
use responses::{input_error, keyed_validation};
pub(super) use sync::{apply_sync_operation, authorize_sync_replay};
pub(super) use writing::{create, patch, put};
