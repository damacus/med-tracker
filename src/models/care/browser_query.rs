use crate::models::{
    access::{self, PersonAccess, TenantTransaction},
    authorization,
    care::{
        doses,
        medications::{self, StockSnapshot},
    },
    entities::{
        dosage, location, medication, medication_take, person, person_medication, schedule,
    },
    errors::OperationError,
};
use loco_rs::model::TenantQueryExt;
use sea_orm::{
    ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder, QuerySelect, QueryTrait,
};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
pub struct Assignment {
    pub id: i64,
    pub person_id: i64,
    pub source_type: &'static str,
    pub person_name: String,
    pub amount: String,
    pub unit: String,
    pub can_record: bool,
    pub available: bool,
}

pub struct Detail {
    pub stock: StockSnapshot,
    pub assignments: Vec<Assignment>,
    pub dose_options: Vec<dosage::Model>,
    pub dose_history: Vec<DoseHistory>,
    pub can_adjust: bool,
    pub location_name: String,
}

pub struct DoseHistory {
    pub person_name: String,
    pub amount: Option<String>,
    pub unit: Option<String>,
    pub taken_at: Option<chrono::NaiveDateTime>,
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
        .in_tenant(household_id)
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
            .in_tenant(household_id)
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
    pub location_id: i64,
    pub name: String,
    pub quantity: Option<String>,
    pub unit: String,
}

#[derive(Serialize)]
pub struct InventoryCard {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub location_name: String,
    pub category: Option<String>,
    pub quantity: Option<String>,
    pub unit: String,
    pub low_stock: bool,
    pub out_of_stock: bool,
}

pub struct Inventory {
    pub medications: Vec<InventoryCard>,
    pub locations: Vec<LocationChoice>,
    pub categories: Vec<String>,
}

pub async fn inventory(
    tenant: &TenantTransaction,
    location_id: Option<i64>,
    category: Option<&str>,
) -> Result<Inventory, OperationError> {
    access::recheck(tenant).await?;
    let locations = location::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .order_by_asc(location::Column::Name)
        .all(tenant.transaction())
        .await?;
    let location_names: HashMap<_, _> = locations
        .iter()
        .map(|row| (row.id, row.name.as_str()))
        .collect();
    let all_visible = access::medication_scope(tenant)
        .all(tenant.transaction())
        .await?;
    let mut categories = all_visible
        .iter()
        .filter_map(|row| {
            row.category
                .as_deref()
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
        })
        .collect::<Vec<_>>();
    categories.sort();
    categories.dedup();
    let mut query = access::medication_scope(tenant);
    if let Some(id) = location_id {
        query = query.filter(medication::Column::LocationId.eq(id));
    }
    if let Some(category) = category.filter(|value| !value.is_empty()) {
        query = query.filter(medication::Column::Category.eq(category));
    }
    let medications = query
        .order_by_asc(medication::Column::Name)
        .order_by_asc(medication::Column::Id)
        .all(tenant.transaction())
        .await?
        .into_iter()
        .map(|row| {
            let out_of_stock = row
                .current_supply
                .is_some_and(|quantity| quantity <= 0.into());
            let low_stock = row
                .current_supply
                .is_some_and(|quantity| quantity <= row.reorder_threshold);
            InventoryCard {
                id: row.id,
                name: row
                    .friendly_name
                    .as_deref()
                    .filter(|value| !value.is_empty())
                    .or(row.name.as_deref())
                    .unwrap_or("Medication")
                    .to_owned(),
                description: row.description,
                location_name: location_names
                    .get(&row.location_id)
                    .copied()
                    .unwrap_or("Location")
                    .to_owned(),
                category: row.category,
                quantity: row
                    .current_supply
                    .map(|value| value.normalize().to_string()),
                unit: row.dose_unit.unwrap_or_default(),
                low_stock,
                out_of_stock,
            }
        })
        .collect();
    Ok(Inventory {
        medications,
        locations: locations
            .into_iter()
            .map(|row| LocationChoice {
                id: row.id,
                name: row.name,
            })
            .collect(),
        categories,
    })
}

pub async fn index(tenant: &TenantTransaction) -> Result<Vec<MedicationCard>, OperationError> {
    medication_cards(tenant, None).await
}

pub async fn index_at_location(
    tenant: &TenantTransaction,
    location_id: i64,
) -> Result<Vec<MedicationCard>, OperationError> {
    medication_cards(tenant, Some(location_id)).await
}

async fn medication_cards(
    tenant: &TenantTransaction,
    location_id: Option<i64>,
) -> Result<Vec<MedicationCard>, OperationError> {
    access::recheck(tenant).await?;
    let mut query = access::medication_scope(tenant);
    if let Some(location_id) = location_id {
        query = query.filter(medication::Column::LocationId.eq(location_id));
    }
    let records = query
        .order_by_asc(medication::Column::Name)
        .order_by_asc(medication::Column::Id)
        .all(tenant.transaction())
        .await?;
    Ok(records.into_iter().map(medication_card).collect())
}

pub async fn person_medications(
    tenant: &TenantTransaction,
    person_id: i64,
) -> Result<Vec<MedicationCard>, OperationError> {
    access::require_person_access(tenant, person_id, PersonAccess::View).await?;
    let assignments = person_medication::Entity::find()
        .select_only()
        .column(person_medication::Column::MedicationId)
        .in_tenant(tenant.scope().household_id)
        .filter(person_medication::Column::PersonId.eq(person_id))
        .filter(person_medication::Column::Active.eq(true))
        .filter(person_medication::Column::RetiredAt.is_null())
        .into_query();
    let records = access::medication_scope(tenant)
        .filter(medication::Column::Id.in_subquery(assignments))
        .order_by_asc(medication::Column::Name)
        .order_by_asc(medication::Column::Id)
        .all(tenant.transaction())
        .await?;
    Ok(records.into_iter().map(medication_card).collect())
}

fn medication_card(record: medication::Model) -> MedicationCard {
    MedicationCard {
        id: record.id,
        location_id: record.location_id,
        name: record
            .friendly_name
            .filter(|name| !name.is_empty())
            .or(record.name)
            .unwrap_or_else(|| "Medication".into()),
        quantity: record
            .current_supply
            .map(|value| value.normalize().to_string()),
        unit: record.dose_unit.unwrap_or_default(),
    }
}

pub async fn detail(
    tenant: &TenantTransaction,
    id: &str,
    zone: chrono_tz::Tz,
) -> Result<Detail, OperationError> {
    let stock = medications::read_stock_snapshot(tenant, id).await?;
    let household_id = tenant.scope().household_id;
    let location_name = location::Entity::find_by_id(stock.medication.location_id)
        .in_tenant(household_id)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?
        .name;
    let dose_options = dosage::Entity::find()
        .in_tenant(household_id)
        .filter(dosage::Column::MedicationId.eq(stock.medication.id))
        .order_by_asc(dosage::Column::Id)
        .all(tenant.transaction())
        .await?;
    let visible_people = access::participating_people(tenant.membership());
    let linked_schedules = schedule::Entity::find()
        .select_only()
        .column(schedule::Column::Id)
        .in_tenant(household_id)
        .filter(schedule::Column::MedicationId.eq(stock.medication.id))
        .filter(schedule::Column::PersonId.in_subquery(visible_people.clone()))
        .into_query();
    let linked_assignments = person_medication::Entity::find()
        .select_only()
        .column(person_medication::Column::Id)
        .in_tenant(household_id)
        .filter(person_medication::Column::MedicationId.eq(stock.medication.id))
        .filter(person_medication::Column::PersonId.in_subquery(visible_people))
        .into_query();
    let takes = medication_take::Entity::find()
        .in_tenant(household_id)
        .filter(
            Condition::any()
                .add(medication_take::Column::ScheduleId.in_subquery(linked_schedules))
                .add(medication_take::Column::PersonMedicationId.in_subquery(linked_assignments)),
        )
        .order_by_desc(medication_take::Column::TakenAt)
        .order_by_desc(medication_take::Column::Id)
        .limit(5)
        .all(tenant.transaction())
        .await?;
    let history_schedules: HashMap<i64, i64> = schedule::Entity::find()
        .in_tenant(household_id)
        .filter(schedule::Column::Id.is_in(takes.iter().filter_map(|take| take.schedule_id)))
        .all(tenant.transaction())
        .await?
        .into_iter()
        .map(|source| (source.id, source.person_id))
        .collect();
    let history_assignments: HashMap<i64, i64> = person_medication::Entity::find()
        .in_tenant(household_id)
        .filter(
            person_medication::Column::Id
                .is_in(takes.iter().filter_map(|take| take.person_medication_id)),
        )
        .all(tenant.transaction())
        .await?
        .into_iter()
        .map(|source| (source.id, source.person_id))
        .collect();
    let history_people: HashMap<i64, String> =
        crate::models::access::person_queries::household_participants(household_id)
            .filter(
                person::Column::Id.is_in(
                    history_schedules
                        .values()
                        .chain(history_assignments.values())
                        .copied(),
                ),
            )
            .all(tenant.transaction())
            .await?
            .into_iter()
            .map(|person| (person.id, person.name))
            .collect();
    let dose_history = takes
        .into_iter()
        .filter_map(|take| {
            let person_id = take
                .schedule_id
                .and_then(|id| history_schedules.get(&id))
                .or_else(|| {
                    take.person_medication_id
                        .and_then(|id| history_assignments.get(&id))
                })?;
            Some(DoseHistory {
                person_name: history_people.get(person_id)?.clone(),
                amount: take.dose_amount.map(|value| value.normalize().to_string()),
                unit: take.dose_unit,
                taken_at: take.taken_at,
            })
        })
        .collect();
    let records = person_medication::Entity::find()
        .in_tenant(household_id)
        .filter(person_medication::Column::MedicationId.eq(stock.medication.id))
        .filter(person_medication::Column::Active.eq(true))
        .filter(person_medication::Column::RetiredAt.is_null())
        .order_by_asc(person_medication::Column::Position)
        .order_by_asc(person_medication::Column::Id)
        .all(tenant.transaction())
        .await?;
    let today = chrono::Utc::now().with_timezone(&zone).date_naive();
    let schedules = access::schedule_scope(tenant)
        .filter(schedule::Column::MedicationId.eq(stock.medication.id))
        .filter(schedule::Column::Active.eq(true))
        .filter(schedule::Column::StartDate.lte(today))
        .filter(schedule::Column::EndDate.gte(today))
        .order_by_asc(schedule::Column::Id)
        .all(tenant.transaction())
        .await?;
    let people: HashMap<i64, person::Model> =
        crate::models::access::person_queries::household_participants(household_id)
            .filter(
                person::Column::Id.is_in(
                    records
                        .iter()
                        .map(|record| record.person_id)
                        .chain(schedules.iter().map(|record| record.person_id)),
                ),
            )
            .all(tenant.transaction())
            .await?
            .into_iter()
            .map(|person| (person.id, person))
            .collect();
    access::recheck(tenant).await?;
    let permissions: HashMap<i64, (bool, bool)> = people
        .iter()
        .map(|(person_id, subject)| {
            let view = authorization::participating_person_access(
                tenant.membership(),
                subject,
                PersonAccess::View,
            );
            let record = view
                && authorization::participating_person_access(
                    tenant.membership(),
                    subject,
                    PersonAccess::Record,
                );
            (*person_id, (view, record))
        })
        .collect();
    let mut assignments = Vec::new();
    for record in records {
        let permission = permissions
            .get(&record.person_id)
            .copied()
            .unwrap_or_default();
        if !permission.0 {
            continue;
        }
        let Some(subject) = people.get(&record.person_id) else {
            continue;
        };
        let (Some(amount), Some(unit)) = (record.dose_amount, record.dose_unit) else {
            continue;
        };
        assignments.push(Assignment {
            id: record.id,
            person_id: record.person_id,
            source_type: "person_medication",
            person_name: subject.name.clone(),
            amount: amount.normalize().to_string(),
            unit,
            can_record: permission.1,
            available: true,
        });
    }
    for record in schedules {
        let permission = permissions
            .get(&record.person_id)
            .copied()
            .unwrap_or_default();
        if !permission.0 {
            continue;
        }
        let preview = doses::browser_schedule_preview(record.clone(), today);
        let (Some(subject), Some(amount), Some(unit)) =
            (people.get(&record.person_id), preview.amount, preview.unit)
        else {
            continue;
        };
        assignments.push(Assignment {
            id: record.id,
            person_id: record.person_id,
            source_type: "schedule",
            person_name: subject.name.clone(),
            amount,
            unit,
            can_record: permission.1,
            available: preview.available,
        });
    }
    Ok(Detail {
        stock,
        assignments,
        dose_options,
        dose_history,
        can_adjust: can_adjust(tenant),
        location_name,
    })
}

pub fn can_adjust(tenant: &TenantTransaction) -> bool {
    access::can_manage_household(tenant)
}
