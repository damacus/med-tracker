use super::*;

#[derive(Clone, Copy)]
pub(super) enum Failure {
    Malformed,
    Invalid(&'static str, &'static str),
    NotFound,
    Forbidden,
    AlreadyResolved,
    InvalidOccurrence,
    PreconditionRequired,
    SyncConflict,
    KeyConflict,
}

pub(super) async fn fail(
    db: DatabaseTransaction,
    context: &AuthContext,
    kind: Kind,
    method: &str,
    action: &str,
    failure: Failure,
) -> Result<Response, ApiError> {
    let (status, code, message, errors) = match failure {
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
        Failure::AlreadyResolved => (
            StatusCode::CONFLICT,
            "already_resolved",
            "Occurrence is already resolved",
            None,
        ),
        Failure::InvalidOccurrence => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_occurrence",
            "Occurrence is unavailable",
            None,
        ),
        Failure::PreconditionRequired => (
            StatusCode::PRECONDITION_REQUIRED,
            "precondition_required",
            "A current version is required",
            None,
        ),
        Failure::SyncConflict => (
            StatusCode::CONFLICT,
            "sync_conflict",
            "Occurrence has changed",
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
        db,
        context,
        method,
        kind.controller(),
        kind.policy(),
        action,
        status,
        code,
        message,
        errors,
    )
    .await
}

pub(super) async fn fail_api(
    db: DatabaseTransaction,
    context: &AuthContext,
    kind: Kind,
    method: &str,
    action: &str,
    error: ApiError,
) -> Result<Response, ApiError> {
    error_response(
        db,
        context,
        method,
        kind.controller(),
        kind.policy(),
        action,
        error.status,
        error.code,
        error.message,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn finish(
    db: DatabaseTransaction,
    context: &AuthContext,
    kind: Kind,
    method: &str,
    action: &str,
    body: Value,
    etag: Option<&str>,
    request_id: &str,
) -> Result<Response, ApiError> {
    finish_with_request_id(
        db,
        context,
        request_id,
        method,
        kind.controller(),
        kind.policy(),
        action,
        StatusCode::OK,
        true,
        body,
        etag,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn fail_mutation(
    db: DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    kind: Kind,
    method: &str,
    action: &str,
    path: &str,
    request: &Value,
    failure: Failure,
) -> Result<Response, ApiError> {
    let (status, code, message, errors) = match failure {
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
        Failure::InvalidOccurrence => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_occurrence",
            "Occurrence is unavailable",
            None,
        ),
        Failure::PreconditionRequired => (
            StatusCode::PRECONDITION_REQUIRED,
            "precondition_required",
            "A current version is required",
            None,
        ),
        _ => return fail(db, context, kind, method, action, failure).await,
    };
    finish_cached_error(
        db, context, headers, kind, method, action, path, request, status, code, message, errors,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn finish_cached_error(
    db: DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    kind: Kind,
    method: &str,
    action: &str,
    path: &str,
    request: &Value,
    status: StatusCode,
    code: &str,
    message: &str,
    errors: Option<Value>,
) -> Result<Response, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let mut body = json!({"error": {"code": code, "message": message}});
    if let Some(errors) = errors {
        body["error"]["errors"] = errors;
    }
    let mut stored = body.clone();
    stored["error"]["request_id"] = json!(request_id);
    if let Some(key) = mutation_idempotency::key(headers) {
        let digest = mutation_idempotency::digest(method, path, request);
        mutation_idempotency::store(
            &db,
            context,
            StoredResponse {
                key,
                method,
                path,
                digest: &digest,
                status,
                body: stored,
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
        kind.controller(),
        kind.policy(),
        action,
        status,
        false,
        body,
        None,
    )
    .await
}

pub(super) fn sync_error(failure: Failure) -> ApiError {
    let (status, code, message) = match failure {
        Failure::Malformed => (
            StatusCode::BAD_REQUEST,
            "bad_request",
            "Invalid request body",
        ),
        Failure::Invalid(_, _) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_occurrence",
            "Outcome is invalid",
        ),
        Failure::NotFound => (StatusCode::NOT_FOUND, "not_found", "Record not found"),
        Failure::Forbidden => (
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
        ),
        Failure::AlreadyResolved => (
            StatusCode::CONFLICT,
            "already_resolved",
            "Occurrence is already resolved",
        ),
        Failure::InvalidOccurrence => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_occurrence",
            "Occurrence is unavailable",
        ),
        Failure::PreconditionRequired => (
            StatusCode::PRECONDITION_REQUIRED,
            "precondition_required",
            "A current version is required",
        ),
        Failure::SyncConflict => (
            StatusCode::CONFLICT,
            "sync_conflict",
            "Occurrence has changed",
        ),
        Failure::KeyConflict => (
            StatusCode::CONFLICT,
            "idempotency_key_reused",
            "Idempotency key has already been used for a different request",
        ),
    };
    ApiError {
        status,
        code,
        message,
        preserve_activity: false,
    }
}
