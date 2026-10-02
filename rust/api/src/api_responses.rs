use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[derive(Debug)]
pub(super) struct ApiError {
    pub(super) status: StatusCode,
    pub(super) code: &'static str,
    pub(super) message: &'static str,
    pub(super) preserve_activity: bool,
}

impl ApiError {
    pub(super) fn unauthorized() -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code: "unauthorized",
            message: "Authentication required",
            preserve_activity: false,
        }
    }

    pub(super) fn forbidden() -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            code: "forbidden",
            message: "You are not authorized to perform this action.",
            preserve_activity: false,
        }
    }

    pub(super) fn not_found() -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "not_found",
            message: "Record not found",
            preserve_activity: false,
        }
    }

    pub(super) fn internal() -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "internal_error",
            message: "Internal server error",
            preserve_activity: false,
        }
    }

    pub(super) fn invalid_filter() -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: "unprocessable_content",
            message: "updated_since must be ISO8601",
            preserve_activity: false,
        }
    }

    pub(super) fn invalid_pagination() -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: "unprocessable_content",
            message: "page must be positive and per_page must be between 1 and 100",
            preserve_activity: false,
        }
    }

    pub(super) fn preserve_activity(mut self) -> Self {
        self.preserve_activity = true;
        self
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let request_id = uuid::Uuid::new_v4().to_string();
        let mut response = (
            self.status,
            Json(json!({"error": {"code": self.code, "message": self.message, "request_id": request_id}})),
        )
            .into_response();
        response.headers_mut().insert(
            "x-request-id",
            HeaderValue::from_str(&request_id).expect("UUID request ID is a valid header"),
        );
        response
    }
}

pub(super) fn database_error(_: sea_orm::DbErr) -> ApiError {
    eprintln!("medication database operation failed");
    ApiError::internal()
}

pub(super) fn if_none_match_matches(headers: &HeaderMap, etag: &str) -> bool {
    headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value.split(',').any(|tag| {
                let tag = tag.trim();
                tag == "*" || tag.strip_prefix("W/").unwrap_or(tag) == etag
            })
        })
}

pub(super) fn representation_etag(body: &Value) -> String {
    format!(
        "\"{}\"",
        hex::encode(Sha256::digest(
            serde_json::to_vec(body).expect("JSON value must serialize")
        ))
    )
}

#[derive(Deserialize)]
pub(super) struct Pagination {
    pub(super) page: Option<i64>,
    pub(super) per_page: Option<i64>,
    pub(super) updated_since: Option<String>,
}
