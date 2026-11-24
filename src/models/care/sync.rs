pub(crate) mod health_events;
mod input;
mod medicines;
mod occurrences;
mod operations;
mod pauses;
mod persistence;
mod portable;
pub(super) use portable::export_people;
pub(crate) use portable::{
    PortableFormat, api_export_payload, bundle_payload, ensure_bundle_stock_exportable,
    export_payload, mobile_payload,
};
mod reading;
mod replay;
pub(crate) mod review_prompts;
mod sources;

use crate::models::{
    access::{self, PersonAccess, TenantTransaction},
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
    identity::credentials::CredentialProvenance,
};
use crate::views::{
    DataResponse,
    sync::{AppliedBatch as BatchResponseData, BatchOperation, SyncResult},
};
use chrono::Utc;
pub(crate) use input::Operation;
pub(crate) use reading::{changes, snapshot};
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

pub async fn lock_batch(tenant: &TenantTransaction, body: &Value) -> Result<(), OperationError> {
    if input::parse(body)?
        .iter()
        .any(|operation| operation.resource_type == "person" && operation.action == "create")
    {
        super::guardian_declarations::lock_creator_account(tenant).await?;
    }
    lock(tenant).await
}

pub struct AppliedBatch<T = Value> {
    pub body: T,
    pub takes: Vec<medication_take::Model>,
}

pub(crate) async fn apply(
    tenant: &TenantTransaction,
    body: &Value,
    zone: chrono_tz::Tz,
    secret: Option<&std::sync::Arc<[u8]>>,
    provenance: &CredentialProvenance,
) -> Result<AppliedBatch<DataResponse<BatchResponseData>>, OperationError> {
    let operations = input::parse(body)?;
    let mut results = Vec::with_capacity(operations.len());
    let mut takes = Vec::new();
    for (index, operation) in operations.iter().enumerate() {
        let result = operations::apply(tenant, operation, zone, secret, provenance, &mut takes)
            .await
            .map_err(|error| validation_error(operation, error))?;
        results.push(BatchOperation {
            result,
            index,
            action: operation.action.clone(),
        });
    }
    Ok(AppliedBatch {
        body: DataResponse {
            data: BatchResponseData {
                applied: true,
                results,
            },
        },
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
    record_type: &'static str,
    record_id: i64,
    record_portable_id: Option<&str>,
    etag: Option<String>,
    replayed: Option<bool>,
) -> SyncResult {
    SyncResult {
        record_type,
        record_id: Some(record_id.to_string()),
        record_portable_id: record_portable_id.map(str::to_owned),
        etag,
        replayed,
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

#[cfg(test)]
mod response_tests {
    use super::*;

    #[test]
    fn operation_result_preserves_string_ids_and_omission() -> Result<(), serde_json::Error> {
        let response = result("MedicationTake", 42, Some("take-id"), None, Some(false));
        assert_eq!(response.record_id.as_deref(), Some("42"));
        assert_eq!(
            serde_json::to_value(response)?,
            json!({
                "record_type": "MedicationTake", "record_id": "42",
                "record_portable_id": "take-id", "replayed": false
            })
        );
        Ok(())
    }
}
