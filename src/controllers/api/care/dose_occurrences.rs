use super::administration::{complete, key, request_id, store};
use super::*;
use crate::models::care::{dose_occurrences, locations::SavedResponse};
use std::sync::Arc;
type Range = std::result::Result<
    axum::extract::Query<dose_occurrences::RangeQuery>,
    axum::extract::rejection::QueryRejection,
>;

fn signing_key(ctx: &AppContext) -> std::result::Result<Arc<[u8]>, response::Failure> {
    dose_occurrences::configured_signing_key(ctx.config.settings.as_ref())
        .map_err(|_| response::unavailable())
}
async fn list(
    ctx: AppContext,
    path: (i64, String),
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    query: Range,
    kind: &'static str,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, path.0, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let result = async {
        dose_occurrences::authorize(&tenant, kind, &path.1, "index")
            .await
            .map_err(response::operation)?;
        let axum::extract::Query(query) =
            query.map_err(|_| response::Failure::field("date_range", "is invalid"))?;
        let secret = signing_key(&ctx)?;
        dose_occurrences::with_dashboard_timezone(
            principal.time_zone(),
            dose_occurrences::list(&tenant, kind, &path.1, query, &secret),
        )
        .await
        .map(|body| (StatusCode::OK, body, None))
        .map_err(|error| match error {
            OperationError::Validation { details } => {
                response::Failure::fields(details["errors"].clone())
            }
            error => response::operation(error),
        })
    }
    .await;
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::dose_occurrence("GET", "index", kind),
        result,
        &request_id,
    )
    .await
}
async fn mutation(
    ctx: AppContext,
    path: (i64, String),
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
    operation: (&'static str, &'static str),
) -> Response {
    let (kind, action) = operation;
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, path.0, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let savepoint = match tenant.transaction().begin().await {
        Ok(value) => value,
        Err(_) => return response::error(response::unavailable(), &request_id),
    };
    let method = if action == "reopen" { "PATCH" } else { "POST" };
    let result = process(
        &ctx,
        &tenant,
        &principal,
        &path,
        &headers,
        body,
        (kind, action),
    )
    .await;
    let restored = if result.is_ok() {
        savepoint.commit().await
    } else {
        savepoint.rollback().await
    };
    if restored.is_err() {
        return response::error(response::unavailable(), &request_id);
    }
    complete(
        tenant,
        &principal,
        audit::RequestAudit::dose_occurrence(method, action, kind),
        result,
        &request_id,
    )
    .await
}
async fn process(
    ctx: &AppContext,
    tenant: &TenantTransaction,
    principal: &ValidatedPrincipal,
    path: &(i64, String),
    headers: &HeaderMap,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
    operation: (&str, &str),
) -> std::result::Result<(StatusCode, Value, Option<String>, bool), response::Failure> {
    let (kind, action) = operation;
    dose_occurrences::authorize(tenant, kind, &path.1, action)
        .await
        .map_err(response::operation)?;
    let AxumJson(body) =
        body.map_err(|_| response::Failure::bad_request("Invalid JSON request body"))?;
    let method = if action == "reopen" { "PATCH" } else { "POST" };
    let segment = if kind == "schedule" {
        "schedules"
    } else {
        "person_medications"
    };
    let request_path = format!(
        "/api/v1/households/{}/{segment}/{}/dose_occurrences/{action}",
        path.0, path.1
    );
    let key = key(headers);
    if let Some(key) = key
        && let Some(reply) =
            locations::keyed_replay(tenant, key, method, &request_path, &body).await?
    {
        return Ok(reply);
    }
    let etag = headers
        .get(axum::http::header::IF_MATCH)
        .and_then(|value| value.to_str().ok());
    let secret = if body
        .get("dose_occurrence")
        .and_then(Value::as_object)
        .is_some()
    {
        Some(signing_key(ctx)?)
    } else {
        None
    };
    let savepoint = tenant
        .transaction()
        .begin()
        .await
        .map_err(|_| response::unavailable())?;
    let operation = if let Some(secret) = secret {
        dose_occurrences::with_dashboard_timezone(
            principal.time_zone(),
            dose_occurrences::change(
                tenant,
                (kind, &path.1),
                action,
                &body,
                etag,
                &secret,
                Some(principal.provenance()),
            ),
        )
        .await
    } else {
        Err(OperationError::Validation {
            details: json!({"code":"bad_request","message":"Invalid request body"}),
        })
    };
    let (status, reply, etag) = match operation {
        Ok((body, etag)) => {
            savepoint
                .commit()
                .await
                .map_err(|_| response::unavailable())?;
            (StatusCode::OK, body, Some(etag))
        }
        Err(OperationError::Validation { details }) => {
            savepoint
                .rollback()
                .await
                .map_err(|_| response::unavailable())?;
            let code = details
                .get("code")
                .and_then(Value::as_str)
                .unwrap_or("validation_failed");
            let status = if code == "bad_request" {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::UNPROCESSABLE_ENTITY
            };
            let message = details
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("Validation failed");
            let mut reply = json!({"error":{"code":code,"message":message,"request_id":tenant.scope().request_id}});
            if let Some(errors) = details.get("errors") {
                reply["error"]["errors"] = errors.clone();
            }
            (status, reply, None)
        }
        Err(OperationError::Conflict { code, .. }) if code == "precondition_required" => {
            savepoint
                .rollback()
                .await
                .map_err(|_| response::unavailable())?;
            (
                StatusCode::PRECONDITION_REQUIRED,
                json!({"error":{"code":code,"message":"A current version is required","request_id":tenant.scope().request_id}}),
                None,
            )
        }
        Err(error) => {
            savepoint
                .rollback()
                .await
                .map_err(|_| response::unavailable())?;
            return Err(response::operation(error));
        }
    };
    if let Some(key) = key {
        return store(
            tenant,
            principal,
            SavedResponse {
                key,
                method,
                path: &request_path,
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

pub(super) async fn list_schedule(
    State(ctx): State<AppContext>,
    Path(path): Path<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    query: Range,
) -> Response {
    list(ctx, path, headers, request, query, "schedule").await
}
pub(super) async fn not_taken_schedule(
    State(ctx): State<AppContext>,
    Path(path): Path<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    mutation(ctx, path, headers, request, body, ("schedule", "not_taken")).await
}
pub(super) async fn reopen_schedule(
    State(ctx): State<AppContext>,
    Path(path): Path<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    mutation(ctx, path, headers, request, body, ("schedule", "reopen")).await
}
pub(super) async fn take_schedule(
    State(ctx): State<AppContext>,
    Path(path): Path<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    mutation(ctx, path, headers, request, body, ("schedule", "take")).await
}

pub(super) async fn list_assignment(
    State(ctx): State<AppContext>,
    Path(path): Path<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    query: Range,
) -> Response {
    list(ctx, path, headers, request, query, "person_medication").await
}
pub(super) async fn not_taken_assignment(
    State(ctx): State<AppContext>,
    Path(path): Path<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    mutation(
        ctx,
        path,
        headers,
        request,
        body,
        ("person_medication", "not_taken"),
    )
    .await
}
pub(super) async fn reopen_assignment(
    State(ctx): State<AppContext>,
    Path(path): Path<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    mutation(
        ctx,
        path,
        headers,
        request,
        body,
        ("person_medication", "reopen"),
    )
    .await
}
pub(super) async fn take_assignment(
    State(ctx): State<AppContext>,
    Path(path): Path<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    mutation(
        ctx,
        path,
        headers,
        request,
        body,
        ("person_medication", "take"),
    )
    .await
}
