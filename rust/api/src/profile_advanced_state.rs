use crate::entities::{account, membership};
use crate::{restricted_role, tenant_setting, AppState};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter,
    QuerySelect, Set, Statement, TransactionTrait,
};
use serde_json::{json, Value};

pub(crate) const WIZARDS: &[&str] = &["fullpage", "modal", "slideover"];
pub(crate) const DASHBOARDS: &[&str] = &["current", "time_first", "family_lanes", "calm_focus"];
pub(crate) const LAUNCHERS: &[&str] = &["current", "context_aware"];

pub(crate) fn choice(key: &str, value: &str) -> bool {
    match key {
        "wizard_variant" => WIZARDS.contains(&value),
        "dashboard_variant" => DASHBOARDS.contains(&value),
        "medication_launcher_variant" => LAUNCHERS.contains(&value),
        _ => false,
    }
}

pub(crate) async fn membership_id(
    state: &AppState,
    account_id: i64,
    household_id: i64,
) -> Result<Option<i64>, sea_orm::DbErr> {
    let db = state.db.begin().await?;
    restricted_role(&db).await?;
    tenant_setting(&db, "med_tracker.current_account_id", account_id).await?;
    tenant_setting(&db, "med_tracker.current_household_id", household_id).await?;
    let row = membership::Entity::find()
        .filter(membership::Column::AccountId.eq(account_id))
        .filter(membership::Column::HouseholdId.eq(household_id))
        .filter(membership::Column::Status.eq("active"))
        .filter(membership::Column::RevokedAt.is_null())
        .one(&db)
        .await?;
    db.commit().await?;
    Ok(row.map(|row| row.id))
}

pub(crate) async fn preferences(
    state: &AppState,
    account_id: i64,
) -> Result<Value, sea_orm::DbErr> {
    let db = state.db.begin().await?;
    restricted_role(&db).await?;
    tenant_setting(&db, "med_tracker.current_account_id", account_id).await?;
    let row = account::Entity::find_by_id(account_id).one(&db).await?;
    db.commit().await?;
    Ok(row.map_or_else(|| json!({}), |row| row.preferences))
}

pub(crate) async fn save_choice(
    state: &AppState,
    account_id: i64,
    key: &str,
    value: &str,
) -> Result<bool, sea_orm::DbErr> {
    if !choice(key, value) {
        return Ok(false);
    }
    let db = state.db.begin().await?;
    restricted_role(&db).await?;
    tenant_setting(&db, "med_tracker.current_account_id", account_id).await?;
    let Some(row) = account::Entity::find_by_id(account_id)
        .lock_exclusive()
        .one(&db)
        .await?
    else {
        return Ok(false);
    };
    if row.status != 2 {
        return Ok(false);
    }
    let mut preferences = row.preferences.clone();
    if !preferences.is_object() {
        preferences = json!({});
    }
    preferences[key] = json!(value);
    let mut active: account::ActiveModel = row.into();
    active.preferences = Set(preferences);
    active.updated_at = Set(chrono::Utc::now().naive_utc());
    active.update(&db).await?;
    db.commit().await?;
    Ok(true)
}

pub(crate) async fn close_account(
    state: &AppState,
    account_id: i64,
    password: String,
) -> Result<Option<bool>, sea_orm::DbErr> {
    let db = state.db.begin().await?;
    restricted_role(&db).await?;
    tenant_setting(&db, "med_tracker.current_account_id", account_id).await?;
    let Some(row) = account::Entity::find_by_id(account_id)
        .lock_exclusive()
        .one(&db)
        .await?
    else {
        return Ok(None);
    };
    if row.status != 2 {
        return Ok(None);
    }
    let Some(hash) = row.password_hash.as_ref() else {
        return Ok(Some(false));
    };
    if !crate::web_pages::verify_password_hash_for_close(password, hash.clone())
        .await
        .map_err(|_| sea_orm::DbErr::Custom("password verification failed".to_owned()))?
    {
        return Ok(Some(false));
    }
    let now = chrono::Utc::now().naive_utc();
    let mut active: account::ActiveModel = row.into();
    active.status = Set(3);
    active.updated_at = Set(now);
    active.update(&db).await?;
    for table in ["api_app_tokens", "api_sessions", "oauth_grants"] {
        let query = format!("UPDATE {table} SET revoked_at = $2, updated_at = $2 WHERE account_id = $1 AND revoked_at IS NULL");
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            query,
            [account_id.into(), now.into()],
        ))
        .await?;
    }
    for table in [
        "account_active_session_keys",
        "native_device_tokens",
        "push_subscriptions",
    ] {
        let query = format!("DELETE FROM {table} WHERE account_id = $1");
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            query,
            [account_id.into()],
        ))
        .await?;
    }
    db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE users SET active = false, updated_at = $2 WHERE person_id IN (SELECT id FROM people WHERE account_id = $1)",
        [account_id.into(), now.into()])).await?;
    let households = db
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT DISTINCT household_id FROM people WHERE account_id = $1",
            [account_id.into()],
        ))
        .await?;
    for household in households {
        let household_id: i64 = household.try_get("", "household_id")?;
        tenant_setting(&db, "med_tracker.current_household_id", household_id).await?;
        db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE people SET account_id = NULL, updated_at = $2 WHERE account_id = $1 AND household_id = $3",
            [account_id.into(), now.into(), household_id.into()])).await?;
    }
    db.commit().await?;
    Ok(Some(true))
}
