use super::ClinicalStore;
use async_trait::async_trait;
use better_auth_core::{
    AuthError, AuthResult,
    middleware::{EndpointRateLimit, RateLimitDecision},
    store::RateLimitStore,
};

#[async_trait]
impl RateLimitStore for ClinicalStore {
    async fn consume_rate_limit(
        &self,
        _key: &str,
        _rule: EndpointRateLimit,
        _cleanup_window: f64,
    ) -> AuthResult<RateLimitDecision> {
        Err(AuthError::config(
            "Database rate limiting is not configured",
        ))
    }
}
