use super::*;

pub async fn pause_schedule(
    State(ctx): State<AppContext>,
    Path((household, id)): Path<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: axum::body::Bytes,
) -> Response {
    change(ctx, household, id, headers, request, body, (true, true)).await
}
pub async fn resume_schedule(
    State(ctx): State<AppContext>,
    Path((household, id)): Path<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: axum::body::Bytes,
) -> Response {
    change(ctx, household, id, headers, request, body, (true, false)).await
}
pub async fn pause_assignment(
    State(ctx): State<AppContext>,
    Path((household, id)): Path<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: axum::body::Bytes,
) -> Response {
    change(ctx, household, id, headers, request, body, (false, true)).await
}
pub async fn resume_assignment(
    State(ctx): State<AppContext>,
    Path((household, id)): Path<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: axum::body::Bytes,
) -> Response {
    change(ctx, household, id, headers, request, body, (false, false)).await
}

async fn change(
    ctx: AppContext,
    household: i64,
    id: String,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: axum::body::Bytes,
    action: (bool, bool),
) -> Response {
    let (schedule, pause) = action;
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let savepoint = match tenant.transaction().begin().await {
        Ok(value) => value,
        Err(_) => return response::error(response::unavailable(), &request_id),
    };
    let body = if body.is_empty() {
        Ok(json!({}))
    } else {
        serde_json::from_slice::<Value>(&body)
            .map_err(|_| response::Failure::bad_request("Invalid JSON request body"))
    };
    let result = match body {
        Err(error) => Err(error),
        Ok(body) => {
            let operation = match (schedule, pause) {
                (true, true) => {
                    pause_periods::pause_schedule(&tenant, &id, &body, Some(principal.provenance()))
                        .await
                }
                (true, false) => {
                    pause_periods::resume_schedule(
                        &tenant,
                        &id,
                        &body,
                        Some(principal.provenance()),
                    )
                    .await
                }
                (false, true) => {
                    pause_periods::pause_assignment(
                        &tenant,
                        &id,
                        &body,
                        Some(principal.provenance()),
                    )
                    .await
                }
                (false, false) => {
                    pause_periods::resume_assignment(
                        &tenant,
                        &id,
                        &body,
                        Some(principal.provenance()),
                    )
                    .await
                }
            };
            operation
                .map(|(body, etag)| (StatusCode::OK, body, Some(etag), false))
                .map_err(response::operation)
        }
    };
    let restored = if result.is_ok() {
        savepoint.commit().await
    } else {
        savepoint.rollback().await
    };
    if restored.is_err() {
        return response::error(response::unavailable(), &request_id);
    }
    let action = if pause { "pause" } else { "resume" };
    let audit = if schedule {
        audit::RequestAudit::treatment("PATCH", action)
    } else {
        audit::RequestAudit::assignment("PATCH", action)
    };
    complete(tenant, &principal, audit, result, &request_id).await
}
