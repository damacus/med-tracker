use super::administration::{complete, key, request_id, store};
use super::*;
use crate::models::care::{
    dose_occurrences,
    locations::{self as ledger, KeyedResponse, SavedResponse},
    sync,
};

#[derive(serde::Deserialize)]
pub(super) struct ChangeQuery {
    cursor: Option<String>,
}
pub(super) async fn changes(
    State(ctx): State<AppContext>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    query: std::result::Result<
        axum::extract::Query<ChangeQuery>,
        axum::extract::rejection::QueryRejection,
    >,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let result = match query {
        Ok(axum::extract::Query(ChangeQuery {
            cursor: Some(cursor),
        })) => sync::changes(&tenant, &cursor)
            .await
            .map(|body| (StatusCode::OK, body, None))
            .or_else(|error| Ok(error_reply(error, &request_id))),
        Ok(axum::extract::Query(ChangeQuery { cursor: None })) => Ok((
            StatusCode::BAD_REQUEST,
            error_body("bad_request", "cursor is required", &request_id),
            None,
        )),
        Err(_) => Ok((
            StatusCode::UNPROCESSABLE_ENTITY,
            error_body(
                "unprocessable_content",
                "cursor must be ISO8601",
                &request_id,
            ),
            None,
        )),
    };
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::sync("GET", "changes"),
        result,
        &request_id,
    )
    .await
}
pub(super) async fn snapshot(
    State(ctx): State<AppContext>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let result = sync::snapshot(&tenant, principal.time_zone())
        .await
        .map(|mut snapshot| {
            let etags: std::collections::HashMap<_, _> = snapshot
                .takes
                .iter()
                .map(|take| (take.portable_id.as_str(), projection::take_etag(take)))
                .collect();
            for row in snapshot.body["data"]["records"]["medication_takes"]
                .as_array_mut()
                .into_iter()
                .flatten()
            {
                if let Some(etag) = row["portable_id"].as_str().and_then(|id| etags.get(id)) {
                    row["etag"] = json!(etag);
                }
            }
            (StatusCode::OK, snapshot.body, None)
        })
        .or_else(|error| Ok(error_reply(error, &request_id)));
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::sync("GET", "snapshot"),
        result,
        &request_id,
    )
    .await
}
pub(super) async fn create(
    State(ctx): State<AppContext>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let result = process(&ctx, &tenant, &principal, &headers, body).await;
    complete(
        tenant,
        &principal,
        audit::RequestAudit::sync("POST", "create"),
        result,
        &request_id,
    )
    .await
}
async fn process(
    ctx: &AppContext,
    tenant: &TenantTransaction,
    principal: &ValidatedPrincipal,
    headers: &HeaderMap,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> std::result::Result<(StatusCode, Value, Option<String>, bool), response::Failure> {
    sync::lock(tenant).await.map_err(response::operation)?;
    let AxumJson(body) =
        body.map_err(|_| response::Failure::bad_request("Invalid JSON request body"))?;
    let path = format!(
        "/api/v1/households/{}/sync/batches",
        tenant.scope().household_id
    );
    let key = key(headers);
    if let Some(key) = key {
        match ledger::lookup(tenant, key, "POST", &path, &body)
            .await
            .map_err(response::operation)?
        {
            KeyedResponse::New => {}
            KeyedResponse::Conflict => {
                return Err(response::operation(OperationError::Conflict {
                    code: "idempotency_key_reused".into(),
                    details: json!({"message":"Idempotency key has already been used for a different request"}),
                }));
            }
            KeyedResponse::Replay(saved) => {
                let status = StatusCode::from_u16(
                    u16::try_from(saved.response_status).map_err(|_| response::unavailable())?,
                )
                .map_err(|_| response::unavailable())?;
                if status.is_success() {
                    sync::authorize_replay(
                        tenant,
                        &body,
                        &saved.response_body,
                        saved.response_headers["x-request-id"].as_str(),
                        principal.provenance(),
                    )
                    .await
                    .map_err(response::operation)?;
                }
                let mut reply = saved.response_body;
                if let Some(error) = reply.get_mut("error").and_then(Value::as_object_mut) {
                    error.insert("request_id".into(), json!(tenant.scope().request_id));
                }
                return Ok((status, reply, None, true));
            }
        }
    }
    let secret = if body["batch"]["operations"]
        .as_array()
        .is_some_and(|operations| {
            operations
                .iter()
                .any(|operation| operation["resource_type"] == "medication_dose_occurrence")
        }) {
        Some(
            dose_occurrences::configured_signing_key(ctx.config.settings.as_ref())
                .map_err(response::operation)?,
        )
    } else {
        None
    };
    let savepoint = tenant
        .transaction()
        .begin()
        .await
        .map_err(|_| response::unavailable())?;
    let result = dose_occurrences::with_dashboard_timezone(
        principal.time_zone(),
        sync::apply(
            tenant,
            &body,
            principal.time_zone(),
            secret.as_ref(),
            principal.provenance(),
        ),
    )
    .await;
    let (status, reply, etag) = match result {
        Ok(mut applied) => {
            for result in applied.body["data"]["results"]
                .as_array_mut()
                .into_iter()
                .flatten()
            {
                if result["record_type"] == "MedicationTake"
                    && let Some(take) = applied.takes.iter().find(|take| {
                        result["record_id"].as_str() == Some(take.id.to_string().as_str())
                    })
                {
                    result["etag"] = json!(projection::take_etag(take));
                }
            }
            savepoint
                .commit()
                .await
                .map_err(|_| response::unavailable())?;
            (StatusCode::CREATED, applied.body, None)
        }
        Err(error) => {
            savepoint
                .rollback()
                .await
                .map_err(|_| response::unavailable())?;
            error_reply(error, &tenant.scope().request_id)
        }
    };
    if let Some(key) = key
        && status.as_u16() < 500
        && status != StatusCode::CONFLICT
    {
        return store(
            tenant,
            principal,
            SavedResponse {
                key,
                method: "POST",
                path: &path,
                request: &body,
                status: status.as_u16(),
                body: reply,
                etag: etag.as_deref(),
            },
        )
        .await;
    }
    Ok((status, reply, etag, false))
}
fn error_body(code: &str, message: &str, request_id: &str) -> Value {
    json!({"error":{"code":code,"message":message,"request_id":request_id}})
}
fn error_reply(error: OperationError, request_id: &str) -> (StatusCode, Value, Option<String>) {
    match error {
        OperationError::Validation { details } => {
            let status = details["status"]
                .as_u64()
                .and_then(|status| u16::try_from(status).ok())
                .and_then(|status| StatusCode::from_u16(status).ok())
                .unwrap_or(StatusCode::UNPROCESSABLE_ENTITY);
            (
                status,
                error_body(
                    details["code"].as_str().unwrap_or("unprocessable_content"),
                    details["message"]
                        .as_str()
                        .or_else(|| details["error"].as_str())
                        .unwrap_or("Attributes are invalid"),
                    request_id,
                ),
                None,
            )
        }
        OperationError::Conflict { code, details } => (
            if code == "precondition_required" {
                StatusCode::PRECONDITION_REQUIRED
            } else {
                StatusCode::CONFLICT
            },
            error_body(
                &code,
                details["error"]
                    .as_str()
                    .or_else(|| details["message"].as_str())
                    .unwrap_or("Record has changed since it was last read"),
                request_id,
            ),
            None,
        ),
        OperationError::Forbidden => (
            StatusCode::FORBIDDEN,
            error_body(
                "forbidden",
                "You are not authorized to perform this action.",
                request_id,
            ),
            None,
        ),
        OperationError::NotFound => (
            StatusCode::NOT_FOUND,
            error_body("not_found", "Record not found", request_id),
            None,
        ),
        OperationError::Unauthenticated => (
            StatusCode::UNAUTHORIZED,
            error_body("unauthorized", "Authentication required", request_id),
            None,
        ),
        OperationError::Unavailable => (
            StatusCode::INTERNAL_SERVER_ERROR,
            error_body("internal_error", "Internal server error", request_id),
            None,
        ),
    }
}
