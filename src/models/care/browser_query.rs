use crate::models::{
    access::{self, PersonAccess, TenantTransaction},
    authorization,
    care::medications::{self, StockSnapshot},
    entities::{
        dosage, grant, location, medication, medication_take, person, person_medication, schedule,
    },
    errors::OperationError,
};
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, QueryTrait, Statement,
};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
pub struct Assignment {
    pub id: i64,
    pub source_type: &'static str,
    pub person_name: String,
    pub amount: String,
    pub unit: String,
    pub can_record: bool,
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
        .filter(person_medication::Column::HouseholdId.eq(tenant.scope().household_id))
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
        .filter(location::Column::HouseholdId.eq(household_id))
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?
        .name;
    let dose_options = dosage::Entity::find()
        .filter(dosage::Column::HouseholdId.eq(household_id))
        .filter(dosage::Column::MedicationId.eq(stock.medication.id))
        .order_by_asc(dosage::Column::Id)
        .all(tenant.transaction())
        .await?;
    let visible_people = access::granted_people(tenant.membership());
    let linked_schedules = schedule::Entity::find()
        .select_only()
        .column(schedule::Column::Id)
        .filter(schedule::Column::HouseholdId.eq(household_id))
        .filter(schedule::Column::MedicationId.eq(stock.medication.id))
        .filter(schedule::Column::PersonId.in_subquery(visible_people.clone()))
        .into_query();
    let linked_assignments = person_medication::Entity::find()
        .select_only()
        .column(person_medication::Column::Id)
        .filter(person_medication::Column::HouseholdId.eq(household_id))
        .filter(person_medication::Column::MedicationId.eq(stock.medication.id))
        .filter(person_medication::Column::PersonId.in_subquery(visible_people))
        .into_query();
    let takes = medication_take::Entity::find()
        .filter(medication_take::Column::HouseholdId.eq(household_id))
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
        .filter(schedule::Column::HouseholdId.eq(household_id))
        .filter(schedule::Column::Id.is_in(takes.iter().filter_map(|take| take.schedule_id)))
        .all(tenant.transaction())
        .await?
        .into_iter()
        .map(|source| (source.id, source.person_id))
        .collect();
    let history_assignments: HashMap<i64, i64> = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(household_id))
        .filter(
            person_medication::Column::Id
                .is_in(takes.iter().filter_map(|take| take.person_medication_id)),
        )
        .all(tenant.transaction())
        .await?
        .into_iter()
        .map(|source| (source.id, source.person_id))
        .collect();
    let history_people: HashMap<i64, String> = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(household_id))
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
        .filter(person_medication::Column::HouseholdId.eq(household_id))
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
    let people: HashMap<i64, person::Model> = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(household_id))
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
    let permission_grants: HashMap<i64, grant::Model> = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(tenant.membership().id))
        .filter(grant::Column::PersonId.is_in(people.keys().copied()))
        .filter(grant::Column::RevokedAt.is_null())
        .all(tenant.transaction())
        .await?
        .into_iter()
        .map(|record| (record.person_id, record))
        .collect();
    let clock = tenant
        .transaction()
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT timezone('UTC', clock_timestamp()) AS now",
        ))
        .await?
        .ok_or(OperationError::Unavailable)?;
    let now: chrono::NaiveDateTime = clock.try_get("", "now")?;
    let permissions: HashMap<i64, (bool, bool)> = people
        .iter()
        .map(|(person_id, subject)| {
            let view = permission_grants.get(person_id).is_some_and(|record| {
                authorization::person_access(
                    tenant.membership(),
                    subject,
                    record,
                    PersonAccess::View,
                    now,
                )
            });
            let record = view
                && permission_grants.get(person_id).is_some_and(|record| {
                    authorization::person_access(
                        tenant.membership(),
                        subject,
                        record,
                        PersonAccess::Record,
                        now,
                    )
                });
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
            source_type: "person_medication",
            person_name: subject.name.clone(),
            amount: amount.normalize().to_string(),
            unit,
            can_record: permission.1,
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
        let (Some(subject), Some(amount), Some(unit)) = (
            people.get(&record.person_id),
            record.dose_amount,
            record.dose_unit,
        ) else {
            continue;
        };
        assignments.push(Assignment {
            id: record.id,
            source_type: "schedule",
            person_name: subject.name.clone(),
            amount: amount.normalize().to_string(),
            unit,
            can_record: permission.1,
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
