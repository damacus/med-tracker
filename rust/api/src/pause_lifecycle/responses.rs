use super::*;

pub(super) async fn fail(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    controller: &str,
    policy: &str,
    action: &str,
    failure: Failure,
) -> Result<Response, ApiError> {
    let (status, code, message, errors) = match failure {
        Failure::PreconditionRequired => (
            StatusCode::PRECONDITION_REQUIRED,
            "precondition_required",
            "If-Match is required",
            None,
        ),
        Failure::Malformed => (
            StatusCode::BAD_REQUEST,
            "bad_request",
            "Invalid request body",
            None,
        ),
        Failure::Invalid(field, message) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            "Validation failed",
            Some(json!({field: [message]})),
        ),
        Failure::NotFound => (StatusCode::NOT_FOUND, "not_found", "Record not found", None),
        Failure::Forbidden => (
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
            None,
        ),
        Failure::Conflict => (
            StatusCode::CONFLICT,
            "conflict",
            "Record has changed since it was last read",
            None,
        ),
        Failure::KeyConflict => (
            StatusCode::CONFLICT,
            "idempotency_key_reused",
            "Idempotency key has already been used for a different request",
            None,
        ),
    };
    error_response(
        db, context, method, controller, policy, action, status, code, message, errors,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn finish(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    controller: &str,
    policy: &str,
    action: &str,
    status: StatusCode,
    body: Value,
    etag: Option<&str>,
    request_id: &str,
) -> Result<Response, ApiError> {
    finish_with_request_id(
        db, context, request_id, method, controller, policy, action, status, true, body, etag,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn keyed_invalid(
    db: DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    method: &str,
    action: &str,
    path: &str,
    request: &Value,
    field: &'static str,
    message: &'static str,
) -> Result<Response, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let mut body = json!({"error": {
        "code": "validation_failed",
        "message": "Validation failed",
        "errors": {field: [message]},
    }});
    let mut stored = body.clone();
    stored["error"]["request_id"] = json!(request_id);
    store_key(
        &db,
        context,
        headers,
        method,
        path,
        request,
        StatusCode::UNPROCESSABLE_ENTITY,
        &stored,
        &request_id,
        None,
    )
    .await?;
    finish_with_request_id(
        db,
        context,
        &request_id,
        method,
        PERIOD_CONTROLLER,
        PERIOD_POLICY,
        action,
        StatusCode::UNPROCESSABLE_ENTITY,
        false,
        std::mem::take(&mut body),
        None,
    )
    .await
}
