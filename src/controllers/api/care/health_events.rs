use super::*;
use crate::models::care::{health_events, people::Pagination};

fn operation(error: OperationError) -> response::Failure {
    match error {
        OperationError::Validation { details } if details["status"] == 400 => {
            response::Failure::bad_request("Invalid request body")
        }
        error => response::operation(error),
    }
}

pub(super) async fn index(
    State(ctx): State<AppContext>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    page: std::result::Result<
        axum::extract::Query<Pagination>,
        axum::extract::rejection::QueryRejection,
    >,
) -> Response {
    let request_id = request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let reply = match page {
        Ok(axum::extract::Query(page)) => health_events::list(&tenant, page)
            .await
            .map(|body| (StatusCode::OK, body, None))
            .map_err(operation),
        Err(_) => Err(response::Failure::validation("Invalid pagination")),
    };
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::health_event("GET", "index"),
        reply,
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
    let request_id = request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let reply = health_events::read(&tenant, &id)
        .await
        .map(|(body, etag)| (StatusCode::OK, body, Some(etag)))
        .map_err(operation);
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::health_event("GET", "show"),
        reply,
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
    mutate(ctx, household_id, None, headers, request, body, "POST").await
}

pub(super) async fn update(
    State(ctx): State<AppContext>,
    Path((household_id, id)): Path<(i64, String)>,
    method: axum::http::Method,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    mutate(
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

async fn mutate(
    ctx: AppContext,
    household_id: i64,
    id: Option<String>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
    method: &'static str,
) -> Response {
    let request_id = request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let savepoint = match tenant.transaction().begin().await {
        Ok(value) => value,
        Err(_) => return response::error(response::unavailable(), &request_id),
    };
    let result = match body {
        Ok(AxumJson(body)) => health_events::mutate(
            &tenant,
            id.as_deref(),
            &body,
            headers.get("if-match").and_then(|v| v.to_str().ok()),
            principal.provenance(),
        )
        .await
        .map(|(body, etag)| {
            (
                if id.is_some() {
                    StatusCode::OK
                } else {
                    StatusCode::CREATED
                },
                body,
                Some(etag),
            )
        })
        .map_err(operation),
        Err(_) => Err(response::Failure::bad_request("Invalid JSON request body")),
    };
    let saved = if result.is_ok() {
        savepoint.commit().await
    } else {
        savepoint.rollback().await
    };
    if saved.is_err() {
        return response::error(response::unavailable(), &request_id);
    }
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::health_event(method, if id.is_some() { "update" } else { "create" }),
        result,
        &request_id,
    )
    .await
}
