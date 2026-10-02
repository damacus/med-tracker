mod admin_settings;
mod api_request_guards;
mod api_responses;
mod api_routes;
mod app_tokens;
mod audit;
mod audit_logs;
pub mod auth_compatibility;
mod auth_sessions;
mod dosage_options;
mod dose;
mod dose_occurrences;
mod entities;
mod external_integrations;
mod health_events;
mod invitations;
mod locations;
mod medication_forecast;
mod medication_management;
mod medication_projection;
mod medication_reads;
mod memberships;
mod mutation_idempotency;
mod native_device_tokens;
mod notification_preferences;
mod oauth;
mod pause_lifecycle;
mod people;
mod person_grants;
mod person_medication_writes;
mod portable_crypto;
mod portable_exports;
mod portable_imports;
mod portable_projection;
mod profile;
mod push_subscriptions;
mod rate_limit;
mod read_entities;
mod read_resources;
mod reports;
mod request_authentication;
mod review_evidence;
mod review_prompts;
mod schedule_writes;
mod stock_removals;
mod sync_batch;
mod sync_events;
mod sync_reads;
mod ui_capabilities;
mod web_pages;
pub mod webauthn;

use api_request_guards::rate_middleware;
use api_responses::{
    database_error, if_none_match_matches, representation_etag, ApiError, Pagination,
};
use api_routes::api_router;
use axum::http::StatusCode;
use axum::{middleware, routing::get, Router};
use medication_projection::decimal_string;
pub(crate) use medication_projection::serialize_many;
use medication_reads::{granted_people, scope};
pub(crate) use request_authentication::tenant_setting;
use request_authentication::{
    authenticate, configured_lifetime_days, restricted_role, within_login_lifetime, AuthContext,
    CredentialKind,
};
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement,
    TransactionTrait,
};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct AppState {
    db: DatabaseConnection,
    oauth: oauth::OAuthState,
    invitation_mail: Arc<invitations::MailConfig>,
    avatar_storage: profile::AvatarStorage,
    rate_limiter: Arc<rate_limit::RateLimiter>,
    trusted_proxy_ips: Arc<std::collections::HashSet<IpAddr>>,
}

pub async fn connect(url: &str) -> Result<AppState, sea_orm::DbErr> {
    let mut options = ConnectOptions::new(url);
    options.max_connections(4).min_connections(1);
    options.connect_timeout(Duration::from_secs(5));
    options.acquire_timeout(Duration::from_secs(5));
    options.sqlx_logging(false);
    let db = Database::connect(options).await?;
    let transaction = db.begin().await?;
    restricted_role(&transaction).await?;
    let role = transaction
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT current_role AS role, row_security_active('medications'::regclass) AS rls_active",
        ))
        .await?
        .ok_or_else(|| sea_orm::DbErr::Custom("database role check returned no row".to_owned()))?;
    let name: String = role.try_get("", "role")?;
    let rls_active: bool = role.try_get("", "rls_active")?;
    if name != "med_tracker_app" || !rls_active {
        return Err(sea_orm::DbErr::Custom(
            "medication RLS must be active under med_tracker_app".to_owned(),
        ));
    }
    transaction.rollback().await?;
    let oauth = oauth::OAuthState::from_env().map_err(sea_orm::DbErr::Custom)?;
    let invitation_mail =
        Arc::new(invitations::MailConfig::from_env().map_err(sea_orm::DbErr::Custom)?);
    let avatar_storage = profile::AvatarStorage::from_env().map_err(sea_orm::DbErr::Custom)?;
    Ok(AppState {
        db,
        oauth,
        invitation_mail,
        avatar_storage,
        rate_limiter: Arc::new(rate_limit::RateLimiter::default()),
        trusted_proxy_ips: Arc::new(rate_limit::trusted_proxy_ips()),
    })
}

pub fn router(state: AppState) -> Router {
    let rate_state = state.clone();
    Router::new()
        .route("/up", get(|| async { StatusCode::OK }))
        .merge(oauth::routes().with_state(state.clone()))
        .merge(web_pages::routes().with_state(state.clone()))
        .merge(api_router(state))
        .layer(middleware::from_fn_with_state(rate_state, rate_middleware))
}
