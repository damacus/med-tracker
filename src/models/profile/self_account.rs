use super::{Changes, avatar, gravatar};
use crate::models::{entities::account, errors::OperationError, ids::AccountId};
use sea_orm::{DatabaseTransaction, EntityTrait};

pub(crate) struct Snapshot {
    pub account_id: AccountId,
    pub name: String,
    pub email: String,
    pub time_zone: String,
    pub gravatar_enabled: bool,
    pub mobile_shortcuts: Vec<String>,
    pub avatar_attached: bool,
}

pub(crate) struct ValidatedChanges {
    name: Option<String>,
    preferences: Changes,
}

impl ValidatedChanges {
    pub(crate) fn parse(
        name: Option<String>,
        preferences: Changes,
    ) -> Result<Self, OperationError> {
        super::validate(&preferences)?;
        if preferences.date_of_birth.is_some() {
            return Err(super::validation(
                "profile",
                "contains an unsupported field",
            ));
        }
        let name = name.map(|value| value.trim().to_owned());
        if name.as_deref().is_some_and(str::is_empty) {
            return Err(super::validation("name", "can't be blank"));
        }
        Ok(Self { name, preferences })
    }

    pub(crate) async fn apply(
        self,
        transaction: &DatabaseTransaction,
        account_id: AccountId,
        request_id: &str,
    ) -> Result<(), OperationError> {
        if let Some(name) = self.name {
            super::update_account_name(transaction, account_id, request_id, &name).await?;
        }
        super::update_account_preferences(transaction, account_id, request_id, self.preferences)
            .await
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
    let time_zone = crate::models::identity::time_zone::preferred(&account.preferences, settings)
        .map_err(|_| OperationError::Unavailable)?
        .to_string();
    let avatar_attached = avatar::account_attachment(transaction, account_id)
        .await?
        .is_some();
    Ok(Snapshot {
        account_id,
        name: account.name.ok_or(OperationError::Unavailable)?,
        email: account.email,
        time_zone,
        gravatar_enabled: gravatar::enabled(&account.preferences),
        mobile_shortcuts: account
            .preferences
            .get("mobile_shortcuts")
            .and_then(serde_json::Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_else(|| vec!["dashboard".into(), "inventory".into(), "finder".into()]),
        avatar_attached,
    })
}
