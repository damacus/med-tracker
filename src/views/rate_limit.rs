use serde::Serialize;

#[derive(Serialize)]
pub struct RateLimitResponse {
    pub error: RateLimitError,
}

#[derive(Serialize)]
pub struct RateLimitError {
    pub code: &'static str,
    pub message: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

impl Default for RateLimitResponse {
    fn default() -> Self {
        Self {
            error: RateLimitError {
                code: "rate_limited",
                message: "Too many requests",
                trace_id: crate::telemetry::ids::current_trace_id(),
            },
        }
    }
}

impl RateLimitResponse {
    pub fn source_unavailable() -> Self {
        Self {
            error: RateLimitError {
                code: "service_unavailable",
                message: "Client source unavailable",
                trace_id: crate::telemetry::ids::current_trace_id(),
            },
        }
    }
}
