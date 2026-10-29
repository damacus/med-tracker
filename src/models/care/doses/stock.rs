use super::*;
use loco_rs::model::TenantQueryExt;

const NHS_DMD_SYSTEM: &str = "https://dmd.nhs.uk";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DmdProductClass {
    Vmp,
    Amp,
    Vmpp,
    Ampp,
}

impl DmdProductClass {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "VMP" => Some(Self::Vmp),
            "AMP" => Some(Self::Amp),
            "VMPP" => Some(Self::Vmpp),
            "AMPP" => Some(Self::Ampp),
            _ => None,
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Vmp => "VMP",
            Self::Amp => "AMP",
            Self::Vmpp => "VMPP",
            Self::Ampp => "AMPP",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ExactProductIdentity<'a> {
    class: DmdProductClass,
    code: &'a str,
    stock_unit: &'a str,
}

impl<'a> ExactProductIdentity<'a> {
    fn from_stock(stock: &'a medication::Model) -> Option<Self> {
        if stock.dmd_system.as_deref()? != NHS_DMD_SYSTEM {
            return None;
        }
        let class = DmdProductClass::parse(stock.dmd_concept_class.as_deref()?)?;
        let code = stock
            .dmd_code
            .as_deref()
            .filter(|value| !value.trim().is_empty())?;
        let stock_unit = stock
            .dose_unit
            .as_deref()
            .filter(|value| !value.trim().is_empty())?;
        Some(Self {
            class,
            code,
            stock_unit,
        })
    }

    fn condition(self) -> Condition {
        Condition::all()
            .add(medication::Column::DmdSystem.eq(NHS_DMD_SYSTEM))
            .add(medication::Column::DmdConceptClass.eq(self.class.as_str()))
            .add(medication::Column::DmdCode.eq(self.code))
            .add(medication::Column::DoseUnit.eq(self.stock_unit))
    }
}

pub(crate) fn same_stock_signature(left: &medication::Model, right: &medication::Model) -> bool {
    if left.household_id == right.household_id
        && left.id == right.id
        && left.portable_id == right.portable_id
    {
        return true;
    }
    match (
        ExactProductIdentity::from_stock(left),
        ExactProductIdentity::from_stock(right),
    ) {
        (Some(left_identity), Some(right_identity)) => {
            left_identity == right_identity
                && left.dose_amount == right.dose_amount
                && left.dose_amount.is_none_or(f64::is_finite)
        }
        _ => false,
    }
}

pub(crate) fn stock_candidate_condition(original: &medication::Model) -> Condition {
    let same_row = Condition::all()
        .add(medication::Column::HouseholdId.eq(original.household_id))
        .add(medication::Column::Id.eq(original.id))
        .add(medication::Column::PortableId.eq(&original.portable_id));
    let mut candidates = Condition::any().add(same_row);
    if let Some(identity) = ExactProductIdentity::from_stock(original)
        && original.dose_amount.is_none_or(f64::is_finite)
    {
        let dose_default = match original.dose_amount {
            Some(amount) => medication::Column::DoseAmount.eq(amount),
            None => medication::Column::DoseAmount.is_null(),
        };
        candidates = candidates.add(identity.condition().add(dose_default));
    }
    candidates
}

pub(super) async fn decrement_stock(
    db: &DatabaseTransaction,
    context: &DoseContext<'_>,
    request_id: &str,
    proposed: &ProposedTake,
) -> Result<(), ApiError> {
    lock_medication(db, proposed.selected.id).await?;
    let selected = medication::Entity::find_by_id(proposed.selected.id)
        .in_tenant(context.scope().household_id)
        .one(db)
        .await
        .map_err(OperationError::from)?
        .ok_or(OperationError::NotFound)?;
    let tracked = dosage::Entity::find()
        .in_tenant(context.scope().household_id)
        .filter(dosage::Column::MedicationId.eq(selected.id))
        .filter(dosage::Column::CurrentSupply.is_not_null())
        .all(db)
        .await
        .map_err(OperationError::from)?;
    if tracked.is_empty() {
        if let Some(supply) = selected.current_supply {
            let needed = quantity(proposed.amount, &proposed.unit);
            if !sufficient_stock(Some(supply), proposed.amount, &proposed.unit) {
                return Err(error(
                    ErrorKind::Validation,
                    "Cannot take medication: out of stock",
                ));
            }
            let selected_id = selected.id;
            let selected_portable_id = selected.portable_id.clone();
            let mut update: medication::ActiveModel = selected.into();
            update.current_supply = Set(Some(supply - needed));
            update.updated_at = Set(Utc::now().naive_utc());
            update.update(db).await.map_err(OperationError::from)?;
            stock_version(
                db,
                context,
                request_id,
                StockVersionChange {
                    item_type: "Medication",
                    item_id: selected_id,
                    previous: supply,
                    current: supply - needed,
                    event: "dose_decrement",
                },
            )
            .await?;
            record_change(
                db,
                context,
                request_id,
                SyncRecord {
                    record_type: "Medication",
                    record_id: selected_id,
                    portable_id: &selected_portable_id,
                    action: "update",
                    person_portable_id: None,
                },
            )
            .await?;
        }
        return Ok(());
    }
    let source_option = if let Some(id) = proposed.source.source_dosage_option_id {
        Some(
            dosage::Entity::find_by_id(id)
                .in_tenant(context.scope().household_id)
                .one(db)
                .await
                .map_err(OperationError::from)?
                .ok_or(OperationError::NotFound)?,
        )
    } else {
        None
    };
    let selected_option = selected_tracked_dosage(
        &tracked,
        selected.id,
        source_option.as_ref(),
        proposed.source.dose_amount,
        proposed.source.dose_unit.as_deref(),
    )
    .ok_or_else(|| {
        error(
            ErrorKind::Validation,
            "Selected location is unavailable for this medication.",
        )
    })?;
    lock_dosage(db, selected_option.id).await?;
    let selected_option = dosage::Entity::find_by_id(selected_option.id)
        .in_tenant(context.scope().household_id)
        .one(db)
        .await
        .map_err(OperationError::from)?
        .ok_or(OperationError::NotFound)?;
    let supply = selected_option
        .current_supply
        .ok_or(OperationError::Unavailable)?;
    let needed = quantity(proposed.amount, &selected_option.unit);
    if !sufficient_stock(Some(supply), proposed.amount, &selected_option.unit) {
        return Err(error(
            ErrorKind::Validation,
            "Cannot take medication: out of stock",
        ));
    }
    let option_id = selected_option.id;
    let option_portable_id = selected_option.portable_id.clone();
    let mut update: dosage::ActiveModel = selected_option.into();
    update.current_supply = Set(Some(supply - needed));
    update.updated_at = Set(Utc::now().naive_utc());
    update.update(db).await.map_err(OperationError::from)?;
    stock_version(
        db,
        context,
        request_id,
        StockVersionChange {
            item_type: "MedicationDosageOption",
            item_id: option_id,
            previous: supply,
            current: supply - needed,
            event: "update",
        },
    )
    .await?;
    record_change(
        db,
        context,
        request_id,
        SyncRecord {
            record_type: "MedicationDosageOption",
            record_id: option_id,
            portable_id: &option_portable_id,
            action: "update",
            person_portable_id: None,
        },
    )
    .await?;
    let total = dosage::Entity::find()
        .in_tenant(context.scope().household_id)
        .filter(dosage::Column::MedicationId.eq(proposed.selected.id))
        .filter(dosage::Column::CurrentSupply.is_not_null())
        .all(db)
        .await
        .map_err(OperationError::from)?
        .into_iter()
        .filter_map(|v| v.current_supply)
        .sum::<Decimal>();
    if total >= Decimal::from(100_000_000) {
        return Err(error(ErrorKind::Validation, "Invalid dose configured"));
    }
    let previous = selected.current_supply;
    let selected_id = selected.id;
    let selected_portable_id = selected.portable_id.clone();
    let mut inventory: medication::ActiveModel = selected.into();
    inventory.current_supply = Set(Some(total));
    inventory.updated_at = Set(Utc::now().naive_utc());
    inventory.update(db).await.map_err(OperationError::from)?;
    if let Some(previous) = previous {
        stock_version(
            db,
            context,
            request_id,
            StockVersionChange {
                item_type: "Medication",
                item_id: selected_id,
                previous,
                current: total,
                event: "dose_decrement",
            },
        )
        .await?;
    }
    record_change(
        db,
        context,
        request_id,
        SyncRecord {
            record_type: "Medication",
            record_id: selected_id,
            portable_id: &selected_portable_id,
            action: "update",
            person_portable_id: None,
        },
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod exact_stock_tests {
    use super::*;

    fn stock(id: i64) -> medication::Model {
        let now = Utc::now().naive_utc();
        medication::Model {
            id,
            household_id: 1,
            portable_id: format!("stock-{id}"),
            name: Some("Synthetic medicine".into()),
            dmd_system: Some("https://dmd.nhs.uk".into()),
            dmd_concept_class: Some("VMP".into()),
            dmd_code: Some("123456".into()),
            dose_amount: Some(2.0),
            dose_unit: Some("tablet".into()),
            location_id: id,
            created_at: now,
            updated_at: now,
            barcode: None,
            category: None,
            created_by_membership_id: None,
            current_supply: Some(Decimal::TEN),
            default_schedule_config: json!({}),
            default_schedule_type: 0,
            description: None,
            expiry_date: None,
            friendly_name: None,
            expected_arrival_on: None,
            ordered_at: None,
            order_quantity: None,
            order_supplier: None,
            reorder_status: None,
            reorder_threshold: Decimal::ZERO,
            reordered_at: None,
            supply_at_last_restock: None,
            warnings: None,
        }
    }

    #[test]
    fn exact_stock_rejects_same_name_with_different_catalogue_identity() {
        let original = stock(1);
        for field in ["system", "class", "code"] {
            let mut candidate = stock(2);
            match field {
                "system" => candidate.dmd_system = Some("other-catalogue".into()),
                "class" => candidate.dmd_concept_class = Some("AMP".into()),
                _ => candidate.dmd_code = Some("654321".into()),
            }
            assert!(!same_stock_signature(&original, &candidate), "{field}");
        }
    }

    #[test]
    fn exact_stock_rejects_incomplete_or_broad_catalogue_identity() {
        for field in ["system", "class", "code", "unit"] {
            for missing in [None, Some(String::new()), Some(" ".into())] {
                let mut original = stock(1);
                match field {
                    "system" => original.dmd_system = missing,
                    "class" => original.dmd_concept_class = missing,
                    "code" => original.dmd_code = missing,
                    _ => original.dose_unit = missing,
                }
                let mut candidate = original.clone();
                candidate.id = 2;
                candidate.portable_id = "stock-2".into();
                assert!(!same_stock_signature(&original, &candidate), "{field}");
            }
        }
        for class in ["VTM", "TF", "TFG", "Accessory", "unknown"] {
            let mut original = stock(1);
            original.dmd_concept_class = Some(class.into());
            let mut candidate = original.clone();
            candidate.id = 2;
            candidate.portable_id = "stock-2".into();
            assert!(!same_stock_signature(&original, &candidate), "{class}");
        }
        let mut original = stock(1);
        original.dmd_system = Some("unrecognised-catalogue".into());
        let mut candidate = original.clone();
        candidate.id = 2;
        candidate.portable_id = "stock-2".into();
        assert!(!same_stock_signature(&original, &candidate));
    }

    #[test]
    fn exact_stock_preserves_dose_default_and_stock_unit_compatibility() {
        let original = stock(1);
        let mut candidate = stock(2);
        candidate.dose_amount = Some(3.0);
        assert!(!same_stock_signature(&original, &candidate));
        candidate.dose_amount = original.dose_amount;
        candidate.dose_unit = Some("capsule".into());
        assert!(!same_stock_signature(&original, &candidate));
    }

    #[test]
    fn exact_stock_accepts_supported_product_identity_despite_display_name() {
        for class in ["VMP", "AMP", "VMPP", "AMPP"] {
            let mut original = stock(1);
            original.dmd_concept_class = Some(class.into());
            let mut candidate = original.clone();
            candidate.id = 2;
            candidate.portable_id = "stock-2".into();
            candidate.name = Some("Different display name".into());
            assert!(same_stock_signature(&original, &candidate), "{class}");
        }
    }

    #[test]
    fn exact_stock_keeps_same_row_usable_without_catalogue_identity() {
        let mut original = stock(1);
        original.dmd_system = None;
        original.dmd_concept_class = None;
        original.dmd_code = None;
        assert!(same_stock_signature(&original, &original));
        let mut other_household = original.clone();
        other_household.household_id = 2;
        assert!(!same_stock_signature(&original, &other_household));
    }
}
