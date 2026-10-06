pub(super) mod legacy;
use super::administration::{complete, key, request_id, store};
use super::*;
use crate::models::care::{locations::SavedResponse, pause_periods};

pub(super) async fn index(
    State(ctx): State<AppContext>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    page: std::result::Result<
        axum::extract::Query<pause_periods::Pagination>,
        axum::extract::rejection::QueryRejection,
    >,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let result = match page {
        Ok(axum::extract::Query(page)) => pause_periods::list(&tenant, page)
            .await
            .map(|body| (StatusCode::OK, body, None))
            .map_err(response::operation),
        Err(_) => Err(response::Failure::validation("Invalid pagination")),
    };
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::pause_period("GET", "index"),
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
    let body = body
        .map(|AxumJson(body)| body)
        .map_err(|_| response::Failure::bad_request("Invalid JSON request body"));
    change(ctx, household_id, None, headers, request, body, "POST").await
}
pub(super) async fn resume(
    State(ctx): State<AppContext>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: axum::body::Bytes,
) -> Response {
    let body = if body.is_empty() {
        Ok(json!({}))
    } else {
        serde_json::from_slice(&body)
            .map_err(|_| response::Failure::bad_request("Invalid JSON request body"))
    };
    change(ctx, household_id, Some(id), headers, request, body, "POST").await
}
async fn change(
    ctx: AppContext,
    household_id: i64,
    id: Option<String>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<Value, response::Failure>,
    method: &'static str,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let action = if id.is_some() { "resume" } else { "create" };
    let savepoint = match tenant.transaction().begin().await {
        Ok(value) => value,
        Err(_) => return response::error(response::unavailable(), &request_id),
    };
    let result = process(
        &tenant,
        &principal,
        household_id,
        id.as_deref(),
        &headers,
        body,
        method,
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
        audit::RequestAudit::pause_period(method, action),
        result,
        &request_id,
    )
    .await
}
async fn process(
    tenant: &TenantTransaction,
    principal: &ValidatedPrincipal,
    household_id: i64,
    id: Option<&str>,
    headers: &HeaderMap,
    body: std::result::Result<Value, response::Failure>,
    method: &str,
) -> std::result::Result<(StatusCode, Value, Option<String>, bool), response::Failure> {
    let body = body?;
    if id.is_none()
        && body
            .get("medication_pause_period")
            .and_then(Value::as_object)
            .is_none()
    {
        return Err(response::Failure::bad_request("Invalid request body"));
    }
    if let Some(id) = id {
        pause_periods::authorize_resume(tenant, id).await
    } else {
        pause_periods::authorize_create(tenant, &body).await
    }
    .map_err(response::operation)?;
    let path = if let Some(id) = id {
        format!("/api/v1/households/{household_id}/medication_pause_periods/{id}/resume")
    } else {
        format!("/api/v1/households/{household_id}/medication_pause_periods")
    };
    let key = key(headers);
    if let Some(key) = key
        && let Some(reply) = locations::keyed_replay(tenant, key, method, &path, &body).await?
    {
        return Ok(reply);
    }
    let savepoint = tenant
        .transaction()
        .begin()
        .await
        .map_err(|_| response::unavailable())?;
    let operation = if let Some(id) = id {
        pause_periods::resume(
            tenant,
            id,
            &body,
            headers
                .get(axum::http::header::IF_MATCH)
                .map(|value| value.to_str().unwrap_or("")),
            Some(principal.provenance()),
        )
        .await
    } else {
        pause_periods::create(tenant, &body, Some(principal.provenance())).await
    };
    let (status, reply, etag) = match operation {
        Ok((body, etag)) => {
            savepoint
                .commit()
                .await
                .map_err(|_| response::unavailable())?;
            (
                if id.is_some() {
                    StatusCode::OK
                } else {
                    StatusCode::CREATED
                },
                body,
                Some(etag),
            )
        }
        Err(OperationError::Validation { details }) => {
            savepoint
                .rollback()
                .await
                .map_err(|_| response::unavailable())?;
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                json!({"error":{"code":"validation_failed","message":"Validation failed","errors":details["errors"],"request_id":tenant.scope().request_id}}),
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
