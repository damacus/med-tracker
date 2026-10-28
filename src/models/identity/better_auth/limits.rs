use super::{
    IdentityService,
    policy::{context, database_error, statement},
};
use better_auth_core::{AuthError, AuthRequest, AuthResponse, AuthResult};
use sea_orm::ConnectionTrait;
use sha2::{Digest, Sha256};

#[derive(Clone, Copy)]
struct Limit {
    maximum: i32,
    seconds: i32,
}

const SOURCE_LIMIT: Limit = Limit {
    maximum: 40,
    seconds: 60,
};
const SECURITY_LIMIT: Limit = Limit {
    maximum: 30,
    seconds: 60,
};
const PASSWORD_LIMIT: Limit = Limit {
    maximum: 10,
    seconds: 900,
};
const RESEND_LIMIT: Limit = Limit {
    maximum: 1,
    seconds: 60,
};

fn configured(settings: Option<&serde_json::Value>, category: &str, default: Limit) -> Limit {
    let Some(section) = settings.and_then(|settings| settings.get("identity_rate_limits")) else {
        return default;
    };
    let resolve = |key: &str, fallback: i32| {
        section
            .get(format!("{category}_{key}"))
            .and_then(serde_json::Value::as_i64)
            .and_then(|value| i32::try_from(value).ok())
            .filter(|value| *value > 0)
            .unwrap_or(fallback)
    };
    Limit {
        maximum: resolve("maximum", default.maximum),
        seconds: resolve("window_seconds", default.seconds),
    }
}

async fn consume(
    service: &IdentityService,
    key: &str,
    maximum: i32,
    seconds: i32,
) -> AuthResult<Option<AuthResponse>> {
    let bucket = hex::encode(Sha256::digest(key.as_bytes()));
    let transaction = service.policy.begin_transaction().await?;
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
    let security = path.starts_with("/security/")
        || matches!(
            path,
            "/change-password"
                | "/change-email"
                | "/unlink-account"
                | "/passkey/delete-passkey"
                | "/passkey/generate-register-options"
                | "/passkey/verify-registration"
                | "/two-factor/enable"
                | "/two-factor/disable"
                | "/two-factor/generate-backup-codes"
                | "/revoke-session"
        );
    let guarded = security
        || matches!(
            path,
            "/sign-in/email"
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
    let source_limit = configured(service.policy.settings(), "source", SOURCE_LIMIT);
    if let Some(response) = consume(
        service,
        &format!("source:{source}"),
        source_limit.maximum,
        source_limit.seconds,
    )
    .await?
    {
        return Ok(Some(response));
    }
    let body: serde_json::Value = request
        .body
        .as_deref()
        .and_then(|body| serde_json::from_slice(body).ok())
        .unwrap_or_default();
    let (identity, category, limit) = if security {
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
        (
            session.user_id.display_string()?,
            "security",
            configured(service.policy.settings(), "security", SECURITY_LIMIT),
        )
    } else if let Some(email) = body.get("email").and_then(serde_json::Value::as_str) {
        let resend = path == "/send-verification-email";
        (
            email.trim().to_lowercase(),
            if resend { "email" } else { "password" },
            configured(
                service.policy.settings(),
                if resend { "email" } else { "password" },
                if resend { RESEND_LIMIT } else { PASSWORD_LIMIT },
            ),
        )
    } else {
        return Ok(None);
    };
    consume(
        service,
        &format!("account:{category}:{identity}"),
        limit.maximum,
        limit.seconds,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_limits_fall_back_to_defaults() {
        assert_eq!(configured(None, "source", SOURCE_LIMIT).maximum, 40);
        let settings = serde_json::json!({"other": {}});
        assert_eq!(
            configured(Some(&settings), "source", SOURCE_LIMIT).maximum,
            40
        );
        let settings = serde_json::json!({"identity_rate_limits": {"source_maximum": "invalid", "source_window_seconds": -5}});
        let limit = configured(Some(&settings), "source", SOURCE_LIMIT);
        assert_eq!(limit.maximum, 40);
        assert_eq!(limit.seconds, 60);
        let settings = serde_json::json!({"identity_rate_limits": {"source_maximum": 500, "source_window_seconds": 30}});
        let limit = configured(Some(&settings), "source", SOURCE_LIMIT);
        assert_eq!(limit.maximum, 500);
        assert_eq!(limit.seconds, 30);
        assert_eq!(
            configured(Some(&settings), "password", PASSWORD_LIMIT).maximum,
            10
        );
    }
}
