use crate::models::{
    access::{self, PersonAccess, TenantTransaction},
    care::medications::{self, StockSnapshot},
    entities::{dosage, location, medication, person, person_medication},
    errors::OperationError,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
pub struct Assignment {
    pub id: i64,
    pub person_name: String,
    pub amount: String,
    pub unit: String,
    pub can_record: bool,
}

pub struct Detail {
    pub stock: StockSnapshot,
    pub assignments: Vec<Assignment>,
    pub can_adjust: bool,
}

#[derive(Serialize)]
pub struct LocationChoice {
    pub id: i64,
    pub name: String,
}

pub struct FormContext {
    pub locations: Vec<LocationChoice>,
    pub snapshot: Option<StockSnapshot>,
    pub options_mode: bool,
}

pub async fn form_context(
    tenant: &TenantTransaction,
    id: Option<&str>,
) -> Result<FormContext, OperationError> {
    access::recheck(tenant).await?;
    let household_id = tenant.scope().household_id;
    let locations = location::Entity::find()
        .filter(location::Column::HouseholdId.eq(household_id))
        .order_by_asc(location::Column::Name)
        .order_by_asc(location::Column::Id)
        .all(tenant.transaction())
        .await?
        .into_iter()
        .map(|record| LocationChoice {
            id: record.id,
            name: record.name,
        })
        .collect();
    let snapshot = match id {
        Some(id) => Some(medications::read_stock_snapshot(tenant, id).await?),
        None => None,
    };
    let options_mode = match &snapshot {
        Some(snapshot) => dosage::Entity::find()
            .filter(dosage::Column::HouseholdId.eq(household_id))
            .filter(dosage::Column::MedicationId.eq(snapshot.medication.id))
            .one(tenant.transaction())
            .await?
            .is_some(),
        None => false,
    };
    Ok(FormContext {
        locations,
        snapshot,
        options_mode,
    })
}

#[derive(Serialize)]
pub struct MedicationCard {
    pub id: i64,
    pub name: String,
    pub quantity: Option<String>,
    pub unit: String,
}

pub async fn index(tenant: &TenantTransaction) -> Result<Vec<MedicationCard>, OperationError> {
    access::recheck(tenant).await?;
    let records = access::medication_scope(tenant)
        .order_by_asc(medication::Column::Name)
        .order_by_asc(medication::Column::Id)
        .all(tenant.transaction())
        .await?;
    Ok(records
        .into_iter()
        .map(|record| MedicationCard {
            id: record.id,
            name: record
                .friendly_name
                .filter(|name| !name.is_empty())
                .or(record.name)
                .unwrap_or_else(|| "Medication".into()),
            quantity: record
                .current_supply
                .map(|value| value.normalize().to_string()),
            unit: record.dose_unit.unwrap_or_default(),
        })
        .collect())
}

pub async fn detail(tenant: &TenantTransaction, id: &str) -> Result<Detail, OperationError> {
    let stock = medications::read_stock_snapshot(tenant, id).await?;
    let household_id = tenant.scope().household_id;
    let records = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(household_id))
        .filter(person_medication::Column::MedicationId.eq(stock.medication.id))
        .filter(person_medication::Column::Active.eq(true))
        .filter(person_medication::Column::RetiredAt.is_null())
        .order_by_asc(person_medication::Column::Position)
        .order_by_asc(person_medication::Column::Id)
        .all(tenant.transaction())
        .await?;
    let people: HashMap<i64, String> = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(household_id))
        .filter(person::Column::Id.is_in(records.iter().map(|record| record.person_id)))
        .all(tenant.transaction())
        .await?
        .into_iter()
        .map(|person| (person.id, person.name))
        .collect();
    let mut permissions = HashMap::new();
    let mut assignments = Vec::new();
    for record in records {
        let permission = match permissions.get(&record.person_id) {
            Some(permission) => *permission,
            None => {
                let visible = permitted(tenant, record.person_id, PersonAccess::View).await?;
                let recordable =
                    visible && permitted(tenant, record.person_id, PersonAccess::Record).await?;
                permissions.insert(record.person_id, (visible, recordable));
                (visible, recordable)
            }
        };
        if !permission.0 {
            continue;
        }
        let Some(name) = people.get(&record.person_id) else {
            continue;
        };
        let (Some(amount), Some(unit)) = (record.dose_amount, record.dose_unit) else {
            continue;
        };
        assignments.push(Assignment {
            id: record.id,
            person_name: name.clone(),
            amount: amount.normalize().to_string(),
            unit,
            can_record: permission.1,
        });
    }
    Ok(Detail {
        stock,
        assignments,
        can_adjust: can_adjust(tenant),
    })
}

pub fn can_adjust(tenant: &TenantTransaction) -> bool {
    matches!(tenant.membership().role.as_str(), "owner" | "administrator")
}

async fn permitted(
    tenant: &TenantTransaction,
    person_id: i64,
    level: PersonAccess,
) -> Result<bool, OperationError> {
    match access::require_person_access(tenant, person_id, level).await {
        Ok(()) => Ok(true),
        Err(OperationError::Forbidden | OperationError::NotFound) => Ok(false),
        Err(error) => Err(error),
    }
}
