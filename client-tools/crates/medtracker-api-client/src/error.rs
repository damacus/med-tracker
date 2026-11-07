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
    #[error("{code}: {message}")]
    Server {
        status: u16,
        code: String,
        message: String,
        request_id: Option<String>,
        trace_id: Option<String>,
        retry_after: Option<String>,
        import_diagnostics: Option<ImportDiagnostics>,
    },
}
