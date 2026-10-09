use crate::models::{
    access::{self, PersonAccess, TenantTransaction},
    care::{doses::CredentialProvenance, people},
    entities::{account, active_storage_attachment, person, version},
    errors::OperationError,
};
use chrono::{NaiveDate, Utc};
use chrono_tz::Tz;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set};
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
    access_level: PersonAccess,
) -> Result<person::Model, OperationError> {
    let person_id = tenant
        .membership()
        .person_id
        .ok_or(OperationError::Forbidden)?;
    let current = person::Entity::find_by_id(person_id)
        .filter(person::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(person::Column::AccountId.eq(account_id))
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Forbidden)?;
    access::require_person_access(tenant, current.id, access_level).await?;
    Ok(current)
}

async fn snapshot(
    tenant: &TenantTransaction,
    current: &person::Model,
    account: &account::Model,
) -> Result<Snapshot, OperationError> {
    let preferences = &account.preferences;
    let avatar_attached = active_storage_attachment::Entity::find()
        .filter(active_storage_attachment::Column::HouseholdId.eq(current.household_id))
        .filter(active_storage_attachment::Column::RecordType.eq("Person"))
        .filter(active_storage_attachment::Column::RecordId.eq(current.id))
        .filter(active_storage_attachment::Column::Name.eq("avatar"))
        .one(tenant.transaction())
        .await?
        .is_some();
    let time_zone = preferences["time_zone"]
        .as_str()
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| std::env::var("TZ").unwrap_or_else(|_| "UTC".into()));
    Ok(Snapshot {
        person_id: current.id,
        account_id: account.id,
        date_of_birth: current
            .date_of_birth
            .ok_or_else(|| validation("date_of_birth", "can't be blank"))?,
        time_zone,
        gravatar_enabled: preferences["gravatar_enabled"]
            .as_bool()
            .unwrap_or_else(|| {
                preferences["gravatar_enabled"]
                    .as_str()
                    .is_some_and(|value| matches!(value, "true" | "1"))
            }),
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

pub async fn read(tenant: &TenantTransaction, account_id: i64) -> Result<Snapshot, OperationError> {
    let current = linked_person(tenant, account_id, PersonAccess::View).await?;
    let account = account::Entity::find_by_id(account_id)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    snapshot(tenant, &current, &account).await
}

pub async fn update(
    tenant: &TenantTransaction,
    account_id: i64,
    changes: Changes,
    zone: Tz,
    provenance: Option<&CredentialProvenance>,
) -> Result<Snapshot, OperationError> {
    validate(&changes)?;
    let account = account::Entity::find_by_id(account_id)
        .lock(sea_orm::sea_query::LockType::NoKeyUpdate)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    let current = linked_person(tenant, account_id, PersonAccess::Manage).await?;
    let person = if let Some(date) = changes.date_of_birth {
        people::update(
            tenant,
            &current.id.to_string(),
            json!({"date_of_birth": date.to_string()}),
            zone,
            provenance,
        )
        .await?
    } else {
        current
    };
    let mut account = account;
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
        account = active.update(tenant.transaction()).await?;
        version::ActiveModel {
            item_type: Set("Account".into()),
            item_id: Set(account.id),
            event: Set("update".into()),
            object: Set(Some(json!({"preferences": before.clone()}).to_string())),
            object_changes: Set(Some(json!({"preferences": [before, account.preferences.clone()]}).to_string())),
            whodunnit: Set(Some(tenant.user_id().to_string())),
            request_id: Set(Some(tenant.scope().request_id.clone())),
            household_id: Set(Some(tenant.scope().household_id)),
            actor_membership_id: Set(Some(tenant.membership().id)),
            audit_context: Set(json!({"policy_class":"PersonPolicy","policy_query":"update?","actor_account_id":account_id,"household_id":tenant.scope().household_id,"request_id":tenant.scope().request_id})),
            created_at: Set(Some(Utc::now().naive_utc())),
            ..Default::default()
        }.insert(tenant.transaction()).await?;
    }
    snapshot(tenant, &person, &account).await
}
