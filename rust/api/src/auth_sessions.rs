use crate::entities::{
    account, account_lockout, api_app_token, api_session, household, membership, oauth_grant,
    person, security_audit_event, user, version,
};
use crate::{
    configured_lifetime_days, database_error, restricted_role, tenant_setting,
    within_login_lifetime, ApiError, AppState,
};
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get};
use axum::{Json, Router};
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use chrono::{Duration, Months, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

enum Credential {
    Session(api_session::Model),
    App(api_app_token::Model),
    Mobile(oauth_grant::Model),
    Integration(oauth_grant::Model),
}

impl Credential {
    fn account_id(&self) -> i64 {
        match self {
            Self::Session(row) => row.account_id,
            Self::App(row) => row.account_id,
            Self::Mobile(row) | Self::Integration(row) => row.account_id,
        }
    }

    fn membership_id(&self) -> Option<i64> {
        match self {
            Self::Session(row) => row.household_membership_id,
            Self::App(row) => Some(row.household_membership_id),
            Self::Mobile(_) | Self::Integration(_) => None,
        }
    }
}

struct Authenticated {
    credential: Credential,
    user_id: i64,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/sessions", get(index))
        .route("/api/v1/auth/sessions/{id}", delete(revoke))
        .route("/api/v1/auth/logout", delete(logout))
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .filter(|token| !token.is_empty())
}

fn app_age_months() -> Option<u32> {
    match std::env::var("API_APP_TOKEN_MAX_AGE_MONTHS") {
        Ok(value) => value.parse().ok().filter(|months| *months > 0),
        Err(std::env::VarError::NotPresent) => Some(12),
        Err(std::env::VarError::NotUnicode(_)) => None,
    }
}

pub(super) fn app_unexpired(row: &api_app_token::Model, now: chrono::NaiveDateTime) -> bool {
    app_age_months().is_some_and(|months| app_unexpired_for_months(row, now, months))
}

fn app_unexpired_for_months(
    row: &api_app_token::Model,
    now: chrono::NaiveDateTime,
    months: u32,
) -> bool {
    row.created_at
        .checked_add_months(Months::new(months))
        .is_some_and(|maximum| row.expires_at > now && maximum > now)
}

async fn lookup(
    db: &DatabaseTransaction,
    token: &str,
    now: chrono::NaiveDateTime,
) -> Result<Option<Credential>, ApiError> {
    let hex_digest = format!("{:x}", Sha256::digest(token.as_bytes()));
    if let Some(row) = api_session::Entity::find()
        .filter(api_session::Column::AccessTokenDigest.eq(&hex_digest))
        .filter(api_session::Column::RevokedAt.is_null())
        .one(db)
        .await
        .map_err(database_error)?
    {
        return Ok(Some(Credential::Session(row)));
    }
    if let Some(row) = api_app_token::Entity::find()
        .filter(api_app_token::Column::TokenDigest.eq(hex_digest))
        .filter(api_app_token::Column::RevokedAt.is_null())
        .one(db)
        .await
        .map_err(database_error)?
    {
        return Ok(app_unexpired(&row, now).then_some(Credential::App(row)));
    }
    let digest = URL_SAFE.encode(Sha256::digest(token.as_bytes()));
    let grant = oauth_grant::Entity::find()
        .filter(oauth_grant::Column::TokenHash.eq(digest))
        .filter(oauth_grant::Column::RevokedAt.is_null())
        .filter(oauth_grant::Column::ExpiresIn.gt(now))
        .one(db)
        .await
        .map_err(database_error)?;
    Ok(grant.map(|row| {
        if row.client_kind == "mobile" {
            Credential::Mobile(row)
        } else {
            Credential::Integration(row)
        }
    }))
}

async fn authenticate(
    db: &DatabaseTransaction,
    headers: &HeaderMap,
) -> Result<Authenticated, ApiError> {
    let token = bearer(headers).ok_or_else(ApiError::unauthorized)?;
    let now = Utc::now().naive_utc();
    let credential = lookup(db, token, now)
        .await?
        .ok_or_else(ApiError::unauthorized)?;
    match &credential {
        Credential::Session(row) if row.access_expires_at <= now => {
            return Err(ApiError::unauthorized());
        }
        Credential::Session(row) if row.household_membership_id.is_none() => {
            return Err(ApiError::unauthorized());
        }
        Credential::Mobile(row)
            if !row
                .scopes
                .split_whitespace()
                .any(|scope| scope == "medtracker")
                || !within_login_lifetime(row, now) =>
        {
            return Err(ApiError::unauthorized());
        }
        Credential::Integration(_) => return Err(ApiError::unauthorized()),
        _ => {}
    }
    let account_id = credential.account_id();
    let account = account::Entity::find_by_id(account_id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    if account.status != 2 {
        return Err(ApiError::unauthorized());
    }
    let lockout = account_lockout::Entity::find_by_id(account_id)
        .one(db)
        .await
        .map_err(database_error)?;
    if lockout.is_some_and(|lockout| lockout.deadline > now) {
        return Err(ApiError::unauthorized());
    }
    tenant_setting(db, "med_tracker.current_account_id", account_id)
        .await
        .map_err(database_error)?;
    let linked = person::Entity::find()
        .filter(person::Column::AccountId.eq(account_id))
        .order_by_asc(person::Column::Id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    let actor = user::Entity::find()
        .filter(user::Column::PersonId.eq(linked.id))
        .filter(user::Column::Active.eq(true))
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    if let Some(id) = credential.membership_id() {
        let member = membership::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(database_error)?
            .ok_or_else(ApiError::unauthorized)?;
        let version = match &credential {
            Credential::Session(row) => row.permissions_version,
            Credential::App(row) => row.permissions_version,
            Credential::Mobile(_) | Credential::Integration(_) => unreachable!(),
        };
        if member.account_id != account_id
            || member.status != "active"
            || member.revoked_at.is_some()
            || member.permissions_version != version
        {
            return Err(ApiError::unauthorized());
        }
        let home = household::Entity::find_by_id(member.household_id)
            .one(db)
            .await
            .map_err(database_error)?
            .ok_or_else(ApiError::unauthorized)?;
        if home.status != "active" || home.lifecycle_state != "active" {
            return Err(ApiError::unauthorized());
        }
    }
    let authenticated = Authenticated {
        credential,
        user_id: actor.id,
    };
    touch(db, &authenticated, now).await?;
    Ok(authenticated)
}

async fn touch(
    db: &DatabaseTransaction,
    actor: &Authenticated,
    now: chrono::NaiveDateTime,
) -> Result<(), ApiError> {
    match &actor.credential {
        Credential::Session(row) => {
            let mut active: api_session::ActiveModel = row.clone().into();
            active.last_used_at = Set(now);
            active.updated_at = Set(now);
            active.update(db).await.map_err(database_error)?;
        }
        Credential::App(row) if row.last_used_at < now - Duration::minutes(5) => {
            let mut active: api_app_token::ActiveModel = row.clone().into();
            active.last_used_at = Set(now);
            active.updated_at = Set(now);
            active.update(db).await.map_err(database_error)?;
        }
        Credential::Mobile(row) => {
            let mut active: oauth_grant::ActiveModel = row.clone().into();
            active.last_used_at = Set(Some(now));
            active.updated_at = Set(now);
            active.update(db).await.map_err(database_error)?;
        }
        Credential::App(_) | Credential::Integration(_) => {}
    }
    Ok(())
}

fn timestamp(value: chrono::NaiveDateTime) -> String {
    value.and_utc().to_rfc3339()
}

fn mobile_refresh_expiry(row: &oauth_grant::Model) -> Option<chrono::NaiveDateTime> {
    let last = row.last_used_at?;
    let authenticated = row.authenticated_at?;
    let inactivity = configured_lifetime_days("SESSION_INACTIVITY_TIMEOUT_DAYS", 30, 1)?;
    let maximum = configured_lifetime_days("SESSION_MAX_AGE_DAYS", 0, 0)?;
    let mut deadline = last.checked_add_signed(Duration::try_days(inactivity)?)?;
    if maximum > 0 {
        let absolute = authenticated.checked_add_signed(Duration::try_days(maximum)?)?;
        deadline = deadline.min(absolute);
    }
    Some(deadline)
}

async fn index(State(state): State<AppState>, headers: HeaderMap) -> Response {
    match index_inner(&state, &headers).await {
        Ok(response) => response,
        Err(error) => error.into_response(),
    }
}

async fn index_inner(state: &AppState, headers: &HeaderMap) -> Result<Response, ApiError> {
    let db = state.db.begin().await.map_err(database_error)?;
    restricted_role(&db).await.map_err(database_error)?;
    let actor = authenticate(&db, headers).await?;
    let account_id = actor.credential.account_id();
    let data = if matches!(actor.credential, Credential::Mobile(_)) {
        let now = Utc::now().naive_utc();
        let inactivity = configured_lifetime_days("SESSION_INACTIVITY_TIMEOUT_DAYS", 30, 1)
            .and_then(Duration::try_days)
            .ok_or_else(ApiError::internal)?;
        let inactivity_cutoff = now
            .checked_sub_signed(inactivity)
            .ok_or_else(ApiError::internal)?;
        let mut query = oauth_grant::Entity::find()
            .filter(oauth_grant::Column::AccountId.eq(account_id))
            .filter(oauth_grant::Column::ClientKind.eq("mobile"))
            .filter(oauth_grant::Column::RevokedAt.is_null())
            .filter(oauth_grant::Column::TokenHash.is_not_null())
            .filter(oauth_grant::Column::LastUsedAt.gt(inactivity_cutoff));
        let maximum = configured_lifetime_days("SESSION_MAX_AGE_DAYS", 0, 0)
            .ok_or_else(ApiError::internal)?;
        if maximum > 0 {
            let maximum_cutoff = Duration::try_days(maximum)
                .and_then(|period| now.checked_sub_signed(period))
                .ok_or_else(ApiError::internal)?;
            query = query.filter(oauth_grant::Column::AuthenticatedAt.gt(maximum_cutoff));
        }
        let rows = query
            .order_by_desc(oauth_grant::Column::CreatedAt)
            .all(&db)
            .await
            .map_err(database_error)?;
        rows.into_iter()
            .map(|row| {
                let refresh = mobile_refresh_expiry(&row).ok_or_else(ApiError::internal)?;
                Ok(json!({
                    "id": row.id,
                    "device_name": row.device_name,
                    "last_used_at": timestamp(row.last_used_at.ok_or_else(ApiError::internal)?),
                    "access_token_expires_at": timestamp(row.expires_in),
                    "refresh_token_expires_at": timestamp(refresh),
                    "created_at": timestamp(row.created_at)
                }))
            })
            .collect::<Result<Vec<Value>, ApiError>>()?
    } else {
        let rows = api_session::Entity::find()
            .filter(api_session::Column::AccountId.eq(account_id))
            .filter(api_session::Column::RevokedAt.is_null())
            .order_by_desc(api_session::Column::CreatedAt)
            .all(&db)
            .await
            .map_err(database_error)?;
        let member_ids: Vec<i64> = rows
            .iter()
            .filter_map(|row| row.household_membership_id)
            .collect();
        let households: HashMap<i64, i64> = membership::Entity::find()
            .filter(membership::Column::Id.is_in(member_ids))
            .all(&db)
            .await
            .map_err(database_error)?
            .into_iter()
            .map(|member| (member.id, member.household_id))
            .collect();
        rows.into_iter()
            .map(|row| {
                json!({
                    "id": row.id,
                    "device_name": row.device_name,
                    "household_id": row.household_membership_id.and_then(|id| households.get(&id).copied()),
                    "last_used_at": timestamp(row.last_used_at),
                    "access_token_expires_at": timestamp(row.access_expires_at),
                    "refresh_token_expires_at": timestamp(row.refresh_expires_at),
                    "created_at": timestamp(row.created_at)
                })
            })
            .collect()
    };
    db.commit().await.map_err(database_error)?;
    Ok(Json(json!({"data": data})).into_response())
}

fn numeric_id(value: &str) -> Option<i64> {
    (!value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| value.parse::<i64>().ok().filter(|id| *id > 0))
        .flatten()
}

async fn revoke(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    match revoke_inner(&state, &headers, &id).await {
        Ok(response) => response,
        Err(error) => error.into_response(),
    }
}

async fn revoke_inner(
    state: &AppState,
    headers: &HeaderMap,
    id: &str,
) -> Result<Response, ApiError> {
    let db = state.db.begin().await.map_err(database_error)?;
    restricted_role(&db).await.map_err(database_error)?;
    let actor = authenticate(&db, headers).await?;
    let id = numeric_id(id).ok_or_else(ApiError::not_found)?;
    let account_id = actor.credential.account_id();
    if matches!(actor.credential, Credential::Mobile(_)) {
        let row = oauth_grant::Entity::find_by_id(id)
            .filter(oauth_grant::Column::AccountId.eq(account_id))
            .filter(oauth_grant::Column::ClientKind.eq("mobile"))
            .filter(oauth_grant::Column::RevokedAt.is_null())
            .filter(oauth_grant::Column::TokenHash.is_not_null())
            .lock_exclusive()
            .one(&db)
            .await
            .map_err(database_error)?
            .ok_or_else(ApiError::not_found)?;
        let now = Utc::now().naive_utc();
        if !within_login_lifetime(&row, now) {
            return Err(ApiError::not_found());
        }
        revoke_oauth(&db, row, actor.user_id, now).await?;
    } else {
        let row = api_session::Entity::find_by_id(id)
            .filter(api_session::Column::AccountId.eq(account_id))
            .filter(api_session::Column::RevokedAt.is_null())
            .lock_exclusive()
            .one(&db)
            .await
            .map_err(database_error)?
            .ok_or_else(ApiError::not_found)?;
        let target_member = if let Some(member_id) = row.household_membership_id {
            membership::Entity::find_by_id(member_id)
                .one(&db)
                .await
                .map_err(database_error)?
        } else {
            None
        };
        revoke_session(&db, row, actor.user_id, target_member.as_ref()).await?;
    }
    db.commit().await.map_err(database_error)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    match logout_inner(&state, &headers).await {
        Ok(response) => response,
        Err(error) => error.into_response(),
    }
}

async fn logout_inner(state: &AppState, headers: &HeaderMap) -> Result<Response, ApiError> {
    let Some(token) = bearer(headers) else {
        return Ok(StatusCode::NO_CONTENT.into_response());
    };
    let db = state.db.begin().await.map_err(database_error)?;
    restricted_role(&db).await.map_err(database_error)?;
    let now = Utc::now().naive_utc();
    let Some(credential) = lookup(&db, token, now).await? else {
        return Ok(StatusCode::NO_CONTENT.into_response());
    };
    tenant_setting(
        &db,
        "med_tracker.current_account_id",
        credential.account_id(),
    )
    .await
    .map_err(database_error)?;
    let actor = person::Entity::find()
        .filter(person::Column::AccountId.eq(credential.account_id()))
        .order_by_asc(person::Column::Id)
        .one(&db)
        .await
        .map_err(database_error)?;
    let user_id = if let Some(person) = actor {
        user::Entity::find()
            .filter(user::Column::PersonId.eq(person.id))
            .one(&db)
            .await
            .map_err(database_error)?
            .map(|user| user.id)
    } else {
        None
    };
    match credential {
        Credential::Session(row) => {
            let current = api_session::Entity::find_by_id(row.id)
                .filter(api_session::Column::RevokedAt.is_null())
                .lock_exclusive()
                .one(&db)
                .await
                .map_err(database_error)?;
            if let Some(row) = current {
                let member = if let Some(id) = row.household_membership_id {
                    membership::Entity::find_by_id(id)
                        .one(&db)
                        .await
                        .map_err(database_error)?
                } else {
                    None
                };
                revoke_session(&db, row, user_id.unwrap_or(0), member.as_ref()).await?;
            }
        }
        Credential::App(row) => {
            let current = api_app_token::Entity::find_by_id(row.id)
                .filter(api_app_token::Column::RevokedAt.is_null())
                .lock_exclusive()
                .one(&db)
                .await
                .map_err(database_error)?;
            if let Some(row) = current {
                let member = membership::Entity::find_by_id(row.household_membership_id)
                    .one(&db)
                    .await
                    .map_err(database_error)?;
                revoke_app(&db, row, user_id, member.as_ref()).await?;
            }
        }
        Credential::Mobile(row) | Credential::Integration(row) => {
            let current = oauth_grant::Entity::find_by_id(row.id)
                .filter(oauth_grant::Column::RevokedAt.is_null())
                .lock_exclusive()
                .one(&db)
                .await
                .map_err(database_error)?;
            if let Some(row) = current {
                revoke_oauth(&db, row, user_id.unwrap_or(0), now).await?;
            }
        }
    }
    db.commit().await.map_err(database_error)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn revoke_session(
    db: &DatabaseTransaction,
    row: api_session::Model,
    user_id: i64,
    membership: Option<&membership::Model>,
) -> Result<(), ApiError> {
    let now = Utc::now().naive_utc();
    let mut active: api_session::ActiveModel = row.clone().into();
    active.revoked_at = Set(Some(now));
    active.updated_at = Set(now);
    active.update(db).await.map_err(database_error)?;
    record_token_audit(
        db,
        row.account_id,
        user_id,
        membership,
        "api_session",
        json!({
            "device_name_present": row.device_name.as_ref().is_some_and(|name| !name.is_empty()),
            "device_name_length": row.device_name.as_ref().filter(|name| !name.is_empty()).map(|name| name.chars().count()),
            "user_agent_hash": row.user_agent.as_ref().filter(|agent| !agent.is_empty()).map(|agent| format!("{:x}", Sha256::digest(agent.as_bytes()))),
            "household_membership_id": row.household_membership_id,
            "permissions_version": row.permissions_version,
            "expires_at": timestamp(row.refresh_expires_at)
        }),
    )
    .await
}

async fn revoke_app(
    db: &DatabaseTransaction,
    row: api_app_token::Model,
    user_id: Option<i64>,
    membership: Option<&membership::Model>,
) -> Result<(), ApiError> {
    let now = Utc::now().naive_utc();
    let mut active: api_app_token::ActiveModel = row.clone().into();
    active.revoked_at = Set(Some(now));
    active.updated_at = Set(now);
    active.update(db).await.map_err(database_error)?;
    record_token_audit(
        db,
        row.account_id,
        user_id.unwrap_or(0),
        membership,
        "api_app_token",
        json!({
            "device_name_present": !row.name.is_empty(),
            "device_name_length": (!row.name.is_empty()).then(|| row.name.chars().count()),
            "household_membership_id": row.household_membership_id,
            "permissions_version": row.permissions_version,
            "expires_at": timestamp(row.expires_at)
        }),
    )
    .await
}

async fn record_token_audit(
    db: &DatabaseTransaction,
    account_id: i64,
    user_id: i64,
    membership: Option<&membership::Model>,
    token_type: &str,
    metadata: Value,
) -> Result<(), ApiError> {
    let now = Utc::now().naive_utc();
    let event = format!("auth_token/{token_type}/revoked");
    let household_id = membership.map(|member| member.household_id);
    let member_id = membership.map(|member| member.id);
    let mut object =
        json!({"account_id": account_id, "token_type": token_type, "action": "revoked"});
    if let Some(fields) = metadata.as_object() {
        for (key, value) in fields {
            if !value.is_null() {
                object[key] = value.clone();
            }
        }
    }
    let context = json!({"actor_account_id": account_id, "actor_user_id": user_id, "actor_membership_id": member_id, "household_id": household_id});
    if let Some(household_id) = household_id {
        tenant_setting(db, "med_tracker.current_household_id", household_id)
            .await
            .map_err(database_error)?;
    }
    version::ActiveModel {
        item_type: Set("AuthenticationToken".to_owned()),
        item_id: Set(account_id),
        event: Set(event.clone()),
        object: Set(Some(object.to_string())),
        whodunnit: Set((user_id > 0).then(|| user_id.to_string())),
        household_id: Set(household_id),
        actor_membership_id: Set(member_id),
        audit_context: Set(context.clone()),
        created_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(database_error)?;
    if let Some(household_id) = household_id {
        security_audit_event::ActiveModel {
            household_id: Set(household_id),
            actor_account_id: Set(Some(account_id)),
            actor_membership_id: Set(member_id),
            event_type: Set(event),
            metadata: Set(object),
            audit_context: Set(context),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(database_error)?;
    }
    Ok(())
}

async fn revoke_oauth(
    db: &DatabaseTransaction,
    row: oauth_grant::Model,
    user_id: i64,
    now: chrono::NaiveDateTime,
) -> Result<(), ApiError> {
    let mut active: oauth_grant::ActiveModel = row.clone().into();
    active.revoked_at = Set(Some(now));
    active.updated_at = Set(now);
    active.update(db).await.map_err(database_error)?;
    version::ActiveModel {
        item_type: Set("OauthGrant".to_owned()),
        item_id: Set(row.id),
        event: Set("mobile_oauth.revoked".to_owned()),
        object: Set(Some(
            json!({"account_id": row.account_id, "oauth_application_id": row.oauth_application_id})
                .to_string(),
        )),
        whodunnit: Set((user_id > 0).then(|| user_id.to_string())),
        audit_context: Set(json!({"actor_account_id": row.account_id, "actor_user_id": user_id})),
        created_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(database_error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn app_token_cap_uses_calendar_month_end() {
        let created = NaiveDate::from_ymd_opt(2024, 1, 31)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let row = api_app_token::Model {
            id: 1,
            account_id: 1,
            household_membership_id: 1,
            token_digest: "digest".to_owned(),
            permissions_version: 1,
            name: "device".to_owned(),
            expires_at: NaiveDate::from_ymd_opt(2024, 3, 1)
                .unwrap()
                .and_hms_opt(12, 0, 0)
                .unwrap(),
            revoked_at: None,
            last_used_at: created,
            created_at: created,
            updated_at: created,
        };
        let before = NaiveDate::from_ymd_opt(2024, 2, 29)
            .unwrap()
            .and_hms_opt(11, 59, 59)
            .unwrap();
        let after = NaiveDate::from_ymd_opt(2024, 2, 29)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert!(app_unexpired_for_months(&row, before, 1));
        assert!(!app_unexpired_for_months(&row, after, 1));
    }
}
