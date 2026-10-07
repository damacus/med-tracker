use crate::models::identity::store::digest as token_digest;
use axum_session::{Session, SessionConfig, SessionMode, SessionStore};
use axum_session_sqlx::SessionPgPool;
use base64::{Engine, engine::general_purpose::STANDARD};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, Statement, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

mod otp;
pub mod passkeys;
mod recovery;
pub use otp::{challenge, has_pending_challenge, verify as verify_otp};
pub use recovery::{challenge as recovery_challenge, verify as verify_recovery};

use super::{resource::AuthenticationError, store::Lifetime};
use crate::models::{
    access::{self, Actor, HouseholdScope, TenantTransaction},
    care::doses::{CredentialMethod, CredentialProvenance},
    entities::{account, household, person, user},
};

#[derive(Clone, Serialize, Deserialize)]
struct Identity {
    account_id: i64,
    registry_key: String,
    #[serde(default)]
    additional_factor_verified: bool,
    #[serde(default)]
    better_auth: bool,
}

pub enum SignInOutcome {
    Authenticated,
    OtpRequired,
    RecoveryRequired,
    PasskeyRequired,
}

pub struct BrowserPrincipal {
    identity: Identity,
    provenance: CredentialProvenance,
    time_zone: chrono_tz::Tz,
}

#[derive(Serialize)]
pub struct BrowserHousehold {
    pub id: i64,
    pub name: String,
    pub slug: String,
}

pub struct BrowserInvitationActor {
    pub account_id: i64,
    pub user_id: i64,
    pub person_id: i64,
    pub email: String,
    pub provenance: CredentialProvenance,
}

impl BrowserPrincipal {
    pub async fn revalidate_invitation(
        &self,
        transaction: &DatabaseTransaction,
    ) -> Result<BrowserInvitationActor, AuthenticationError> {
        let account = validate(transaction, &self.identity, false).await?;
        let person = person::Entity::find()
            .filter(person::Column::AccountId.eq(account.id))
            .order_by_asc(person::Column::Id)
            .one(transaction)
            .await
            .map_err(unavailable)?
            .ok_or(AuthenticationError::Unauthenticated)?;
        let user = user::Entity::find()
            .filter(user::Column::PersonId.eq(person.id))
            .filter(user::Column::Active.eq(true))
            .one(transaction)
            .await
            .map_err(unavailable)?
            .ok_or(AuthenticationError::Unauthenticated)?;
        Ok(BrowserInvitationActor {
            account_id: account.id,
            user_id: user.id,
            person_id: person.id,
            email: account.email,
            provenance: self.provenance.clone(),
        })
    }

    pub(super) async fn authorization_transaction(
        &self,
        db: &DatabaseConnection,
    ) -> Result<(DatabaseTransaction, chrono::NaiveDateTime), AuthenticationError> {
        let transaction = transaction(db).await?;
        validate(&transaction, &self.identity, false).await?;
        let query = "SELECT created_at AT TIME ZONE 'UTC' AS created_at FROM public.identity_sessions WHERE account_id=$1 AND token=$2";
        let row = transaction
            .query_one_raw(sql(
                query,
                [
                    self.account_id().into(),
                    token_digest(&self.identity.registry_key).into(),
                ],
            ))
            .await
            .map_err(unavailable)?
            .ok_or(AuthenticationError::Unauthenticated)?;
        let authenticated_at = row.try_get("", "created_at").map_err(unavailable)?;
        Ok((transaction, authenticated_at))
    }
    pub fn account_id(&self) -> i64 {
        self.identity.account_id
    }
    pub fn provenance(&self) -> &CredentialProvenance {
        &self.provenance
    }
    pub fn time_zone(&self) -> chrono_tz::Tz {
        self.time_zone
    }

    pub async fn households(
        &self,
        db: &DatabaseConnection,
    ) -> Result<Vec<BrowserHousehold>, AuthenticationError> {
        let transaction = transaction(db).await?;
        validate(&transaction, &self.identity, false).await?;
        let rows = transaction.query_all_raw(sql(
            "SELECT h.id, h.name, h.slug FROM households h JOIN household_memberships m ON m.household_id = h.id WHERE m.account_id = $1 AND m.status = 'active' AND m.revoked_at IS NULL AND h.status = 'active' AND h.lifecycle_state = 'active' ORDER BY h.name, h.id",
            [self.account_id().into()],
        )).await.map_err(unavailable)?;
        let households = rows
            .into_iter()
            .map(|row| {
                Ok(BrowserHousehold {
                    id: row.try_get("", "id").map_err(unavailable)?,
                    name: row.try_get("", "name").map_err(unavailable)?,
                    slug: row.try_get("", "slug").map_err(unavailable)?,
                })
            })
            .collect::<Result<Vec<_>, AuthenticationError>>()?;
        transaction.commit().await.map_err(unavailable)?;
        Ok(households)
    }

    pub async fn begin_household_slug(
        &self,
        db: &DatabaseConnection,
        slug: &str,
        request_id: String,
    ) -> Result<TenantTransaction, AuthenticationError> {
        let lookup = transaction(db).await?;
        validate(&lookup, &self.identity, false).await?;
        let selected = household::Entity::find()
            .filter(household::Column::Slug.eq(slug))
            .one(&lookup)
            .await
            .map_err(unavailable)?
            .ok_or(AuthenticationError::Forbidden)?;
        lookup.commit().await.map_err(unavailable)?;
        let tenant = access::begin(
            db,
            &HouseholdScope {
                actor: Actor {
                    account_id: self.account_id(),
                },
                household_id: selected.id,
                request_id,
            },
        )
        .await
        .map_err(super::resource::operation_error)?;
        if let Err(error) = validate(tenant.transaction(), &self.identity, false).await {
            tenant.rollback().await.map_err(unavailable)?;
            return Err(error);
        }
        Ok(tenant)
    }
}

fn unavailable<T>(_: T) -> AuthenticationError {
    AuthenticationError::Unavailable
}

fn sql(query: &str, values: impl IntoIterator<Item = sea_orm::Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, query, values)
}

pub(super) async fn transaction(
    db: &DatabaseConnection,
) -> Result<DatabaseTransaction, AuthenticationError> {
    let transaction = db.begin().await.map_err(unavailable)?;
    transaction.execute_unprepared("SET LOCAL ROLE med_tracker_app; SET LOCAL search_path = pg_catalog, public, pg_temp; SET LOCAL lock_timeout = '5s'; SET LOCAL statement_timeout = '30s'; SELECT set_config('med_tracker.current_account_id', '', true), set_config('med_tracker.current_household_id', '', true), set_config('med_tracker.current_membership_id', '', true), set_config('med_tracker.current_invitation_token_digest', '', true)").await.map_err(unavailable)?;
    Ok(transaction)
}

async fn validate(
    transaction: &DatabaseTransaction,
    identity: &Identity,
    _touch: bool,
) -> Result<account::Model, AuthenticationError> {
    if !identity.better_auth {
        return Err(AuthenticationError::Unauthenticated);
    }
    transaction
        .execute_raw(sql(
            "SELECT set_config('med_tracker.current_account_id',$1,true)",
            [identity.account_id.to_string().into()],
        ))
        .await
        .map_err(unavailable)?;
    let valid = transaction.query_one_raw(sql("SELECT account_id FROM public.identity_sessions WHERE account_id=$1 AND token=$2 AND active AND purpose='authenticated' AND expires_at>clock_timestamp() AND created_at>clock_timestamp()-interval '30 days' FOR SHARE", [identity.account_id.into(), token_digest(&identity.registry_key).into()])).await.map_err(unavailable)?;
    if valid.is_none() {
        return Err(AuthenticationError::Unauthenticated);
    }
    access::verify_account_actor(transaction, identity.account_id)
        .await
        .map_err(super::resource::operation_error)?;
    account::Entity::find_by_id(identity.account_id)
        .filter(account::Column::Status.eq(2))
        .one(transaction)
        .await
        .map_err(unavailable)?
        .ok_or(AuthenticationError::Unauthenticated)
}

pub async fn authenticate(
    db: &DatabaseConnection,
    session: &Session<SessionPgPool>,
) -> Result<BrowserPrincipal, AuthenticationError> {
    let identity = session
        .get::<Identity>("identity")
        .ok_or(AuthenticationError::Unauthenticated)?;
    let transaction = transaction(db).await?;
    let account = validate(&transaction, &identity, true).await?;
    let time_zone = super::time_zone::preferred(&account.preferences)?;
    transaction.commit().await.map_err(unavailable)?;
    Ok(BrowserPrincipal {
        provenance: CredentialProvenance {
            method: CredentialMethod::BrowserSession,
            reference: token_digest(&identity.registry_key),
        },
        identity,
        time_zone,
    })
}

pub async fn sign_in(
    db: &DatabaseConnection,
    session: &Session<SessionPgPool>,
    email: String,
    password: String,
) -> Result<SignInOutcome, AuthenticationError> {
    if email.len() > 320 || password.len() > 1024 {
        return Err(AuthenticationError::Unauthenticated);
    }
    let transaction = transaction(db).await?;
    let account = account::Entity::find()
        .filter(account::Column::Email.eq(email))
        .filter(account::Column::Status.eq(2))
        .lock_exclusive()
        .one(&transaction)
        .await
        .map_err(unavailable)?;
    let hash = account
        .as_ref()
        .and_then(|account| account.password_hash.clone())
        .unwrap_or_else(|| "$2b$12$zzzzzzzzzzzzzzzzzzzzzuD0kNWzTTXJvCDkBPuJKtf5Hh4bS5Mk2".into());
    let valid = tokio::task::spawn_blocking(move || bcrypt::verify(password, &hash))
        .await
        .map_err(unavailable)?
        .map_err(unavailable)?;
    let account = account.ok_or(AuthenticationError::Unauthenticated)?;
    if !valid {
        let result = transaction.query_one_raw(sql("INSERT INTO account_login_failures (account_id, number, created_at, updated_at) VALUES ($1, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (account_id) DO UPDATE SET number = account_login_failures.number + 1, updated_at = CURRENT_TIMESTAMP RETURNING number", [account.id.into()])).await.map_err(unavailable)?.ok_or(AuthenticationError::Unavailable)?;
        let failures: i32 = result.try_get("", "number").map_err(unavailable)?;
        if failures >= 5 {
            transaction.execute_raw(sql("INSERT INTO account_lockouts (account_id, deadline, key, created_at, updated_at) VALUES ($1, CURRENT_TIMESTAMP + interval '30 minutes', $2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (account_id) DO UPDATE SET deadline = EXCLUDED.deadline, updated_at = CURRENT_TIMESTAMP", [account.id.into(), uuid::Uuid::new_v4().to_string().into()])).await.map_err(unavailable)?;
        }
        transaction.commit().await.map_err(unavailable)?;
        return Err(AuthenticationError::Unauthenticated);
    }
    complete_primary_authentication(transaction, session, account.id).await
}

pub(super) async fn complete_primary_authentication(
    transaction: DatabaseTransaction,
    session: &Session<SessionPgPool>,
    account_id: i64,
) -> Result<SignInOutcome, AuthenticationError> {
    access::verify_account_actor(&transaction, account_id)
        .await
        .map_err(super::resource::operation_error)?;
    let factor = transaction.query_one_raw(sql("SELECT EXISTS (SELECT 1 FROM account_otp_keys WHERE id=$1) AS otp, EXISTS (SELECT 1 FROM account_webauthn_keys WHERE account_id=$1) AS passkey, timezone('UTC',clock_timestamp()) AS authenticated_at", [account_id.into()])).await.map_err(unavailable)?.ok_or(AuthenticationError::Unavailable)?;
    let authenticated_at = factor
        .try_get("", "authenticated_at")
        .map_err(unavailable)?;
    if factor.try_get::<bool>("", "otp").map_err(unavailable)? {
        otp::begin(transaction, session, account_id, authenticated_at).await?;
        return Ok(SignInOutcome::OtpRequired);
    }
    if factor.try_get::<bool>("", "passkey").map_err(unavailable)? {
        let recovery = transaction
            .query_one_raw(sql(
                "SELECT EXISTS (SELECT 1 FROM account_recovery_codes WHERE id=$1) AS present",
                [account_id.into()],
            ))
            .await
            .map_err(unavailable)?
            .ok_or(AuthenticationError::Unavailable)?;
        if recovery
            .try_get::<bool>("", "present")
            .map_err(unavailable)?
        {
            otp::begin(transaction, session, account_id, authenticated_at).await?;
            return Ok(SignInOutcome::RecoveryRequired);
        }
        otp::begin(transaction, session, account_id, authenticated_at).await?;
        return Ok(SignInOutcome::PasskeyRequired);
    }
    issue_session(transaction, session, account_id, authenticated_at, false).await?;
    Ok(SignInOutcome::Authenticated)
}

pub(super) async fn record_auth_token(
    transaction: &DatabaseTransaction,
    account_id: i64,
    token_type: &str,
    action: &str,
    request_id: Option<&str>,
) -> Result<(), AuthenticationError> {
    passkeys::audit::record(transaction, account_id, token_type, action, request_id).await
}

async fn issue_session(
    transaction: DatabaseTransaction,
    session: &Session<SessionPgPool>,
    account_id: i64,
    authenticated_at: chrono::NaiveDateTime,
    additional_factor_verified: bool,
) -> Result<(), AuthenticationError> {
    let registry_key = hex::encode(Sha256::digest(uuid::Uuid::new_v4().as_bytes()));
    transaction.execute_raw(sql("INSERT INTO account_active_session_keys (account_id, session_id, created_at, last_use) VALUES ($1, $2, $3, timezone('UTC', clock_timestamp()))", [account_id.into(), registry_key.clone().into(), authenticated_at.into()])).await.map_err(unavailable)?;
    transaction
        .execute_raw(sql(
            "DELETE FROM account_login_failures WHERE account_id = $1",
            [account_id.into()],
        ))
        .await
        .map_err(unavailable)?;
    transaction.commit().await.map_err(unavailable)?;
    session.renew();
    session.set_store(true);
    session.remove("otp_pending");
    session.set(
        "identity",
        Identity {
            account_id,
            registry_key,
            additional_factor_verified,
            better_auth: false,
        },
    );
    Ok(())
}

pub async fn sign_out(
    db: &DatabaseConnection,
    session: &Session<SessionPgPool>,
) -> Result<(), AuthenticationError> {
    if let Some(identity) = session.get::<Identity>("identity") {
        let transaction = transaction(db).await?;
        transaction
            .execute_raw(sql(
                "DELETE FROM account_active_session_keys WHERE account_id = $1 AND session_id = $2",
                [identity.account_id.into(), identity.registry_key.into()],
            ))
            .await
            .map_err(unavailable)?;
        transaction.commit().await.map_err(unavailable)?;
    }
    session.destroy();
    Ok(())
}

#[derive(Clone)]
pub struct BrowserLayers {
    store: SessionStore<SessionPgPool>,
    csrf: axum_csrf::CsrfConfig,
}

pub fn route_layers(
    ctx: &loco_rs::app::AppContext,
    routes: loco_rs::controller::Routes,
) -> loco_rs::controller::Routes {
    let routes = match ctx
        .shared_store
        .get::<super::better_auth::IdentityService>()
    {
        Some(service) => routes
            .layer(axum::middleware::from_fn(identity_bridge))
            .layer(axum::Extension(service)),
        None => routes,
    };
    match ctx.shared_store.get::<BrowserLayers>() {
        Some(layers) => routes
            .layer(axum_session::SessionLayer::new(layers.store))
            .layer(axum_csrf::CsrfLayer::new(layers.csrf))
            .layer(tower_http::csrf::CsrfLayer::new()),
        None => routes,
    }
}

async fn identity_bridge(
    axum::Extension(service): axum::Extension<super::better_auth::IdentityService>,
    session: Session<SessionPgPool>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    let mut auth =
        better_auth_core::AuthRequest::new(better_auth_core::HttpMethod::Get, "/get-session");
    auth.headers = request
        .headers()
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.to_string(), value.to_owned()))
        })
        .collect();
    let mut destination = None;
    match super::better_auth::browser_identity(&service, &auth).await {
        Ok(super::better_auth::BrowserIdentity::Authenticated {
            user,
            session: identity,
        }) => {
            let Ok(account_id) = user.id.parse() else {
                return axum::http::StatusCode::UNAUTHORIZED.into_response();
            };
            session.set(
                "identity",
                Identity {
                    account_id,
                    registry_key: identity.token,
                    additional_factor_verified: true,
                    better_auth: true,
                },
            );
            session.set_store(true);
        }
        Ok(super::better_auth::BrowserIdentity::PendingFactor) => {
            session.remove("identity");
            if request.uri().path() == "/" {
                destination = Some("/auth/totp");
            }
        }
        Ok(super::better_auth::BrowserIdentity::Enrolment { .. }) => {
            session.remove("identity");
            if request.uri().path() == "/" {
                destination = Some("/auth/passkey/setup");
            }
        }
        Ok(super::better_auth::BrowserIdentity::SignedOut) => {
            session.remove("identity");
        }
        Err(_) => return axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
    let mut response = if let Some(destination) = destination {
        axum::response::Redirect::to(destination).into_response()
    } else {
        next.run(request).await
    };
    let Ok(mut finalized) =
        better_auth_core::AuthResponse::json(response.status().as_u16(), &serde_json::Value::Null)
    else {
        return axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    for cookie in response.headers().get_all(axum::http::header::SET_COOKIE) {
        let Ok(value) = cookie.to_str() else {
            return axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response();
        };
        finalized.headers.append("set-cookie", value.to_owned());
    }
    if service
        .context()
        .session_manager()
        .finish_response(&auth, &mut finalized)
        .await
        .is_err()
    {
        return axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    response
        .headers_mut()
        .remove(axum::http::header::SET_COOKIE);
    for cookie in finalized.headers.get_all("set-cookie") {
        let Ok(value) = cookie.parse() else {
            return axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response();
        };
        response
            .headers_mut()
            .append(axum::http::header::SET_COOKIE, value);
    }
    response
}

pub async fn layers(
    ctx: &loco_rs::app::AppContext,
) -> Result<Option<BrowserLayers>, AuthenticationError> {
    let db = &ctx.db;
    let settings = ctx
        .config
        .settings
        .as_ref()
        .map(|settings| &settings["browser_session"]);
    let encoded = match std::env::var("MEDTRACKER_SESSION_KEY") {
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) => {
            match settings.and_then(|settings| settings["key"].as_str()) {
                Some(key) => key.to_owned(),
                None if matches!(ctx.environment, loco_rs::environment::Environment::Test) => {
                    return Ok(None);
                }
                None => return Err(AuthenticationError::Unavailable),
            }
        }
        Err(_) => return Err(AuthenticationError::Unavailable),
    };
    let bytes = STANDARD.decode(encoded).map_err(unavailable)?;
    if bytes.len() != 64 {
        return Err(AuthenticationError::Unavailable);
    }
    let key = axum_session::Key::from(&bytes);
    let secure = match std::env::var("MEDTRACKER_COOKIE_SECURE").as_deref() {
        Ok("false") => false,
        Ok("true") => true,
        Err(std::env::VarError::NotPresent) => settings
            .and_then(|settings| settings["secure"].as_bool())
            .unwrap_or(true),
        _ => return Err(AuthenticationError::Unavailable),
    };
    let policy = Lifetime::from_environment().map_err(unavailable)?;
    db.query_one_raw(Statement::from_string(
        DbBackend::Postgres,
        "SELECT id, expires, session FROM public.browser_sessions LIMIT 0",
    ))
    .await
    .map_err(unavailable)?;
    let config = SessionConfig::default()
        .with_table_name("browser_sessions")
        .with_session_name("medtracker_session")
        .with_key(key.clone())
        .with_database_key(key.clone())
        .with_mode(SessionMode::OptIn)
        .with_lifetime(policy.inactivity)
        .with_max_age(Some(policy.inactivity))
        .with_db_update_interval(chrono::Duration::zero())
        .with_secure(secure)
        .with_http_only(true)
        .with_cookie_same_site(axum_session::SameSite::Lax);
    let mut store = SessionStore::<SessionPgPool>::new(None, config)
        .await
        .map_err(unavailable)?;
    store.client = Some(SessionPgPool::from(
        db.get_postgres_connection_pool().clone(),
    ));
    let csrf = axum_csrf::CsrfConfig::default()
        .with_key(Some(key))
        .with_secure(secure)
        .with_http_only(true)
        .with_cookie_name("medtracker_csrf")
        .with_cookie_same_site(axum_csrf::SameSite::Lax);
    Ok(Some(BrowserLayers { store, csrf }))
}
