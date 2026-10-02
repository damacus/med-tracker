use super::*;

pub(super) async fn login(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Some(claim) = pending(&state, &headers) {
        let passkey = passkey_context(&state, &claim.csrf);
        return html(medtracker_web::render_login(&claim.csrf, "", passkey));
    }
    let intent = login_intent(&state, &headers).unwrap_or_else(|| LoginIntent {
        csrf: secret(),
        issued_at: Utc::now().timestamp(),
    });
    let Some(signed) = state.oauth.sign(&intent) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let mut response = html(medtracker_web::render_login(
        &intent.csrf,
        "",
        passkey_context(&state, &intent.csrf),
    ));
    response.headers_mut().append(
        header::SET_COOKIE,
        state.oauth.cookie(LOGIN_INTENT_COOKIE, &signed, 600),
    );
    response
}

pub(super) fn passkey_context(
    state: &AppState,
    csrf: &str,
) -> Option<medtracker_web::PasskeyLogin> {
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(secret().as_bytes()));
    let claim = PasskeyChallenge {
        challenge: challenge.clone(),
        csrf: csrf.to_owned(),
        issued_at: Utc::now().timestamp(),
    };
    let challenge_hmac = state.oauth.sign(&claim)?;
    Some(medtracker_web::PasskeyLogin {
        challenge,
        challenge_hmac,
        rp_id: state.oauth.base_url.host_str()?.to_owned(),
    })
}

pub(super) async fn login_destination(
    db: &DatabaseTransaction,
    oauth_claim: Option<&Pending>,
    account_id: i64,
) -> Result<String, Response> {
    if let Some(claim) = oauth_claim {
        return Ok(claim.request.path());
    }
    match first_active_household(db, account_id).await {
        Ok(Some(household)) => Ok(format!("/households/{}/dashboard", household.slug)),
        Ok(None) => Ok("/".to_owned()),
        Err(error) => Err(error),
    }
}

async fn first_active_household(
    db: &DatabaseTransaction,
    account_id: i64,
) -> Result<Option<household::Model>, Response> {
    tenant_setting(db, "med_tracker.current_account_id", account_id)
        .await
        .map_err(|error| database_error(error).into_response())?;
    let memberships = membership::Entity::find()
        .filter(membership::Column::AccountId.eq(account_id))
        .filter(membership::Column::Status.eq("active"))
        .filter(membership::Column::RevokedAt.is_null())
        .order_by_asc(membership::Column::Id)
        .all(db)
        .await
        .map_err(|error| database_error(error).into_response())?;
    let households = household::Entity::find()
        .filter(household::Column::Id.is_in(memberships.iter().map(|m| m.household_id)))
        .filter(household::Column::Status.eq("active"))
        .filter(household::Column::LifecycleState.eq("active"))
        .all(db)
        .await
        .map_err(|error| database_error(error).into_response())?;
    let households: HashMap<i64, household::Model> = households
        .into_iter()
        .map(|household| (household.id, household))
        .collect();
    Ok(memberships
        .iter()
        .find_map(|membership| households.get(&membership.household_id).cloned()))
}

#[derive(Deserialize)]
pub(super) struct LoginForm {
    email: String,
    password: String,
    authenticity_token: String,
}

async fn check_password(state: &AppState, password: String, hash: String) -> bool {
    let Ok(permit) = state.oauth.password_workers.clone().acquire_owned().await else {
        return false;
    };
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        bcrypt::verify(password, &hash).unwrap_or(false)
    })
    .await
    .unwrap_or(false)
}

pub(super) async fn login_post(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(form): Form<LoginForm>,
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
    if form.email.len() > 320 || form.password.len() > 1024 {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let db = match transaction(&state).await {
        Ok(db) => db,
        Err(error) => return error,
    };
    let row = match account::Entity::find()
        .filter(account::Column::Email.eq(form.email))
        .filter(account::Column::Status.eq(2))
        .one(&db)
        .await
    {
        Ok(row) => row,
        Err(error) => return database_error(error).into_response(),
    };
    let account = row.and_then(|row| row.password_hash.map(|hash| (row.id, hash)));
    let valid = match &account {
        Some((_, hash)) => check_password(&state, form.password, hash.clone()).await,
        None => {
            check_password(
                &state,
                "invalid".to_owned(),
                "$2b$12$zzzzzzzzzzzzzzzzzzzzzuD0kNWzTTXJvCDkBPuJKtf5Hh4bS5Mk2".to_owned(),
            )
            .await
        }
    };
    let Some((account_id, _)) = account else {
        return login_error("Invalid email or password");
    };
    if !valid {
        let count_row = db.query_one_raw(sql("INSERT INTO account_login_failures (account_id, number, created_at, updated_at) VALUES ($1, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (account_id) DO UPDATE SET number = account_login_failures.number + 1, updated_at = CURRENT_TIMESTAMP RETURNING number", [account_id.into()])).await;
        let Ok(Some(count_row)) = count_row else {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        };
        let count: i32 = count_row.try_get("", "number").unwrap_or(0);
        if count >= 5 {
            let lock = db.execute_raw(sql("INSERT INTO account_lockouts (account_id, deadline, key, created_at, updated_at) VALUES ($1, CURRENT_TIMESTAMP + interval '30 minutes', $2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (account_id) DO UPDATE SET deadline = EXCLUDED.deadline, updated_at = CURRENT_TIMESTAMP", [account_id.into(), secret().into()])).await;
            if lock.is_err() {
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        }
        if db.commit().await.is_err() {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        return login_error("Invalid email or password");
    }
    let available = match account_available(&db, account_id).await {
        Ok(value) => value,
        Err(error) => return error,
    };
    if !available {
        return login_error("Sign in is unavailable for this account");
    }
    let required = match factor_required(&db, account_id).await {
        Ok(value) => value,
        Err(error) => return error,
    };
    if required {
        return login_error(
            "This account requires a sign-in method that is not yet supported here.",
        );
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
    if db
        .execute_raw(sql(
            "DELETE FROM account_login_failures WHERE account_id = $1",
            [account_id.into()],
        ))
        .await
        .is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
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
        additional_factor_verified: false,
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

pub(super) async fn home(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let db = match transaction(&state).await {
        Ok(db) => db,
        Err(error) => return error,
    };
    let session = match browser_session(&state, &db, &headers).await {
        Ok(Some(session)) => session,
        Ok(None) => return redirect("/login"),
        Err(error) => return error,
    };
    let destination = match first_active_household(&db, session.account_id).await {
        Ok(Some(household)) => Some(format!("/households/{}/dashboard", household.slug)),
        Ok(None) => None,
        Err(error) => return error,
    };
    if let Err(error) = db.commit().await {
        return database_error(error).into_response();
    }
    let mut response = match destination {
        Some(destination) => redirect(&destination),
        None => html(medtracker_web::render_dashboard(
            "Your households",
            &session.csrf,
            true,
        )),
    };
    if let Some(cookie) = renewed_session_cookie(&state, &session) {
        response.headers_mut().append(header::SET_COOKIE, cookie);
    }
    response
}

pub(super) async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    if !trusted_cookie_origin(&state, &headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let db = match transaction(&state).await {
        Ok(db) => db,
        Err(error) => return error,
    };
    let session = match browser_session(&state, &db, &headers).await {
        Ok(Some(session)) => session,
        Ok(None) => return StatusCode::FORBIDDEN.into_response(),
        Err(error) => return error,
    };
    let supplied = headers
        .get("x-csrf-token")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
        .or_else(|| {
            form_fields(&headers, &body)
                .and_then(|fields| field(&fields, "authenticity_token").map(str::to_owned))
        });
    if supplied.as_deref() != Some(session.csrf.as_str()) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let result =
        active_session_key::Entity::delete_by_id((session.account_id, digest(&session.session_id)))
            .exec(&db)
            .await;
    if let Err(error) = result {
        return database_error(error).into_response();
    }
    if let Err(error) = db.commit().await {
        return database_error(error).into_response();
    }
    let mut response = redirect("/login");
    response.headers_mut().append(
        header::SET_COOKIE,
        state.oauth.cookie(SESSION_COOKIE, "", 0),
    );
    response
}
