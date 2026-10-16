use crate::models::{
    entities::{account, version},
    errors::OperationError,
    identity::time_zone,
    ids::AccountId,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ConnectionTrait, DatabaseTransaction, DbBackend, EntityTrait, Set, Statement,
};
use serde::Serialize;
use serde_json::json;

pub(crate) struct Snapshot {
    pub account_id: AccountId,
    pub name: Option<String>,
    pub email: String,
    pub time_zone: chrono_tz::Tz,
    pub gravatar_enabled: bool,
    pub avatar_attached: bool,
    pub mobile_shortcuts: Vec<Shortcut>,
}

#[derive(Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Shortcut {
    Dashboard,
    Inventory,
    Locations,
    People,
    Finder,
    MedicineReviews,
    Reports,
    Profile,
    Administration,
}

impl Shortcut {
    fn parse(value: &str) -> Result<Self, OperationError> {
        match value {
            "dashboard" => Ok(Self::Dashboard),
            "inventory" => Ok(Self::Inventory),
            "locations" => Ok(Self::Locations),
            "people" => Ok(Self::People),
            "finder" => Ok(Self::Finder),
            "medicine_reviews" => Ok(Self::MedicineReviews),
            "reports" => Ok(Self::Reports),
            "profile" => Ok(Self::Profile),
            "administration" => Ok(Self::Administration),
            _ => Err(super::validation(
                "mobile_shortcuts",
                "contains an unknown shortcut",
            )),
        }
    }
}

enum TimeZone {
    Inherit,
    Named(chrono_tz::Tz),
}

pub(crate) struct Changes {
    name: Option<Name>,
    time_zone: Option<TimeZone>,
    gravatar_enabled: Option<bool>,
    mobile_shortcuts: Option<Vec<Shortcut>>,
}

impl Changes {
    pub(crate) fn parse(
        name: Option<&str>,
        zone: Option<&str>,
        gravatar_enabled: Option<bool>,
        shortcuts: Option<&[String]>,
    ) -> Result<Self, OperationError> {
        let name = name.map(Name::parse).transpose()?;
        let time_zone = zone
            .map(|zone| {
                if zone.is_empty() {
                    Ok(TimeZone::Inherit)
                } else {
                    time_zone::parse(zone)
                        .map(TimeZone::Named)
                        .map_err(|_| super::validation("time_zone", "is invalid"))
                }
            })
            .transpose()?;
        let mobile_shortcuts = shortcuts
            .map(|values| {
                let mut selected = Vec::with_capacity(values.len().min(3));
                if !(1..=3).contains(&values.len()) {
                    return Err(super::validation(
                        "mobile_shortcuts",
                        "must contain one to three unique shortcuts",
                    ));
                }
                for value in values {
                    let shortcut = Shortcut::parse(value)?;
                    if selected.contains(&shortcut) {
                        return Err(super::validation(
                            "mobile_shortcuts",
                            "must contain one to three unique shortcuts",
                        ));
                    }
                    selected.push(shortcut);
                }
                Ok(selected)
            })
            .transpose()?;
        Ok(Self {
            name,
            time_zone,
            gravatar_enabled,
            mobile_shortcuts,
        })
    }
}

pub(crate) struct Name(String);

impl Name {
    pub(crate) fn parse(value: &str) -> Result<Self, OperationError> {
        let value = value.trim();
        if value.is_empty() || value.chars().count() > 100 {
            return Err(super::validation(
                "name",
                "must be between 1 and 100 characters",
            ));
        }
        Ok(Self(value.to_owned()))
    }
}

pub(crate) async fn read(
    transaction: &DatabaseTransaction,
    account_id: AccountId,
    settings: Option<&serde_json::Value>,
) -> Result<Snapshot, OperationError> {
    let account = account::Entity::find_by_id(account_id.get())
        .one(transaction)
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    let time_zone = time_zone::preferred(&account.preferences, settings)
        .map_err(|_| OperationError::Unavailable)?;
    let mobile_shortcuts = match account.preferences.get("mobile_shortcuts") {
        None => vec![Shortcut::Dashboard, Shortcut::Inventory, Shortcut::Finder],
        Some(values) => values
            .as_array()
            .ok_or(OperationError::Unavailable)?
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .ok_or(OperationError::Unavailable)
                    .and_then(Shortcut::parse)
            })
            .collect::<Result<Vec<_>, _>>()?,
    };
    Ok(Snapshot {
        account_id,
        time_zone,
        gravatar_enabled: super::gravatar::enabled(&account.preferences),
        avatar_attached: transaction
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT EXISTS(SELECT 1 FROM account_avatars WHERE account_id=$1) AS attached",
                [account_id.get().into()],
            ))
            .await?
            .ok_or(OperationError::Unavailable)?
            .try_get("", "attached")?,
        mobile_shortcuts,
        name: account.name,
        email: account.email,
    })
}

pub(crate) async fn update(
    transaction: &DatabaseTransaction,
    account_id: AccountId,
    request_id: &str,
    changes: Changes,
) -> Result<(), OperationError> {
    let account = account::Entity::find_by_id(account_id.get())
        .one(transaction)
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    let mut preferences = account
        .preferences
        .as_object()
        .cloned()
        .ok_or(OperationError::Unavailable)?;
    if let Some(zone) = changes.time_zone {
        match zone {
            TimeZone::Inherit => {
                preferences.remove("time_zone");
            }
            TimeZone::Named(zone) => {
                preferences.insert("time_zone".into(), json!(zone.name()));
            }
        }
    }
    if let Some(enabled) = changes.gravatar_enabled {
        preferences.insert("gravatar_enabled".into(), json!(enabled));
    }
    if let Some(shortcuts) = changes.mobile_shortcuts {
        preferences.insert("mobile_shortcuts".into(), json!(shortcuts));
    }
    let preferences = serde_json::Value::Object(preferences);
    let name = changes
        .name
        .map(|name| name.0)
        .or_else(|| account.name.clone());
    if account.name == name && account.preferences == preferences {
        return Ok(());
    }
    let object = json!({"name":account.name,"preferences":account.preferences}).to_string();
    let mut audit_changes = serde_json::Map::new();
    if account.name != name {
        audit_changes.insert("name".into(), json!([account.name, name]));
    }
    if account.preferences != preferences {
        audit_changes.insert(
            "preferences".into(),
            json!([account.preferences, preferences]),
        );
    }
    let mut active: account::ActiveModel = account.into();
    active.name = Set(name);
    active.preferences = Set(preferences);
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(transaction).await?;
    version::ActiveModel {
        item_type: Set("Account".into()),
        item_id: Set(account_id.get()),
        event: Set("update".into()),
        object: Set(Some(object)),
        object_changes: Set(Some(serde_json::Value::Object(audit_changes).to_string())),
        whodunnit: Set(None),
        request_id: Set(Some(request_id.to_owned())),
        audit_context: Set(json!({"actor_account_id":account_id.get(),"request_id":request_id,"server_trace_id":uuid::Uuid::new_v4().to_string(),"policy_query":"update_own_account_profile"})),
        created_at: Set(Some(Utc::now().naive_utc())),
        ..Default::default()
    }.insert(transaction).await?;
    Ok(())
}
