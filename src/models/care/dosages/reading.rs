use super::*;
use sea_orm::QueryTrait;
use std::collections::HashMap;

#[derive(Default, Deserialize)]
pub struct Pagination {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub updated_since: Option<String>,
}

fn value(row: &dosage::Model, portable: &str) -> Result<Value, OperationError> {
    let cycle = match row.default_dose_cycle {
        0 => "daily",
        1 => "weekly",
        2 => "monthly",
        _ => return Err(OperationError::Unavailable),
    };
    Ok(
        json!({"id":row.id,"portable_id":row.portable_id,"medication_id":row.medication_id,"medication_portable_id":portable,
        "amount":super::super::medications::decimal_string(row.amount.to_string()),"unit":row.unit,"frequency":row.frequency,"description":row.description,
        "default_for_adults":row.default_for_adults,"default_for_children":row.default_for_children,"default_max_daily_doses":row.default_max_daily_doses,
        "default_min_hours_between_doses":super::super::medications::decimal_string(row.default_min_hours_between_doses.to_string()),"default_dose_cycle":cycle,
        "current_supply":row.current_supply.map(|value|super::super::medications::decimal_string(value.to_string())),"reorder_threshold":row.reorder_threshold.map(|value|super::super::medications::decimal_string(value.to_string())),"updated_at":row.updated_at.and_utc().to_rfc3339()}),
    )
}

pub(super) async fn representation(
    tenant: &TenantTransaction,
    row: &dosage::Model,
) -> Result<(Value, String), OperationError> {
    let parent = parent(tenant, &row.medication_id.to_string(), false).await?;
    let body = json!({"data":value(row,&parent.portable_id)?});
    let mut canonical = body.clone();
    canonical.sort_all_objects();
    let etag = format!(
        "\"{}\"",
        hex::encode(Sha256::digest(
            serde_json::to_vec(&canonical).map_err(|_| OperationError::Unavailable)?
        ))
    );
    Ok((body, etag))
}

fn scope(tenant: &TenantTransaction) -> sea_orm::Select<dosage::Entity> {
    dosage::Entity::find()
        .filter(dosage::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(
            dosage::Column::MedicationId.in_subquery(
                access::medication_scope(tenant)
                    .select_only()
                    .column(medication::Column::Id)
                    .into_query(),
            ),
        )
}

pub async fn for_medication(
    tenant: &TenantTransaction,
    id: &str,
) -> Result<Vec<Value>, OperationError> {
    access::recheck(tenant).await?;
    let query = access::medication_scope(tenant);
    let query = if let Ok(id) = id.parse::<i64>() {
        query.filter(medication::Column::Id.eq(id))
    } else {
        query.filter(medication::Column::PortableId.eq(id))
    };
    let parent = query
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    let rows = dosage::Entity::find()
        .filter(dosage::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(dosage::Column::MedicationId.eq(parent.id))
        .order_by_asc(dosage::Column::Id)
        .all(tenant.transaction())
        .await?;
    rows.iter()
        .map(|row| value(row, &parent.portable_id))
        .collect()
}

pub async fn read(tenant: &TenantTransaction, id: &str) -> Result<(Value, String), OperationError> {
    access::recheck(tenant).await?;
    let mut query = scope(tenant);
    query = if let Ok(id) = id.parse::<i64>() {
        query.filter(dosage::Column::Id.eq(id))
    } else {
        query.filter(dosage::Column::PortableId.eq(id))
    };
    let row = query
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    representation(tenant, &row).await
}

pub async fn list(
    tenant: &TenantTransaction,
    pagination: Pagination,
) -> Result<Value, OperationError> {
    access::recheck(tenant).await?;
    let page = pagination.page.unwrap_or(1);
    let per_page = pagination.per_page.unwrap_or(20);
    if page < 1 || !(1..=100).contains(&per_page) {
        return Err(invalid());
    }
    let mut query = scope(tenant);
    if let Some(timestamp) = pagination.updated_since {
        let at = chrono::DateTime::parse_from_rfc3339(&timestamp).map_err(|_| invalid())?;
        query = query.filter(dosage::Column::UpdatedAt.gte(at.naive_utc()));
    }
    let count = query.clone().count(tenant.transaction()).await?;
    let rows = query
        .order_by_asc(dosage::Column::Id)
        .limit(per_page as u64)
        .offset(page.saturating_sub(1).saturating_mul(per_page) as u64)
        .all(tenant.transaction())
        .await?;
    let parents = medication::Entity::find()
        .filter(medication::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(medication::Column::Id.is_in(rows.iter().map(|row| row.medication_id)))
        .all(tenant.transaction())
        .await?
        .into_iter()
        .map(|row| (row.id, row.portable_id))
        .collect::<HashMap<_, _>>();
    let values = rows
        .iter()
        .map(|row| {
            value(
                row,
                parents
                    .get(&row.medication_id)
                    .ok_or(OperationError::Unavailable)?,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({"data":values,"meta":{"page":page,"per_page":per_page,"total_count":count}}))
}
