use super::*;
use loco_rs::model::TenantQueryExt;

#[derive(Default, serde::Deserialize)]
pub struct Pagination {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub updated_since: Option<String>,
}

fn scope(tenant: &TenantTransaction) -> sea_orm::Select<person::Entity> {
    access::person_scope(crate::models::ids::HouseholdId::from_database(
        tenant.scope().household_id,
    ))
}

pub(super) async fn selected(
    tenant: &TenantTransaction,
    id: &str,
) -> Result<person::Model, OperationError> {
    access::recheck(tenant).await?;
    let query = scope(tenant);
    let query = if let Ok(id) = id.parse::<i64>() {
        query.filter(person::Column::Id.eq(id))
    } else {
        query.filter(person::Column::PortableId.eq(id))
    };
    query
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)
}

pub async fn read(
    tenant: &TenantTransaction,
    id: &str,
    zone: Tz,
) -> Result<(Value, String), OperationError> {
    let record = selected(tenant, id).await?;
    representation(tenant, &record, zone).await
}

pub async fn list(
    tenant: &TenantTransaction,
    page: Pagination,
    zone: Tz,
) -> Result<Value, OperationError> {
    access::recheck(tenant).await?;
    let number = page.page.unwrap_or(1);
    let size = page.per_page.unwrap_or(20);
    if number < 1 || !(1..=100).contains(&size) {
        return Err(invalid(
            "page",
            "page must be positive and per_page must be between 1 and 100",
        ));
    }
    let mut query = scope(tenant);
    if let Some(value) = page.updated_since {
        let timestamp = chrono::DateTime::parse_from_rfc3339(&value)
            .map_err(|_| invalid("updated_since", "is invalid"))?
            .naive_utc();
        query = query.filter(person::Column::UpdatedAt.gte(timestamp));
    }
    let total = query.clone().count(tenant.transaction()).await?;
    let rows = query
        .order_by_asc(person::Column::Id)
        .limit(size as u64)
        .offset(((number - 1) as u64).saturating_mul(size as u64))
        .all(tenant.transaction())
        .await?;
    Ok(
        json!({"data":values(tenant,&rows,zone).await?,"meta":{"page":number,"per_page":size,"total_count":total}}),
    )
}

pub async fn representation(
    tenant: &TenantTransaction,
    record: &person::Model,
    zone: Tz,
) -> Result<(Value, String), OperationError> {
    let mut body =
        json!({"data":values(tenant,std::slice::from_ref(record),zone).await?.remove(0)});
    body.sort_all_objects();
    let etag = format!(
        "\"{}\"",
        hex::encode(Sha256::digest(body.to_string().as_bytes()))
    );
    Ok((body, etag))
}

pub(crate) async fn values(
    tenant: &TenantTransaction,
    records: &[person::Model],
    zone: Tz,
) -> Result<Vec<Value>, OperationError> {
    values_in(tenant, tenant.scope().household_id, records, zone).await
}

pub(crate) async fn values_in(
    tenant: &TenantTransaction,
    household_id: i64,
    records: &[person::Model],
    zone: Tz,
) -> Result<Vec<Value>, OperationError> {
    if records.is_empty() {
        return Ok(vec![]);
    }
    let preferences: HashMap<crate::models::ids::PersonId, notification_preference::Model> =
        notification_preference::Entity::find()
            .in_tenant(household_id)
            .filter(notification_preference::Column::AccountId.eq(tenant.scope().actor.account_id))
            .all(tenant.transaction())
            .await?
            .into_iter()
            .map(|row| (tenant.person_id(), row))
            .collect();
    let names = crate::models::person_identity::display_names(
        tenant,
        records
            .iter()
            .map(|record| crate::models::ids::PersonId::from_database(record.id)),
    )
    .await?;
    let own_account =
        crate::models::entities::account::Entity::find_by_id(tenant.scope().actor.account_id)
            .one(tenant.transaction())
            .await?;
    let today = Utc::now().with_timezone(&zone).date_naive();
    Ok(records.iter().map(|record|{let preference=preferences.get(&crate::models::ids::PersonId::from_database(record.id));json!({"id":record.id,"portable_id":record.portable_id,"updated_at":record.updated_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),"name":names.get(&crate::models::ids::PersonId::from_database(record.id)),"email":own_account.as_ref().filter(|account| account.person_id == record.id).map(|account| &account.email),"date_of_birth":record.date_of_birth.map(|value|value.to_string()),"person_type":match record.person_type{1=>"minor",2=>"dependent_adult",_=>"adult"},"has_capacity":record.has_capacity,"age":record.date_of_birth.map(|date|age(date,today)),"notification_preference_id":preference.map(|row|row.id),"notification_preference_portable_id":preference.map(|row|&row.portable_id)})}).collect())
}
