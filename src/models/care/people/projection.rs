use super::*;
use crate::views::{
    PaginationMeta,
    people::{Person, PersonCollectionResponse, PersonResponse},
};
use loco_rs::model::TenantQueryExt;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_people_response_preserves_the_canonical_etag() -> Result<(), OperationError> {
        use sha2::{Digest, Sha256};
        let person = PersonResponse {
            data: Person {
                id: 12,
                portable_id: "portable-person".into(),
                updated_at: "2026-11-23T10:11:12Z".into(),
                name: "Synthetic person".into(),
                email: None,
                date_of_birth: None,
                person_type: PersonKind::Adult,
                has_capacity: true,
                age: None,
                location_ids: vec![24],
                location_portable_ids: vec!["portable-location".into()],
                notification_preference_id: None,
                notification_preference_portable_id: None,
            },
        };
        let mut legacy = json!({"data": {
            "id": 12, "portable_id": "portable-person", "updated_at": "2026-11-23T10:11:12Z",
            "name": "Synthetic person", "email": null, "date_of_birth": null, "person_type": "adult",
            "has_capacity": true, "age": null, "location_ids": [24],
            "location_portable_ids": ["portable-location"], "notification_preference_id": null,
            "notification_preference_portable_id": null
        }});
        legacy.sort_all_objects();
        assert_eq!(
            serde_json::to_value(&person).map_err(|_| OperationError::Unavailable)?,
            legacy
        );
        assert_eq!(
            crate::views::canonical_etag(&person)?,
            format!(
                "\"{}\"",
                hex::encode(Sha256::digest(legacy.to_string().as_bytes()))
            )
        );
        Ok(())
    }
}

#[derive(Default, serde::Deserialize)]
pub struct Pagination {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub updated_since: Option<String>,
}

fn scope(tenant: &TenantTransaction) -> sea_orm::Select<person::Entity> {
    access::person_queries::household_people(tenant.scope())
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
) -> Result<(PersonResponse, String), OperationError> {
    let record = selected(tenant, id).await?;
    representation(tenant, &record, zone).await
}

pub async fn list(
    tenant: &TenantTransaction,
    page: Pagination,
    zone: Tz,
) -> Result<PersonCollectionResponse, OperationError> {
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
    Ok(PersonCollectionResponse {
        data: values(tenant, &rows, zone).await?,
        meta: PaginationMeta {
            page: number,
            per_page: size,
            total_count: total,
        },
    })
}

pub async fn representation(
    tenant: &TenantTransaction,
    record: &person::Model,
    zone: Tz,
) -> Result<(PersonResponse, String), OperationError> {
    let body = PersonResponse {
        data: values(tenant, std::slice::from_ref(record), zone)
            .await?
            .remove(0),
    };
    let etag = crate::views::canonical_etag(&body)?;
    Ok((body, etag))
}

pub(crate) async fn values(
    tenant: &TenantTransaction,
    records: &[person::Model],
    zone: Tz,
) -> Result<Vec<Person>, OperationError> {
    values_in(tenant, tenant.scope().household_id, records, zone).await
}

pub(crate) async fn values_in(
    tenant: &TenantTransaction,
    household_id: i64,
    records: &[person::Model],
    zone: Tz,
) -> Result<Vec<Person>, OperationError> {
    if records.is_empty() {
        return Ok(vec![]);
    }
    let ids: Vec<i64> = records.iter().map(|row| row.id).collect();
    let links = location_membership::Entity::find()
        .in_tenant(household_id)
        .filter(location_membership::Column::PersonId.is_in(ids.clone()))
        .order_by_asc(location_membership::Column::Id)
        .all(tenant.transaction())
        .await?;
    let location_ids: Vec<i64> = links.iter().map(|row| row.location_id).collect();
    let locations: HashMap<i64, String> = if location_ids.is_empty() {
        HashMap::new()
    } else {
        location::Entity::find()
            .in_tenant(household_id)
            .filter(location::Column::Id.is_in(location_ids))
            .all(tenant.transaction())
            .await?
            .into_iter()
            .map(|row| (row.id, row.portable_id))
            .collect()
    };
    let preferences: HashMap<i64, notification_preference::Model> =
        notification_preference::Entity::find()
            .in_tenant(household_id)
            .filter(notification_preference::Column::PersonId.is_in(ids))
            .all(tenant.transaction())
            .await?
            .into_iter()
            .map(|row| (row.person_id, row))
            .collect();
    let today = Utc::now().with_timezone(&zone).date_naive();
    Ok(records
        .iter()
        .map(|record| {
            let location_ids: Vec<i64> = links
                .iter()
                .filter(|row| row.person_id == record.id)
                .map(|row| row.location_id)
                .collect();
            let location_portable_ids = location_ids
                .iter()
                .filter_map(|id| locations.get(id).cloned())
                .collect();
            let preference = preferences.get(&record.id);
            Person {
                id: record.id,
                portable_id: record.portable_id.clone(),
                updated_at: record.updated_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                name: record.name.clone(),
                email: record.email.clone(),
                date_of_birth: record.date_of_birth.map(|date| date.to_string()),
                person_type: PersonKind::from_code(record.person_type),
                has_capacity: record.has_capacity,
                age: record.date_of_birth.map(|date| age(date, today)),
                location_ids,
                location_portable_ids,
                notification_preference_id: preference.map(|row| row.id),
                notification_preference_portable_id: preference.map(|row| row.portable_id.clone()),
            }
        })
        .collect())
}
