use super::*;

#[derive(Deserialize)]
pub(super) struct TokenForm {
    grant_type: String,
    client_id: String,
    redirect_uri: Option<String>,
    code: Option<String>,
    code_verifier: Option<String>,
    refresh_token: Option<String>,
}

pub(super) async fn token(State(state): State<AppState>, Form(form): Form<TokenForm>) -> Response {
    let db = match transaction(&state).await {
        Ok(db) => db,
        Err(error) => return error,
    };
    let registered = match client(&db, &form.client_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return oauth_error("invalid_client"),
        Err(error) => return error,
    };
    let result = match form.grant_type.as_str() {
        "authorization_code" => redeem_code(&db, &registered, &form).await,
        "refresh_token" => rotate_refresh(&db, &registered, &form).await,
        _ => return oauth_error("unsupported_grant_type"),
    };
    let body = match result {
        Ok(Some(body)) => body,
        Ok(None) => return oauth_error("invalid_grant"),
        Err(error) => return error,
    };
    if let Err(error) = db.commit().await {
        return database_error(error).into_response();
    }
    (
        [
            (header::CACHE_CONTROL, "no-store"),
            (header::PRAGMA, "no-cache"),
        ],
        Json(body),
    )
        .into_response()
}

async fn redeem_code(
    db: &DatabaseTransaction,
    registered: &Client,
    form: &TokenForm,
) -> Result<Option<Value>, Response> {
    let (Some(code), Some(redirect_uri), Some(verifier)) =
        (&form.code, &form.redirect_uri, &form.code_verifier)
    else {
        return Ok(None);
    };
    if code.len() > 256
        || !(43..=128).contains(&verifier.len())
        || !verifier
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~')
    {
        return Ok(None);
    }
    let row = db.query_one_raw(sql("SELECT id, account_id, redirect_uri, code_challenge, code_challenge_method, scopes, expires_in FROM oauth_grants WHERE oauth_application_id = $1 AND client_kind = 'mobile' AND code = $2 AND revoked_at IS NULL FOR UPDATE", [registered.id.into(), code.clone().into()]))
        .await.map_err(|e| database_error(e).into_response())?;
    let Some(row) = row else { return Ok(None) };
    let id: i64 = row
        .try_get("", "id")
        .map_err(|e| database_error(e).into_response())?;
    let account_id: i64 = row
        .try_get("", "account_id")
        .map_err(|e| database_error(e).into_response())?;
    let stored_redirect: Option<String> = row
        .try_get("", "redirect_uri")
        .map_err(|e| database_error(e).into_response())?;
    let challenge: Option<String> = row
        .try_get("", "code_challenge")
        .map_err(|e| database_error(e).into_response())?;
    let method: Option<String> = row
        .try_get("", "code_challenge_method")
        .map_err(|e| database_error(e).into_response())?;
    let scopes: String = row
        .try_get("", "scopes")
        .map_err(|e| database_error(e).into_response())?;
    let expiry: chrono::NaiveDateTime = row
        .try_get("", "expires_in")
        .map_err(|e| database_error(e).into_response())?;
    let actual_challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    if stored_redirect.as_deref() != Some(redirect_uri)
        || method.as_deref() != Some("S256")
        || challenge.as_deref() != Some(actual_challenge.as_str())
        || expiry <= Utc::now().naive_utc()
        || !account_available(db, account_id).await?
    {
        return Ok(None);
    }
    issue_tokens(db, id, &scopes, true).await
}

async fn rotate_refresh(
    db: &DatabaseTransaction,
    registered: &Client,
    form: &TokenForm,
) -> Result<Option<Value>, Response> {
    let Some(refresh) = &form.refresh_token else {
        return Ok(None);
    };
    if refresh.len() > 256 {
        return Ok(None);
    }
    let row = db.query_one_raw(sql("SELECT id, account_id, scopes, authenticated_at, last_used_at FROM oauth_grants WHERE oauth_application_id = $1 AND client_kind = 'mobile' AND refresh_token_hash = $2 AND revoked_at IS NULL FOR UPDATE", [registered.id.into(), digest(refresh).into()]))
        .await.map_err(|e| database_error(e).into_response())?;
    let Some(row) = row else { return Ok(None) };
    let id: i64 = row
        .try_get("", "id")
        .map_err(|e| database_error(e).into_response())?;
    let account_id: i64 = row
        .try_get("", "account_id")
        .map_err(|e| database_error(e).into_response())?;
    let scopes: String = row
        .try_get("", "scopes")
        .map_err(|e| database_error(e).into_response())?;
    let authenticated: Option<chrono::NaiveDateTime> = row
        .try_get("", "authenticated_at")
        .map_err(|e| database_error(e).into_response())?;
    let last_used: Option<chrono::NaiveDateTime> = row
        .try_get("", "last_used_at")
        .map_err(|e| database_error(e).into_response())?;
    let now = Utc::now().naive_utc();
    let inactivity =
        configured_lifetime_days("SESSION_INACTIVITY_TIMEOUT_DAYS", 30, 1).unwrap_or(0);
    let maximum = configured_lifetime_days("SESSION_MAX_AGE_DAYS", 0, 0).unwrap_or(-1);
    if !scopes
        .split_whitespace()
        .any(|scope| scope == "offline_access")
        || inactivity < 1
        || maximum < 0
        || !last_used.is_some_and(|time| time > now - Duration::days(inactivity))
        || !authenticated.is_some_and(|time| maximum == 0 || time > now - Duration::days(maximum))
        || !account_available(db, account_id).await?
    {
        return Ok(None);
    }
    issue_tokens(db, id, &scopes, false).await
}

async fn issue_tokens(
    db: &DatabaseTransaction,
    id: i64,
    scopes: &str,
    initial: bool,
) -> Result<Option<Value>, Response> {
    let access = secret();
    let refresh = scopes
        .split_whitespace()
        .any(|scope| scope == "offline_access")
        .then(secret);
    let update = if initial {
        "UPDATE oauth_grants SET code = NULL, token_hash = $2, refresh_token_hash = $3, expires_in = CURRENT_TIMESTAMP + interval '15 minutes', last_used_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = $1"
    } else {
        "UPDATE oauth_grants SET token_hash = $2, refresh_token_hash = $3, expires_in = CURRENT_TIMESTAMP + interval '15 minutes', updated_at = CURRENT_TIMESTAMP WHERE id = $1"
    };
    db.execute_raw(sql(
        update,
        [
            id.into(),
            digest(&access).into(),
            refresh.as_deref().map(digest).into(),
        ],
    ))
    .await
    .map_err(|e| database_error(e).into_response())?;
    let mut body =
        json!({"access_token": access, "token_type": "bearer", "expires_in": 900, "scope": scopes});
    if let Some(refresh) = refresh {
        body["refresh_token"] = json!(refresh);
    }
    Ok(Some(body))
}

#[derive(Deserialize)]
pub(super) struct RevokeInput {
    client_id: String,
    token: String,
    token_type_hint: Option<String>,
}

pub(super) async fn revoke(
    State(state): State<AppState>,
    Json(input): Json<RevokeInput>,
) -> Response {
    if input.token.len() > 256 {
        return oauth_error("invalid_request");
    }
    let db = match transaction(&state).await {
        Ok(db) => db,
        Err(error) => return error,
    };
    let registered = match client(&db, &input.client_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return oauth_error("invalid_client"),
        Err(error) => return error,
    };
    if input
        .token_type_hint
        .as_deref()
        .is_some_and(|hint| hint != "access_token" && hint != "refresh_token")
    {
        return oauth_error("unsupported_token_type");
    }
    let hash = digest(&input.token);
    let result = db.execute_raw(sql("UPDATE oauth_grants SET token_hash = NULL, refresh_token_hash = NULL, revoked_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE oauth_application_id = $1 AND client_kind = 'mobile' AND (token_hash = $2 OR refresh_token_hash = $2) AND revoked_at IS NULL", [registered.id.into(), hash.into()])).await;
    if let Err(error) = result {
        return database_error(error).into_response();
    }
    if let Err(error) = db.commit().await {
        return database_error(error).into_response();
    }
    (StatusCode::OK, Json(json!({}))).into_response()
}
