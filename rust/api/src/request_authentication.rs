use crate::entities::{
    account, account_lockout, api_app_token, api_session, household, membership, oauth_grant,
    person, user,
};
use crate::{auth_sessions, database_error, oauth, ApiError, AppState};
use axum::http::{header, HeaderMap};
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use chrono::{Duration as ChronoDuration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, DbBackend, EntityTrait,
    QueryFilter, QueryOrder, Set, Statement,
};
use sha2::{Digest, Sha256};

pub(super) async fn restricted_role(db: &DatabaseTransaction) -> Result<(), sea_orm::DbErr> {
    db.execute_raw(Statement::from_string(
        DbBackend::Postgres,
        "SET LOCAL ROLE med_tracker_app",
    ))
    .await?;
    Ok(())
}

pub(crate) async fn tenant_setting(
    db: &DatabaseTransaction,
    name: &str,
    value: i64,
) -> Result<(), sea_orm::DbErr> {
    db.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT set_config($1, $2, true)",
        [name.into(), value.to_string().into()],
    ))
    .await?;
    Ok(())
}

pub(super) struct AuthContext {
    pub(super) account_id: i64,
    pub(super) user_id: i64,
    pub(super) credential_reference: String,
    pub(super) credential_kind: CredentialKind,
    pub(super) membership: membership::Model,
}

pub(super) enum CredentialKind {
    ApiSession,
    ApiAppToken,
    OauthGrant,
    BrowserSession,
}

pub(super) async fn authenticate(
    state: &AppState,
    db: &DatabaseTransaction,
    headers: &HeaderMap,
    household_id: i64,
) -> Result<AuthContext, ApiError> {
    restricted_role(db).await.map_err(database_error)?;
    if !headers.contains_key(header::AUTHORIZATION) {
        return authenticate_browser_session(state, db, headers, household_id).await;
    }
    let token = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|value| !value.is_empty())
        .ok_or_else(ApiError::unauthorized)?;
    let digest = format!("{:x}", Sha256::digest(token.as_bytes()));
    let session = api_session::Entity::find()
        .filter(api_session::Column::AccessTokenDigest.eq(&digest))
        .one(db)
        .await
        .map_err(database_error)?;
    let app = if session.is_none() {
        api_app_token::Entity::find()
            .filter(api_app_token::Column::TokenDigest.eq(digest))
            .one(db)
            .await
            .map_err(database_error)?
    } else {
        None
    };
    let now = Utc::now().naive_utc();
    let (account_id, membership_id, permissions_version, credential_reference, credential_kind) =
        if let Some(session) = session {
            if session.revoked_at.is_some() || session.access_expires_at <= now {
                return Err(ApiError::unauthorized());
            }
            (
                session.account_id,
                session
                    .household_membership_id
                    .ok_or_else(ApiError::unauthorized)?,
                session.permissions_version,
                session.id.to_string(),
                CredentialKind::ApiSession,
            )
        } else if let Some(app) = &app {
            if app.revoked_at.is_some() || !auth_sessions::app_unexpired(app, now) {
                return Err(ApiError::unauthorized());
            }
            (
                app.account_id,
                app.household_membership_id,
                app.permissions_version,
                app.id.to_string(),
                CredentialKind::ApiAppToken,
            )
        } else {
            return authenticate_mobile_oauth(db, token, household_id).await;
        };
    let account = account::Entity::find_by_id(account_id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    if account.status != 2 {
        return Err(ApiError::unauthorized());
    }
    let lockout = account_lockout::Entity::find_by_id(account.id)
        .one(db)
        .await
        .map_err(database_error)?;
    if lockout.is_some_and(|lockout| lockout.deadline > Utc::now().naive_utc()) {
        return Err(ApiError::unauthorized());
    }
    tenant_setting(db, "med_tracker.current_account_id", account.id)
        .await
        .map_err(database_error)?;
    let membership = membership::Entity::find_by_id(membership_id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    if membership.account_id != account.id
        || membership.status != "active"
        || membership.revoked_at.is_some()
        || membership.permissions_version != permissions_version
    {
        return Err(ApiError::unauthorized());
    }
    let home = household::Entity::find_by_id(membership.household_id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    if home.status != "active" || home.lifecycle_state != "active" {
        return Err(ApiError::unauthorized());
    }
    let person = person::Entity::find()
        .filter(person::Column::AccountId.eq(account.id))
        .order_by_asc(person::Column::Id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    let user = user::Entity::find()
        .filter(user::Column::PersonId.eq(person.id))
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    if !user.active {
        return Err(ApiError::unauthorized());
    }
    let household = household::Entity::find_by_id(household_id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?;
    if household.status != "active" || household.lifecycle_state != "active" {
        return Err(ApiError::forbidden());
    }
    if membership.household_id != household_id {
        return Err(ApiError::forbidden());
    }
    tenant_setting(db, "med_tracker.current_household_id", household_id)
        .await
        .map_err(database_error)?;
    tenant_setting(db, "med_tracker.current_membership_id", membership.id)
        .await
        .map_err(database_error)?;
    if let Some(app) = app {
        if app.last_used_at < now - ChronoDuration::minutes(5) {
            let mut active: api_app_token::ActiveModel = app.into();
            active.last_used_at = Set(now);
            active.updated_at = Set(now);
            active.update(db).await.map_err(database_error)?;
        }
    }
    Ok(AuthContext {
        account_id: account.id,
        user_id: user.id,
        credential_reference,
        credential_kind,
        membership,
    })
}

pub(super) fn configured_lifetime_days(name: &str, default: i64, minimum: i64) -> Option<i64> {
    match std::env::var(name) {
        Ok(value) => value.parse::<i64>().ok().filter(|days| *days >= minimum),
        Err(std::env::VarError::NotPresent) => Some(default),
        Err(std::env::VarError::NotUnicode(_)) => None,
    }
}

pub(super) fn within_login_lifetime(
    grant: &oauth_grant::Model,
    now: chrono::NaiveDateTime,
) -> bool {
    let Some(last_used_at) = grant.last_used_at else {
        return false;
    };
    let Some(authenticated_at) = grant.authenticated_at else {
        return false;
    };
    let Some(inactivity_days) = configured_lifetime_days("SESSION_INACTIVITY_TIMEOUT_DAYS", 30, 1)
    else {
        return false;
    };
    let Some(maximum_age_days) = configured_lifetime_days("SESSION_MAX_AGE_DAYS", 0, 0) else {
        return false;
    };
    let inactivity_active = ChronoDuration::try_days(inactivity_days)
        .and_then(|period| now.checked_sub_signed(period))
        .is_some_and(|deadline| last_used_at > deadline);
    let maximum_age_active = maximum_age_days == 0
        || ChronoDuration::try_days(maximum_age_days)
            .and_then(|period| now.checked_sub_signed(period))
            .is_some_and(|deadline| authenticated_at > deadline);
    inactivity_active && maximum_age_active
}

async fn current_household_membership(
    db: &DatabaseTransaction,
    account_id: i64,
    household_id: i64,
    preserve_activity: bool,
) -> Result<membership::Model, ApiError> {
    let missing = || {
        if preserve_activity {
            ApiError::not_found().preserve_activity()
        } else {
            ApiError::not_found()
        }
    };
    let forbidden = || {
        if preserve_activity {
            ApiError::forbidden().preserve_activity()
        } else {
            ApiError::forbidden()
        }
    };
    let household = household::Entity::find_by_id(household_id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(missing)?;
    if household.status != "active" || household.lifecycle_state != "active" {
        return Err(forbidden());
    }
    let membership = membership::Entity::find()
        .filter(membership::Column::AccountId.eq(account_id))
        .filter(membership::Column::HouseholdId.eq(household_id))
        .filter(membership::Column::Status.eq("active"))
        .filter(membership::Column::RevokedAt.is_null())
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(forbidden)?;
    tenant_setting(db, "med_tracker.current_household_id", household_id)
        .await
        .map_err(database_error)?;
    tenant_setting(db, "med_tracker.current_membership_id", membership.id)
        .await
        .map_err(database_error)?;
    Ok(membership)
}

async fn authenticate_browser_session(
    state: &AppState,
    db: &DatabaseTransaction,
    headers: &HeaderMap,
    household_id: i64,
) -> Result<AuthContext, ApiError> {
    let session = oauth::browser_session(state, db, headers)
        .await
        .map_err(|_| ApiError::internal())?
        .ok_or_else(ApiError::unauthorized)?;
    tenant_setting(db, "med_tracker.current_account_id", session.account_id)
        .await
        .map_err(database_error)?;
    let membership =
        current_household_membership(db, session.account_id, household_id, false).await?;
    let person = person::Entity::find()
        .filter(person::Column::AccountId.eq(session.account_id))
        .order_by_asc(person::Column::Id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    let user = user::Entity::find()
        .filter(user::Column::PersonId.eq(person.id))
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    if !user.active {
        return Err(ApiError::unauthorized());
    }
    Ok(AuthContext {
        account_id: session.account_id,
        user_id: user.id,
        credential_reference: oauth::session_key_digest(&session.session_id),
        credential_kind: CredentialKind::BrowserSession,
        membership,
    })
}

async fn authenticate_mobile_oauth(
    db: &DatabaseTransaction,
    token: &str,
    household_id: i64,
) -> Result<AuthContext, ApiError> {
    let digest = URL_SAFE.encode(Sha256::digest(token.as_bytes()));
    let grant = oauth_grant::Entity::find()
        .filter(oauth_grant::Column::TokenHash.eq(digest))
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    let now = Utc::now().naive_utc();
    if grant.client_kind != "mobile"
        || grant.revoked_at.is_some()
        || grant.expires_in < now
        || !grant
            .scopes
            .split_whitespace()
            .any(|scope| scope == "medtracker")
        || !within_login_lifetime(&grant, now)
    {
        return Err(ApiError::unauthorized());
    }
    let account = account::Entity::find_by_id(grant.account_id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    if account.status != 2 {
        return Err(ApiError::unauthorized());
    }
    let lockout = account_lockout::Entity::find_by_id(account.id)
        .one(db)
        .await
        .map_err(database_error)?;
    if lockout.is_some_and(|lockout| lockout.deadline > now) {
        return Err(ApiError::unauthorized());
    }
    tenant_setting(db, "med_tracker.current_account_id", account.id)
        .await
        .map_err(database_error)?;
    let mut active_grant: oauth_grant::ActiveModel = grant.clone().into();
    active_grant.last_used_at = Set(Some(now));
    active_grant.updated_at = Set(now);
    active_grant.update(db).await.map_err(database_error)?;
    let membership = current_household_membership(db, account.id, household_id, true).await?;
    let person = person::Entity::find()
        .filter(person::Column::AccountId.eq(account.id))
        .order_by_asc(person::Column::Id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    let user = user::Entity::find()
        .filter(user::Column::PersonId.eq(person.id))
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    if !user.active {
        return Err(ApiError::unauthorized());
    }
    Ok(AuthContext {
        account_id: account.id,
        user_id: user.id,
        credential_reference: grant.id.to_string(),
        credential_kind: CredentialKind::OauthGrant,
        membership,
    })
}
