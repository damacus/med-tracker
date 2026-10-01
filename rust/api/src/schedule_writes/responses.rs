use super::*;

pub(super) async fn input_error(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    action: &str,
    failure: InputFailure,
) -> Result<Response, ApiError> {
    match failure {
        InputFailure::Malformed => {
            error_response(
                db,
                context,
                method,
                CONTROLLER,
                POLICY,
                action,
                StatusCode::BAD_REQUEST,
                "bad_request",
                "Invalid request body",
                None,
            )
            .await
        }
        InputFailure::NotFound => {
            error_response(
                db,
                context,
                method,
                CONTROLLER,
                POLICY,
                action,
                StatusCode::NOT_FOUND,
                "not_found",
                "Record not found",
                None,
            )
            .await
        }
        InputFailure::Forbidden => {
            error_response(
                db,
                context,
                method,
                CONTROLLER,
                POLICY,
                action,
                StatusCode::FORBIDDEN,
                "forbidden",
                "You are not authorized to perform this action.",
                None,
            )
            .await
        }
        InputFailure::Invalid(field, message) => {
            error_response(
                db,
                context,
                method,
                CONTROLLER,
                POLICY,
                action,
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_failed",
                "Validation failed",
                Some(json!({field: [message]})),
            )
            .await
        }
        InputFailure::InvalidFields(errors) => {
            error_response(
                db,
                context,
                method,
                CONTROLLER,
                POLICY,
                action,
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_failed",
                "Validation failed",
                Some(errors),
            )
            .await
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn keyed_validation(
    db: DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    method: &str,
    action: &str,
    path: &str,
    request: &Value,
    failure: InputFailure,
) -> Result<Response, ApiError> {
    let errors = match failure {
        InputFailure::Invalid(field, message) => json!({field: [message]}),
        InputFailure::InvalidFields(errors) => errors,
        other => return input_error(db, context, method, action, other).await,
    };
    let response_body = json!({"error": {
        "code": "validation_failed",
        "message": "Validation failed",
        "errors": errors,
    }});
    let request_id = Uuid::new_v4().to_string();
    let mut stored_body = response_body.clone();
    stored_body["error"]["request_id"] = json!(request_id);
    store_key(
        &db,
        context,
        headers,
        method,
        path,
        request,
        StatusCode::UNPROCESSABLE_ENTITY,
        &stored_body,
        &request_id,
        None,
    )
    .await?;
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
        response_body,
        None,
    )
    .await
}
