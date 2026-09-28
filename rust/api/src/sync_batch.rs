use crate::medication_management::{finish_with_request_id, request_context};
use crate::mutation_idempotency::{self, Lookup, StoredResponse};
use crate::{audit, database_error, ApiError, AppState, AuthContext};
use axum::extract::{rejection::JsonRejection, Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use sea_orm::{DatabaseTransaction, TransactionTrait};
use serde_json::{json, Map, Value};
use std::sync::Arc;
use uuid::Uuid;

const CONTROLLER: &str = "api/v1/sync/batches";
const POLICY: &str = "HouseholdPolicy";
const ACTION: &str = "create";

pub(super) struct SyncOperation {
    pub resource_type: String,
    pub action: String,
    pub id: Option<String>,
    pub if_match: Option<String>,
    pub attributes: Map<String, Value>,
}

pub(super) struct SyncResult {
    pub record_type: &'static str,
    pub record_id: Option<i64>,
    pub record_portable_id: Option<String>,
    pub etag: Option<String>,
    pub replayed: Option<bool>,
}

impl SyncResult {
    fn value(self, index: usize, action: &str) -> Value {
        let mut result = json!({"index": index, "action": action, "record_type": self.record_type});
        if let Some(record_id) = self.record_id {
            result["record_id"] = json!(record_id.to_string());
        }
        if let Some(portable_id) = self.record_portable_id {
            result["record_portable_id"] = json!(portable_id);
        }
        if let Some(etag) = self.etag {
            result["etag"] = json!(etag);
        }
        if let Some(replayed) = self.replayed {
            result["replayed"] = json!(replayed);
        }
        result
    }
}

pub(super) fn routes() -> Router<AppState> {
    Router::new().route(
        "/api/v1/households/{household_id}/sync/batches",
        post(create),
    )
}

fn error(status: StatusCode, code: &'static str, message: &'static str) -> ApiError {
    ApiError {
        status,
        code,
        message,
        preserve_activity: false,
    }
}

fn invalid() -> ApiError {
    error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "unprocessable_content",
        "Batch is invalid",
    )
}

fn identifier_matches_result(identifier: Option<&str>, result: &Value) -> bool {
    identifier.is_some_and(|identifier| {
        result.get("record_portable_id").and_then(Value::as_str) == Some(identifier)
            || result.get("record_id").and_then(Value::as_str) == Some(identifier)
    })
}

fn superseding_delete<'a>(
    operations: &'a [SyncOperation],
    results: &'a [Value],
    index: usize,
) -> Option<(&'a SyncOperation, &'a Value)> {
    let operation = &operations[index];
    let result = &results[index];
    if operation.action == "delete" || !identifier_matches_result(operation.id.as_deref(), result) {
        return None;
    }
    operations
        .iter()
        .zip(results)
        .enumerate()
        .skip(index + 1)
        .find_map(|(later_index, (later, saved))| {
            (later.resource_type == operation.resource_type
                && later.action == "delete"
                && saved.get("index").and_then(Value::as_u64) == Some(later_index as u64)
                && saved.get("action").and_then(Value::as_str) == Some("delete")
                && saved.get("record_type") == result.get("record_type")
                && saved.get("record_id") == result.get("record_id")
                && saved.get("record_portable_id") == result.get("record_portable_id")
                && identifier_matches_result(later.id.as_deref(), saved))
            .then_some((later, saved))
        })
}

fn allowed_action(resource_type: &str, action: &str) -> bool {
    let actions: &[&str] = match resource_type {
        "medication_take" => &["create"],
        "medication_dose_occurrence" => &["create", "update"],
        "medication_pause_period" => &["create", "close"],
        "medication" => &[
            "create",
            "update",
            "delete",
            "adjust_inventory",
            "mark_as_ordered",
            "mark_as_received",
            "remove_stock",
        ],
        "medication_dosage_option" => &["create", "update"],
        "person" => &["create", "update"],
        "health_event" | "location" => &["create", "update", "delete"],
        "medication_review_prompt" => &["update"],
        "schedule" => &["create", "update", "delete", "pause", "resume"],
        "person_medication" => &["create", "update", "delete", "pause", "resume", "reorder"],
        _ => &[],
    };
    actions.contains(&action)
}

fn parse_operations(request: &Value) -> Result<Vec<SyncOperation>, ApiError> {
    let outer = request.as_object().ok_or_else(invalid)?;
    if outer.len() != 1 {
        return Err(invalid());
    }
    let batch = outer
        .get("batch")
        .and_then(Value::as_object)
        .ok_or_else(invalid)?;
    if batch.len() != 1 {
        return Err(invalid());
    }
    let operations = batch
        .get("operations")
        .and_then(Value::as_array)
        .filter(|rows| !rows.is_empty())
        .ok_or_else(invalid)?;
    operations
        .iter()
        .map(|row| {
            let item = row.as_object().ok_or_else(invalid)?;
            if item.keys().any(|key| {
                !matches!(
                    key.as_str(),
                    "resource_type" | "action" | "id" | "if_match" | "attributes"
                )
            }) {
                return Err(invalid());
            }
            let resource_type = item
                .get("resource_type")
                .and_then(Value::as_str)
                .ok_or_else(invalid)?;
            let action = item
                .get("action")
                .and_then(Value::as_str)
                .ok_or_else(invalid)?;
            if !allowed_action(resource_type, action) {
                return Err(error(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "sync_operation_unsupported",
                    "Operation is not supported offline",
                ));
            }
            let id = match item.get("id") {
                None => None,
                Some(Value::String(id)) if crate::dosage_options::valid_identifier(id) => {
                    Some(id.clone())
                }
                _ => return Err(invalid()),
            };
            let if_match = match item.get("if_match") {
                None => None,
                Some(Value::String(etag)) => Some(etag.clone()),
                _ => return Err(invalid()),
            };
            let attributes = match item.get("attributes") {
                None => Map::new(),
                Some(Value::Object(attributes)) => attributes.clone(),
                _ => return Err(invalid()),
            };
            if action != "create" && id.is_none() {
                return Err(invalid());
            }
            Ok(SyncOperation {
                resource_type: resource_type.to_owned(),
                action: action.to_owned(),
                id,
                if_match,
                attributes,
            })
        })
        .collect()
}

pub(super) async fn create(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    let (db, _) = request_context(&state, &headers, household_id).await?;
    let (_, context) = mutation_idempotency::lock_household_and_reauthenticate(
        &state,
        &db,
        &headers,
        household_id,
    )
    .await?;
    let path = format!("/api/v1/households/{household_id}/sync/batches");
    let Json(request) = match payload {
        Ok(value) => value,
        Err(_) => {
            let request_id = Uuid::new_v4().to_string();
            return failed(
                db,
                &context,
                None,
                &path,
                "",
                &request_id,
                error(
                    StatusCode::BAD_REQUEST,
                    "bad_request",
                    "Invalid JSON request body",
                ),
            )
            .await;
        }
    };
    let key = mutation_idempotency::key(&headers);
    let digest = mutation_idempotency::digest("POST", &path, &request);
    if let Some(key) = key {
        match mutation_idempotency::lookup(&db, &context, key, "POST", &path, &digest).await? {
            Lookup::New => {}
            Lookup::Conflict => {
                let request_id = Uuid::new_v4().to_string();
                return failed(
                    db,
                    &context,
                    None,
                    &path,
                    &digest,
                    &request_id,
                    error(
                        StatusCode::CONFLICT,
                        "idempotency_key_reused",
                        "Idempotency key was already used",
                    ),
                )
                .await;
            }
            Lookup::Replay(saved) => {
                let status = StatusCode::from_u16(saved.response_status as u16)
                    .map_err(|_| ApiError::internal())?;
                if status.is_success() {
                    let operations = match parse_operations(&request) {
                        Ok(operations) => operations,
                        Err(problem) => {
                            let request_id = Uuid::new_v4().to_string();
                            return failed(
                                db,
                                &context,
                                None,
                                &path,
                                &digest,
                                &request_id,
                                problem,
                            )
                            .await;
                        }
                    };
                    if let Err(problem) = authorize_replay(
                        &db,
                        &context,
                        &operations,
                        &saved.response_body,
                        saved
                            .response_headers
                            .get("x-request-id")
                            .and_then(Value::as_str),
                    )
                    .await
                    {
                        let request_id = Uuid::new_v4().to_string();
                        return failed(db, &context, None, &path, &digest, &request_id, problem)
                            .await;
                    }
                }
                let request_id = Uuid::new_v4().to_string();
                audit::record_resource_request_with_id(
                    &db,
                    &context,
                    &request_id,
                    "POST",
                    CONTROLLER,
                    POLICY,
                    ACTION,
                    status,
                    status.is_success(),
                )
                .await
                .map_err(database_error)?;
                db.commit().await.map_err(database_error)?;
                let mut body = saved.response_body.clone();
                if !status.is_success() && body.get("error").is_some() {
                    body["error"]["request_id"] = json!(request_id);
                }
                let mut response = (status, Json(body)).into_response();
                response
                    .headers_mut()
                    .insert("idempotency-replayed", HeaderValue::from_static("true"));
                response.headers_mut().insert(
                    "x-request-id",
                    HeaderValue::from_str(&request_id).map_err(|_| ApiError::internal())?,
                );
                return Ok(response);
            }
        }
    }
    let operations = match parse_operations(&request) {
        Ok(operations) => operations,
        Err(problem) => {
            let request_id = Uuid::new_v4().to_string();
            return failed(db, &context, key, &path, &digest, &request_id, problem).await;
        }
    };
    let request_id = Uuid::new_v4().to_string();
    let savepoint = db.begin().await.map_err(database_error)?;
    let secret = state.oauth.occurrence_key_secret();
    let results = apply_operations(
        &savepoint,
        &context,
        household_id,
        &operations,
        &request_id,
        &secret,
    )
    .await;
    let results = match results {
        Ok(results) => {
            savepoint.commit().await.map_err(database_error)?;
            results
        }
        Err(problem) => {
            savepoint.rollback().await.map_err(database_error)?;
            return failed(db, &context, key, &path, &digest, &request_id, problem).await;
        }
    };
    let body = json!({"data": {"applied": true, "results": results}});
    if let Some(key) = key {
        mutation_idempotency::store(
            &db,
            &context,
            StoredResponse {
                key,
                method: "POST",
                path: &path,
                digest: &digest,
                status: StatusCode::CREATED,
                body: body.clone(),
                request_id: &request_id,
                etag: None,
            },
        )
        .await?;
    }
    finish_with_request_id(
        db,
        &context,
        &request_id,
        "POST",
        CONTROLLER,
        POLICY,
        ACTION,
        StatusCode::CREATED,
        true,
        body,
        None,
    )
    .await
}

async fn failed(
    db: DatabaseTransaction,
    context: &AuthContext,
    key: Option<&str>,
    path: &str,
    digest: &str,
    request_id: &str,
    problem: ApiError,
) -> Result<Response, ApiError> {
    let body = json!({"error": {"code": problem.code, "message": problem.message, "request_id": request_id}});
    if let Some(key) = key {
        mutation_idempotency::store(
            &db,
            context,
            StoredResponse {
                key,
                method: "POST",
                path,
                digest,
                status: problem.status,
                body: body.clone(),
                request_id,
                etag: None,
            },
        )
        .await?;
    }
    finish_with_request_id(
        db,
        context,
        request_id,
        "POST",
        CONTROLLER,
        POLICY,
        ACTION,
        problem.status,
        false,
        body,
        None,
    )
    .await
}

async fn apply_operations(
    db: &DatabaseTransaction,
    context: &AuthContext,
    household_id: i64,
    operations: &[SyncOperation],
    request_id: &str,
    secret: &Arc<[u8]>,
) -> Result<Vec<Value>, ApiError> {
    let mut results = Vec::with_capacity(operations.len());
    for (index, operation) in operations.iter().enumerate() {
        let result =
            apply_operation(db, context, household_id, operation, request_id, secret).await?;
        results.push(result.value(index, &operation.action));
    }
    Ok(results)
}

async fn apply_operation(
    db: &DatabaseTransaction,
    context: &AuthContext,
    household_id: i64,
    operation: &SyncOperation,
    request_id: &str,
    secret: &Arc<[u8]>,
) -> Result<SyncResult, ApiError> {
    match operation.resource_type.as_str() {
        "location" => {
            crate::locations::apply_sync_operation(db, context, operation, request_id).await
        }
        "medication" if operation.action == "remove_stock" => {
            crate::stock_removals::apply_sync_operation(db, context, operation, request_id).await
        }
        "medication" => {
            crate::medication_management::apply_sync_operation(db, context, operation, request_id)
                .await
        }
        "medication_dosage_option" => {
            crate::dosage_options::apply_sync_operation(db, context, operation, request_id).await
        }
        "medication_take" => {
            apply_medication_take(db, context, household_id, operation, request_id).await
        }
        "medication_dose_occurrence" => {
            crate::dose_occurrences::apply_sync_operation(
                db, context, operation, request_id, secret,
            )
            .await
        }
        "medication_pause_period" => {
            crate::pause_lifecycle::apply_sync_operation(db, context, operation, request_id).await
        }
        "schedule" if matches!(operation.action.as_str(), "pause" | "resume") => {
            crate::pause_lifecycle::apply_sync_operation(db, context, operation, request_id).await
        }
        "schedule" => {
            crate::schedule_writes::apply_sync_operation(db, context, operation, request_id).await
        }
        "person_medication"
            if matches!(operation.action.as_str(), "pause" | "resume" | "reorder") =>
        {
            crate::pause_lifecycle::apply_sync_operation(db, context, operation, request_id).await
        }
        "person_medication" => {
            crate::person_medication_writes::apply_sync_operation(
                db, context, operation, request_id,
            )
            .await
        }
        "medication_review_prompt" => {
            crate::review_prompts::apply_sync_operation(db, context, operation, request_id).await
        }
        "health_event" => {
            crate::health_events::apply_sync_operation(db, context, operation, request_id).await
        }
        "person" => crate::people::apply_sync_operation(db, context, operation, request_id).await,
        _ => Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "sync_operation_unsupported",
            "Operation is not supported offline",
        )),
    }
}

async fn apply_medication_take(
    db: &DatabaseTransaction,
    context: &AuthContext,
    household_id: i64,
    operation: &SyncOperation,
    request_id: &str,
) -> Result<SyncResult, ApiError> {
    let body = json!({"medication_take": operation.attributes});
    let (status, take) =
        crate::dose::create_in_transaction(db, context, household_id, &body, request_id).await?;
    let etag = crate::dose::take_etag(&take);
    Ok(SyncResult {
        record_type: "MedicationTake",
        record_id: Some(take.id),
        record_portable_id: Some(take.portable_id),
        etag: Some(etag),
        replayed: Some(status == StatusCode::OK),
    })
}

async fn authorize_replay(
    db: &DatabaseTransaction,
    context: &AuthContext,
    operations: &[SyncOperation],
    saved: &Value,
    original_request_id: Option<&str>,
) -> Result<(), ApiError> {
    if saved
        .get("data")
        .and_then(|value| value.get("applied"))
        .and_then(Value::as_bool)
        != Some(true)
    {
        return Err(ApiError::forbidden());
    }
    let results = saved
        .get("data")
        .and_then(|value| value.get("results"))
        .and_then(Value::as_array)
        .ok_or_else(ApiError::forbidden)?;
    if results.len() != operations.len() {
        return Err(ApiError::forbidden());
    }
    for (index, (operation, result)) in operations.iter().zip(results).enumerate() {
        let expected = match operation.resource_type.as_str() {
            "medication" => "Medication",
            "medication_dosage_option" => "MedicationDosageOption",
            "medication_take" => "MedicationTake",
            "medication_dose_occurrence" => "MedicationDoseOccurrence",
            "medication_pause_period" => "MedicationPausePeriod",
            "person" => "Person",
            "health_event" => "HealthEvent",
            "location" => "Location",
            "medication_review_prompt" => "MedicationReviewPrompt",
            "schedule" => "Schedule",
            "person_medication" => "PersonMedication",
            _ => return Err(ApiError::forbidden()),
        };
        if result.get("index").and_then(Value::as_u64) != Some(index as u64)
            || result.get("action").and_then(Value::as_str) != Some(operation.action.as_str())
            || result.get("record_type").and_then(Value::as_str) != Some(expected)
        {
            return Err(ApiError::forbidden());
        }
        let (authority_operation, authority_result) =
            superseding_delete(operations, results, index).unwrap_or((operation, result));
        match authority_operation.resource_type.as_str() {
            "location" => {
                crate::locations::authorize_sync_replay(
                    db,
                    context,
                    authority_operation,
                    authority_result,
                )
                .await?
            }
            "medication" => {
                crate::medication_management::authorize_sync_replay(
                    db,
                    context,
                    authority_operation,
                    authority_result,
                )
                .await?
            }
            "medication_dosage_option" => {
                crate::dosage_options::authorize_sync_replay(
                    db,
                    context,
                    authority_operation,
                    authority_result,
                )
                .await?
            }
            "medication_take" => {
                crate::dose::authorize_sync_replay(
                    db,
                    context,
                    authority_operation,
                    authority_result,
                )
                .await?
            }
            "medication_dose_occurrence" => {
                crate::dose_occurrences::authorize_sync_replay(
                    db,
                    context,
                    authority_operation,
                    authority_result,
                )
                .await?
            }
            "medication_pause_period" => {
                crate::pause_lifecycle::authorize_sync_replay(
                    db,
                    context,
                    authority_operation,
                    authority_result,
                )
                .await?
            }
            "schedule" if matches!(authority_operation.action.as_str(), "pause" | "resume") => {
                crate::pause_lifecycle::authorize_sync_replay(
                    db,
                    context,
                    authority_operation,
                    authority_result,
                )
                .await?
            }
            "schedule" => {
                crate::schedule_writes::authorize_sync_replay(
                    db,
                    context,
                    authority_operation,
                    authority_result,
                )
                .await?
            }
            "person_medication"
                if matches!(
                    authority_operation.action.as_str(),
                    "pause" | "resume" | "reorder"
                ) =>
            {
                crate::pause_lifecycle::authorize_sync_replay(
                    db,
                    context,
                    authority_operation,
                    authority_result,
                )
                .await?
            }
            "person_medication" => {
                crate::person_medication_writes::authorize_sync_replay(
                    db,
                    context,
                    authority_operation,
                    authority_result,
                )
                .await?
            }
            "medication_review_prompt" => {
                crate::review_prompts::authorize_sync_replay(
                    db,
                    context,
                    authority_operation,
                    authority_result,
                )
                .await?
            }
            "health_event" => {
                crate::health_events::authorize_sync_replay(
                    db,
                    context,
                    authority_operation,
                    authority_result,
                    original_request_id.ok_or_else(ApiError::forbidden)?,
                )
                .await?
            }
            "person" => {
                crate::people::authorize_sync_replay(
                    db,
                    context,
                    authority_operation,
                    authority_result,
                )
                .await?
            }
            _ => return Err(ApiError::forbidden()),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_malformed_operation_identifier() {
        for identifier in [
            "01",
            "-1",
            "not-a-uuid",
            "11111111-1111-4111-1111-111111111111",
        ] {
            let request = json!({"batch": {"operations": [{
                "resource_type": "location", "action": "update", "id": identifier, "attributes": {}
            }]}});
            assert!(
                matches!(parse_operations(&request), Err(problem) if problem.status == StatusCode::UNPROCESSABLE_ENTITY)
            );
        }
    }
}
