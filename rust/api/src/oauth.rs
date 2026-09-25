#![allow(clippy::result_large_err)]

use crate::entities::{
    account, account_lockout, active_session_key, household, membership, oauth_application,
    oauth_grant, otp_key, person, recovery_code, user, webauthn_key,
};
use crate::{configured_lifetime_days, restricted_role, tenant_setting, AppState};
use axum::extract::{Form, Path, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode, Uri};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::{
    engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD},
    Engine as _,
};
use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, DbBackend, EntityTrait,
    QueryFilter, QueryOrder, Set, Statement, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use url::{form_urlencoded, Url};

type HmacSha256 = Hmac<Sha256>;
const PENDING_COOKIE: &str = "mt_oauth_pending";
const SESSION_COOKIE: &str = "mt_oauth_session";
const LOGIN_INTENT_COOKIE: &str = "mt_web_login_intent";

#[derive(Clone)]
pub struct OAuthState {
    secret: Arc<[u8]>,
    base_url: Url,
    password_workers: Arc<Semaphore>,
}

impl OAuthState {
    pub fn from_env() -> Result<Self, String> {
        let secret = std::env::var("AUTH_SESSION_SECRET")
            .map_err(|_| "AUTH_SESSION_SECRET missing".to_owned())?;
        if secret.len() < 32 {
            return Err("AUTH_SESSION_SECRET must contain at least 32 bytes".to_owned());
        }
        let base_url = Url::parse(
            &std::env::var("PUBLIC_BASE_URL").map_err(|_| "PUBLIC_BASE_URL missing".to_owned())?,
        )
        .map_err(|_| "PUBLIC_BASE_URL invalid".to_owned())?;
        let local = matches!(
            base_url.host_str(),
            Some("localhost" | "127.0.0.1" | "[::1]" | "::1")
        );
        if (base_url.scheme() != "https" && !(base_url.scheme() == "http" && local))
            || base_url.path() != "/"
            || base_url.query().is_some()
            || base_url.fragment().is_some()
            || !base_url.username().is_empty()
            || base_url.password().is_some()
        {
            return Err(
                "PUBLIC_BASE_URL must be HTTPS origin or explicit loopback HTTP origin".to_owned(),
            );
        }
        Ok(Self {
            secret: Arc::from(secret.into_bytes()),
            base_url,
            password_workers: Arc::new(Semaphore::new(4)),
        })
    }

    fn sign<T: Serialize>(&self, data: &T) -> Option<String> {
        let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(data).ok()?);
        let mut mac = HmacSha256::new_from_slice(&self.secret).ok()?;
        mac.update(payload.as_bytes());
        Some(format!(
            "{payload}.{}",
            URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
        ))
    }

    fn verify<T: for<'a> Deserialize<'a>>(&self, value: &str) -> Option<T> {
        if value.len() > 4096 {
            return None;
        }
        let (payload, signature) = value.split_once('.')?;
        let signature = URL_SAFE_NO_PAD.decode(signature).ok()?;
        let mut mac = HmacSha256::new_from_slice(&self.secret).ok()?;
        mac.update(payload.as_bytes());
        mac.verify_slice(&signature).ok()?;
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload).ok()?).ok()
    }

    fn cookie(&self, name: &str, value: &str, max_age: i64) -> HeaderValue {
        let secure = if self.base_url.scheme() == "https" {
            "; Secure"
        } else {
            ""
        };
        HeaderValue::from_str(&format!(
            "{name}={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}{secure}"
        ))
        .expect("signed cookie is valid header")
    }
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
struct AuthorizationRequest {
    response_type: String,
    response_mode: String,
    client_id: String,
    redirect_uri: String,
    scope: String,
    state: String,
    code_challenge: String,
    code_challenge_method: String,
}

impl AuthorizationRequest {
    fn path(&self) -> String {
        let mut query = form_urlencoded::Serializer::new(String::new());
        query
            .append_pair("response_type", &self.response_type)
            .append_pair("response_mode", &self.response_mode)
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", &self.redirect_uri)
            .append_pair("scope", &self.scope)
            .append_pair("state", &self.state)
            .append_pair("code_challenge", &self.code_challenge)
            .append_pair("code_challenge_method", &self.code_challenge_method);
        format!("/authorize?{}", query.finish())
    }
}

#[derive(Deserialize, Serialize)]
struct Pending {
    request: AuthorizationRequest,
    csrf: String,
    issued_at: i64,
}

#[derive(Deserialize, Serialize)]
struct LoginIntent {
    csrf: String,
    issued_at: i64,
}

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct BrowserSession {
    pub(crate) account_id: i64,
    pub(crate) session_id: String,
    issued_at: i64,
    pub(crate) csrf: String,
}

#[derive(Clone)]
struct Client {
    id: i64,
    name: String,
    client_id: String,
    redirect_uri: String,
    scopes: String,
}

fn secret() -> String {
    format!(
        "{}{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

fn digest(token: &str) -> String {
    URL_SAFE.encode(Sha256::digest(token.as_bytes()))
}

pub(crate) fn session_key_digest(token: &str) -> String {
    digest(token)
}

fn browser_cookie_age() -> i64 {
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

fn pending(state: &AppState, headers: &HeaderMap) -> Option<Pending> {
    let claim: Pending = state.oauth.verify(cookie_value(headers, PENDING_COOKIE)?)?;
    (Utc::now().timestamp() - claim.issued_at)
        .abs()
        .le(&600)
        .then_some(claim)
}

fn login_intent(state: &AppState, headers: &HeaderMap) -> Option<LoginIntent> {
    let claim: LoginIntent = state
        .oauth
        .verify(cookie_value(headers, LOGIN_INTENT_COOKIE)?)?;
    (Utc::now().timestamp() - claim.issued_at)
        .abs()
        .le(&600)
        .then_some(claim)
}

fn redirect(location: &str) -> Response {
    (
        StatusCode::FOUND,
        [(header::LOCATION, location)],
        [(header::CACHE_CONTROL, "no-store")],
    )
        .into_response()
}

fn oauth_error(code: &'static str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        [(header::CACHE_CONTROL, "no-store")],
        Json(json!({"error": code})),
    )
        .into_response()
}

fn database_error(_: sea_orm::DbErr) -> crate::ApiError {
    eprintln!("OAuth database operation failed");
    crate::ApiError::internal()
}

fn html(body: String) -> Response {
    (
        [
            (header::CACHE_CONTROL, "no-store"),
            (
                header::CONTENT_SECURITY_POLICY,
                "default-src 'none'; style-src 'self'; form-action 'self'; base-uri 'none'; frame-ancestors 'none'",
            ),
        ],
        Html(body),
    )
        .into_response()
}

fn sql(query: &str, values: impl IntoIterator<Item = sea_orm::Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, query, values)
}

async fn transaction(state: &AppState) -> Result<DatabaseTransaction, Response> {
    let db = state
        .db
        .begin()
        .await
        .map_err(|e| database_error(e).into_response())?;
    restricted_role(&db)
        .await
        .map_err(|e| database_error(e).into_response())?;
    Ok(db)
}

async fn client(db: &DatabaseTransaction, client_id: &str) -> Result<Option<Client>, Response> {
    let record = oauth_application::Entity::find()
        .filter(oauth_application::Column::ClientId.eq(client_id))
        .filter(oauth_application::Column::ClientKind.eq("mobile"))
        .filter(oauth_application::Column::TokenEndpointAuthMethod.eq("none"))
        .filter(oauth_application::Column::ClientSecret.is_null())
        .filter(oauth_application::Column::ClientSecretHash.is_null())
        .one(db)
        .await
        .map_err(|e| database_error(e).into_response())?;
    Ok(record.map(|r| Client {
        id: r.id,
        name: r.name,
        client_id: r.client_id,
        redirect_uri: r.redirect_uri,
        scopes: r.scopes,
    }))
}

fn valid_request(request: &AuthorizationRequest, client: &Client) -> bool {
    request.response_type == "code"
        && request.response_mode == "query"
        && request.client_id == client.client_id
        && client
            .redirect_uri
            .split_whitespace()
            .any(|uri| uri == request.redirect_uri)
        && !request.state.is_empty()
        && request.state.len() <= 256
        && request.code_challenge_method == "S256"
        && (43..=128).contains(&request.code_challenge.len())
        && request
            .code_challenge
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        && !request.scope.is_empty()
        && request.scope.split_whitespace().all(|scope| {
            client
                .scopes
                .split_whitespace()
                .any(|allowed| allowed == scope)
        })
        && request
            .scope
            .split_whitespace()
            .any(|scope| scope == "medtracker")
        && request.scope.split_whitespace().count()
            == request
                .scope
                .split_whitespace()
                .collect::<std::collections::HashSet<_>>()
                .len()
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/.well-known/oauth-authorization-server", get(discovery))
        .route("/api/v1/capabilities", get(capabilities))
        .route("/authorize", get(authorize).post(consent))
        .route("/login", get(login).post(login_post))
        .route("/logout", post(logout))
        .route("/", get(home))
        .route("/households/{slug}/dashboard", get(dashboard))
        .route("/token", post(token))
        .route("/revoke", post(revoke))
        .route("/api/v1/auth/households", get(households))
        .route("/reset-password-request", get(reset_password_unavailable))
        .route("/auth.css", get(styles))
}

async fn reset_password_unavailable() -> Response {
    (
        StatusCode::NOT_IMPLEMENTED,
        html(medtracker_web::render_reset_unavailable()),
    )
        .into_response()
}

async fn styles() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        medtracker_web::stylesheet(),
    )
        .into_response()
}

async fn discovery(State(state): State<AppState>) -> Response {
    let base = state.oauth.base_url.as_str().trim_end_matches('/');
    (
        [(header::CACHE_CONTROL, "no-store")],
        Json(json!({
            "issuer": base,
            "authorization_endpoint": format!("{base}/authorize"),
            "token_endpoint": format!("{base}/token"),
            "revocation_endpoint": format!("{base}/revoke"),
            "response_types_supported": ["code"],
            "grant_types_supported": ["authorization_code", "refresh_token"],
            "code_challenge_methods_supported": ["S256"],
            "token_endpoint_auth_methods_supported": ["none"]
        })),
    )
        .into_response()
}

async fn capabilities(State(state): State<AppState>) -> Response {
    let db = match transaction(&state).await {
        Ok(db) => db,
        Err(error) => return error,
    };
    let rows = match oauth_application::Entity::find()
        .filter(oauth_application::Column::ClientKind.eq("mobile"))
        .filter(oauth_application::Column::TokenEndpointAuthMethod.eq("none"))
        .order_by_asc(oauth_application::Column::ClientId)
        .all(&db)
        .await
    {
        Ok(rows) => rows,
        Err(error) => return database_error(error).into_response(),
    };
    let clients: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "client_id": r.client_id, "name": r.name,
                "redirect_uris": r.redirect_uri.split_whitespace().collect::<Vec<_>>(),
                "scopes": r.scopes.split_whitespace().collect::<Vec<_>>()
            })
        })
        .collect();
    let _ = db.commit().await;
    let base = state.oauth.base_url.as_str().trim_end_matches('/');
    ([(header::CACHE_CONTROL, "no-store")], Json(json!({"data": {
        "format": "medtracker.api.capabilities.v1", "api_version": "v1",
        "authentication": {"methods": ["oauth_bearer", "api_app_token"], "hosted_mobile": "rodauth_authorization_code_pkce", "mobile_oauth": {
            "discovery_url": format!("{base}/.well-known/oauth-authorization-server"), "household_binding": "account",
            "inactivity_timeout_days": configured_lifetime_days("SESSION_INACTIVITY_TIMEOUT_DAYS", 30, 1).unwrap_or(0),
            "maximum_age_days": configured_lifetime_days("SESSION_MAX_AGE_DAYS", 0, 0).unwrap_or(0), "clients": clients
        }}, "administration": {"household": true}, "profile": {"online_only": true}
    }}))).into_response()
}

async fn account_available(db: &DatabaseTransaction, account_id: i64) -> Result<bool, Response> {
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
        || factor_required(db, session.account_id).await?
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

async fn factor_required(db: &DatabaseTransaction, account_id: i64) -> Result<bool, Response> {
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

async fn authorize(State(state): State<AppState>, headers: HeaderMap, uri: Uri) -> Response {
    let Some(query) = uri.query() else {
        return oauth_error("invalid_request");
    };
    if query.len() > 2048 {
        return oauth_error("invalid_request");
    }
    let pairs = form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect::<Vec<_>>();
    let mut query = HashMap::new();
    for (key, value) in pairs {
        if query.insert(key, value).is_some() {
            return oauth_error("invalid_request");
        }
    }
    if query.len() != 8 {
        return oauth_error("invalid_request");
    }
    let request = match serde_json::from_value::<AuthorizationRequest>(json!(query)) {
        Ok(request) => request,
        Err(_) => return oauth_error("invalid_request"),
    };
    let db = match transaction(&state).await {
        Ok(db) => db,
        Err(error) => return error,
    };
    let registered = match client(&db, &request.client_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return oauth_error("unauthorized_client"),
        Err(error) => return error,
    };
    if !valid_request(&request, &registered) {
        return oauth_error("invalid_request");
    }
    let session = match browser_session(&state, &db, &headers).await {
        Ok(value) => value,
        Err(error) => return error,
    };
    if let Err(error) = db.commit().await {
        return database_error(error).into_response();
    }
    let pending = match pending(&state, &headers) {
        Some(value) if value.request == request => value,
        _ => Pending {
            request,
            csrf: secret(),
            issued_at: Utc::now().timestamp(),
        },
    };
    let Some(cookie) = state.oauth.sign(&pending) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let mut response = if let Some(session) = session {
        let fields = vec![
            (
                "response_type".to_owned(),
                pending.request.response_type.clone(),
            ),
            (
                "response_mode".to_owned(),
                pending.request.response_mode.clone(),
            ),
            ("client_id".to_owned(), pending.request.client_id.clone()),
            (
                "redirect_uri".to_owned(),
                pending.request.redirect_uri.clone(),
            ),
            ("state".to_owned(), pending.request.state.clone()),
            (
                "code_challenge".to_owned(),
                pending.request.code_challenge.clone(),
            ),
            (
                "code_challenge_method".to_owned(),
                pending.request.code_challenge_method.clone(),
            ),
        ];
        let mut response = html(medtracker_web::render_consent(
            &session.csrf,
            &registered.name,
            &pending
                .request
                .scope
                .split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>(),
            &fields,
        ));
        if let Some(cookie) = renewed_session_cookie(&state, &session) {
            response.headers_mut().append(header::SET_COOKIE, cookie);
        }
        response
    } else {
        redirect("/login")
    };
    response.headers_mut().append(
        header::SET_COOKIE,
        state.oauth.cookie(PENDING_COOKIE, &cookie, 600),
    );
    response
}

async fn login(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Some(claim) = pending(&state, &headers) {
        return html(medtracker_web::render_login(&claim.csrf, ""));
    }
    let intent = login_intent(&state, &headers).unwrap_or_else(|| LoginIntent {
        csrf: secret(),
        issued_at: Utc::now().timestamp(),
    });
    let Some(signed) = state.oauth.sign(&intent) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let mut response = html(medtracker_web::render_login(&intent.csrf, ""));
    response.headers_mut().append(
        header::SET_COOKIE,
        state.oauth.cookie(LOGIN_INTENT_COOKIE, &signed, 600),
    );
    response
}

fn trusted_origin(state: &AppState, headers: &HeaderMap, allow_missing: bool) -> bool {
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
struct LoginForm {
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

async fn login_post(
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
        return html(medtracker_web::render_login(
            intent_csrf,
            "Invalid email or password",
        ));
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
        return html(medtracker_web::render_login(
            intent_csrf,
            "Invalid email or password",
        ));
    }
    let available = match account_available(&db, account_id).await {
        Ok(value) => value,
        Err(error) => return error,
    };
    if !available {
        return html(medtracker_web::render_login(
            intent_csrf,
            "Sign in is unavailable for this account",
        ));
    }
    let required = match factor_required(&db, account_id).await {
        Ok(value) => value,
        Err(error) => return error,
    };
    if required {
        return html(medtracker_web::render_login(
            intent_csrf,
            "This account requires a sign-in method that is not yet supported here.",
        ));
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
    let destination = if let Some(claim) = oauth_claim.as_ref() {
        claim.request.path()
    } else {
        match first_active_household(&db, account_id).await {
            Ok(Some(household)) => format!("/households/{}/dashboard", household.slug),
            Ok(None) => "/".to_owned(),
            Err(error) => return error,
        }
    };
    if let Err(error) = db.commit().await {
        return database_error(error).into_response();
    }
    let session = BrowserSession {
        account_id,
        session_id,
        issued_at: Utc::now().timestamp(),
        csrf,
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

async fn home(State(state): State<AppState>, headers: HeaderMap) -> Response {
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

async fn dashboard(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Response {
    let db = match transaction(&state).await {
        Ok(db) => db,
        Err(error) => return error,
    };
    let session = match browser_session(&state, &db, &headers).await {
        Ok(Some(session)) => session,
        Ok(None) => return redirect("/login"),
        Err(error) => return error,
    };
    if let Err(error) =
        tenant_setting(&db, "med_tracker.current_account_id", session.account_id).await
    {
        return database_error(error).into_response();
    }
    let household = match household::Entity::find()
        .filter(household::Column::Slug.eq(slug))
        .filter(household::Column::Status.eq("active"))
        .filter(household::Column::LifecycleState.eq("active"))
        .one(&db)
        .await
    {
        Ok(Some(household)) => household,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => return database_error(error).into_response(),
    };
    let membership = membership::Entity::find()
        .filter(membership::Column::AccountId.eq(session.account_id))
        .filter(membership::Column::HouseholdId.eq(household.id))
        .filter(membership::Column::Status.eq("active"))
        .filter(membership::Column::RevokedAt.is_null())
        .one(&db)
        .await;
    match membership {
        Ok(Some(_)) => {}
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => return database_error(error).into_response(),
    }
    if let Err(error) = db.commit().await {
        return database_error(error).into_response();
    }
    let mut response = html(medtracker_web::render_dashboard(
        &household.name,
        &session.csrf,
        false,
    ));
    if let Some(cookie) = renewed_session_cookie(&state, &session) {
        response.headers_mut().append(header::SET_COOKIE, cookie);
    }
    response
}

async fn logout(
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

fn form_fields(headers: &HeaderMap, body: &[u8]) -> Option<Vec<(String, String)>> {
    if body.len() > 8192
        || !headers
            .get(header::CONTENT_TYPE)?
            .to_str()
            .ok()?
            .starts_with("application/x-www-form-urlencoded")
    {
        return None;
    }
    Some(form_urlencoded::parse(body).into_owned().collect())
}

fn field<'a>(fields: &'a [(String, String)], name: &str) -> Option<&'a str> {
    let mut values = fields
        .iter()
        .filter(|(key, _)| key == name)
        .map(|(_, value)| value.as_str());
    let value = values.next()?;
    values.next().is_none().then_some(value)
}

async fn consent(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let Some(pending) = pending(&state, &headers) else {
        return StatusCode::FORBIDDEN.into_response();
    };
    let Some(fields) = form_fields(&headers, &body) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let db = match transaction(&state).await {
        Ok(db) => db,
        Err(error) => return error,
    };
    let session = match browser_session(&state, &db, &headers).await {
        Ok(Some(session)) => session,
        Ok(None) => return StatusCode::FORBIDDEN.into_response(),
        Err(error) => return error,
    };
    if field(&fields, "authenticity_token") != Some(session.csrf.as_str()) {
        return StatusCode::FORBIDDEN.into_response();
    }
    for (key, expected) in [
        ("response_type", pending.request.response_type.as_str()),
        ("response_mode", pending.request.response_mode.as_str()),
        ("client_id", pending.request.client_id.as_str()),
        ("redirect_uri", pending.request.redirect_uri.as_str()),
        ("state", pending.request.state.as_str()),
        ("code_challenge", pending.request.code_challenge.as_str()),
        (
            "code_challenge_method",
            pending.request.code_challenge_method.as_str(),
        ),
    ] {
        if field(&fields, key) != Some(expected) {
            return oauth_error("invalid_request");
        }
    }
    let requested = pending.request.scope.split_whitespace().collect::<Vec<_>>();
    let selected = fields
        .iter()
        .filter(|(key, _)| key == "scope[]")
        .map(|(_, value)| value.as_str())
        .collect::<Vec<_>>();
    if selected.is_empty()
        || !selected.contains(&"medtracker")
        || selected.len()
            != selected
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
        || !selected.iter().all(|scope| requested.contains(scope))
    {
        return oauth_error("invalid_scope");
    }
    let approved_scopes = selected.join(" ");
    let registered = match client(&db, &pending.request.client_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return oauth_error("unauthorized_client"),
        Err(error) => return error,
    };
    if !valid_request(&pending.request, &registered) {
        return oauth_error("invalid_request");
    }
    if let Err(error) =
        tenant_setting(&db, "med_tracker.current_account_id", session.account_id).await
    {
        return database_error(error).into_response();
    }
    let code = secret();
    let now = Utc::now().naive_utc();
    let authenticated_at = match chrono::DateTime::<Utc>::from_timestamp(session.issued_at, 0) {
        Some(value) => value.naive_utc(),
        None => return StatusCode::FORBIDDEN.into_response(),
    };
    let grant = oauth_grant::ActiveModel {
        account_id: Set(session.account_id),
        oauth_application_id: Set(registered.id),
        client_kind: Set("mobile".to_owned()),
        scopes: Set(approved_scopes),
        code: Set(Some(code.clone())),
        code_challenge: Set(Some(pending.request.code_challenge.clone())),
        code_challenge_method: Set(Some("S256".to_owned())),
        redirect_uri: Set(Some(pending.request.redirect_uri.clone())),
        expires_in: Set(now + Duration::minutes(5)),
        authenticated_at: Set(Some(authenticated_at)),
        last_used_at: Set(Some(now)),
        device_name: Set(Some(registered.name)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    let insert = grant.insert(&db).await;
    if let Err(error) = insert {
        return database_error(error).into_response();
    }
    if let Err(error) = db.commit().await {
        return database_error(error).into_response();
    }
    let separator = if pending.request.redirect_uri.contains('?') {
        '&'
    } else {
        '?'
    };
    let mut query = form_urlencoded::Serializer::new(String::new());
    query
        .append_pair("code", &code)
        .append_pair("state", &pending.request.state);
    let callback = format!(
        "{}{separator}{}",
        pending.request.redirect_uri,
        query.finish()
    );
    let mut response = redirect(&callback);
    if let Some(cookie) = state.oauth.sign(&session) {
        response.headers_mut().append(
            header::SET_COOKIE,
            state
                .oauth
                .cookie(SESSION_COOKIE, &cookie, browser_cookie_age()),
        );
    }
    response.headers_mut().append(
        header::SET_COOKIE,
        state.oauth.cookie(PENDING_COOKIE, "", 0),
    );
    response
}

#[derive(Deserialize)]
struct TokenForm {
    grant_type: String,
    client_id: String,
    redirect_uri: Option<String>,
    code: Option<String>,
    code_verifier: Option<String>,
    refresh_token: Option<String>,
}

async fn token(State(state): State<AppState>, Form(form): Form<TokenForm>) -> Response {
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
struct RevokeInput {
    client_id: String,
    token: String,
    token_type_hint: Option<String>,
}

async fn revoke(State(state): State<AppState>, Json(input): Json<RevokeInput>) -> Response {
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

async fn households(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let Some(token) = headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
    else {
        return crate::ApiError::unauthorized().into_response();
    };
    if token.len() > 256 || token.is_empty() {
        return crate::ApiError::unauthorized().into_response();
    }
    let db = match transaction(&state).await {
        Ok(db) => db,
        Err(error) => return error,
    };
    let grant = oauth_grant::Entity::find()
        .filter(oauth_grant::Column::TokenHash.eq(digest(token)))
        .filter(oauth_grant::Column::ClientKind.eq("mobile"))
        .filter(oauth_grant::Column::RevokedAt.is_null())
        .one(&db)
        .await;
    let grant = match grant {
        Ok(Some(grant)) => grant,
        Ok(None) => return crate::ApiError::unauthorized().into_response(),
        Err(error) => return database_error(error).into_response(),
    };
    let account_id = grant.account_id;
    let now = Utc::now().naive_utc();
    let inactivity =
        configured_lifetime_days("SESSION_INACTIVITY_TIMEOUT_DAYS", 30, 1).unwrap_or(0);
    let maximum = configured_lifetime_days("SESSION_MAX_AGE_DAYS", 0, 0).unwrap_or(-1);
    if grant.expires_in <= now
        || !grant
            .scopes
            .split_whitespace()
            .any(|scope| scope == "medtracker")
        || inactivity < 1
        || maximum < 0
        || !grant
            .last_used_at
            .is_some_and(|time| time > now - Duration::days(inactivity))
        || !grant
            .authenticated_at
            .is_some_and(|time| maximum == 0 || time > now - Duration::days(maximum))
        || !matches!(account_available(&db, account_id).await, Ok(true))
    {
        return crate::ApiError::unauthorized().into_response();
    }
    if let Err(error) = tenant_setting(&db, "med_tracker.current_account_id", account_id).await {
        return database_error(error).into_response();
    }
    let memberships = membership::Entity::find()
        .filter(membership::Column::AccountId.eq(account_id))
        .filter(membership::Column::Status.eq("active"))
        .filter(membership::Column::RevokedAt.is_null())
        .order_by_asc(membership::Column::Id)
        .all(&db)
        .await;
    let memberships = match memberships {
        Ok(rows) => rows,
        Err(error) => return database_error(error).into_response(),
    };
    let household_ids: Vec<i64> = memberships.iter().map(|m| m.household_id).collect();
    let households = match household::Entity::find()
        .filter(household::Column::Id.is_in(household_ids))
        .filter(household::Column::Status.eq("active"))
        .filter(household::Column::LifecycleState.eq("active"))
        .all(&db)
        .await
    {
        Ok(rows) => rows,
        Err(error) => return database_error(error).into_response(),
    };
    let households: HashMap<i64, household::Model> =
        households.into_iter().map(|h| (h.id, h)).collect();
    let data: Vec<Value> = memberships.into_iter().filter_map(|m| {
        let h = households.get(&m.household_id)?;
        Some(json!({"id": h.id, "slug": h.slug, "name": h.name, "role": m.role, "membership_id": m.id}))
    }).collect();
    let mut active: oauth_grant::ActiveModel = grant.into();
    active.last_used_at = Set(Some(now));
    active.updated_at = Set(now);
    if let Err(error) = active.update(&db).await {
        return database_error(error).into_response();
    }
    if let Err(error) = db.commit().await {
        return database_error(error).into_response();
    }
    (
        [(header::CACHE_CONTROL, "no-store")],
        Json(json!({"account_id": account_id, "data": data})),
    )
        .into_response()
}
