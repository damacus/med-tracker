pub(crate) mod health_events;
mod input;
mod medicines;
mod occurrences;
mod operations;
mod pauses;
mod persistence;
mod portable;
mod reading;
mod replay;
pub(crate) mod review_prompts;
mod sources;

use crate::models::{
    access::{self, PersonAccess, TenantTransaction},
    care::doses::CredentialProvenance,
    care::{
        assignments, dosages, doses, locations, medications, orders, pause_periods, people,
        treatments,
    },
    entities::{
        api_change_event, api_tombstone, dosage, dose_occurrence, health_event,
        health_event_medication, household, medication, medication_take, pause_period, person,
        person_medication, review_prompt, schedule, security_audit_event,
    },
    errors::OperationError,
};
use chrono::Utc;
pub(crate) use input::Operation;
pub use reading::{changes, snapshot};
pub use replay::authorize_replay;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde_json::{Map, Value, json};
use std::collections::{HashMap, HashSet};

pub async fn lock(tenant: &TenantTransaction) -> Result<(), OperationError> {
    household::Entity::find_by_id(tenant.scope().household_id)
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    access::recheck(tenant).await
}

pub struct AppliedBatch {
    pub body: Value,
    pub takes: Vec<medication_take::Model>,
}

pub async fn apply(
    tenant: &TenantTransaction,
    body: &Value,
    zone: chrono_tz::Tz,
    secret: Option<&std::sync::Arc<[u8]>>,
    provenance: &CredentialProvenance,
) -> Result<AppliedBatch, OperationError> {
    let operations = input::parse(body)?;
    let mut results = Vec::with_capacity(operations.len());
    let mut takes = Vec::new();
    for (index, operation) in operations.iter().enumerate() {
        let mut result = operations::apply(tenant, operation, zone, secret, provenance, &mut takes)
            .await
            .map_err(|error| validation_error(operation, error))?;
        result["index"] = json!(index);
        result["action"] = json!(operation.action);
        results.push(result);
    }
    Ok(AppliedBatch {
        body: json!({"data":{"applied":true,"results":results}}),
        takes,
    })
}

fn validation_error(operation: &Operation, error: OperationError) -> OperationError {
    let OperationError::Validation { mut details } = error else {
        return error;
    };
    if details["message"].is_string()
        || details["error"].as_str().is_some_and(|message| {
            !matches!(
                message,
                "Validation failed" | "Stock removal could not be recorded"
            )
        })
    {
        return OperationError::Validation { details };
    }
    let message = match operation.resource_type.as_str() {
        "location" => "Location attributes are invalid",
        "medication" if operation.action == "remove_stock" => "Stock removal is invalid",
        "medication" => "Medication attributes are invalid",
        "medication_dosage_option" => "Dosage option attributes are invalid",
        "person" => "Person is invalid",
        "schedule" => "Schedule is invalid",
        "person_medication" => "Person medication is invalid",
        _ => return OperationError::Validation { details },
    };
    if let Some(fields) = details.as_object_mut() {
        fields.insert("message".into(), json!(message));
    }
    OperationError::Validation { details }
}

fn result(
    record_type: &str,
    id: i64,
    portable_id: Option<&str>,
    etag: Option<String>,
    replayed: Option<bool>,
) -> Value {
    let mut value = json!({"record_type":record_type,"record_id":id.to_string()});
    if let Some(id) = portable_id {
        value["record_portable_id"] = json!(id);
    }
    if let Some(etag) = etag {
        value["etag"] = json!(etag);
    }
    if let Some(replayed) = replayed {
        value["replayed"] = json!(replayed);
    }
    value
}

struct SyncResult {
    record_type: &'static str,
    record_id: Option<i64>,
    record_portable_id: Option<String>,
    etag: Option<String>,
    replayed: Option<bool>,
}
impl SyncResult {
    fn value(self) -> Value {
        let mut value = json!({"record_type":self.record_type});
        if let Some(id) = self.record_id {
            value["record_id"] = json!(id.to_string());
        }
        if let Some(id) = self.record_portable_id {
            value["record_portable_id"] = json!(id);
        }
        if let Some(etag) = self.etag {
            value["etag"] = json!(etag);
        }
        if let Some(replayed) = self.replayed {
            value["replayed"] = json!(replayed);
        }
        value
    }
}

fn required_etag(operation: &Operation, current: &str) -> Result<(), OperationError> {
    match operation
        .if_match
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        None => Err(OperationError::Conflict {
            code: "precondition_required".into(),
            details: json!({"error":match operation.resource_type.as_str() {
                "person" | "schedule" | "person_medication" | "medication_pause_period" => "A current resource version is required",
                "medication_dose_occurrence" => "A current version is required",
                _ => "if_match is required",
            }}),
        }),
        Some(value) if value != current => Err(OperationError::Conflict {
            code: "sync_conflict".into(),
            details: json!({"error":if operation.resource_type == "medication_dose_occurrence" {"Occurrence has changed"} else {"Record has changed since it was last read"}}),
        }),
        Some(_) => Ok(()),
    }
}

fn envelope(operation: &Operation) -> Value {
    let key = if operation.resource_type == "medication_dosage_option" {
        "dosage_option"
    } else {
        operation.resource_type.as_str()
    };
    json!({key: operation.attributes})
}
