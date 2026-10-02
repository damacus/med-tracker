use super::*;

const PASSKEY_ERROR: &str =
    "We could not sign you in with that passkey. Try again or use your password.";

#[derive(Deserialize, Serialize)]
pub(super) struct PasskeyChallenge {
    pub(super) challenge: String,
    pub(super) csrf: String,
    pub(super) issued_at: i64,
}

#[derive(Deserialize)]
pub(super) struct PasskeyForm {
    authenticity_token: String,
    webauthn_auth_challenge: String,
    webauthn_auth_challenge_hmac: String,
    webauthn_auth: String,
}

pub(super) async fn passkey_script() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        medtracker_web::passkey_script(),
    )
        .into_response()
}

pub(super) async fn passkey_login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(form): Form<PasskeyForm>,
) -> Response {
    let oauth_claim = pending(&state, &headers);
    let web_claim = if oauth_claim.is_none() {
        login_intent(&state, &headers)
    } else {
        None
    };
    let Some(intent_csrf) = oauth_claim
        .as_ref()
        .map(|claim| claim.csrf.as_str())
        .or_else(|| web_claim.as_ref().map(|claim| claim.csrf.as_str()))
    else {
        return StatusCode::FORBIDDEN.into_response();
    };
    let login_error = |error: &str| {
        html(medtracker_web::render_login(
            intent_csrf,
            error,
            passkey_context(&state, intent_csrf),
        ))
    };
    if intent_csrf != form.authenticity_token || !trusted_origin(&state, &headers, true) {
        return StatusCode::FORBIDDEN.into_response();
    }
    if form.webauthn_auth_challenge.len() > 1024
        || form.webauthn_auth_challenge_hmac.len() > 4096
        || form.webauthn_auth.len() > 16_384
    {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let Some(claim) = state
        .oauth
        .verify::<PasskeyChallenge>(&form.webauthn_auth_challenge_hmac)
    else {
        return login_error(PASSKEY_ERROR);
    };
    if claim.csrf != intent_csrf
        || claim.challenge != form.webauthn_auth_challenge
        || (Utc::now().timestamp() - claim.issued_at).abs() > 600
    {
        return login_error(PASSKEY_ERROR);
    }
    let Ok(credential_id) = crate::webauthn::credential_id(&form.webauthn_auth) else {
        return login_error(PASSKEY_ERROR);
    };
    let db = match transaction(&state).await {
        Ok(db) => db,
        Err(error) => return error,
    };
    let key = match webauthn_key::Entity::find()
        .filter(webauthn_key::Column::WebauthnId.eq(&credential_id))
        .one(&db)
        .await
    {
        Ok(key) => key,
        Err(error) => return database_error(error).into_response(),
    };
    let Some(key) = key else {
        return login_error(PASSKEY_ERROR);
    };
    let Some(rp_id) = state.oauth.base_url.host_str() else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let origin = state.oauth.base_url.origin().ascii_serialization();
    let stored_count = u64::from(key.sign_count.max(0) as u32);
    let verified = match crate::webauthn::verify_passkey_assertion(
        &form.webauthn_auth,
        &form.webauthn_auth_challenge,
        &origin,
        rp_id,
        &key.public_key,
        stored_count,
    ) {
        Ok(verified) => verified,
        Err(_) => return login_error(PASSKEY_ERROR),
    };
    if let Some(handle) = &verified.user_handle {
        let handle = URL_SAFE_NO_PAD.encode(handle);
        let binding = webauthn_user_id::Entity::find()
            .filter(webauthn_user_id::Column::WebauthnId.eq(handle))
            .one(&db)
            .await;
        let binding = match binding {
            Ok(binding) => binding,
            Err(error) => return database_error(error).into_response(),
        };
        if !binding.is_some_and(|binding| binding.account_id == key.account_id) {
            return login_error(PASSKEY_ERROR);
        }
    }
    let account_id = key.account_id;
    let available = match account_available(&db, account_id).await {
        Ok(value) => value,
        Err(error) => return error,
    };
    if !available {
        return login_error("Sign in is unavailable for this account");
    }
    let mut updated: webauthn_key::ActiveModel = key.into();
    updated.sign_count = Set(i32::try_from(verified.sign_count).unwrap_or(i32::MAX));
    updated.last_use = Set(Some(Utc::now().naive_utc()));
    updated.updated_at = Set(Utc::now().naive_utc());
    if let Err(error) = updated.update(&db).await {
        return database_error(error).into_response();
    }
    let session_id = secret();
    let csrf = secret();
    let now = Utc::now().naive_utc();
    let new_session = active_session_key::ActiveModel {
        account_id: Set(account_id),
        session_id: Set(digest(&session_id)),
        created_at: Set(now),
        last_use: Set(now),
    };
    if let Err(error) = new_session.insert(&db).await {
        return database_error(error).into_response();
    }
    let destination = match login_destination(&db, oauth_claim.as_ref(), account_id).await {
        Ok(destination) => destination,
        Err(error) => return error,
    };
    if let Err(error) = db.commit().await {
        return database_error(error).into_response();
    }
    let session = BrowserSession {
        account_id,
        session_id,
        issued_at: Utc::now().timestamp(),
        csrf,
        additional_factor_verified: true,
    };
    let Some(cookie) = state.oauth.sign(&session) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let mut response = redirect(&destination);
    response.headers_mut().append(
        header::SET_COOKIE,
        state
            .oauth
            .cookie(SESSION_COOKIE, &cookie, browser_cookie_age()),
    );
    if web_claim.is_some() {
        response.headers_mut().append(
            header::SET_COOKIE,
            state.oauth.cookie(LOGIN_INTENT_COOKIE, "", 0),
        );
    }
    response
}
