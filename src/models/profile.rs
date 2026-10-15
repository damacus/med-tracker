pub mod avatar;
pub mod gravatar;
pub(crate) mod self_account;

use crate::models::{
    access::{self, TenantTransaction},
    care::{doses::CredentialProvenance, people},
    entities::{account, person, version},
    errors::OperationError,
};
use chrono::{NaiveDate, Utc};
use sea_orm::{ActiveModelTrait, EntityTrait, QuerySelect, Set};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashSet};

const SHORTCUTS: [&str; 9] = [
    "dashboard",
    "inventory",
    "locations",
    "people",
    "finder",
    "medicine_reviews",
    "reports",
    "profile",
    "administration",
];

#[derive(Clone, Serialize)]
pub struct Snapshot {
    pub person_id: i64,
    pub account_id: i64,
    pub date_of_birth: NaiveDate,
    pub time_zone: String,
    pub gravatar_enabled: bool,
    pub mobile_shortcuts: Vec<String>,
    pub avatar_attached: bool,
}

#[derive(Default)]
pub struct Changes {
    pub date_of_birth: Option<NaiveDate>,
    pub time_zone: Option<String>,
    pub gravatar_enabled: Option<bool>,
    pub mobile_shortcuts: Option<Vec<String>>,
}

pub fn supported_zones() -> Vec<(String, String)> {
    let retained: BTreeMap<String, String> =
        serde_json::from_str(include_str!("identity/rails_time_zones.json"))
            .expect("Retained Rails timezone mapping must be valid");
    let mut zones = retained.into_iter().collect::<Vec<_>>();
    for zone in ["UTC", "Europe/London"] {
        if !zones.iter().any(|(_, value)| value == zone) {
            zones.push((zone.to_owned(), zone.to_owned()));
        }
    }
    zones
}

fn valid_zone(value: &str) -> bool {
    value.is_empty()
        || value.parse::<chrono_tz::Tz>().is_ok()
        || supported_zones().iter().any(|(name, _)| name == value)
}

/// # Errors
///
/// Returns a validation error for an unsupported time zone or invalid mobile shortcuts.
pub fn validate(changes: &Changes) -> Result<(), OperationError> {
    if changes
        .time_zone
        .as_deref()
        .is_some_and(|zone| !valid_zone(zone))
    {
        return Err(validation("time_zone", "is invalid"));
    }
    if let Some(shortcuts) = &changes.mobile_shortcuts {
        let unique = shortcuts.iter().collect::<HashSet<_>>();
        if !(1..=3).contains(&shortcuts.len())
            || unique.len() != shortcuts.len()
            || shortcuts
                .iter()
                .any(|item| !SHORTCUTS.contains(&item.as_str()))
        {
            return Err(validation(
                "mobile_shortcuts",
                "must contain one to three unique shortcuts",
            ));
        }
    }
    Ok(())
}

/// # Errors
///
/// Returns a validation error when the date is not a valid `YYYY-MM-DD` calendar date.
pub fn parse_date(value: &str) -> Result<NaiveDate, OperationError> {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes
            .iter()
            .enumerate()
            .any(|(index, byte)| index != 4 && index != 7 && !byte.is_ascii_digit())
    {
        return Err(validation("date_of_birth", "is invalid"));
    }
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| validation("date_of_birth", "is invalid"))
}

pub fn validation(field: &str, message: &str) -> OperationError {
    OperationError::Validation {
        details: json!({field: [message]}),
    }
}

pub(crate) async fn linked_person(
    tenant: &TenantTransaction,
    account_id: i64,
) -> Result<person::Model, OperationError> {
    linked_person_scoped(tenant, account_id, OperationError::Forbidden).await
}

pub(crate) async fn linked_person_scoped(
    tenant: &TenantTransaction,
    account_id: i64,
    out_of_scope: OperationError,
) -> Result<person::Model, OperationError> {
    if account_id != tenant.scope().actor.account_id {
        return Err(out_of_scope);
    }
    let current = person::Entity::find_by_id(tenant.person_id().get())
        .one(tenant.transaction())
        .await?
        .ok_or_else(|| out_of_scope.clone())?;
    if access::participates(tenant, current.id).await? {
        Ok(current)
    } else {
        Err(out_of_scope)
    }
}

async fn snapshot(
    tenant: &TenantTransaction,
    current: &person::Model,
    account: &account::Model,
) -> Result<Snapshot, OperationError> {
    let preferences = &account.preferences;
    let avatar_attached = avatar::person_attachment(tenant, current.id)
        .await?
        .is_some();
    let time_zone = preferences["time_zone"]
        .as_str()
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| {
            crate::models::settings::text(tenant.settings(), &["time_zone"])
                .unwrap_or("UTC")
                .to_owned()
        });
    Ok(Snapshot {
        person_id: current.id,
        account_id: account.id,
        date_of_birth: current
            .date_of_birth
            .ok_or_else(|| validation("date_of_birth", "can't be blank"))?,
        time_zone,
        gravatar_enabled: gravatar::enabled(preferences),
        mobile_shortcuts: preferences["mobile_shortcuts"]
            .as_array()
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_else(|| vec!["dashboard".into(), "inventory".into(), "finder".into()]),
        avatar_attached,
    })
}

/// # Errors
///
/// Returns an error if account access is denied, the account is unavailable, or its profile or avatar cannot be read.
pub async fn read(tenant: &TenantTransaction, account_id: i64) -> Result<Snapshot, OperationError> {
    let current = linked_person_scoped(tenant, account_id, OperationError::NotFound).await?;
    let account = account::Entity::find_by_id(account_id)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    snapshot(tenant, &current, &account).await
}

/// # Errors
///
/// Returns an error if changes are invalid, account access fails, or the person, preferences, or resulting profile cannot be saved or read.
pub async fn update(
    tenant: &TenantTransaction,
    account_id: i64,
    changes: Changes,
    today: NaiveDate,
    provenance: Option<&CredentialProvenance>,
) -> Result<Snapshot, OperationError> {
    validate(&changes)?;
    let account = account::Entity::find_by_id(account_id)
        .lock(sea_orm::sea_query::LockType::NoKeyUpdate)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    let current = linked_person(tenant, account_id).await?;
    let person = if let Some(date) = changes.date_of_birth {
        people::update(
            tenant,
            &current.id.to_string(),
            json!({"date_of_birth": date.to_string()}),
            today,
            provenance,
        )
        .await?
    } else {
        current
    };
    let account = save_preferences(tenant, account, changes).await?;
    snapshot(tenant, &person, &account).await
}

/// # Errors
///
/// Returns an error for invalid or unsupported changes, denied account access, or a failed preference save.
pub async fn update_preferences(
    tenant: &TenantTransaction,
    account_id: i64,
    changes: Changes,
) -> Result<(), OperationError> {
    validate(&changes)?;
    if changes.date_of_birth.is_some() {
        return Err(validation("profile", "contains an unsupported field"));
    }
    linked_person(tenant, account_id).await?;
    let account = account::Entity::find_by_id(account_id)
        .lock(sea_orm::sea_query::LockType::NoKeyUpdate)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    save_preferences(tenant, account, changes).await?;
    Ok(())
}

/// # Errors
///
/// Returns an error if the name is blank, the account is unavailable, or the update or audit write fails.
pub async fn update_account_name(
    transaction: &sea_orm::DatabaseTransaction,
    account_id: crate::models::ids::AccountId,
    request_id: &str,
    name: &str,
) -> Result<(), OperationError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(validation("name", "can't be blank"));
    }
    let account = account::Entity::find_by_id(account_id.get())
        .lock(sea_orm::sea_query::LockType::NoKeyUpdate)
        .one(transaction)
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    if account.name.as_deref() == Some(name) {
        return Ok(());
    }
    let previous = account.name.clone();
    let person_id = crate::models::ids::PersonId::from_database(account.person_id);
    let mut active: account::ActiveModel = account.into();
    active.name = Set(Some(name.to_owned()));
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(transaction).await?;
    version::ActiveModel {
        item_type: Set("Account".into()), item_id: Set(account_id.get()), event: Set("update".into()),
        object: Set(Some(json!({"name":previous}).to_string())),
        object_changes: Set(Some(json!({"name":[previous,name]}).to_string())),
        whodunnit: Set(Some(person_id.get().to_string())), request_id: Set(Some(request_id.to_owned())),
        audit_context: Set(json!({"actor_account_id":account_id,"actor_person_id":person_id,"request_id":request_id,"policy_query":"update_own_profile"})),
        created_at: Set(Some(Utc::now().naive_utc())), ..Default::default()
    }.insert(transaction).await?;
    Ok(())
}

/// # Errors
///
/// Returns an error for invalid or unsupported changes, an unavailable account, or a failed preference or audit write.
pub async fn update_account_preferences(
    transaction: &sea_orm::DatabaseTransaction,
    account_id: crate::models::ids::AccountId,
    request_id: &str,
    changes: Changes,
) -> Result<(), OperationError> {
    validate(&changes)?;
    if changes.date_of_birth.is_some() {
        return Err(validation("profile", "contains an unsupported field"));
    }
    let account = account::Entity::find_by_id(account_id.get())
        .lock(sea_orm::sea_query::LockType::NoKeyUpdate)
        .one(transaction)
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    let context = json!({"actor_account_id":account_id,"actor_person_id":account.person_id,"request_id":request_id,"policy_query":"update_own_profile"});
    persist_preferences(
        transaction,
        account,
        changes,
        request_id,
        context,
        None,
        None,
    )
    .await?;
    Ok(())
}

async fn save_preferences(
    tenant: &TenantTransaction,
    account: account::Model,
    changes: Changes,
) -> Result<account::Model, OperationError> {
    persist_preferences(
        tenant.transaction(),
        account,
        changes,
        &tenant.scope().request_id,
        crate::models::audit_context::household(tenant, None),
        Some(crate::models::ids::HouseholdId::from_database(
            tenant.scope().household_id,
        )),
        Some(tenant.membership().id),
    )
    .await
}

async fn persist_preferences(
    transaction: &sea_orm::DatabaseTransaction,
    mut account: account::Model,
    changes: Changes,
    request_id: &str,
    context: Value,
    household_id: Option<crate::models::ids::HouseholdId>,
    membership_id: Option<i64>,
) -> Result<account::Model, OperationError> {
    let mut preferences = account.preferences.as_object().cloned().unwrap_or_default();
    if let Some(zone) = changes.time_zone {
        preferences.insert("time_zone".into(), json!(zone));
    }
    if let Some(enabled) = changes.gravatar_enabled {
        preferences.insert("gravatar_enabled".into(), json!(enabled));
    }
    if let Some(shortcuts) = changes.mobile_shortcuts {
        preferences.insert("mobile_shortcuts".into(), json!(shortcuts));
    }
    if Value::Object(preferences.clone()) != account.preferences {
        let before = account.preferences.clone();
        let mut active: account::ActiveModel = account.into();
        active.preferences = Set(Value::Object(preferences));
        active.updated_at = Set(Utc::now().naive_utc());
        account = active.update(transaction).await?;
        version::ActiveModel {
            item_type: Set("Account".into()),
            item_id: Set(account.id),
            event: Set("update".into()),
            object: Set(Some(json!({"preferences": before.clone()}).to_string())),
            object_changes: Set(Some(
                json!({"preferences": [before, account.preferences.clone()]}).to_string(),
            )),
            whodunnit: Set(Some(account.person_id.to_string())),
            request_id: Set(Some(request_id.to_owned())),
            household_id: Set(household_id.map(crate::models::ids::HouseholdId::get)),
            actor_membership_id: Set(membership_id),
            audit_context: Set(context),
            created_at: Set(Some(Utc::now().naive_utc())),
            ..Default::default()
        }
        .insert(transaction)
        .await?;
    }
    Ok(account)
}
