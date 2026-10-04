use super::*;

pub(super) const FACTOR_COOKIE: &str = "mt_login_factor";

#[derive(Deserialize, Serialize)]
pub(super) struct FactorIntent {
    account_id: i64,
    nonce: String,
    csrf: String,
    issued_at: i64,
    authorization: Option<AuthorizationRequest>,
}

impl configuration::AuthenticationClaim for FactorIntent {
    const PURPOSE: &'static str = FACTOR_COOKIE;
}

pub(super) fn valid_issued_at(issued_at: i64, now: i64) -> bool {
    now.checked_sub(issued_at)
        .is_some_and(|age| (0..300).contains(&age))
}

fn intent(state: &AppState, headers: &HeaderMap) -> Option<FactorIntent> {
    let cookie = headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .filter_map(|part| part.trim().split_once('='))
        .find_map(|(name, value)| (name == FACTOR_COOKIE).then_some(value))?;
    let claim: FactorIntent = state.oauth.verify(cookie)?;
    valid_issued_at(claim.issued_at, Utc::now().timestamp()).then_some(claim)
}

fn pending_digest(nonce: &str) -> String {
    format!("mfa:{}", digest(nonce))
}

fn page(csrf: &str, failed: bool) -> Response {
    let alert = if failed {
        "<p class=\"form-alert\" role=\"alert\">Invalid or expired code. Try again.</p>"
    } else {
        ""
    };
    html(format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>Two-factor authentication · MedTracker</title><link rel=\"stylesheet\" href=\"/auth.css\"><script defer src=\"/profile.js\"></script></head><body><main class=\"auth-page\"><div class=\"auth-shell\"><section class=\"form-panel\"><p class=\"eyebrow\">SECURE SIGN IN</p><h1>Two-factor authentication</h1><p class=\"form-intro\">Enter an authenticator code or use a recovery code. If you use a passkey, return to sign in and choose Continue with Passkey.</p>{alert}<form class=\"auth-form\" action=\"/login-factor\" method=\"post\"><input type=\"hidden\" name=\"authenticity_token\" value=\"{csrf}\"><div class=\"form-field\"><label for=\"factor\">Verification method</label><select id=\"factor\" name=\"factor\"><option value=\"otp\">Authenticator code</option><option value=\"recovery\">Recovery code</option></select></div><div class=\"form-field\"><label for=\"code\">Code</label><input id=\"code\" name=\"code\" autocomplete=\"one-time-code\" maxlength=\"256\" required autofocus></div><button class=\"primary-button\" type=\"submit\">Verify and sign in</button><a href=\"/login\">Start again</a></form></section></div></main></body></html>"
    ))
}

pub(super) async fn begin(
    state: &AppState,
    db: DatabaseTransaction,
    account_id: i64,
    authorization: Option<&Pending>,
) -> Response {
    let claim = FactorIntent {
        account_id,
        nonce: secret(),
        csrf: secret(),
        issued_at: Utc::now().timestamp(),
        authorization: authorization.map(|pending| pending.request.clone()),
    };
    let Some(cookie) = state.oauth.sign(&claim) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    if let Err(error) = db.execute_raw(sql("DELETE FROM account_active_session_keys WHERE account_id=$1 AND session_id LIKE 'mfa:%' AND created_at <= CURRENT_TIMESTAMP - interval '5 minutes'", [account_id.into()])).await { return database_error(error).into_response(); }
    let expired_lockout = db
        .execute_raw(sql(
            "DELETE FROM account_lockouts WHERE account_id=$1 AND deadline <= CURRENT_TIMESTAMP",
            [account_id.into()],
        ))
        .await;
    match expired_lockout {
        Ok(result) if result.rows_affected() > 0 => {
            if let Err(error) = db
                .execute_raw(sql(
                    "DELETE FROM account_login_failures WHERE account_id=$1",
                    [account_id.into()],
                ))
                .await
            {
                return database_error(error).into_response();
            }
        }
        Ok(_) => {}
        Err(error) => return database_error(error).into_response(),
    }
    let now = Utc::now().naive_utc();
    let result = active_session_key::ActiveModel {
        account_id: Set(account_id),
        session_id: Set(pending_digest(&claim.nonce)),
        created_at: Set(now),
        last_use: Set(now),
    }
    .insert(&db)
    .await;
    if let Err(error) = result {
        return database_error(error).into_response();
    }
    if let Err(error) = db.commit().await {
        return database_error(error).into_response();
    }
    let mut response = redirect("/login-factor");
    response.headers_mut().append(
        header::SET_COOKIE,
        state.oauth.cookie(FACTOR_COOKIE, &cookie, 300),
    );
    response
}

pub(super) async fn show(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let Some(claim) = intent(&state, &headers) else {
        return redirect("/login");
    };
    let db = match transaction(&state).await {
        Ok(db) => db,
        Err(error) => return error,
    };
    let row =
        active_session_key::Entity::find_by_id((claim.account_id, pending_digest(&claim.nonce)))
            .one(&db)
            .await;
    match row {
        Ok(Some(row)) if row.created_at > Utc::now().naive_utc() - Duration::minutes(5) => {
            page(&claim.csrf, false)
        }
        Ok(_) => redirect("/login"),
        Err(error) => database_error(error).into_response(),
    }
}

#[derive(Deserialize)]
pub(super) struct FactorForm {
    authenticity_token: String,
    factor: String,
    code: String,
}

async fn verify_otp(
    db: &DatabaseTransaction,
    account_id: i64,
    code: &str,
) -> Result<bool, Response> {
    let secrets = crate::rodauth_secrets::load()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE.into_response())?;
    let row = db.query_one_raw(sql("SELECT key, EXTRACT(EPOCH FROM last_use)::bigint AS last_use_epoch FROM account_otp_keys WHERE id=$1 FOR UPDATE", [account_id.into()])).await.map_err(|error| database_error(error).into_response())?;
    let Some(row) = row else {
        return Ok(false);
    };
    let key: String = row
        .try_get("", "key")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let last_use: i64 = row
        .try_get("", "last_use_epoch")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let verified = crate::auth_compatibility::verify_rodauth_otp(
        &key,
        Some(secrets.current()),
        secrets.old(),
        code,
        Utc::now().timestamp(),
        last_use,
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    if let Some(verified) = verified {
        db.execute_raw(sql(
            "UPDATE account_otp_keys SET last_use=to_timestamp($2) AT TIME ZONE 'UTC', num_failures=0 WHERE id=$1",
            [
                account_id.into(),
                (verified.matched_step as i64 * 30).into(),
            ],
        ))
        .await
        .map_err(|error| database_error(error).into_response())?;
        Ok(true)
    } else {
        db.execute_raw(sql(
            "UPDATE account_otp_keys SET num_failures=num_failures+1 WHERE id=$1",
            [account_id.into()],
        ))
        .await
        .map_err(|error| database_error(error).into_response())?;
        Ok(false)
    }
}

pub(super) async fn complete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(form): Form<FactorForm>,
) -> Response {
    let Some(claim) = intent(&state, &headers) else {
        return StatusCode::FORBIDDEN.into_response();
    };
    if !trusted_origin(&state, &headers, true) || claim.csrf != form.authenticity_token {
        return StatusCode::FORBIDDEN.into_response();
    }
    if form.code.len() > 256 || !matches!(form.factor.as_str(), "otp" | "recovery") {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let db = match transaction(&state).await {
        Ok(db) => db,
        Err(error) => return error,
    };
    let locked_account = db
        .query_one_raw(sql(
            "SELECT id FROM accounts WHERE id=$1 FOR UPDATE",
            [claim.account_id.into()],
        ))
        .await;
    match locked_account {
        Ok(Some(_)) => {}
        Ok(None) => return StatusCode::FORBIDDEN.into_response(),
        Err(error) => return database_error(error).into_response(),
    }
    let pending = db.query_one_raw(sql("SELECT session_id FROM account_active_session_keys WHERE account_id=$1 AND session_id=$2 AND created_at > CURRENT_TIMESTAMP - interval '5 minutes' FOR UPDATE", [claim.account_id.into(), pending_digest(&claim.nonce).into()])).await;
    match pending {
        Ok(Some(_)) => {}
        Ok(None) => return StatusCode::FORBIDDEN.into_response(),
        Err(error) => return database_error(error).into_response(),
    }
    match account_available(&db, claim.account_id).await {
        Ok(true) => {}
        Ok(false) => return StatusCode::FORBIDDEN.into_response(),
        Err(error) => return error,
    }
    let verified = if form.factor == "otp" {
        match verify_otp(&db, claim.account_id, &form.code).await {
            Ok(value) => value,
            Err(error) => return error,
        }
    } else {
        match db
            .execute_raw(sql(
                "DELETE FROM account_recovery_codes WHERE id=$1 AND code=$2",
                [claim.account_id.into(), form.code.into()],
            ))
            .await
        {
            Ok(result) => result.rows_affected() == 1,
            Err(error) => return database_error(error).into_response(),
        }
    };
    if !verified {
        let failure = db.query_one_raw(sql("INSERT INTO account_login_failures (account_id, number, created_at, updated_at) VALUES ($1, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (account_id) DO UPDATE SET number=account_login_failures.number+1, updated_at=CURRENT_TIMESTAMP RETURNING number", [claim.account_id.into()])).await;
        let count: i32 = match failure {
            Ok(Some(row)) => match row.try_get("", "number") {
                Ok(count) => count,
                Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            },
            Ok(None) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            Err(error) => return database_error(error).into_response(),
        };
        if count >= 5 {
            if let Err(error) = db.execute_raw(sql("INSERT INTO account_lockouts (account_id, deadline, key, created_at, updated_at) VALUES ($1, CURRENT_TIMESTAMP + interval '30 minutes', $2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (account_id) DO UPDATE SET deadline=EXCLUDED.deadline, updated_at=CURRENT_TIMESTAMP", [claim.account_id.into(), secret().into()])).await { return database_error(error).into_response(); }
            if let Err(error) = active_session_key::Entity::delete_by_id((
                claim.account_id,
                pending_digest(&claim.nonce),
            ))
            .exec(&db)
            .await
            {
                return database_error(error).into_response();
            }
        }
        if let Err(error) = db.commit().await {
            return database_error(error).into_response();
        }
        return page(&claim.csrf, true);
    }
    if let Err(error) =
        active_session_key::Entity::delete_by_id((claim.account_id, pending_digest(&claim.nonce)))
            .exec(&db)
            .await
    {
        return database_error(error).into_response();
    }
    let session_id = secret();
    let csrf = secret();
    let now = Utc::now().naive_utc();
    if let Err(error) = (active_session_key::ActiveModel {
        account_id: Set(claim.account_id),
        session_id: Set(digest(&session_id)),
        created_at: Set(now),
        last_use: Set(now),
    })
    .insert(&db)
    .await
    {
        return database_error(error).into_response();
    }
    if let Err(error) = db
        .execute_raw(sql(
            "DELETE FROM account_login_failures WHERE account_id=$1",
            [claim.account_id.into()],
        ))
        .await
    {
        return database_error(error).into_response();
    }
    let destination = if let Some(authorization) = claim.authorization {
        authorization.path()
    } else {
        match login::login_destination(&db, None, claim.account_id).await {
            Ok(value) => value,
            Err(error) => return error,
        }
    };
    let session = BrowserSession {
        account_id: claim.account_id,
        session_id,
        issued_at: Utc::now().timestamp(),
        csrf,
        additional_factor_verified: true,
    };
    let Some(cookie) = state.oauth.sign(&session) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    if let Err(error) = db.commit().await {
        return database_error(error).into_response();
    }
    let mut response = redirect(&destination);
    response.headers_mut().append(
        header::SET_COOKIE,
        state
            .oauth
            .cookie(SESSION_COOKIE, &cookie, browser_cookie_age()),
    );
    response
        .headers_mut()
        .append(header::SET_COOKIE, state.oauth.cookie(FACTOR_COOKIE, "", 0));
    response.headers_mut().append(
        header::SET_COOKIE,
        state.oauth.cookie(LOGIN_INTENT_COOKIE, "", 0),
    );
    response
}
