use reqwest::header::ToStrError;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ImportDiagnosticReason {
    UnsupportedField,
    InvalidField,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ImportDiagnostic {
    pub path: String,
    pub reason: ImportDiagnosticReason,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ImportDiagnostics {
    pub items: Vec<ImportDiagnostic>,
    pub truncated: bool,
}

#[derive(thiserror::Error)]
#[error("{code}: {message}")]
pub struct ImportValidationError {
    pub status: u16,
    pub code: String,
    pub message: String,
    pub request_id: Option<String>,
    pub trace_id: Option<String>,
    pub retry_after: Option<String>,
    pub diagnostics: ImportDiagnostics,
}

impl std::fmt::Debug for ImportValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ImportValidationError")
            .field("status", &self.status)
            .field("code", &self.code)
            .field("request_id", &self.request_id)
            .field("trace_id", &self.trace_id)
            .field("retry_after", &self.retry_after)
            .field("diagnostic_count", &self.diagnostics.items.len())
            .field("truncated", &self.diagnostics.truncated)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Deserialize)]
pub struct ErrorEnvelope {
    pub error: ErrorBody,
}

#[derive(Debug, Deserialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
    pub request_id: Option<String>,
    pub trace_id: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("invalid HTTP server origin")]
    InvalidServerOrigin,
    #[error("invalid request header")]
    InvalidRequestHeader,
    #[error("invalid base URL: {0}")]
    InvalidBaseUrl(#[from] url::ParseError),
    #[error("invalid response header: {0}")]
    InvalidHeader(#[from] ToStrError),
    #[error("HTTP client error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("unsupported_by_server: {feature}")]
    UnsupportedByServer { feature: String },
    #[error(transparent)]
    ImportValidation(Box<ImportValidationError>),
    #[error("{code}: {message}")]
    Server {
        status: u16,
        code: String,
        message: String,
        request_id: Option<String>,
        trace_id: Option<String>,
        retry_after: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::ApiError;

    #[test]
    fn api_error_fits_result_large_err_limit() {
        assert!(std::mem::size_of::<ApiError>() <= 128);
    }
}
