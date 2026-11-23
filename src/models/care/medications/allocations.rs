mod entity;

use super::*;
use crate::models::{
    access::PersonAccess,
    care::{guardian_declarations, treatments},
    identity::resource::{AuthenticationError, ValidatedPrincipal},
    ids::{MedicationId, PersonId},
};
use loco_rs::model::TenantQueryExt;
use sea_orm::{IntoActiveModel, QueryOrder};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
struct DosageId(i64);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AllocationInput {
    submission_id: Uuid,
    person_id: PersonId,
    quantity: String,
    stock_unit: String,
    dosage_id: Option<DosageId>,
}

pub(crate) struct AllocateStock {
    submission_id: Uuid,
    person_id: PersonId,
    quantity: Decimal,
    stock_unit: String,
    dosage_id: Option<DosageId>,
}

impl TryFrom<AllocationInput> for AllocateStock {
    type Error = OperationError;

    fn try_from(input: AllocationInput) -> Result<Self, Self::Error> {
        let quantity = Decimal::from_str_exact(&input.quantity)
            .map_err(|_| validation("Quantity must be a positive decimal with up to two places"))?
            .normalize();
        if quantity <= Decimal::ZERO
            || quantity >= Decimal::from(100_000_000)
            || quantity.scale() > 2
        {
            return Err(validation(
                "Quantity must be a positive decimal with up to two places",
            ));
        }
        if input.person_id.get() <= 0
            || input.dosage_id.is_some_and(|id| id.0 <= 0)
            || input.submission_id.is_nil()
        {
            return Err(validation("Stock allocation identifiers must be valid"));
        }
        if !treatments::supported_unit(&input.stock_unit) {
            return Err(validation("Stock unit is invalid"));
        }
        Ok(Self {
            submission_id: input.submission_id,
            person_id: input.person_id,
            quantity,
            stock_unit: input.stock_unit,
            dosage_id: input.dosage_id,
        })
    }
}

#[derive(Serialize)]
pub(crate) struct AllocationReceipt {
    submission_id: Uuid,
    medication_id: i64,
    dosage_id: Option<i64>,
    person_id: i64,
    quantity: String,
    stock_unit: String,
}

impl From<&entity::Model> for AllocationReceipt {
    fn from(record: &entity::Model) -> Self {
        Self {
            submission_id: record.submission_id,
            medication_id: record.medication_id,
            dosage_id: record.dosage_id,
            person_id: record.person_id,
            quantity: record.quantity.normalize().to_string(),
            stock_unit: record.stock_unit.clone(),
        }
    }
}

pub(crate) async fn reserve(
    tenant: &TenantTransaction,
    principal: &ValidatedPrincipal,
    medication_id: MedicationId,
    input: AllocateStock,
) -> Result<(AllocationReceipt, bool), OperationError> {
    let db = tenant.transaction();
    let scope = tenant.scope();
    lock_household(db, scope.household_id).await?;
    guardian_declarations::lock_creator_account(tenant).await?;
    principal
        .revalidate_household(tenant)
        .await
        .map_err(|error| match error {
            AuthenticationError::Unauthenticated => OperationError::Unauthenticated,
            AuthenticationError::Forbidden | AuthenticationError::InsufficientScope { .. } => {
                OperationError::Forbidden
            }
            AuthenticationError::NotFound => OperationError::NotFound,
            AuthenticationError::Unavailable => OperationError::Unavailable,
        })?;
    access::require_person_access(tenant, input.person_id.get(), PersonAccess::Manage).await?;

    if let Some(saved) = entity::Entity::find()
        .filter(entity::Column::HouseholdId.eq(scope.household_id))
        .filter(entity::Column::SubmissionId.eq(input.submission_id))
        .one(db)
        .await?
    {
        if saved.actor_account_id == scope.actor.account_id
            && saved.person_id == input.person_id.get()
            && saved.medication_id == medication_id.get()
            && saved.dosage_id == input.dosage_id.map(|id| id.0)
            && saved.quantity == input.quantity
            && saved.stock_unit == input.stock_unit
        {
            return Ok((AllocationReceipt::from(&saved), true));
        }
        return Err(OperationError::Conflict {
            code: "stock_allocation_submission_reused".into(),
            details: json!({"message": "Submission identifier is already in use"}),
        });
    }

    let medication = access::medication_scope(tenant)
        .filter(medication::Column::Id.eq(medication_id.get()))
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or(OperationError::NotFound)?;
    let supply = exact_source_supply(tenant, &medication, &input).await?;
    let source = StockSource {
        household_id: scope.household_id,
        medication_id,
        dosage_id: input.dosage_id,
    };
    require_communal_stock(db, source, supply - input.quantity).await?;

    let record = entity::ActiveModel {
        household_id: Set(scope.household_id),
        medication_id: Set(medication_id.get()),
        dosage_id: Set(input.dosage_id.map(|id| id.0)),
        person_id: Set(input.person_id.get()),
        actor_account_id: Set(scope.actor.account_id),
        submission_id: Set(input.submission_id),
        stock_unit: Set(input.stock_unit),
        quantity: Set(input.quantity),
        remaining_quantity: Set(input.quantity),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(db)
    .await?;
    let result = AllocationReceipt::from(&record);
    let context = StockContext {
        tenant,
        provenance: Some(principal.provenance()),
    };
    audit::record_version(
        db,
        &context,
        &scope.request_id,
        "StockAllocation",
        record.id,
        "create",
        None,
        Some(json!(&result)),
    )
    .await?;
    Ok((result, false))
}

async fn exact_source_supply(
    tenant: &TenantTransaction,
    medication: &medication::Model,
    input: &AllocateStock,
) -> Result<Decimal, OperationError> {
    let db = tenant.transaction();
    let options = dosage::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .filter(dosage::Column::MedicationId.eq(medication.id))
        .filter(dosage::Column::CurrentSupply.is_not_null());
    let invalid_source = || validation("Select tracked stock with the exact stock unit");
    if let Some(dosage_id) = input.dosage_id {
        let dosage = options
            .filter(dosage::Column::Id.eq(dosage_id.0))
            .lock_exclusive()
            .one(db)
            .await?
            .ok_or_else(invalid_source)?;
        if dosage.unit != input.stock_unit {
            return Err(invalid_source());
        }
        return dosage.current_supply.ok_or_else(invalid_source);
    }
    if medication.dose_unit.as_deref() != Some(input.stock_unit.as_str())
        || options.one(db).await?.is_some()
    {
        return Err(invalid_source());
    }
    medication.current_supply.ok_or_else(invalid_source)
}

#[derive(Clone, Copy)]
pub(crate) struct StockSource {
    household_id: i64,
    medication_id: MedicationId,
    dosage_id: Option<DosageId>,
}

impl StockSource {
    pub(crate) fn from_inventory(
        household_id: i64,
        medication_id: i64,
        dosage_id: Option<i64>,
    ) -> Self {
        Self {
            household_id,
            medication_id: MedicationId::from_database(medication_id),
            dosage_id: dosage_id.map(DosageId),
        }
    }

    fn active(self) -> sea_orm::Select<entity::Entity> {
        let allocations = self.active_for_medication();
        match self.dosage_id {
            Some(id) => allocations.filter(entity::Column::DosageId.eq(id.0)),
            None => allocations.filter(entity::Column::DosageId.is_null()),
        }
    }

    fn active_for_medication(self) -> sea_orm::Select<entity::Entity> {
        entity::Entity::find()
            .filter(entity::Column::HouseholdId.eq(self.household_id))
            .filter(entity::Column::MedicationId.eq(self.medication_id.get()))
            .filter(entity::Column::RemainingQuantity.gt(Decimal::ZERO))
    }
}

pub(crate) async fn require_communal_stock(
    db: &DatabaseTransaction,
    source: StockSource,
    remaining_physical: Decimal,
) -> Result<(), OperationError> {
    require_physical_stock(db, source.active(), remaining_physical).await
}

pub(super) async fn require_medication_physical_stock(
    db: &DatabaseTransaction,
    medication: &medication::Model,
    remaining_physical: Decimal,
) -> Result<(), OperationError> {
    let source = StockSource::from_inventory(medication.household_id, medication.id, None);
    require_physical_stock(db, source.active_for_medication(), remaining_physical).await
}

async fn require_physical_stock(
    db: &DatabaseTransaction,
    allocations: sea_orm::Select<entity::Entity>,
    remaining_physical: Decimal,
) -> Result<(), OperationError> {
    let reserved: Option<Decimal> = allocations
        .select_only()
        .column_as(entity::Column::RemainingQuantity.sum(), "reserved")
        .into_tuple()
        .one(db)
        .await?
        .flatten();
    if remaining_physical < reserved.unwrap_or(Decimal::ZERO) {
        return Err(validation("Insufficient unallocated stock"));
    }
    Ok(())
}

pub(crate) async fn use_for_dose(
    db: &DatabaseTransaction,
    source: StockSource,
    person_id: PersonId,
    supply: Decimal,
    quantity: Decimal,
) -> Result<(), OperationError> {
    let allocations = source
        .active()
        .order_by_asc(entity::Column::Id)
        .all(db)
        .await?;
    let reserved_for_others = allocations
        .iter()
        .filter(|allocation| allocation.person_id != person_id.get())
        .map(|allocation| allocation.remaining_quantity)
        .sum::<Decimal>();
    if quantity > supply - reserved_for_others {
        return Err(validation("Cannot take medication: out of stock"));
    }
    let mut unassigned_quantity = quantity;
    for allocation in allocations
        .into_iter()
        .filter(|row| row.person_id == person_id.get())
    {
        if unassigned_quantity.is_zero() {
            break;
        }
        let used = allocation.remaining_quantity.min(unassigned_quantity);
        let remaining = allocation.remaining_quantity - used;
        unassigned_quantity -= used;
        let mut active = allocation.into_active_model();
        active.remaining_quantity = Set(remaining);
        active.update(db).await?;
    }
    Ok(())
}
