use super::*;

pub(super) const PENDING_COOKIE: &str = "mt_oauth_pending";

pub(super) const SESSION_COOKIE: &str = "mt_oauth_session";

pub(super) const LOGIN_INTENT_COOKIE: &str = "mt_web_login_intent";

#[derive(Deserialize, Serialize)]
pub(super) struct Pending {
    pub(super) request: AuthorizationRequest,
    pub(super) csrf: String,
    pub(super) issued_at: i64,
}

#[derive(Deserialize, Serialize)]
pub(super) struct LoginIntent {
    pub(super) csrf: String,
    pub(super) issued_at: i64,
}

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct BrowserSession {
    pub(crate) account_id: i64,
    pub(crate) session_id: String,
    pub(super) issued_at: i64,
    pub(crate) csrf: String,
    #[serde(default)]
    pub(super) additional_factor_verified: bool,
}

pub(super) fn browser_cookie_age() -> i64 {
    let inactivity =
        configured_lifetime_days("SESSION_INACTIVITY_TIMEOUT_DAYS", 30, 1).unwrap_or(0);
    let maximum = configured_lifetime_days("SESSION_MAX_AGE_DAYS", 0, 0).unwrap_or(0);
    let days = if maximum > 0 {
        inactivity.min(maximum)
    } else {
        inactivity
    };
    days.saturating_mul(86_400)
}

fn remaining_browser_cookie_age(session: &BrowserSession) -> i64 {
    let inactivity = browser_cookie_age();
    let maximum = configured_lifetime_days("SESSION_MAX_AGE_DAYS", 0, 0).unwrap_or(0);
    if maximum == 0 {
        return inactivity;
    }
    let absolute_end = session
        .issued_at
        .saturating_add(maximum.saturating_mul(86_400));
    inactivity.min(absolute_end.saturating_sub(Utc::now().timestamp()).max(0))
}

pub(crate) fn renewed_session_cookie(
    state: &AppState,
    session: &BrowserSession,
) -> Option<HeaderValue> {
    let signed = state.oauth.sign(session)?;
    Some(state.oauth.cookie(
        SESSION_COOKIE,
        &signed,
        remaining_browser_cookie_age(session),
    ))
}

fn cookie_value<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .filter_map(|part| part.trim().split_once('='))
        .find_map(|(key, value)| (key == name).then_some(value))
}

pub(super) fn pending(state: &AppState, headers: &HeaderMap) -> Option<Pending> {
    let claim: Pending = state.oauth.verify(cookie_value(headers, PENDING_COOKIE)?)?;
    (Utc::now().timestamp() - claim.issued_at)
        .abs()
        .le(&600)
        .then_some(claim)
}

pub(super) fn login_intent(state: &AppState, headers: &HeaderMap) -> Option<LoginIntent> {
    let claim: LoginIntent = state
        .oauth
        .verify(cookie_value(headers, LOGIN_INTENT_COOKIE)?)?;
    (Utc::now().timestamp() - claim.issued_at)
        .abs()
        .le(&600)
        .then_some(claim)
}

pub(super) async fn account_available(
    db: &DatabaseTransaction,
    account_id: i64,
) -> Result<bool, Response> {
    let account = account::Entity::find_by_id(account_id)
        .one(db)
        .await
        .map_err(|e| database_error(e).into_response())?;
    if !account.is_some_and(|a| a.status == 2) {
        return Ok(false);
    }
    let lockout = account_lockout::Entity::find_by_id(account_id)
        .one(db)
        .await
        .map_err(|e| database_error(e).into_response())?;
    if lockout.is_some_and(|l| l.deadline > Utc::now().naive_utc()) {
        return Ok(false);
    }
    let linked = person::Entity::find()
        .filter(person::Column::AccountId.eq(account_id))
        .order_by_asc(person::Column::Id)
        .one(db)
        .await
        .map_err(|e| database_error(e).into_response())?;
    if let Some(person) = linked {
        return Ok(user::Entity::find()
            .filter(user::Column::PersonId.eq(person.id))
            .filter(user::Column::Active.eq(true))
            .one(db)
            .await
            .map_err(|e| database_error(e).into_response())?
            .is_some());
    }
    Ok(false)
}

pub(crate) async fn browser_session(
    state: &AppState,
    db: &DatabaseTransaction,
    headers: &HeaderMap,
) -> Result<Option<BrowserSession>, Response> {
    let Some(value) = cookie_value(headers, SESSION_COOKIE) else {
        return Ok(None);
    };
    let Some(session): Option<BrowserSession> = state.oauth.verify(value) else {
        return Ok(None);
    };
    if Utc::now().timestamp() < session.issued_at {
        return Ok(None);
    }
    let row =
        active_session_key::Entity::find_by_id((session.account_id, digest(&session.session_id)))
            .one(db)
            .await
            .map_err(|e| database_error(e).into_response())?;
    let Some(row) = row else { return Ok(None) };
    let created_at = row.created_at;
    let last_use = row.last_use;
    let now = Utc::now().naive_utc();
    let inactivity =
        configured_lifetime_days("SESSION_INACTIVITY_TIMEOUT_DAYS", 30, 1).unwrap_or(0);
    let maximum = configured_lifetime_days("SESSION_MAX_AGE_DAYS", 0, 0).unwrap_or(-1);
    if inactivity < 1
        || maximum < 0
        || now - last_use > Duration::days(inactivity)
        || (maximum > 0 && now - created_at > Duration::days(maximum))
        || !account_available(db, session.account_id).await?
        || (!session.additional_factor_verified && factor_required(db, session.account_id).await?)
    {
        return Ok(None);
    }
    let mut active: active_session_key::ActiveModel = row.into();
    active.last_use = Set(now);
    active
        .update(db)
        .await
        .map_err(|e| database_error(e).into_response())?;
    Ok(Some(session))
}

pub(super) async fn factor_required(
    db: &DatabaseTransaction,
    account_id: i64,
) -> Result<bool, Response> {
    let otp = otp_key::Entity::find_by_id(account_id)
        .one(db)
        .await
        .map_err(|e| database_error(e).into_response())?;
    if otp.is_some() {
        return Ok(true);
    }
    let passkey = webauthn_key::Entity::find()
        .filter(webauthn_key::Column::AccountId.eq(account_id))
        .one(db)
        .await
        .map_err(|e| database_error(e).into_response())?;
    if passkey.is_some() {
        return Ok(true);
    }
    Ok(recovery_code::Entity::find()
        .filter(recovery_code::Column::Id.eq(account_id))
        .one(db)
        .await
        .map_err(|e| database_error(e).into_response())?
        .is_some())
}

pub(super) fn trusted_origin(state: &AppState, headers: &HeaderMap, allow_missing: bool) -> bool {
    if let Some(value) = headers.get(header::ORIGIN) {
        let Some(value) = value.to_str().ok().and_then(|value| Url::parse(value).ok()) else {
            return false;
        };
        return value.origin() == state.oauth.base_url.origin()
            && value.path() == "/"
            && value.query().is_none()
            && value.fragment().is_none();
    }
    let Some(value) = headers.get(header::REFERER) else {
        return allow_missing;
    };
    value
        .to_str()
        .ok()
        .and_then(|value| Url::parse(value).ok())
        .is_some_and(|value| value.origin() == state.oauth.base_url.origin())
}

pub(crate) fn trusted_cookie_origin(state: &AppState, headers: &HeaderMap) -> bool {
    trusted_origin(state, headers, false)
}
