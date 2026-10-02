use super::*;

pub(super) fn path(household_id: i64, id: Option<&str>) -> String {
    let base = format!("/api/v1/households/{household_id}/person_medications");
    id.map_or(base.clone(), |id| format!("{base}/{id}"))
}

pub(super) async fn failure(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    action: &str,
    status: StatusCode,
) -> Result<Response, ApiError> {
    let (code, message, errors) = match status {
        StatusCode::BAD_REQUEST => ("bad_request", "Invalid request", None),
        StatusCode::FORBIDDEN => (
            "forbidden",
            "You are not authorized to perform this action.",
            None,
        ),
        StatusCode::NOT_FOUND => ("not_found", "Record not found", None),
        StatusCode::CONFLICT => (
            "conflict",
            "Record has changed since it was last read",
            None,
        ),
        _ => (
            "validation_failed",
            "Validation failed",
            Some(json!({"person_medication": ["is invalid"]})),
        ),
    };
    error_response(
        db, context, method, CONTROLLER, POLICY, action, status, code, message, errors,
    )
    .await
}

pub(super) async fn validation_failure(
    db: DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    method: &str,
    action: &str,
    request_path: &str,
    request_digest: &str,
) -> Result<Response, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let body = json!({"error": {
        "code": "validation_failed",
        "message": "Validation failed",
        "errors": {"person_medication": ["is invalid"]},
        "request_id": request_id
    }});
    if let Some(key) = mutation_idempotency::key(headers) {
        mutation_idempotency::store(
            &db,
            context,
            StoredResponse {
                key,
                method,
                path: request_path,
                digest: request_digest,
                status: StatusCode::UNPROCESSABLE_ENTITY,
                body: body.clone(),
                request_id: &request_id,
                etag: None,
            },
        )
        .await?;
    }
    finish_with_request_id(
        db,
        context,
        &request_id,
        method,
        CONTROLLER,
        POLICY,
        action,
        StatusCode::UNPROCESSABLE_ENTITY,
        false,
        body,
        None,
    )
    .await
}

pub(super) async fn representation(
    db: &DatabaseTransaction,
    context: &AuthContext,
    id: i64,
) -> Result<(Value, String), ApiError> {
    let row = read_assignment::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?;
    let data = serialize_assignments(db, context, vec![row])
        .await?
        .pop()
        .ok_or_else(ApiError::internal)?;
    let body = json!({"data": data});
    let etag = representation_etag(&body);
    Ok((body, etag))
}

pub(super) async fn finish_write(
    db: DatabaseTransaction,
    write: WriteCompletion<'_>,
) -> Result<Response, ApiError> {
    let WriteCompletion {
        context,
        headers,
        method,
        action,
        request_path,
        request_digest,
        request_id,
        status,
        body,
        etag,
    } = write;
    if let Some(key) = mutation_idempotency::key(headers) {
        mutation_idempotency::store(
            &db,
            context,
            StoredResponse {
                key,
                method,
                path: request_path,
                digest: request_digest,
                status,
                body: body.clone(),
                request_id,
                etag: Some(etag),
            },
        )
        .await?;
    }
    finish_with_request_id(
        db,
        context,
        request_id,
        method,
        CONTROLLER,
        POLICY,
        action,
        status,
        true,
        body,
        Some(etag),
    )
    .await
}
