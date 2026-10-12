use super::*;
use crate::models::care::{
    locations::SavedResponse,
    review_prompts,
    sync::{lock, review_prompts as prompts},
};
use axum::extract::rejection::QueryRejection;
use axum::http::{HeaderValue, header};

fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

pub(super) async fn index(
    State(ctx): State<AppContext>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    query: std::result::Result<axum::extract::Query<review_prompts::ListQuery>, QueryRejection>,
) -> Response {
    let request_id = administration::request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return no_store(response::error(error, &request_id)),
    };
    let query = query.map(|axum::extract::Query(query)| query).ok();
    let reply = review_prompts::index(&tenant, query)
        .await
        .map(|body| (StatusCode::OK, body, None))
        .map_err(response::operation);
    no_store(
        finish(
            tenant,
            principal.provenance(),
            audit::RequestAudit::review_prompt("GET", "index"),
            reply,
            &request_id,
        )
        .await,
    )
}

pub(super) async fn show(
    State(ctx): State<AppContext>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
) -> Response {
    let request_id = administration::request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return no_store(response::error(error, &request_id)),
    };
    let reply = review_prompts::show(&tenant, &id)
        .await
        .map(|(body, etag)| (StatusCode::OK, body, Some(etag)))
        .map_err(response::operation);
    no_store(
        finish(
            tenant,
            principal.provenance(),
            audit::RequestAudit::review_prompt("GET", "show"),
            reply,
            &request_id,
        )
        .await,
    )
}

pub(super) async fn update(
    State(ctx): State<AppContext>,
    Path((household_id, id)): Path<(i64, String)>,
    method: axum::http::Method,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    let method = if method == axum::http::Method::PUT {
        "PUT"
    } else {
        "PATCH"
    };
    let request_id = administration::request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return no_store(response::error(error, &request_id)),
    };
    let result = mutation(
        &tenant,
        &principal,
        household_id,
        &id,
        &headers,
        body,
        method,
    )
    .await;
    no_store(
        administration::complete(
            tenant,
            &principal,
            audit::RequestAudit::review_prompt(method, "update"),
            result,
            &request_id,
        )
        .await,
    )
}

type Stored = std::result::Result<(StatusCode, Value, Option<String>, bool), response::Failure>;

#[allow(clippy::too_many_arguments)]
async fn keyed(
    tenant: &TenantTransaction,
    principal: &ValidatedPrincipal,
    key: Option<&str>,
    method: &str,
    path: &str,
    request: &Value,
    status: StatusCode,
    body: Value,
    etag: Option<String>,
) -> Stored {
    if let Some(key) = key {
        administration::store(
            tenant,
            principal,
            SavedResponse {
                key,
                method,
                path,
                request,
                status: status.as_u16(),
                body,
                etag: etag.as_deref(),
            },
        )
        .await
    } else {
        Ok((status, body, etag, false))
    }
}

async fn mutation(
    tenant: &TenantTransaction,
    principal: &ValidatedPrincipal,
    household_id: i64,
    id: &str,
    headers: &HeaderMap,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
    method: &str,
) -> Stored {
    let context = prompts::AuthContext { tenant };
    if !prompts::actor_adult(tenant.transaction(), &context)
        .await
        .map_err(response::operation)?
    {
        return Err(response::operation(OperationError::Forbidden));
    }
    let Some(record) = prompts::visible_prompt(tenant.transaction(), &context, id)
        .await
        .map_err(response::operation)?
    else {
        return Err(response::operation(OperationError::NotFound));
    };
    if !prompts::person_access(tenant.transaction(), &context, record.person_id, true)
        .await
        .map_err(response::operation)?
    {
        return Err(response::operation(OperationError::Forbidden));
    }
    lock(tenant).await.map_err(response::operation)?;
    if !prompts::actor_adult(tenant.transaction(), &context)
        .await
        .map_err(response::operation)?
        || !prompts::person_access(tenant.transaction(), &context, record.person_id, true)
            .await
            .map_err(response::operation)?
    {
        return Err(response::operation(OperationError::Forbidden));
    }
    let Some(record) = prompts::visible_prompt(tenant.transaction(), &context, id)
        .await
        .map_err(response::operation)?
    else {
        return Err(response::operation(OperationError::NotFound));
    };
    let (body, rejected) = match body {
        Ok(AxumJson(value)) => (value, false),
        Err(_) => (Value::Null, true),
    };
    let path = format!(
        "/api/v1/households/{household_id}/medication_review_prompts/{}",
        record.id
    );
    let key = administration::key(headers);
    if let Some(key) = key
        && let Some(reply) = locations::keyed_replay(tenant, key, method, &path, &body).await?
    {
        return Ok(reply);
    }
    if rejected {
        return Ok((
            StatusCode::BAD_REQUEST,
            json!({"error":{"code":"bad_request","message":"JSON is invalid","request_id":tenant.scope().request_id}}),
            None,
            false,
        ));
    }
    let changes = match prompts::Changes::parse(&body) {
        Ok(changes) => changes,
        Err(code) => {
            let (status, error_code, message) = if code == StatusCode::BAD_REQUEST {
                (
                    StatusCode::BAD_REQUEST,
                    "bad_request",
                    "medication_review_prompt is required",
                )
            } else {
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "validation_failed",
                    "Validation failed",
                )
            };
            return keyed(
                tenant,
                principal,
                key,
                method,
                &path,
                &body,
                status,
                json!({"error":{"code":error_code,"message":message,"request_id":tenant.scope().request_id}}),
                None,
            )
            .await;
        }
    };
    let expected = headers
        .get(header::IF_MATCH)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty());
    let Some(expected) = expected else {
        return keyed(
            tenant,
            principal,
            key,
            method,
            &path,
            &body,
            StatusCode::PRECONDITION_REQUIRED,
            json!({"error":{"code":"precondition_required","message":"A current review version is required","request_id":tenant.scope().request_id}}),
            None,
        )
        .await;
    };
    if expected != prompts::tag(&record) {
        return Ok((
            StatusCode::CONFLICT,
            json!({"error":{"code":"conflict","message":"A current review version is required","request_id":tenant.scope().request_id}}),
            None,
            false,
        ));
    }
    let Some(active) = changes.apply(&record, tenant.membership().id) else {
        return keyed(
            tenant,
            principal,
            key,
            method,
            &path,
            &body,
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({"error":{"code":"validation_failed","message":"Review could not be saved","request_id":tenant.scope().request_id}}),
            None,
        )
        .await;
    };
    let savepoint = tenant
        .transaction()
        .begin()
        .await
        .map_err(|_| response::unavailable())?;
    let updated = match active.update(tenant.transaction()).await {
        Ok(updated) => match prompts::record_rest_update(tenant, &record, &updated).await {
            Ok(()) => {
                savepoint
                    .commit()
                    .await
                    .map_err(|_| response::unavailable())?;
                updated
            }
            Err(error) => {
                savepoint
                    .rollback()
                    .await
                    .map_err(|_| response::unavailable())?;
                return Err(response::operation(error));
            }
        },
        Err(_) => {
            savepoint
                .rollback()
                .await
                .map_err(|_| response::unavailable())?;
            return Err(response::unavailable());
        }
    };
    let etag = prompts::tag(&updated);
    keyed(
        tenant,
        principal,
        key,
        method,
        &path,
        &body,
        StatusCode::OK,
        json!({"data": review_prompts::value(&updated)}),
        Some(etag),
    )
    .await
}
