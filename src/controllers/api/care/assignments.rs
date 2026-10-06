use super::administration::{complete, key, request_id, store};
use super::*;
use crate::models::care::{assignments, locations::SavedResponse};
use headers::HeaderMapExt;

pub(super) async fn index(
    State(ctx): State<AppContext>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    page: std::result::Result<
        axum::extract::Query<assignments::Pagination>,
        axum::extract::rejection::QueryRejection,
    >,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let result = match page {
        Ok(axum::extract::Query(page)) => assignments::list(&tenant, page)
            .await
            .map(|body| (StatusCode::OK, body, None))
            .map_err(response::operation),
        Err(_) => Err(response::Failure::validation("Invalid pagination")),
    };
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::assignment("GET", "index"),
        result,
        &request_id,
    )
    .await
}
pub(super) async fn show(
    State(ctx): State<AppContext>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let result = assignments::read(&tenant, &id)
        .await
        .map(|(body, etag)| {
            let unchanged = headers
                .typed_get::<headers::IfNoneMatch>()
                .zip(etag.parse::<headers::ETag>().ok())
                .is_some_and(|(condition, etag)| !condition.precondition_passes(&etag));
            (
                if unchanged {
                    StatusCode::NOT_MODIFIED
                } else {
                    StatusCode::OK
                },
                if unchanged { Value::Null } else { body },
                Some(etag),
            )
        })
        .map_err(response::operation);
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::assignment("GET", "show"),
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
    change(ctx, household_id, None, headers, request, body, "POST").await
}
pub(super) async fn update(
    State(ctx): State<AppContext>,
    Path((household_id, id)): Path<(i64, String)>,
    method: axum::http::Method,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    change(
        ctx,
        household_id,
        Some(id),
        headers,
        request,
        body,
        if method == axum::http::Method::PUT {
            "PUT"
        } else {
            "PATCH"
        },
    )
    .await
}
async fn change(
    ctx: AppContext,
    household_id: i64,
    id: Option<String>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
    method: &'static str,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let action = if id.is_some() { "update" } else { "create" };
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
        audit::RequestAudit::assignment(method, action),
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
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
    method: &str,
) -> std::result::Result<(StatusCode, Value, Option<String>, bool), response::Failure> {
    let AxumJson(body) =
        body.map_err(|_| response::Failure::bad_request("Invalid JSON request body"))?;
    if assignments::validate_body(&body, id.is_none()) == Err(StatusCode::BAD_REQUEST) {
        return Err(response::Failure::bad_request("Invalid request body"));
    }
    if let Some(id) = id {
        assignments::authorize_update(tenant, id).await
    } else {
        assignments::authorize_create(tenant, &body).await
    }
    .map_err(response::operation)?;
    let path = if let Some(id) = id {
        format!("/api/v1/households/{household_id}/person_medications/{id}")
    } else {
        format!("/api/v1/households/{household_id}/person_medications")
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
        assignments::update(
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
        assignments::create(tenant, &body, Some(principal.provenance())).await
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
