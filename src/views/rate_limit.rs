use serde::Serialize;

#[derive(Serialize)]
pub struct RateLimitResponse {
    pub error: RateLimitError,
}

#[derive(Serialize)]
pub struct RateLimitError {
    pub code: &'static str,
    pub message: &'static str,
}

impl Default for RateLimitResponse {
    fn default() -> Self {
        Self {
            error: RateLimitError {
                code: "rate_limited",
                message: "Too many requests",
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
            },
        }
    }
}
