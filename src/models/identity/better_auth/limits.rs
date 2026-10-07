use super::{
    IdentityService,
    store::{context, database_error, statement},
};
use better_auth_core::{AuthError, AuthRequest, AuthResponse, AuthResult};
use sea_orm::ConnectionTrait;
use sha2::{Digest, Sha256};

async fn consume(
    service: &IdentityService,
    key: &str,
    maximum: i32,
    seconds: i32,
) -> AuthResult<Option<AuthResponse>> {
    let bucket = hex::encode(Sha256::digest(key.as_bytes()));
    let transaction = service.store.begin_transaction().await?;
    context(&transaction, "med_tracker.identity_rate_bucket", &bucket).await?;
    transaction.execute_raw(statement("DELETE FROM public.identity_rate_limits WHERE bucket IN (SELECT bucket FROM public.identity_rate_limits WHERE expires_at<=clock_timestamp() ORDER BY expires_at LIMIT 100)", [])).await.map_err(database_error)?;
    let row = transaction.query_one_raw(statement("INSERT INTO public.identity_rate_limits(bucket,attempts,expires_at) VALUES($1,1,clock_timestamp()+$3::integer*interval '1 second') ON CONFLICT(bucket) DO UPDATE SET attempts=CASE WHEN identity_rate_limits.expires_at<=clock_timestamp() THEN 1 ELSE LEAST(identity_rate_limits.attempts+1,$2+1) END,expires_at=CASE WHEN identity_rate_limits.expires_at<=clock_timestamp() THEN EXCLUDED.expires_at ELSE identity_rate_limits.expires_at END RETURNING attempts, GREATEST(1,ceil(extract(epoch FROM expires_at-clock_timestamp())))::integer AS retry_after", [bucket.into(), maximum.into(), seconds.into()])).await.map_err(database_error)?.ok_or_else(|| AuthError::internal("Authentication limit unavailable"))?;
    let attempts: i32 = row.try_get("", "attempts").map_err(database_error)?;
    let retry_after: i32 = row.try_get("", "retry_after").map_err(database_error)?;
    transaction.commit().await.map_err(database_error)?;
    if attempts <= maximum {
        return Ok(None);
    }
    Ok(Some(
        AuthResponse::json(
            429,
            &serde_json::json!({"error":"Too many authentication attempts. Try again later."}),
        )?
        .with_header("Retry-After", retry_after.to_string()),
    ))
}

pub(super) async fn check(
    service: &IdentityService,
    request: &AuthRequest,
) -> AuthResult<Option<AuthResponse>> {
    let path = request.path.as_str();
    let guarded = path.starts_with("/security/")
        || matches!(
            path,
            "/sign-in/email"
                | "/recovery/login"
                | "/onboarding/signup"
                | "/send-verification-email"
                | "/onboarding/confirm-email"
                | "/passkey/generate-authenticate-options"
                | "/passkey/verify-authentication"
        );
    if !guarded {
        return Ok(None);
    }
    let source = request
        .headers
        .get("x-forwarded-for")
        .map_or("unknown", String::as_str);
    if let Some(response) = consume(service, &format!("source:{source}"), 40, 60).await? {
        return Ok(Some(response));
    }
    let body: serde_json::Value = request
        .body
        .as_deref()
        .and_then(|body| serde_json::from_slice(body).ok())
        .unwrap_or_default();
    let (identity, category, maximum, seconds) = if path.starts_with("/security/") {
        let Some(token) = service
            .context()
            .session_manager()
            .extract_session_token(request)
        else {
            return Ok(None);
        };
        let Some(session) = service.context().database.get_session(&token).await? else {
            return Ok(None);
        };
        let recovery_email = matches!(
            path,
            "/security/password/start" | "/security/operation/start"
        ) && session
            .additional_fields
            .get("authentication_method")
            .and_then(serde_json::Value::as_str)
            == Some("recovery");
        (
            session.user_id,
            if recovery_email {
                "recovery-email"
            } else {
                "security"
            },
            if recovery_email { 1 } else { 30 },
            60,
        )
    } else if let Some(email) = body.get("email").and_then(serde_json::Value::as_str) {
        let resend = path == "/send-verification-email";
        (
            email.trim().to_lowercase(),
            if resend { "email" } else { "password" },
            if resend { 1 } else { 10 },
            if resend { 60 } else { 900 },
        )
    } else {
        return Ok(None);
    };
    consume(
        service,
        &format!("account:{category}:{identity}"),
        maximum,
        seconds,
    )
    .await
}
