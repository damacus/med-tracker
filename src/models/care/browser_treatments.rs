use crate::models::{
    access::{self, PersonAccess, TenantTransaction},
    care::{people, treatments::projection},
    entities::{dosage, medication, person, person_medication, schedule},
    errors::OperationError,
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};
use serde_json::{Value, json};

pub async fn index(
    tenant: &TenantTransaction,
    id: &str,
    page: i64,
    zone: chrono_tz::Tz,
) -> Result<Value, OperationError> {
    if page < 1 {
        return Err(OperationError::Validation {
            details: json!({"errors":{"page":["must be positive"]}}),
        });
    }
    let (person_data, _) = people::read(tenant, id, zone).await?;
    let person_id = person_data["data"]["id"]
        .as_str()
        .and_then(|value| value.parse::<i64>().ok())
        .or_else(|| person_data["data"]["id"].as_i64())
        .ok_or(OperationError::Unavailable)?;
    let schedule_query =
        access::schedule_scope(tenant).filter(schedule::Column::PersonId.eq(person_id));
    let assignment_query = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(person_medication::Column::PersonId.eq(person_id))
        .filter(person_medication::Column::RetiredAt.is_null());
    let schedule_count = schedule_query.clone().count(tenant.transaction()).await?;
    let assignment_count = assignment_query.clone().count(tenant.transaction()).await?;
    let offset = page.saturating_sub(1).saturating_mul(20) as u64;
    let schedules = schedule_query
        .order_by_asc(schedule::Column::Id)
        .limit(20)
        .offset(offset)
        .all(tenant.transaction())
        .await?;
    let assignments = assignment_query
        .order_by_asc(person_medication::Column::Id)
        .limit(20)
        .offset(offset)
        .all(tenant.transaction())
        .await?;
    let schedules =
        projection::serialize_schedules(tenant.transaction(), tenant, schedules).await?;
    let assignments =
        projection::serialize_assignments(tenant.transaction(), tenant, assignments).await?;
    let can_manage = access::can_access_person(tenant, person_id, PersonAccess::Manage).await?;
    let medications = super::browser_query::index(tenant).await?;
    Ok(
        json!({"person":person_data["data"],"schedules":schedules,"assignments":assignments,"medications":medications,"can_manage":can_manage,"page":page,"has_next":schedule_count.max(assignment_count)>(page as u64).saturating_mul(20)}),
    )
}

pub async fn options(tenant: &TenantTransaction, person_id: &str) -> Result<Value, OperationError> {
    access::recheck(tenant).await?;
    let query =
        person::Entity::find().filter(person::Column::HouseholdId.eq(tenant.scope().household_id));
    let query = if let Ok(id) = person_id.parse::<i64>() {
        query.filter(person::Column::Id.eq(id))
    } else {
        query.filter(person::Column::PortableId.eq(person_id))
    };
    let row = query
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    access::require_person_access(tenant, row.id, PersonAccess::Manage).await?;
    let medications = access::medication_scope(tenant)
        .order_by_asc(medication::Column::Name)
        .all(tenant.transaction())
        .await?;
    let ids: Vec<i64> = medications.iter().map(|row| row.id).collect();
    let dosages = dosage::Entity::find()
        .filter(dosage::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(dosage::Column::MedicationId.is_in(ids))
        .order_by_asc(dosage::Column::Id)
        .all(tenant.transaction())
        .await?;
    Ok(
        json!({"person_id":row.id,"person_name":row.name,"medications":medications.iter().map(|row| json!({"id":row.id,"name":row.friendly_name.as_deref().filter(|name| !name.is_empty()).or(row.name.as_deref()).unwrap_or("Medication")})).collect::<Vec<_>>(),"dosages":dosages.iter().map(|row|json!({"id":row.id,"medication_id":row.medication_id,"amount":row.amount.normalize().to_string(),"unit":row.unit})).collect::<Vec<_>>()}),
    )
}

pub async fn source_option(
    tenant: &TenantTransaction,
    is_schedule: bool,
    id: i64,
) -> Result<Option<i64>, OperationError> {
    if is_schedule {
        Ok(schedule::Entity::find_by_id(id)
            .filter(schedule::Column::HouseholdId.eq(tenant.scope().household_id))
            .one(tenant.transaction())
            .await?
            .ok_or(OperationError::NotFound)?
            .source_dosage_option_id)
    } else {
        Ok(person_medication::Entity::find_by_id(id)
            .filter(person_medication::Column::HouseholdId.eq(tenant.scope().household_id))
            .one(tenant.transaction())
            .await?
            .ok_or(OperationError::NotFound)?
            .source_dosage_option_id)
    }
}
