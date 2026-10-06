use super::*;
use chrono::Datelike;
use sea_orm::{IntoActiveModel, PaginatorTrait, QueryOrder};
use serde::Deserialize;

#[derive(Deserialize, Default)]
pub struct Pagination {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub updated_since: Option<String>,
}

pub async fn list(tenant: &TenantTransaction, page: Pagination) -> Result<Value, OperationError> {
    access::recheck(tenant).await?;
    let person_id = tenant
        .membership()
        .person_id
        .ok_or(OperationError::Forbidden)?;
    let actor = person::Entity::find_by_id(person_id)
        .filter(person::Column::HouseholdId.eq(tenant.scope().household_id))
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Forbidden)?;
    let today = Utc::now()
        .with_timezone(&crate::models::care::doses::app_zone())
        .date_naive();
    let age = actor.date_of_birth.map(|birth| {
        today.year()
            - birth.year()
            - i32::from((today.month(), today.day()) < (birth.month(), birth.day()))
    });
    if actor.person_type != 0 && !age.is_some_and(|age| age >= 18) {
        return Err(OperationError::Forbidden);
    }
    let page_number = page.page.unwrap_or(1);
    let per_page = page.per_page.unwrap_or(20);
    if page_number < 1 || !(1..=100).contains(&per_page) {
        return Err(failure(InputFailure::Invalid("pagination", "is invalid")));
    }
    let mut query = access::schedule_scope(tenant);
    if let Some(value) = page.updated_since {
        let timestamp = chrono::DateTime::parse_from_rfc3339(&value)
            .map(|value| value.naive_utc())
            .map_err(|_| failure(InputFailure::Invalid("updated_since", "is invalid")))?;
        query = query.filter(schedule::Column::UpdatedAt.gte(timestamp));
    }
    let count = query.clone().count(tenant.transaction()).await?;
    let rows = query
        .order_by_asc(schedule::Column::Id)
        .limit(per_page as u64)
        .offset(page_number.saturating_sub(1).saturating_mul(per_page) as u64)
        .all(tenant.transaction())
        .await?;
    let data = super::projection::serialize_schedules(tenant.transaction(), tenant, rows).await?;
    Ok(json!({"data":data,"meta":{"page":page_number,"per_page":per_page,"total_count":count}}))
}
async fn find(
    tenant: &TenantTransaction,
    id: &str,
    lock: bool,
) -> Result<schedule::Model, OperationError> {
    access::recheck(tenant).await?;
    let query = access::schedule_scope(tenant);
    let query = if let Ok(id) = id.parse::<i64>() {
        query.filter(schedule::Column::Id.eq(id))
    } else {
        query.filter(schedule::Column::PortableId.eq(id))
    };
    let query = if lock { query.lock_exclusive() } else { query };
    let row = query
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    if !person_access(tenant.transaction(), tenant, row.person_id, false).await? {
        return Err(OperationError::NotFound);
    }
    Ok(row)
}
pub async fn read(tenant: &TenantTransaction, id: &str) -> Result<(Value, String), OperationError> {
    project(tenant, find(tenant, id, false).await?).await
}
pub(super) async fn project(
    tenant: &TenantTransaction,
    row: schedule::Model,
) -> Result<(Value, String), OperationError> {
    let data =
        super::projection::serialize_schedules(tenant.transaction(), tenant, vec![row]).await?;
    let body = json!({"data":data.into_iter().next().ok_or(OperationError::NotFound)?});
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
pub async fn authorize_update(tenant: &TenantTransaction, id: &str) -> Result<(), OperationError> {
    household::Entity::find_by_id(tenant.scope().household_id)
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    let row = find(tenant, id, true).await?;
    access::require_person_access(tenant, row.person_id, PersonAccess::Manage).await
}
pub async fn update(
    tenant: &TenantTransaction,
    id: &str,
    body: &Value,
    etag: Option<&str>,
    provenance: Option<&CredentialProvenance>,
) -> Result<(Value, String), OperationError> {
    authorize_update(tenant, id).await?;
    let row = find(tenant, id, true).await?;
    let before = project(tenant, row.clone()).await?;
    let snapshot = representation(&row).0["data"].clone();
    if etag.is_some_and(|value| !value.is_empty() && value != before.1) {
        return Err(OperationError::Conflict {
            code: "conflict".into(),
            details: json!({"error":"Record has changed since it was last read"}),
        });
    }
    let attributes = input::attributes(body).map_err(failure)?;
    let mut fields = fields_from_record(&row);
    input::apply_attributes(
        tenant.transaction(),
        tenant,
        attributes,
        &mut fields,
        Some(&row),
    )
    .await?
    .map_err(failure)?;
    fields.validate().map_err(failure)?;
    if fields == fields_from_record(&row) {
        return Ok(before);
    }
    let mut active = row.into_active_model();
    fields.assign(&mut active);
    active.updated_at = Set(Utc::now().naive_utc());
    let row = active.update(tenant.transaction()).await?;
    persist(tenant, &row, Some(snapshot), "update", provenance).await?;
    project(tenant, row).await
}

fn fields_from_record(row: &schedule::Model) -> ScheduleFields {
    ScheduleFields {
        person_id: Some(row.person_id),
        medication_id: Some(row.medication_id),
        source_dosage_option_id: row.source_dosage_option_id,
        dose_amount: row.dose_amount,
        dose_unit: row.dose_unit.clone(),
        frequency: row.frequency.clone(),
        start_date: row.start_date,
        end_date: row.end_date,
        notes: row.notes.clone(),
        max_daily_doses: row.max_daily_doses,
        min_hours_between_doses: row.min_hours_between_doses,
        dose_cycle: row.dose_cycle,
        schedule_type: row.schedule_type,
        schedule_config: row.schedule_config.clone(),
    }
}
