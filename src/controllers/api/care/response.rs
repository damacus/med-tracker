use super::*;
use axum::http::{HeaderValue, header};

pub(super) struct Failure {
    pub status: StatusCode,
    code: String,
    message: String,
    details: Option<Value>,
    authenticate: Option<String>,
}

impl Failure {
    pub fn fields(errors: Value) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: "validation_failed".into(),
            message: "Validation failed".into(),
            details: Some(errors),
            authenticate: None,
        }
    }
    pub fn bad_request(message: &str) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "bad_request".into(),
            message: message.into(),
            details: None,
            authenticate: None,
        }
    }
    pub fn field(field: &str, message: &str) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: "validation_failed".into(),
            message: "Validation failed".into(),
            details: Some(json!({field: [message]})),
            authenticate: None,
        }
    }
    pub fn numeric_dose() -> Self {
        Self::field("dose_amount", "must be a string")
    }
    pub fn validation(message: &str) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: "unprocessable_content".into(),
            message: message.into(),
            details: None,
            authenticate: None,
        }
    }
}

pub(super) fn authentication(error: AuthenticationError) -> Failure {
    match error {
        AuthenticationError::Unauthenticated => Failure {
            status: StatusCode::UNAUTHORIZED,
            code: "unauthorized".into(),
            message: "Authentication required".into(),
            details: None,
            authenticate: None,
        },
        AuthenticationError::InsufficientScope { authenticate } => Failure {
            status: StatusCode::FORBIDDEN,
            code: "forbidden".into(),
            message: "You are not authorized to perform this action.".into(),
            details: None,
            authenticate: Some(authenticate),
        },
        AuthenticationError::Forbidden => operation(OperationError::Forbidden),
        AuthenticationError::Unavailable => unavailable(),
    }
}

pub(super) fn unavailable() -> Failure {
    Failure {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        code: "internal_error".into(),
        message: "Internal server error".into(),
        details: None,
        authenticate: None,
    }
}

pub(super) fn operation(error: OperationError) -> Failure {
    match error {
        OperationError::Unauthenticated => authentication(AuthenticationError::Unauthenticated),
        OperationError::Forbidden => Failure {
            status: StatusCode::FORBIDDEN,
            code: "forbidden".into(),
            message: "You are not authorized to perform this action.".into(),
            details: None,
            authenticate: None,
        },
        OperationError::NotFound => Failure {
            status: StatusCode::NOT_FOUND,
            code: "not_found".into(),
            message: "Record not found".into(),
            details: None,
            authenticate: None,
        },
        OperationError::Unavailable => unavailable(),
        OperationError::Validation { details } => Failure {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: "unprocessable_content".into(),
            message: details
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("Invalid operation")
                .into(),
            details: None,
            authenticate: None,
        },
        OperationError::Conflict { code, details } => Failure {
            status: StatusCode::CONFLICT,
            code,
            message: details
                .get("error")
                .or_else(|| details.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("Record has changed since it was last read")
                .into(),
            details: None,
            authenticate: None,
        },
    }
}

pub(super) fn error(error: Failure, request_id: &str) -> Response {
    let mut body =
        json!({"error": {"code": error.code, "message": error.message, "request_id": request_id}});
    if let Some(details) = error.details {
        body["error"]["errors"] = details;
    }
    let mut response = success(error.status, body, request_id, None);
    if let Some(challenge) = error
        .authenticate
        .and_then(|value| HeaderValue::from_str(&value).ok())
    {
        response
            .headers_mut()
            .insert(header::WWW_AUTHENTICATE, challenge);
    } else if error.status == StatusCode::UNAUTHORIZED {
        response
            .headers_mut()
            .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
    }
    response
}

pub(super) fn success(
    status: StatusCode,
    body: Value,
    request_id: &str,
    etag: Option<String>,
) -> Response {
    let mut response = (status, AxumJson(body)).into_response();
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(request_id).expect("Generated request UUID"),
    );
    if let Some(etag) = etag {
        response.headers_mut().insert(
            header::ETAG,
            HeaderValue::from_str(&etag).expect("Generated quoted ETag"),
        );
    }
    response
}
