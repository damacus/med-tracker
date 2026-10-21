use async_trait::async_trait;
use better_auth_core::{
    AuthError, AuthResult,
    middleware::{EndpointRateLimit, RateLimitDecision},
    store::RateLimitStore,
};
use sea_orm::ConnectionTrait;
use sha2::{Digest, Sha256};

use super::{ClinicalStore, context, database_error, statement};

#[async_trait]
impl RateLimitStore for ClinicalStore {
    async fn consume_rate_limit(
        &self,
        key: &str,
        rule: EndpointRateLimit,
        cleanup_window: f64,
    ) -> AuthResult<RateLimitDecision> {
        if !rule.window.is_finite()
            || rule.window <= 0.0
            || !rule.max_requests.is_finite()
            || rule.max_requests < 0.0
            || rule.max_requests > f64::from(i32::MAX - 1)
            || !cleanup_window.is_finite()
            || cleanup_window < 0.0
        {
            return Err(AuthError::config("Invalid identity rate limit"));
        }
        let bucket = hex::encode(Sha256::digest(key.as_bytes()));
        let transaction = self.begin_transaction().await?;
        context(&transaction, "med_tracker.identity_rate_bucket", &bucket).await?;
        transaction.execute_raw(statement(
            "DELETE FROM public.identity_rate_limits WHERE bucket IN (SELECT bucket FROM public.identity_rate_limits WHERE expires_at <= clock_timestamp() - $1::double precision * interval '1 second' ORDER BY expires_at LIMIT 100)",
            [cleanup_window.into()],
        )).await.map_err(database_error)?;
        let row = transaction.query_one_raw(statement(
            "INSERT INTO public.identity_rate_limits(bucket,attempts,expires_at) VALUES($1,1,clock_timestamp()+$2::double precision*interval '1 second') ON CONFLICT(bucket) DO UPDATE SET attempts=CASE WHEN identity_rate_limits.expires_at<=clock_timestamp() THEN 1 ELSE LEAST(identity_rate_limits.attempts+1,$3::integer) END,expires_at=CASE WHEN identity_rate_limits.expires_at<=clock_timestamp() THEN EXCLUDED.expires_at ELSE identity_rate_limits.expires_at END RETURNING attempts, GREATEST(0,extract(epoch FROM expires_at-clock_timestamp()))::double precision AS retry_after",
            [bucket.into(), rule.window.into(), ((rule.max_requests.floor() as i32) + 1).into()],
        )).await.map_err(database_error)?.ok_or_else(|| AuthError::internal("Authentication limit unavailable"))?;
        let attempts: i32 = row.try_get("", "attempts").map_err(database_error)?;
        let retry_after: f64 = row.try_get("", "retry_after").map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        let allowed = f64::from(attempts) <= rule.max_requests;
        Ok(RateLimitDecision {
            allowed,
            retry_after: (!allowed).then_some(retry_after),
        })
    }
}
