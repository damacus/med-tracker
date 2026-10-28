mod collection;
mod idempotency;
mod rails_digest;
pub use collection::{Pagination, collection};
mod lifecycle;
mod memberships;
pub use idempotency::{KeyedResponse, SavedResponse, lookup, store};
pub use lifecycle::{can_manage, list, read, retire, update};
pub use memberships::{
    LocationMembershipInput, create_membership, destroy_membership,
    representation as membership_representation,
};

use crate::models::{
    access::{self, TenantTransaction},
    care::doses::CredentialProvenance,
    entities::{api_change_event, household, location, version},
    errors::OperationError,
};
use chrono::Utc;
use loco_rs::model::TenantActiveModelExt;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug)]
pub enum LocationField<T> {
    Missing,
    Value(T),
    Invalid,
}

#[derive(Clone, Debug)]
pub struct LocationInput {
    name: LocationField<String>,
    description: LocationField<Option<String>>,
    invalid_shape: bool,
    unknown_field: bool,
    empty: bool,
}

impl LocationInput {
    pub fn from_json(value: Value) -> Self {
        match value {
            Value::Object(fields) => Self::from_fields(fields),
            _ => Self {
                name: LocationField::Missing,
                description: LocationField::Missing,
                invalid_shape: true,
                unknown_field: false,
                empty: true,
            },
        }
    }

    pub fn from_fields(fields: serde_json::Map<String, Value>) -> Self {
        let name = match fields.get("name") {
            None => LocationField::Missing,
            Some(Value::String(value)) => LocationField::Value(value.clone()),
            Some(_) => LocationField::Invalid,
        };
        let description = match fields.get("description") {
            None => LocationField::Missing,
            Some(Value::Null) => LocationField::Value(None),
            Some(Value::String(value)) => LocationField::Value(Some(value.clone())),
            Some(_) => LocationField::Invalid,
        };
        Self {
            name,
            description,
            invalid_shape: false,
            unknown_field: fields
                .keys()
                .any(|key| key != "name" && key != "description"),
            empty: fields.is_empty(),
        }
    }

    pub fn from_form(name: &str, description: &str) -> Self {
        Self {
            name: LocationField::Value(name.into()),
            description: LocationField::Value(Some(description.into())),
            invalid_shape: false,
            unknown_field: false,
            empty: false,
        }
    }

    fn values(
        &self,
        current: Option<&location::Model>,
    ) -> Result<(String, Option<String>), OperationError> {
        if self.invalid_shape || (current.is_some() && self.empty) {
            return Err(invalid("location", "is invalid"));
        }
        let name = match &self.name {
            LocationField::Missing => current.map_or_else(
                || Err(invalid("name", "can't be blank")),
                |row| Ok(row.name.clone()),
            )?,
            LocationField::Value(value) if value.trim().is_empty() => {
                return Err(invalid("name", "can't be blank"));
            }
            LocationField::Value(value) => value.clone(),
            LocationField::Invalid => return Err(invalid("name", "is invalid")),
        };
        if self.unknown_field {
            return Err(invalid("location", "is invalid"));
        }
        let description = match &self.description {
            LocationField::Missing => current.and_then(|row| row.description.clone()),
            LocationField::Value(value) => value.clone(),
            LocationField::Invalid => return Err(invalid("location", "is invalid")),
        };
        Ok((name, description))
    }
}

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

fn invalid(field: &str, message: &str) -> OperationError {
    OperationError::Validation {
        details: json!({"errors":{field:[message]}}),
    }
}

pub async fn create(
    tenant: &TenantTransaction,
    input: LocationInput,
    provenance: Option<&CredentialProvenance>,
) -> Result<location::Model, OperationError> {
    authorize(tenant).await?;
    let (name, description) = input.values(None)?;
    let now = Utc::now().naive_utc();
    let record = location::ActiveModel {
        name: Set(name),
        description: Set(description),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .set_tenant(tenant.scope().household_id)
    .map_err(|_| OperationError::Unavailable)?
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
    let mut context = crate::models::audit_context::household(tenant, provenance);
    context["policy_class"] = json!("LocationPolicy");
    context["policy_query"] = json!(format!("{event}?"));
    version::ActiveModel {
        item_type: Set("Location".into()),
        item_id: Set(record.id),
        event: Set(event.into()),
        object: Set(before.as_ref().map(Value::to_string)),
        object_changes: Set(Some(Value::Object(changes).to_string())),
        whodunnit: Set(Some(tenant.scope().actor.account_id.to_string())),
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

#[cfg(test)]
mod input_tests {
    use super::*;

    #[test]
    fn location_patch_distinguishes_omitted_description_from_explicit_clear() {
        let omitted = LocationInput::from_fields(serde_json::Map::new());
        let clear = LocationInput::from_fields(serde_json::Map::from_iter([(
            "description".into(),
            Value::Null,
        )]));

        assert!(matches!(omitted.description, LocationField::Missing));
        assert!(matches!(clear.description, LocationField::Value(None)));
    }
}
