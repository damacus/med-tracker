use super::*;
use serde::Deserialize;

#[derive(Default, Deserialize)]
pub struct Pagination {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub updated_since: Option<String>,
}
fn scope(tenant: &TenantTransaction) -> sea_orm::Select<person_medication::Entity> {
    person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(person_medication::Column::RetiredAt.is_null())
        .filter(
            person_medication::Column::PersonId
                .in_subquery(access::granted_people(tenant.membership())),
        )
}
pub async fn list(tenant: &TenantTransaction, page: Pagination) -> Result<Value, OperationError> {
    access::recheck(tenant).await?;
    let number = page.page.unwrap_or(1);
    let per_page = page.per_page.unwrap_or(20);
    if number < 1 || !(1..=100).contains(&per_page) {
        return Err(invalid());
    }
    let mut query = scope(tenant);
    if let Some(value) = page.updated_since {
        let value = chrono::DateTime::parse_from_rfc3339(&value)
            .map(|value| value.naive_utc())
            .map_err(|_| invalid())?;
        query = query.filter(person_medication::Column::UpdatedAt.gte(value));
    }
    let count = query.clone().count(tenant.transaction()).await?;
    let rows = query
        .order_by_asc(person_medication::Column::Id)
        .limit(per_page as u64)
        .offset(number.saturating_sub(1).saturating_mul(per_page) as u64)
        .all(tenant.transaction())
        .await?;
    let data = super::super::treatments::projection::serialize_assignments(
        tenant.transaction(),
        tenant,
        rows,
    )
    .await?;
    Ok(json!({"data":data,"meta":{"page":number,"per_page":per_page,"total_count":count}}))
}
pub(super) async fn find(
    tenant: &TenantTransaction,
    id: &str,
    lock: bool,
) -> Result<person_medication::Model, OperationError> {
    access::recheck(tenant).await?;
    let query = scope(tenant);
    let query = if let Ok(id) = id.parse::<i64>() {
        query.filter(person_medication::Column::Id.eq(id))
    } else {
        query.filter(person_medication::Column::PortableId.eq(id))
    };
    let query = if lock { query.lock_exclusive() } else { query };
    let row = query
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    if !access::can_access_person(tenant, row.person_id, PersonAccess::View).await? {
        return Err(OperationError::NotFound);
    }
    Ok(row)
}
pub async fn read(tenant: &TenantTransaction, id: &str) -> Result<(Value, String), OperationError> {
    project(tenant, find(tenant, id, false).await?).await
}
pub(super) async fn project(
    tenant: &TenantTransaction,
    row: person_medication::Model,
) -> Result<(Value, String), OperationError> {
    let rows = super::super::treatments::projection::serialize_assignments(
        tenant.transaction(),
        tenant,
        vec![row],
    )
    .await?;
    let body = json!({"data":rows.into_iter().next().ok_or(OperationError::NotFound)?});
    let mut sorted = body.clone();
    sorted.sort_all_objects();
    use sha2::{Digest, Sha256};
    let etag = format!(
        "\"{}\"",
        hex::encode(Sha256::digest(
            serde_json::to_vec(&sorted).expect("JSON must serialize")
        ))
    );
    Ok((body, etag))
}
