use crate::models::{
    access::{PersonAccess, TenantTransaction},
    entities::{api_change_event, household, notification_preference, person, version},
    errors::OperationError,
    profile,
};
use chrono::{NaiveTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, QuerySelect, Set,
};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

pub mod managed;

#[derive(Default)]
pub struct Changes {
    pub enabled: Option<bool>,
    pub dose_due_enabled: Option<bool>,
    pub missed_dose_enabled: Option<bool>,
    pub low_stock_enabled: Option<bool>,
    pub private_text_enabled: Option<bool>,
    pub morning_time: Option<Option<NaiveTime>>,
    pub afternoon_time: Option<Option<NaiveTime>>,
    pub evening_time: Option<Option<NaiveTime>>,
    pub night_time: Option<Option<NaiveTime>>,
}

fn invalid() -> OperationError {
    profile::validation("notification_preference", "is invalid")
}

fn boolean(fields: &Map<String, Value>, key: &str) -> Result<Option<bool>, OperationError> {
    fields
        .get(key)
        .map(|value| value.as_bool().ok_or_else(invalid))
        .transpose()
}

fn time(
    fields: &Map<String, Value>,
    key: &str,
) -> Result<Option<Option<NaiveTime>>, OperationError> {
    match fields.get(key) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(value)) if value.len() == 5 || value.len() == 8 => {
            let bytes = value.as_bytes();
            if bytes[2] != b':'
                || (bytes.len() == 8 && bytes[5] != b':')
                || bytes
                    .iter()
                    .enumerate()
                    .any(|(index, byte)| index != 2 && index != 5 && !byte.is_ascii_digit())
                || (bytes.len() == 8 && &value[6..8] > "59")
            {
                return Err(invalid());
            }
            let pattern = if value.len() == 5 {
                "%H:%M"
            } else {
                "%H:%M:%S"
            };
            NaiveTime::parse_from_str(value, pattern)
                .map(Some)
                .map(Some)
                .map_err(|_| invalid())
        }
        _ => Err(invalid()),
    }
}

pub fn parse(body: &Value) -> Result<Changes, OperationError> {
    let outer = body.as_object().ok_or_else(invalid)?;
    let fields = outer
        .get("notification_preference")
        .and_then(Value::as_object)
        .ok_or_else(invalid)?;
    if outer.len() != 1
        || fields.is_empty()
        || fields.keys().any(|key| {
            !matches!(
                key.as_str(),
                "enabled"
                    | "dose_due_enabled"
                    | "missed_dose_enabled"
                    | "low_stock_enabled"
                    | "private_text_enabled"
                    | "morning_time"
                    | "afternoon_time"
                    | "evening_time"
                    | "night_time"
            )
        })
    {
        return Err(invalid());
    }
    Ok(Changes {
        enabled: boolean(fields, "enabled")?,
        dose_due_enabled: boolean(fields, "dose_due_enabled")?,
        missed_dose_enabled: boolean(fields, "missed_dose_enabled")?,
        low_stock_enabled: boolean(fields, "low_stock_enabled")?,
        private_text_enabled: boolean(fields, "private_text_enabled")?,
        morning_time: time(fields, "morning_time")?,
        afternoon_time: time(fields, "afternoon_time")?,
        evening_time: time(fields, "evening_time")?,
        night_time: time(fields, "night_time")?,
    })
}

fn matches(changes: &Changes, row: &notification_preference::Model) -> bool {
    changes.enabled.is_none_or(|value| value == row.enabled)
        && changes
            .dose_due_enabled
            .is_none_or(|value| value == row.dose_due_enabled)
        && changes
            .missed_dose_enabled
            .is_none_or(|value| value == row.missed_dose_enabled)
        && changes
            .low_stock_enabled
            .is_none_or(|value| value == row.low_stock_enabled)
        && changes
            .private_text_enabled
            .is_none_or(|value| value == row.private_text_enabled)
        && changes
            .morning_time
            .is_none_or(|value| value == row.morning_time)
        && changes
            .afternoon_time
            .is_none_or(|value| value == row.afternoon_time)
        && changes
            .evening_time
            .is_none_or(|value| value == row.evening_time)
        && changes
            .night_time
            .is_none_or(|value| value == row.night_time)
}

fn apply(changes: Changes, active: &mut notification_preference::ActiveModel) {
    if let Some(value) = changes.enabled {
        active.enabled = Set(value);
    }
    if let Some(value) = changes.dose_due_enabled {
        active.dose_due_enabled = Set(value);
    }
    if let Some(value) = changes.missed_dose_enabled {
        active.missed_dose_enabled = Set(value);
    }
    if let Some(value) = changes.low_stock_enabled {
        active.low_stock_enabled = Set(value);
    }
    if let Some(value) = changes.private_text_enabled {
        active.private_text_enabled = Set(value);
    }
    if let Some(value) = changes.morning_time {
        active.morning_time = Set(value);
    }
    if let Some(value) = changes.afternoon_time {
        active.afternoon_time = Set(value);
    }
    if let Some(value) = changes.evening_time {
        active.evening_time = Set(value);
    }
    if let Some(value) = changes.night_time {
        active.night_time = Set(value);
    }
}

pub fn representation(
    row: &notification_preference::Model,
    owner: &person::Model,
) -> (Value, String) {
    let formatted =
        |value: Option<NaiveTime>| value.map(|time| time.format("%H:%M:%S").to_string());
    let body = json!({"data": {
        "id":row.id,
        "portable_id":row.portable_id,
        "person_id":row.person_id,
        "person_portable_id":owner.portable_id,
        "enabled":row.enabled,
        "dose_due_enabled":row.dose_due_enabled,
        "missed_dose_enabled":row.missed_dose_enabled,
        "low_stock_enabled":row.low_stock_enabled,
        "private_text_enabled":row.private_text_enabled,
        "morning_time":formatted(row.morning_time),
        "afternoon_time":formatted(row.afternoon_time),
        "evening_time":formatted(row.evening_time),
        "night_time":formatted(row.night_time),
        "updated_at":row.updated_at.and_utc().to_rfc3339(),
    }});
    let digest = Sha256::digest(serde_json::to_vec(&body).expect("Notification JSON"));
    let etag = format!("\"{}\"", hex::encode(digest));
    (body, etag)
}

pub async fn read(
    tenant: &TenantTransaction,
    account_id: i64,
) -> Result<(notification_preference::Model, person::Model), OperationError> {
    let owner = profile::linked_person(tenant, account_id, PersonAccess::View).await?;
    let row = notification_preference::Entity::find()
        .filter(notification_preference::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(notification_preference::Column::PersonId.eq(owner.id))
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    Ok((row, owner))
}

pub async fn update(
    tenant: &TenantTransaction,
    account_id: i64,
    changes: Changes,
) -> Result<(notification_preference::Model, person::Model), OperationError> {
    household::Entity::find_by_id(tenant.scope().household_id)
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    let owner = profile::linked_person(tenant, account_id, PersonAccess::Manage).await?;
    let previous = notification_preference::Entity::find()
        .filter(notification_preference::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(notification_preference::Column::PersonId.eq(owner.id))
        .one(tenant.transaction())
        .await?;
    if let Some(row) = &previous
        && matches(&changes, row)
    {
        return Ok((row.clone(), owner));
    }
    let now = Utc::now().naive_utc();
    let mut active = if let Some(row) = previous.clone() {
        row.into_active_model()
    } else {
        notification_preference::ActiveModel {
            household_id: Set(tenant.scope().household_id),
            person_id: Set(owner.id),
            created_at: Set(now),
            ..Default::default()
        }
    };
    active.updated_at = Set(now);
    apply(changes, &mut active);
    let row = if previous.is_some() {
        active.update(tenant.transaction()).await?
    } else {
        active.insert(tenant.transaction()).await?
    };
    let event = if previous.is_some() {
        "update"
    } else {
        "create"
    };
    let old = previous
        .as_ref()
        .map(|value| representation(value, &owner).0["data"].clone());
    let new = representation(&row, &owner).0["data"].clone();
    let changes: Map<String, Value> = new
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(key, after)| {
            let before = old
                .as_ref()
                .and_then(|value| value.get(key))
                .unwrap_or(&Value::Null);
            (before != after).then(|| (key.clone(), json!([before, after])))
        })
        .collect();
    version::ActiveModel {
        item_type: Set("NotificationPreference".into()),
        item_id: Set(row.id),
        event: Set(event.into()),
        object: Set(old.map(|value| value.to_string())),
        object_changes: Set(Some(Value::Object(changes).to_string())),
        whodunnit: Set(Some(tenant.user_id().to_string())),
        request_id: Set(Some(tenant.scope().request_id.clone())),
        household_id: Set(Some(tenant.scope().household_id)),
        actor_membership_id: Set(Some(tenant.membership().id)),
        audit_context: Set(json!({"policy_class":"NotificationPreferencePolicy","policy_query":"update?","actor_account_id":account_id,"household_id":tenant.scope().household_id,"request_id":tenant.scope().request_id})),
        created_at: Set(Some(now)),
        ..Default::default()
    }.insert(tenant.transaction()).await?;
    api_change_event::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        household_membership_id: Set(Some(tenant.membership().id)),
        account_id: Set(Some(account_id)),
        action: Set(event.into()),
        record_type: Set("NotificationPreference".into()),
        record_id: Set(row.id),
        record_portable_id: Set(Some(row.portable_id.clone())),
        request_id: Set(Some(tenant.scope().request_id.clone())),
        metadata: Set(json!({"record_type":"NotificationPreference","record_id":row.id,"portable_id":row.portable_id,"person_portable_id":owner.portable_id})),
        occurred_at: Set(now), created_at: Set(now), updated_at: Set(now),
        ..Default::default()
    }.insert(tenant.transaction()).await?;
    Ok((row, owner))
}
