#![allow(clippy::result_large_err)]

mod authorization;
mod configuration;
mod discovery;
mod helpers;
mod login;
mod passkey;
mod pkce;
mod sessions;
mod tokens;

use crate::entities::{
    account, account_lockout, active_session_key, household, membership, oauth_application,
    otp_key, person, recovery_code, user, webauthn_key,
};
use crate::{configured_lifetime_days, restricted_role, tenant_setting, AppState};
use axum::extract::{Form, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, DbBackend, EntityTrait,
    QueryFilter, QueryOrder, Set, Statement, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use url::{form_urlencoded, Url};

use authorization::authorize;
use authorization::consent;
use authorization::AuthorizationRequest;
use configuration::digest;
use configuration::secret;
pub(crate) use configuration::session_key_digest;
pub use configuration::OAuthState;
use discovery::capabilities;
use discovery::discovery;
use discovery::households;
use discovery::reset_password_unavailable;
use discovery::styles;
use helpers::database_error;
use helpers::field;
use helpers::form_fields;
use helpers::html;
use helpers::redirect;
use helpers::sql;
use helpers::transaction;
use login::home;
use login::login;
use login::login_post;
use login::logout;
use passkey::passkey_login;
use passkey::passkey_script;
use passkey::PasskeyChallenge;
use sessions::account_available;
use sessions::browser_cookie_age;
pub(crate) use sessions::browser_session;
use sessions::factor_required;
use sessions::login_intent;
use sessions::pending;
pub(crate) use sessions::renewed_session_cookie;
pub(crate) use sessions::trusted_cookie_origin;
use sessions::trusted_origin;
pub(crate) use sessions::BrowserSession;
use sessions::LoginIntent;
use sessions::Pending;
use sessions::LOGIN_INTENT_COOKIE;
use sessions::SESSION_COOKIE;

pub(crate) fn clear_browser_session_cookie(state: &AppState) -> HeaderValue {
    state.oauth.cookie(SESSION_COOKIE, "", 0)
}
use tokens::revoke;
use tokens::token;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/.well-known/oauth-authorization-server", get(discovery))
        .route("/authorize", get(authorize).post(consent))
        .route("/login", get(login).post(login_post))
        .route("/webauthn-login", post(passkey_login))
        .route("/auth-passkey.js", get(passkey_script))
        .route("/logout", post(logout))
        .route("/", get(home))
        .route("/token", post(token))
        .route("/revoke", post(revoke))
        .route("/reset-password-request", get(reset_password_unavailable))
        .route("/auth.css", get(styles))
}

pub fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/capabilities", get(capabilities))
        .route("/api/v1/auth/households", get(households))
}
