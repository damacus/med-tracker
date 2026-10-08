mod collection;
mod idempotency;
mod rails_digest;
pub use collection::{Pagination, collection};
mod lifecycle;
pub use idempotency::{KeyedResponse, SavedResponse, lookup, store};
pub use lifecycle::{can_manage, list, read, retire, update};

use crate::models::{
    access::{self, TenantTransaction},
    care::doses::{CredentialMethod, CredentialProvenance},
    entities::{api_change_event, household, location, version},
    errors::OperationError,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub async fn authorize(tenant: &TenantTransaction) -> Result<(), OperationError> {
    household::Entity::find_by_id(tenant.scope().household_id)
        .lock_exclusive()
        .one(tenant.transaction())
        .await
        .map_err(|_| OperationError::Unavailable)?
        .ok_or(OperationError::NotFound)?;
    access::recheck(tenant).await?;
    if !access::can_manage_household(tenant) {
        return Err(OperationError::Forbidden);
    }
    Ok(())
}

pub fn validate(attributes: &Value) -> Result<(String, Option<String>), OperationError> {
    let inner = attributes
        .as_object()
        .ok_or_else(|| invalid("location", "is invalid"))?;
    let name = match inner.get("name") {
        Some(Value::String(name)) if !name.trim().is_empty() => name.clone(),
        Some(Value::String(_)) | None => return Err(invalid("name", "can't be blank")),
        _ => return Err(invalid("name", "is invalid")),
    };
    if inner
        .keys()
        .any(|key| key != "name" && key != "description")
    {
        return Err(invalid("location", "is invalid"));
    }
    let description = match inner.get("description") {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) => Some(value.clone()),
        _ => return Err(invalid("location", "is invalid")),
    };
    Ok((name, description))
}

fn invalid(field: &str, message: &str) -> OperationError {
    OperationError::Validation {
        details: json!({"errors":{field:[message]}}),
    }
}

pub async fn create(
    tenant: &TenantTransaction,
    attributes: Value,
    provenance: Option<&CredentialProvenance>,
) -> Result<location::Model, OperationError> {
    authorize(tenant).await?;
    let (name, description) = validate(&attributes)?;
    let now = Utc::now().naive_utc();
    let record = location::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        name: Set(name),
        description: Set(description),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await
    .map_err(|error| {
        if matches!(
            error.sql_err(),
            Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
        ) {
            invalid("name", "has already been taken")
        } else {
            OperationError::Unavailable
        }
    })?;
    record_change(tenant, &record, "create", None, provenance).await?;
    Ok(record)
}

pub fn representation(record: &location::Model) -> (Value, String) {
    let mut body = json!({"data":{"id":record.id,"portable_id":record.portable_id,"name":record.name,"description":record.description,"updated_at":record.updated_at.and_utc().to_rfc3339()}});
    body.sort_all_objects();
    let etag = format!(
        "\"{}\"",
        hex::encode(Sha256::digest(body.to_string().as_bytes()))
    );
    (body, etag)
}

fn snapshot(record: &location::Model) -> Value {
    json!({"id":record.id,"household_id":record.household_id,"portable_id":record.portable_id,"name":record.name,"description":record.description,"created_at":record.created_at,"updated_at":record.updated_at})
}

fn write_error(error: sea_orm::DbErr) -> OperationError {
    if matches!(
        error.sql_err(),
        Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
    ) {
        invalid("name", "has already been taken")
    } else {
        OperationError::Unavailable
    }
}

async fn record_change(
    tenant: &TenantTransaction,
    record: &location::Model,
    event: &str,
    before: Option<Value>,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    let now = Utc::now().naive_utc();
    let after = if event == "destroy" {
        None
    } else {
        Some(snapshot(record))
    };
    let mut changes = serde_json::Map::new();
    let mut fields = std::collections::BTreeSet::new();
    for row in [before.as_ref(), after.as_ref()].into_iter().flatten() {
        fields.extend(
            row.as_object()
                .ok_or(OperationError::Unavailable)?
                .keys()
                .cloned(),
        );
    }
    for field in fields {
        let old = before
            .as_ref()
            .and_then(|row| row.get(&field))
            .cloned()
            .unwrap_or(Value::Null);
        let new = after
            .as_ref()
            .and_then(|row| row.get(&field))
            .cloned()
            .unwrap_or(Value::Null);
        if old != new {
            changes.insert(field, json!([old, new]));
        }
    }
    let mut context = json!({"actor_account_id":tenant.scope().actor.account_id,"actor_user_id":tenant.user_id(),"actor_membership_id":tenant.membership().id,"household_id":tenant.scope().household_id,"request_id":tenant.scope().request_id,"active_role":tenant.membership().role,"permissions_version":tenant.membership().permissions_version,"policy_class":"LocationPolicy","policy_query":"create?"});
    if let Some(provenance) = provenance {
        let (method, prefix) = match provenance.method {
            CredentialMethod::ApiSession => ("api_session", "api_session"),
            CredentialMethod::ApiAppToken => ("api_app_token", "api_app_token"),
            CredentialMethod::OauthGrant => ("oauth", "oauth_grant"),
            CredentialMethod::PersonalApiKey => ("personal_api_key", "personal_api_key"),
            CredentialMethod::BrowserSession => ("browser_session", "browser_session"),
        };
        context["authentication_method"] = json!(method);
        context["session_reference"] = json!(format!("{}:{}", prefix, provenance.reference));
    }
    context["policy_query"] = json!(format!("{event}?"));
    version::ActiveModel {
        item_type: Set("Location".into()),
        item_id: Set(record.id),
        event: Set(event.into()),
        object: Set(before.as_ref().map(Value::to_string)),
        object_changes: Set(Some(Value::Object(changes).to_string())),
        whodunnit: Set(Some(tenant.user_id().to_string())),
        request_id: Set(Some(tenant.scope().request_id.clone())),
        household_id: Set(Some(tenant.scope().household_id)),
        actor_membership_id: Set(Some(tenant.membership().id)),
        audit_context: Set(context),
        created_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await
    .map_err(|_| OperationError::Unavailable)?;
    if event == "destroy" {
        return Ok(());
    }
    api_change_event::ActiveModel { household_id:Set(tenant.scope().household_id),household_membership_id:Set(Some(tenant.membership().id)),account_id:Set(Some(tenant.scope().actor.account_id)),action:Set(event.into()),record_type:Set("Location".into()),record_id:Set(record.id),record_portable_id:Set(Some(record.portable_id.clone())),request_id:Set(Some(tenant.scope().request_id.clone())),metadata:Set(json!({"record_type":"Location","record_id":record.id,"portable_id":record.portable_id})),occurred_at:Set(now),created_at:Set(now),updated_at:Set(now),..Default::default() }.insert(tenant.transaction()).await.map_err(|_|OperationError::Unavailable)?;
    Ok(())
}
