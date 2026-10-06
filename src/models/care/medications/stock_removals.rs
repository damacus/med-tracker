use super::*;

#[derive(Clone, Debug)]
pub struct RemoveStock {
    pub medication_id: String,
    pub quantity: String,
    pub reason: String,
    pub note: Option<String>,
    pub dosage_id: Option<String>,
    pub submission_id: String,
}

mod mutation;
use sea_orm::{PaginatorTrait, QueryOrder, QuerySelect};

pub async fn create(
    tenant: &TenantTransaction,
    input: RemoveStock,
    provenance: Option<&CredentialProvenance>,
) -> Result<Value, OperationError> {
    let context = StockContext { tenant, provenance };
    lock_row(
        tenant.transaction(),
        "households",
        tenant.scope().household_id,
    )
    .await?;
    let found = authorize(tenant, &input.medication_id).await?;
    let (quantity, payload) = validate(&input)?;
    lock_row(tenant.transaction(), "medications", found.id).await?;
    let found = authorize(tenant, &input.medication_id).await?;
    if let Some(id) = input.dosage_id.as_deref() {
        let owned = dosage::Entity::find_by_id(id.parse::<i64>().map_err(|_| invalid())?)
            .filter(dosage::Column::HouseholdId.eq(tenant.scope().household_id))
            .filter(dosage::Column::MedicationId.eq(found.id))
            .one(tenant.transaction())
            .await
            .map_err(database_error)?
            .is_some();
        if !owned {
            return Err(invalid());
        }
    }
    let prior_id = tenant.transaction().query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,"SELECT id FROM versions WHERE household_id=$1 AND item_type='MedicationStockRemoval' AND item_id=$2 AND event='stock_removal' AND CASE WHEN item_type='MedicationStockRemoval' THEN object::jsonb->>'submission_id' END=$3 ORDER BY id DESC LIMIT 1",[tenant.scope().household_id.into(),found.id.into(),input.submission_id.clone().into()])).await.map_err(database_error)?;
    let prior = match prior_id {
        Some(row) => {
            let id: i64 = row
                .try_get("", "id")
                .map_err(|_| OperationError::Unavailable)?;
            history_query(tenant, found.id)
                .filter(version::Column::Id.eq(id))
                .one(tenant.transaction())
                .await
                .map_err(database_error)?
        }
        None => None,
    };
    if let Some(prior) = prior {
        let values: Value = prior
            .object
            .as_deref()
            .and_then(|raw| serde_json::from_str(raw).ok())
            .unwrap_or(Value::Null);
        if ["quantity", "reason", "note", "submission_id", "dosage_id"]
            .iter()
            .any(|key| values[*key] != payload[*key])
        {
            return Err(invalid());
        }
        return Ok(row(&prior));
    }
    mutation::apply(&context, found, quantity, payload, input.dosage_id).await
}

pub async fn history(
    tenant: &TenantTransaction,
    medication_id: &str,
    page: i64,
    per_page: i64,
) -> Result<Value, OperationError> {
    let found = authorize(tenant, medication_id).await?;
    if page < 1 || !(1..=100).contains(&per_page) {
        return Err(validation("Invalid pagination parameters"));
    }
    let query = history_query(tenant, found.id);
    let total = query
        .clone()
        .count(tenant.transaction())
        .await
        .map_err(database_error)?;
    let rows = query
        .order_by_desc(version::Column::Id)
        .limit(per_page as u64)
        .offset(page.saturating_sub(1).saturating_mul(per_page) as u64)
        .all(tenant.transaction())
        .await
        .map_err(database_error)?;
    Ok(
        json!({"data": rows.iter().map(row).collect::<Vec<_>>(), "meta": {"page": page, "per_page": per_page, "total_count": total}}),
    )
}

pub async fn authorize(
    tenant: &TenantTransaction,
    id: &str,
) -> Result<medication::Model, OperationError> {
    access::recheck(tenant).await?;
    let found = visible_medication(tenant, id)
        .await?
        .ok_or(OperationError::NotFound)?;
    if !access::can_manage_household(tenant) {
        return Err(OperationError::Forbidden);
    }
    Ok(found)
}

fn invalid() -> OperationError {
    validation("Stock removal could not be recorded")
}

fn history_query(tenant: &TenantTransaction, id: i64) -> sea_orm::Select<version::Entity> {
    version::Entity::find()
        .filter(version::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(version::Column::ItemType.eq("MedicationStockRemoval"))
        .filter(version::Column::ItemId.eq(id))
        .filter(version::Column::Event.eq("stock_removal"))
}

fn validate(input: &RemoveStock) -> Result<(Decimal, Value), OperationError> {
    let mut parts = input.quantity.split('.');
    let whole = parts.next().unwrap_or("");
    let fraction = parts.next();
    if whole.is_empty()
        || !whole.bytes().all(|v| v.is_ascii_digit())
        || fraction
            .is_some_and(|v| v.is_empty() || v.len() > 2 || !v.bytes().all(|v| v.is_ascii_digit()))
        || parts.next().is_some()
    {
        return Err(invalid());
    }
    let quantity = Decimal::from_str(&input.quantity).map_err(|_| invalid())?;
    if quantity <= Decimal::ZERO || quantity >= Decimal::from(100_000_000) {
        return Err(invalid());
    }
    if ![
        "dropped",
        "damaged",
        "expired",
        "discarded",
        "lost",
        "transferred_out",
        "other",
    ]
    .contains(&input.reason.as_str())
    {
        return Err(invalid());
    }
    let note = input.note.as_deref().unwrap_or("").trim();
    if note.chars().count() > 1000
        || uuid::Uuid::parse_str(&input.submission_id).is_err()
        || input.submission_id.len() != 36
        || !input.submission_id.bytes().enumerate().all(|(i, v)| {
            if [8, 13, 18, 23].contains(&i) {
                v == b'-'
            } else {
                v.is_ascii_digit() || (b'a'..=b'f').contains(&v)
            }
        })
    {
        return Err(invalid());
    }
    if input.dosage_id.as_deref().is_some_and(|id| {
        !id.as_bytes()
            .first()
            .is_some_and(|v| (b'1'..=b'9').contains(v))
            || !id.bytes().all(|v| v.is_ascii_digit())
            || id.parse::<i64>().is_err()
    }) {
        return Err(invalid());
    }
    Ok((
        quantity,
        json!({"quantity": format_quantity(quantity), "reason": input.reason, "note": note, "submission_id": input.submission_id, "dosage_id": input.dosage_id.as_deref().unwrap_or("")}),
    ))
}

fn format_quantity(value: Decimal) -> String {
    value.normalize().to_string()
}

fn row(event: &version::Model) -> Value {
    let values: Value = event
        .object
        .as_deref()
        .and_then(|value| serde_json::from_str(value).ok())
        .unwrap_or(Value::Null);
    json!({"id": event.id.to_string(), "medication_id": event.item_id.to_string(), "dosage_id": values.get("dosage_id").and_then(Value::as_str).filter(|value| !value.is_empty()), "quantity": values.get("quantity"), "reason": values.get("reason"), "note": values.get("note"), "submission_id": values.get("submission_id"), "previous_quantity": values.get("previous_quantity"), "remaining_quantity": values.get("remaining_quantity"), "unit": values.get("unit"), "created_at": event.created_at.map(|v|v.format("%Y-%m-%dT%H:%M:%SZ").to_string()), "actor_membership_id": event.actor_membership_id.map(|id|id.to_string())})
}
