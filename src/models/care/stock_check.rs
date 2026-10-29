use crate::models::{
    access::{self, TenantTransaction},
    care::{dosages, locations, medications},
    entities::{dosage, medication},
    errors::OperationError,
    identity::credentials::CredentialProvenance,
};
use loco_rs::model::TenantQueryExt;
use sea_orm::prelude::Decimal;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::Serialize;
use serde_json::json;
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

#[derive(Serialize)]
pub struct Row {
    pub key: String,
    pub medication_id: i64,
    pub option_id: Option<i64>,
    pub medication_name: String,
    pub label: String,
    pub strength: Option<String>,
    pub location_name: String,
    pub quantity: Option<String>,
    pub snapshot: String,
    pub unit: String,
    pub adjustable: bool,
}

#[derive(Serialize)]
pub struct Location {
    pub id: i64,
    pub name: String,
}

pub struct Page {
    pub rows: Vec<Row>,
    pub locations: Vec<Location>,
}

pub struct SubmittedRow {
    pub key: String,
    pub snapshot: String,
    pub quantity: String,
}

fn invalid() -> OperationError {
    OperationError::Validation {
        details: json!({"quantity": "invalid"}),
    }
}

fn stale() -> OperationError {
    OperationError::Conflict {
        code: "stale_stock_check".into(),
        details: json!({}),
    }
}

pub async fn apply(
    tenant: &TenantTransaction,
    mut submitted: Vec<SubmittedRow>,
    reason: &str,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    if submitted.is_empty()
        || submitted.len() > 250
        || !matches!(
            reason,
            "house_stock_check" | "manual_correction" | "damaged_or_disposed" | "other"
        )
    {
        return Err(invalid());
    }
    medications::lock_household(tenant.transaction(), tenant.scope().household_id).await?;
    access::recheck(tenant).await?;
    if !access::can_manage_household(tenant) {
        return Err(OperationError::Forbidden);
    }
    let visible = read(tenant, None)
        .await?
        .rows
        .into_iter()
        .filter(|row| row.adjustable)
        .map(|row| (row.key.clone(), row))
        .collect::<HashMap<_, _>>();
    let mut seen = HashSet::new();
    let mut checked = Vec::new();
    for item in submitted.drain(..) {
        if !seen.insert(item.key.clone()) {
            return Err(invalid());
        }
        let row = visible.get(&item.key).ok_or(OperationError::NotFound)?;
        if row.snapshot != item.snapshot {
            return Err(stale());
        }
        let quantity = Decimal::from_str(&item.quantity).map_err(|_| invalid())?;
        if quantity < Decimal::ZERO
            || quantity.normalize().scale() > 2
            || quantity >= Decimal::from(100_000_000)
        {
            return Err(invalid());
        }
        checked.push((row.medication_id, row.option_id, quantity));
    }
    checked.sort_by_key(|(medication_id, option_id, _)| (*medication_id, *option_id));
    for (medication_id, option_id, quantity) in checked {
        if let Some(option_id) = option_id {
            let (_, etag) = dosages::read(tenant, &option_id.to_string()).await?;
            dosages::update_stock_checked(
                tenant,
                &option_id.to_string(),
                dosages::DosageInput::stock_quantity(quantity),
                Some(&etag),
                provenance,
                reason,
            )
            .await?;
        } else {
            let snapshot =
                medications::read_stock_snapshot(tenant, &medication_id.to_string()).await?;
            medications::execute_with_options(
                tenant,
                medications::Command::AdjustStock(medications::AdjustStock {
                    medication_id: medication_id.to_string(),
                    new_quantity: quantity.normalize().to_string(),
                    reason: Some(reason.to_owned()),
                }),
                Some(&medications::ScalarPrecondition {
                    original_etag: snapshot.etag,
                }),
                provenance,
            )
            .await?;
        }
    }
    Ok(())
}

pub async fn read(
    tenant: &TenantTransaction,
    location_id: Option<i64>,
) -> Result<Page, OperationError> {
    access::recheck(tenant).await?;
    if !access::can_manage_household(tenant) {
        return Err(OperationError::Forbidden);
    }
    let locations = locations::list(tenant).await?;
    let location_names: HashMap<_, _> = locations
        .iter()
        .map(|row| (row.id, row.name.as_str()))
        .collect();
    let mut query = access::medication_scope(tenant);
    if let Some(id) = location_id {
        query = query.filter(medication::Column::LocationId.eq(id));
    }
    let medicines = query
        .order_by_asc(medication::Column::Name)
        .order_by_asc(medication::Column::Id)
        .all(tenant.transaction())
        .await?;
    let ids = medicines.iter().map(|row| row.id).collect::<Vec<_>>();
    let options = dosage::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .filter(dosage::Column::MedicationId.is_in(ids))
        .order_by_asc(dosage::Column::MedicationId)
        .order_by_asc(dosage::Column::Amount)
        .order_by_asc(dosage::Column::Id)
        .all(tenant.transaction())
        .await?;
    let mut options_by_medication: HashMap<i64, Vec<dosage::Model>> = HashMap::new();
    for option in options {
        options_by_medication
            .entry(option.medication_id)
            .or_default()
            .push(option);
    }
    let mut rows = Vec::new();
    for medicine in medicines {
        let name = medicine
            .friendly_name
            .as_deref()
            .filter(|value| !value.is_empty())
            .or(medicine.name.as_deref())
            .unwrap_or("Medication")
            .to_owned();
        let location_name = location_names
            .get(&medicine.location_id)
            .copied()
            .unwrap_or("Location")
            .to_owned();
        let options = options_by_medication
            .remove(&medicine.id)
            .unwrap_or_default();
        let tracked = options
            .iter()
            .filter(|option| option.current_supply.is_some())
            .collect::<Vec<_>>();
        if tracked.is_empty() {
            rows.push(Row {
                key: format!("medication_{}", medicine.id),
                medication_id: medicine.id,
                option_id: None,
                medication_name: name.clone(),
                label: name,
                strength: None,
                location_name,
                quantity: medicine
                    .current_supply
                    .map(|value| value.normalize().to_string()),
                snapshot: medicine.updated_at.to_string(),
                unit: medicine.dose_unit.unwrap_or_default(),
                adjustable: true,
            });
            continue;
        }
        for option in tracked {
            rows.push(Row {
                key: format!("option_{}", option.id),
                medication_id: medicine.id,
                option_id: Some(option.id),
                medication_name: name.clone(),
                label: format!("{} {} {}", name, option.amount.normalize(), option.unit),
                strength: Some(format!("{} {}", option.amount.normalize(), option.unit)),
                location_name: location_name.clone(),
                quantity: option
                    .current_supply
                    .map(|value| value.normalize().to_string()),
                snapshot: option.updated_at.to_string(),
                unit: option.unit.clone(),
                adjustable: true,
            });
        }
    }
    Ok(Page {
        rows,
        locations: locations
            .into_iter()
            .map(|row| Location {
                id: row.id,
                name: row.name,
            })
            .collect(),
    })
}
