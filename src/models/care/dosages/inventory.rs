use super::*;

fn inventory_totals(
    count: i64,
    total: Option<Decimal>,
    threshold: Option<Decimal>,
    existing_baseline: Option<Decimal>,
) -> Option<(Option<Decimal>, Decimal, Option<Decimal>)> {
    if count == 0 {
        return Some((None, Decimal::ZERO, None));
    }
    let total = storage_decimal(total?, 8, 2)?;
    let threshold = storage_decimal(threshold.unwrap_or(Decimal::ZERO), 8, 2)?;
    let baseline = existing_baseline.map_or(total, |baseline| baseline.max(total));
    Some((Some(total), threshold, Some(baseline)))
}

pub(super) async fn synchronize_inventory(
    db: &DatabaseTransaction,
    parent: medication::Model,
    now: chrono::NaiveDateTime,
    clear_dose: bool,
) -> Result<Option<medication::Model>, OperationError> {
    let aggregated: Option<(i64, Option<Decimal>, Option<Decimal>)> = dosage::Entity::find()
        .select_only()
        .column_as(dosage::Column::Id.count(), "tracked_count")
        .column_as(dosage::Column::CurrentSupply.sum(), "supply_total")
        .column_as(dosage::Column::ReorderThreshold.sum(), "threshold_total")
        .filter(dosage::Column::MedicationId.eq(parent.id))
        .filter(dosage::Column::CurrentSupply.is_not_null())
        .into_tuple()
        .one(db)
        .await
        .map_err(database_error)?;
    let (count, total, threshold) = aggregated.ok_or(OperationError::Unavailable)?;
    let Some((current_supply, reorder_threshold, baseline)) =
        inventory_totals(count, total, threshold, parent.supply_at_last_restock)
    else {
        return Ok(None);
    };
    let mut active: medication::ActiveModel = parent.into();
    active.current_supply = Set(current_supply);
    active.reorder_threshold = Set(reorder_threshold);
    active.supply_at_last_restock = Set(baseline);
    if clear_dose {
        active.dose_amount = Set(None);
    }
    active.updated_at = Set(now);
    active.update(db).await.map(Some).map_err(database_error)
}
