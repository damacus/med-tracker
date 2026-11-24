use super::*;
use axum::http::{HeaderValue, header};

#[derive(serde::Serialize)]
pub(super) struct ErrorEnvelope {
    error: ErrorBody,
}

impl ErrorEnvelope {
    pub(super) fn new(
        code: impl Into<String>,
        message: impl Into<String>,
        request_id: &str,
        errors: Option<Value>,
    ) -> Self {
        Self {
            error: ErrorBody {
                code: code.into(),
                message: message.into(),
                request_id: request_id.into(),
                errors: errors.map(Box::new),
            },
        }
    }
}

#[derive(serde::Serialize)]
struct ErrorBody {
    code: String,
    message: String,
    request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    errors: Option<Box<Value>>,
}

#[derive(serde::Serialize)]
#[serde(untagged)]
pub(super) enum MutationBody<T> {
    Resource(T),
    Error(ErrorEnvelope),
    Replay(Value),
    Empty,
}

pub(super) struct Failure {
    pub status: StatusCode,
    code: String,
    message: String,
    details: Option<Box<Value>>,
    authenticate: Option<String>,
}

impl Failure {
    pub(super) fn into_body(self, request_id: &str) -> ErrorEnvelope {
        ErrorEnvelope {
            error: ErrorBody {
                code: self.code,
                message: self.message,
                request_id: request_id.into(),
                errors: self.details,
            },
        }
    }

    pub fn avatar_unavailable() -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "avatar_unavailable".into(),
            message: "Avatar storage is temporarily unavailable".into(),
            details: None,
            authenticate: None,
        }
    }
    pub fn report_unavailable() -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "report_unavailable".into(),
            message: "Report is temporarily unavailable".into(),
            details: None,
            authenticate: None,
        }
    }
    pub fn invitation_delivery_unavailable() -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "invitation_delivery_unavailable".into(),
            message: "Invitation delivery is temporarily unavailable".into(),
            details: None,
            authenticate: None,
        }
    }
    pub fn fields(errors: Value) -> Self {
        Self::validation_fields("Validation failed", errors)
    }
    pub fn validation_fields(message: &str, errors: Value) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: "validation_failed".into(),
            message: message.into(),
            details: Some(Box::new(errors)),
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
    pub fn precondition_required(message: &str) -> Self {
        Self {
            status: StatusCode::PRECONDITION_REQUIRED,
            code: "precondition_required".into(),
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
            details: Some(Box::new(json!({field: [message]}))),
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
        AuthenticationError::NotFound => operation(OperationError::NotFound),
        AuthenticationError::Unavailable => unavailable(),
    }
}

pub(super) fn unavailable() -> Failure {
    Failure {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "service_unavailable".into(),
        message: "Service is temporarily unavailable".into(),
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

pub(super) fn error(mut error: Failure, request_id: &str) -> Response {
    let status = error.status;
    let challenge = error
        .authenticate
        .take()
        .and_then(|value| HeaderValue::from_str(&value).ok());
    let mut response = success(status, error.into_body(request_id), request_id, None);
    if let Some(challenge) = challenge {
        response
            .headers_mut()
            .insert(header::WWW_AUTHENTICATE, challenge);
    } else if status == StatusCode::UNAUTHORIZED {
        response
            .headers_mut()
            .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
    }
    response
}

pub(super) fn success(
    status: StatusCode,
    body: impl serde::Serialize,
    request_id: &str,
    etag: Option<String>,
) -> Response {
    let mut response = if (status.is_client_error() || status.is_server_error())
        && let Ok(mut body) = serde_json::to_value(&body)
    {
        crate::telemetry::ids::enrich_error(&mut body);
        (status, AxumJson(body)).into_response()
    } else {
        (status, AxumJson(body)).into_response()
    };
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unavailable_maps_to_service_unavailable_envelope() {
        for failure in [
            operation(OperationError::Unavailable),
            authentication(AuthenticationError::Unavailable),
            unavailable(),
        ] {
            let response = error(failure, "request-id");
            assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
            let bytes = axum::body::to_bytes(response.into_body(), 4096)
                .await
                .expect("error envelope body");
            let body: Value = serde_json::from_slice(&bytes).expect("error envelope JSON");
            assert_eq!(body["error"]["code"], "service_unavailable");
            assert_eq!(body["error"]["request_id"], "request-id");
        }
    }
}
