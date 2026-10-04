use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};

pub(super) fn redirect(location: impl AsRef<str>, cookie: Option<HeaderValue>) -> Response {
    let mut response = (
        StatusCode::SEE_OTHER,
        [
            (header::LOCATION, location.as_ref().to_owned()),
            (header::CACHE_CONTROL, "no-store".to_owned()),
        ],
    )
        .into_response();
    if let Some(cookie) = cookie {
        response.headers_mut().append(header::SET_COOKIE, cookie);
    }
    response
}

pub(super) fn page(body: String, cookie: Option<HeaderValue>) -> Response {
    page_status(body, cookie, StatusCode::OK)
}

pub(super) fn profile_page_status(
    body: String,
    cookie: Option<HeaderValue>,
    status: StatusCode,
) -> Response {
    let mut response = page_status(body, cookie, status);
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'none'; style-src 'self'; script-src 'self'; connect-src 'self'; font-src 'self'; worker-src 'self'; img-src 'self'; form-action 'self'; base-uri 'none'; frame-ancestors 'none'"),
    );
    response
}

pub(super) fn profile_page(body: String, cookie: Option<HeaderValue>) -> Response {
    profile_page_status(body, cookie, StatusCode::OK)
}

pub(super) fn dashboard_page(body: String, cookie: Option<HeaderValue>) -> Response {
    let mut response = page(body, cookie);
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'none'; style-src 'self'; script-src 'self' 'wasm-unsafe-eval'; connect-src 'self'; font-src 'self'; worker-src 'self'; manifest-src 'self'; img-src 'self'; form-action 'self'; base-uri 'none'; frame-ancestors 'none'"),
    );
    response
}

pub(super) fn page_status(
    body: String,
    cookie: Option<HeaderValue>,
    status: StatusCode,
) -> Response {
    let mut response = (
        status,
        [
            (header::CACHE_CONTROL, "private, no-store"),
            (header::CONTENT_SECURITY_POLICY, "default-src 'none'; style-src 'self'; script-src 'self'; font-src 'self'; worker-src 'self'; form-action 'self'; base-uri 'none'; frame-ancestors 'none'"),
        ],
        axum::response::Html(body),
    ).into_response();
    if let Some(cookie) = cookie {
        response.headers_mut().append(header::SET_COOKIE, cookie);
    }
    response
}

pub(super) fn failure(status: StatusCode) -> Response {
    (status, [(header::CACHE_CONTROL, "no-store")]).into_response()
}

pub(super) fn login_redirect() -> Response {
    (
        StatusCode::FOUND,
        [
            (header::LOCATION, "/login"),
            (header::CACHE_CONTROL, "no-store"),
        ],
    )
        .into_response()
}

#[derive(Debug)]
pub(super) enum PageError {
    Status(StatusCode),
    Login,
    InvalidJson(serde_json::Error),
}

impl PageError {
    pub(super) fn response(self) -> Response {
        match self {
            Self::Status(status) => failure(status),
            Self::Login => login_redirect(),
            Self::InvalidJson(error) => {
                eprintln!("Internal API JSON failed validation: {error}");
                failure(StatusCode::BAD_GATEWAY)
            }
        }
    }
}

pub(super) fn error(status: StatusCode) -> PageError {
    PageError::Status(status)
}
