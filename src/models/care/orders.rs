use crate::models::{
    access::{self, TenantTransaction},
    care::{administration, doses::CredentialProvenance, medications},
    entities::{api_change_event, household, medication},
    errors::OperationError,
};
use chrono::{NaiveDate, Utc};
use sea_orm::prelude::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, QuerySelect, Set,
};
use serde_json::{Value, json};
use std::str::FromStr;

pub async fn lock_visible(
    tenant: &TenantTransaction,
    id: &str,
) -> Result<medication::Model, OperationError> {
    household::Entity::find_by_id(tenant.scope().household_id)
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    access::recheck(tenant).await?;
    let query = access::medication_scope(tenant);
    let query = if let Ok(id) = id.parse::<i64>() {
        query.filter(medication::Column::Id.eq(id))
    } else {
        query.filter(medication::Column::PortableId.eq(id))
    };
    query
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)
}

pub async fn mark_as_ordered(
    tenant: &TenantTransaction,
    id: &str,
    body: &Value,
    provenance: Option<&CredentialProvenance>,
) -> Result<medication::Model, OperationError> {
    change(tenant, id, body, true, provenance).await
}
pub async fn mark_as_received(
    tenant: &TenantTransaction,
    id: &str,
    body: &Value,
    provenance: Option<&CredentialProvenance>,
) -> Result<medication::Model, OperationError> {
    change(tenant, id, body, false, provenance).await
}
fn invalid(field: &str, message: &str) -> OperationError {
    OperationError::Validation {
        details: json!({"errors":{field:[message]}}),
    }
}
async fn change(
    tenant: &TenantTransaction,
    id: &str,
    body: &Value,
    ordered: bool,
    provenance: Option<&CredentialProvenance>,
) -> Result<medication::Model, OperationError> {
    let row = lock_visible(tenant, id).await?;
    let outer = body
        .as_object()
        .ok_or_else(|| invalid("order_details", "is invalid"))?;
    if outer.keys().any(|key| key != "order_details") || (!ordered && !outer.is_empty()) {
        return Err(invalid("order_details", "contains an unsupported field"));
    }
    let empty = json!({});
    let fields = outer
        .get("order_details")
        .unwrap_or(&empty)
        .as_object()
        .ok_or_else(|| invalid("order_details", "must be an object"))?;
    if fields.keys().any(|key| {
        !matches!(
            key.as_str(),
            "supplier" | "quantity" | "expected_arrival_on"
        )
    }) {
        return Err(invalid("order_details", "contains an unsupported field"));
    }
    let supplier = fields
        .get("supplier")
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| invalid("supplier", "must be a string"))
        })
        .transpose()?;
    let quantity = fields
        .get("quantity")
        .map(|value| {
            let raw = value
                .as_str()
                .ok_or_else(|| invalid("quantity", "must be a decimal string"))?;
            let amount = Decimal::from_str(raw)
                .map_err(|_| invalid("quantity", "must be a decimal string"))?;
            if amount < Decimal::ZERO
                || amount >= Decimal::from(100_000_000)
                || amount.normalize().scale() > 2
            {
                return Err(invalid("quantity", "is outside stock precision"));
            }
            Ok(amount)
        })
        .transpose()?;
    let arrival = fields
        .get("expected_arrival_on")
        .map(|value| {
            let raw = value
                .as_str()
                .ok_or_else(|| invalid("expected_arrival_on", "must be an ISO date"))?;
            NaiveDate::parse_from_str(raw, "%Y-%m-%d")
                .ok()
                .filter(|date| date.format("%Y-%m-%d").to_string() == raw)
                .ok_or_else(|| invalid("expected_arrival_on", "must be an ISO date"))
        })
        .transpose()?;
    let before = medications::medication_snapshot(&row);
    let mut active = row.into_active_model();
    let now = Utc::now().naive_utc();
    active.reorder_status = Set(Some(if ordered { 1 } else { 2 }));
    if ordered {
        active.ordered_at = Set(Some(now));
        active.order_supplier = Set(supplier);
        active.order_quantity = Set(quantity);
        active.expected_arrival_on = Set(arrival);
    } else {
        active.reordered_at = Set(Some(now));
    }
    active.updated_at = Set(now);
    let row = active.update(tenant.transaction()).await?;
    administration::persistence::record_version_as(
        tenant,
        "Medication",
        row.id,
        if ordered {
            "mark_as_ordered"
        } else {
            "mark_as_received"
        },
        Some(before),
        medications::medication_snapshot(&row),
        provenance,
    )
    .await?;
    api_change_event::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        household_membership_id: Set(Some(tenant.membership().id)),
        account_id: Set(Some(tenant.scope().actor.account_id)),
        action: Set("update".into()),
        record_type: Set("Medication".into()),
        record_id: Set(row.id),
        record_portable_id: Set(Some(row.portable_id.clone())),
        request_id: Set(Some(tenant.scope().request_id.clone())),
        metadata: Set(
            json!({"record_type":"Medication","record_id":row.id,"portable_id":row.portable_id}),
        ),
        occurred_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await?;
    Ok(row)
}
