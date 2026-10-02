use crate::database_error;
use crate::entities::medication;
use crate::entities::version;
use crate::ApiError;
use crate::AuthContext;
use chrono::Utc;
use sea_orm::prelude::Decimal;
use sea_orm::ActiveModelTrait;
use sea_orm::DatabaseTransaction;
use sea_orm::Set;
use serde_json::json;
use serde_json::Value;

pub(crate) fn medication_snapshot(record: &medication::Model) -> Value {
    json!({
        "id": record.id,
        "portable_id": record.portable_id,
        "name": record.name,
        "friendly_name": record.friendly_name,
        "dose_amount": record.dose_amount,
        "dose_unit": record.dose_unit,
        "location_id": record.location_id,
        "current_supply": record.current_supply.map(|value| value.to_string()),
        "reorder_threshold": record.reorder_threshold.to_string(),
        "reorder_status": record.reorder_status
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn record_version(
    db: &DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    item_type: &str,
    item_id: i64,
    event: &str,
    before: Option<Value>,
    after: Option<Value>,
) -> Result<(), ApiError> {
    insert_version(
        db,
        context,
        request_id,
        VersionChange {
            item_type,
            item_id,
            event,
            before,
            after,
            inventory_adjustment: None,
        },
    )
    .await
}

pub(crate) async fn record_inventory_adjustment(
    db: &DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    medication: &medication::Model,
    before: Value,
    quantity: Decimal,
    reason: Option<&str>,
) -> Result<(), ApiError> {
    let quantity = quantity.normalize().to_string();
    let quantity_event = format!("adjust inventory (qty: {quantity})");
    let event = reason
        .filter(|reason| !reason.trim().is_empty())
        .map(|reason| format!("adjust inventory (qty: {quantity}, reason: {reason})"))
        .filter(|event| event.len() <= 1024)
        .unwrap_or(quantity_event);
    insert_version(
        db,
        context,
        request_id,
        VersionChange {
            item_type: "Medication",
            item_id: medication.id,
            event: &event,
            before: Some(before),
            after: Some(medication_snapshot(medication)),
            inventory_adjustment: Some(json!({"reason": reason, "new_quantity": quantity})),
        },
    )
    .await
}

struct VersionChange<'a> {
    item_type: &'a str,
    item_id: i64,
    event: &'a str,
    before: Option<Value>,
    after: Option<Value>,
    inventory_adjustment: Option<Value>,
}

async fn insert_version(
    db: &DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    change: VersionChange<'_>,
) -> Result<(), ApiError> {
    let VersionChange {
        item_type,
        item_id,
        event,
        before,
        after,
        inventory_adjustment,
    } = change;
    let mut changes = serde_json::Map::new();
    let mut fields = std::collections::HashSet::new();
    if let Some(before) = before.as_ref().and_then(Value::as_object) {
        fields.extend(before.keys().cloned());
    }
    if let Some(after) = after.as_ref().and_then(Value::as_object) {
        fields.extend(after.keys().cloned());
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
    let mut audit_context = json!({
        "actor_account_id": context.account_id,
        "actor_user_id": context.user_id,
        "actor_membership_id": context.membership.id,
        "household_id": context.membership.household_id,
        "request_id": request_id
    });
    if let Some(adjustment) = inventory_adjustment {
        audit_context["inventory_adjustment"] = adjustment;
    }
    version::ActiveModel {
        item_type: Set(item_type.to_owned()),
        item_id: Set(item_id),
        event: Set(event.to_owned()),
        object: Set(before.as_ref().map(Value::to_string)),
        object_changes: Set(Some(Value::Object(changes).to_string())),
        whodunnit: Set(Some(context.user_id.to_string())),
        request_id: Set(Some(request_id.to_owned())),
        household_id: Set(Some(context.membership.household_id)),
        actor_membership_id: Set(Some(context.membership.id)),
        audit_context: Set(audit_context),
        created_at: Set(Some(Utc::now().naive_utc())),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(database_error)?;
    Ok(())
}
