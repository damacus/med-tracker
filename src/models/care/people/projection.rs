use super::*;

#[derive(Default, serde::Deserialize)]
pub struct Pagination {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub updated_since: Option<String>,
}

fn scope(tenant: &TenantTransaction) -> sea_orm::Select<person::Entity> {
    person::Entity::find()
        .filter(person::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(
            person::Column::Id.in_subquery(
                active_grants(tenant)
                    .filter(grant::Column::AccessLevel.is_in(["view", "record", "manage"]))
                    .select_only()
                    .column(grant::Column::PersonId)
                    .into_query(),
            ),
        )
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
    let size = std::cmp::min(page.per_page.unwrap_or(20), 100);
    if number < 1 || size < 1 {
        return Err(invalid(
            "page",
            "must be positive and per_page must be positive",
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

async fn values(
    tenant: &TenantTransaction,
    records: &[person::Model],
    zone: Tz,
) -> Result<Vec<Value>, OperationError> {
    if records.is_empty() {
        return Ok(vec![]);
    }
    let ids: Vec<i64> = records.iter().map(|row| row.id).collect();
    let links = location_membership::Entity::find()
        .filter(location_membership::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(location_membership::Column::PersonId.is_in(ids.clone()))
        .order_by_asc(location_membership::Column::Id)
        .all(tenant.transaction())
        .await?;
    let location_ids: Vec<i64> = links.iter().map(|row| row.location_id).collect();
    let locations: HashMap<i64, String> = if location_ids.is_empty() {
        HashMap::new()
    } else {
        location::Entity::find()
            .filter(location::Column::HouseholdId.eq(tenant.scope().household_id))
            .filter(location::Column::Id.is_in(location_ids))
            .all(tenant.transaction())
            .await?
            .into_iter()
            .map(|row| (row.id, row.portable_id))
            .collect()
    };
    let preferences: HashMap<i64, notification_preference::Model> =
        notification_preference::Entity::find()
            .filter(notification_preference::Column::HouseholdId.eq(tenant.scope().household_id))
            .filter(notification_preference::Column::PersonId.is_in(ids))
            .all(tenant.transaction())
            .await?
            .into_iter()
            .map(|row| (row.person_id, row))
            .collect();
    let today = Utc::now().with_timezone(&zone).date_naive();
    Ok(records.iter().map(|record|{let location_ids:Vec<i64>=links.iter().filter(|row|row.person_id==record.id).map(|row|row.location_id).collect();let portable:Vec<&String>=location_ids.iter().filter_map(|id|locations.get(id)).collect();let preference=preferences.get(&record.id);json!({"id":record.id,"portable_id":record.portable_id,"updated_at":record.updated_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),"name":record.name,"email":record.email,"date_of_birth":record.date_of_birth.map(|value|value.to_string()),"person_type":match record.person_type{1=>"minor",2=>"dependent_adult",_=>"adult"},"has_capacity":record.has_capacity,"age":record.date_of_birth.map(|date|age(date,today)),"location_ids":location_ids,"location_portable_ids":portable,"notification_preference_id":preference.map(|row|row.id),"notification_preference_portable_id":preference.map(|row|&row.portable_id)})}).collect())
}
