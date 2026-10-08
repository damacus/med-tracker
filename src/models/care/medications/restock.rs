use super::*;
use chrono::NaiveDate;

pub(super) async fn execute(
    context: &StockContext<'_>,
    input: Restock,
) -> Result<medication::Model, OperationError> {
    let quantity = Decimal::from_str(&input.quantity)
        .map_err(|_| validation("Quantity must be greater than 0"))?;
    if quantity <= Decimal::ZERO
        || quantity.normalize().scale() > 2
        || quantity >= Decimal::from(100_000_000)
    {
        return Err(validation("Quantity must be greater than 0"));
    }
    let restock_date = NaiveDate::parse_from_str(&input.restock_date, "%Y-%m-%d")
        .ok()
        .filter(|date| date.format("%Y-%m-%d").to_string() == input.restock_date)
        .ok_or_else(|| validation("Restock date is invalid"))?;
    let tenant = context.tenant;
    let db = tenant.transaction();
    lock_row(db, "households", tenant.scope().household_id).await?;
    access::recheck(tenant).await?;
    let found = visible_medication(tenant, &input.medication_id)
        .await?
        .ok_or(OperationError::NotFound)?;
    lock_row(db, "medications", found.id).await?;
    let current = visible_medication(tenant, &input.medication_id)
        .await?
        .ok_or(OperationError::NotFound)?;
    let current = snapshot(db, current).await?;
    if input.original_etag != current.etag {
        return Err(conflict());
    }
    let has_options = dosage::Entity::find()
        .filter(dosage::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(dosage::Column::MedicationId.eq(found.id))
        .one(db)
        .await?
        .is_some();
    if has_options {
        return Err(validation(
            "Update dose option stock to refill this medication",
        ));
    }
    let new_supply = current.medication.current_supply.unwrap_or(Decimal::ZERO) + quantity;
    if new_supply >= Decimal::from(100_000_000) {
        return Err(validation("Resulting stock is outside stock precision"));
    }
    let before = audit::medication_snapshot(&current.medication);
    let mut active: medication::ActiveModel = current.medication.into();
    active.current_supply = Set(Some(new_supply));
    active.supply_at_last_restock = Set(Some(new_supply));
    active.reorder_status = Set(None);
    active.ordered_at = Set(None);
    active.reordered_at = Set(None);
    let now = Utc::now().naive_utc();
    active.updated_at = Set(now);
    let updated = active.update(db).await?;
    let event = format!(
        "restock (qty: {}, date: {})",
        quantity.normalize(),
        restock_date.format("%Y-%m-%d")
    );
    audit::record_version(
        db,
        context,
        &tenant.scope().request_id,
        "Medication",
        updated.id,
        &event,
        Some(before),
        Some(audit::medication_snapshot(&updated)),
    )
    .await?;
    api_change_event::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        household_membership_id: Set(Some(tenant.membership().id)),
        account_id: Set(Some(tenant.scope().actor.account_id)),
        action: Set("update".into()),
        record_type: Set("Medication".into()),
        record_id: Set(updated.id),
        record_portable_id: Set(Some(updated.portable_id.clone())),
        request_id: Set(Some(tenant.scope().request_id.clone())),
        metadata: Set(json!({"record_type":"Medication","record_id":updated.id,"portable_id":updated.portable_id})),
        occurred_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(updated)
}
