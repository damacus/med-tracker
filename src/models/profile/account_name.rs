use crate::models::{
    entities::{account, version},
    errors::OperationError,
    ids::AccountId,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseTransaction, EntityTrait, Set};
use serde_json::json;

pub(crate) struct Snapshot {
    pub account_id: AccountId,
    pub name: Option<String>,
    pub email: String,
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
) -> Result<Snapshot, OperationError> {
    let account = account::Entity::find_by_id(account_id.get())
        .one(transaction)
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    Ok(Snapshot {
        account_id,
        name: account.name,
        email: account.email,
    })
}

pub(crate) async fn update(
    transaction: &DatabaseTransaction,
    account_id: AccountId,
    request_id: &str,
    name: Name,
) -> Result<(), OperationError> {
    let name = name.0;
    let account = account::Entity::find_by_id(account_id.get())
        .one(transaction)
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    if account.name.as_deref() == Some(name.as_str()) {
        return Ok(());
    }
    let previous = account.name.clone();
    let object = json!({"name":previous}).to_string();
    let changes = json!({"name":[previous,name]}).to_string();
    let mut active: account::ActiveModel = account.into();
    active.name = Set(Some(name));
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(transaction).await?;
    version::ActiveModel {
        item_type: Set("Account".into()),
        item_id: Set(account_id.get()),
        event: Set("update".into()),
        object: Set(Some(object)),
        object_changes: Set(Some(changes)),
        whodunnit: Set(None),
        request_id: Set(Some(request_id.to_owned())),
        audit_context: Set(json!({"actor_account_id":account_id.get(),"request_id":request_id,"server_trace_id":uuid::Uuid::new_v4().to_string(),"policy_query":"update_own_account_profile"})),
        created_at: Set(Some(Utc::now().naive_utc())),
        ..Default::default()
    }.insert(transaction).await?;
    Ok(())
}
