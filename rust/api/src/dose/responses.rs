use super::*;

pub(super) fn error(status: StatusCode, message: &'static str) -> ApiError {
    ApiError {
        status,
        code: if status == StatusCode::CONFLICT {
            "conflict"
        } else {
            "unprocessable_content"
        },
        message,
        preserve_activity: false,
    }
}

pub(super) fn database_error(error: sea_orm::DbErr) -> ApiError {
    let class = match error.sql_err() {
        Some(sea_orm::SqlErr::UniqueConstraintViolation(_)) => "unique_constraint",
        Some(sea_orm::SqlErr::ForeignKeyConstraintViolation(_)) => "foreign_key",
        Some(_) => "sql_other",
        None => match error {
            sea_orm::DbErr::RecordNotInserted => "not_inserted",
            sea_orm::DbErr::RecordNotFound(_) => "not_found",
            sea_orm::DbErr::Query(_) => "query",
            sea_orm::DbErr::Exec(_) => "execution",
            _ => "other",
        },
    };
    eprintln!("dose database operation failed: {class}");
    ApiError::internal()
}

pub(super) fn request_error_response(error: ApiError, request_id: &str) -> Response {
    let mut response = (error.status, Json(json!({"error": {"code": error.code, "message": error.message, "request_id": request_id}}))).into_response();
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(request_id).unwrap_or_else(|_| HeaderValue::from_static("invalid")),
    );
    response
}

pub(super) fn success_response(
    status: StatusCode,
    body: Value,
    request_id: &str,
    etag: Option<String>,
) -> Response {
    let mut response = (status, Json(body)).into_response();
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(request_id).unwrap_or_else(|_| HeaderValue::from_static("invalid")),
    );
    if let Some(etag) = etag {
        response.headers_mut().insert(
            header::ETAG,
            HeaderValue::from_str(&etag).unwrap_or_else(|_| HeaderValue::from_static("invalid")),
        );
    }
    response
}

#[derive(Clone, Copy)]
pub(super) enum TakeFailureCause {
    Paused,
    OutOfStock,
    NumericDoseAmount,
}

pub(crate) struct TakeFailure {
    pub(super) error: ApiError,
    pub(super) cause: Option<TakeFailureCause>,
}

impl From<ApiError> for TakeFailure {
    fn from(error: ApiError) -> Self {
        Self { error, cause: None }
    }
}

impl TakeFailure {
    pub(crate) fn into_api_error(self) -> ApiError {
        self.error
    }

    pub(crate) fn into_occurrence_error(self) -> ApiError {
        let code = match self.cause {
            Some(TakeFailureCause::Paused) => "paused",
            Some(TakeFailureCause::OutOfStock) => "out_of_stock",
            _ => return self.error,
        };
        ApiError {
            status: self.error.status,
            code,
            message: "Dose could not be recorded",
            preserve_activity: self.error.preserve_activity,
        }
    }
}

pub(super) fn take_error_response(failure: TakeFailure, request_id: &str) -> Response {
    if matches!(failure.cause, Some(TakeFailureCause::NumericDoseAmount)) {
        return success_response(
            failure.error.status,
            json!({"error": {"code": "validation_failed", "message": "Validation failed",
                "errors": {"dose_amount": ["must be a string"]}, "request_id": request_id}}),
            request_id,
            None,
        );
    }
    request_error_response(failure.error, request_id)
}
