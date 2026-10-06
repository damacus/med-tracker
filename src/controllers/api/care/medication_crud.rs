use super::*;

pub(super) async fn create(
    State(ctx): State<AppContext>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    let request_id = request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(context) => context,
        Err(error) => return response::error(error, &request_id),
    };
    let savepoint = match tenant.transaction().begin().await {
        Ok(savepoint) => savepoint,
        Err(_) => return response::error(response::unavailable(), &request_id),
    };
    let result = match attributes(body) {
        Ok(attributes) => {
            medications::crud::create(&tenant, attributes, Some(principal.provenance()))
                .await
                .map_err(failure)
        }
        Err(error) => Err(error),
    };
    let reply = representation(&tenant, result, StatusCode::CREATED).await;
    let completed = if reply.is_ok() {
        savepoint.commit().await
    } else {
        savepoint.rollback().await
    };
    if completed.is_err() {
        return response::error(response::unavailable(), &request_id);
    }
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::medication("POST", "create"),
        reply,
        &request_id,
    )
    .await
}

pub(super) async fn update(
    State(ctx): State<AppContext>,
    Path((household_id, id)): Path<(i64, String)>,
    method: axum::http::Method,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    let request_id = request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(context) => context,
        Err(error) => return response::error(error, &request_id),
    };
    let savepoint = match tenant.transaction().begin().await {
        Ok(savepoint) => savepoint,
        Err(_) => return response::error(response::unavailable(), &request_id),
    };
    let result = match attributes(body) {
        Ok(attributes) => medications::crud::update(
            &tenant,
            &id,
            attributes,
            headers.get("if-match").and_then(|v| v.to_str().ok()),
            Some(principal.provenance()),
        )
        .await
        .map_err(failure),
        Err(error) => Err(error),
    };
    let reply = representation(&tenant, result, StatusCode::OK).await;
    let completed = if reply.is_ok() {
        savepoint.commit().await
    } else {
        savepoint.rollback().await
    };
    if completed.is_err() {
        return response::error(response::unavailable(), &request_id);
    }
    let method = if method == axum::http::Method::PUT {
        "PUT"
    } else {
        "PATCH"
    };
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::medication(method, "update"),
        reply,
        &request_id,
    )
    .await
}

async fn representation(
    tenant: &TenantTransaction,
    result: std::result::Result<crate::models::entities::medication::Model, response::Failure>,
    status: StatusCode,
) -> Reply {
    match result {
        Ok(record) => medications::read_stock_snapshot(tenant, &record.id.to_string())
            .await
            .map(|snapshot| (status, snapshot.representation, Some(snapshot.etag)))
            .map_err(response::operation),
        Err(error) => Err(error),
    }
}

fn attributes(
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> std::result::Result<Value, response::Failure> {
    let AxumJson(body) =
        body.map_err(|_| response::Failure::bad_request("Invalid JSON request body"))?;
    let outer = body
        .as_object()
        .ok_or_else(|| response::Failure::bad_request("medication is required"))?;
    let attributes = outer
        .get("medication")
        .filter(|value| value.is_object())
        .ok_or_else(|| response::Failure::bad_request("medication is required"))?;
    if outer.len() != 1 {
        return Err(response::Failure::field(
            "medication",
            "contains an unknown root field",
        ));
    }
    Ok(attributes.clone())
}

fn failure(error: OperationError) -> response::Failure {
    if let OperationError::Validation { details } = &error
        && let Some(errors) = details.get("errors").filter(|value| value.is_object())
    {
        return response::Failure::fields(errors.clone());
    }
    response::operation(error)
}
