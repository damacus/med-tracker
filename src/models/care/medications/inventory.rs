use super::*;

pub(super) async fn adjust(
    context: &StockContext<'_>,
    input: AdjustStock,
    scalar: Option<&ScalarPrecondition>,
) -> Result<medication::Model, OperationError> {
    let tenant = context.tenant;
    let db = tenant.transaction();
    lock_row(db, "households", tenant.scope().household_id).await?;
    access::recheck(tenant).await?;
    let found = visible_medication(tenant, &input.medication_id)
        .await?
        .ok_or(OperationError::NotFound)?;
    if !matches!(tenant.membership().role.as_str(), "owner" | "administrator") {
        return Err(OperationError::Forbidden);
    }
    if let Some(scalar) = scalar {
        lock_row(db, "medications", found.id).await?;
        let current = visible_medication(tenant, &input.medication_id)
            .await?
            .ok_or(OperationError::NotFound)?;
        let current = snapshot(db, current).await?;
        let has_options = dosage::Entity::find()
            .filter(dosage::Column::HouseholdId.eq(tenant.scope().household_id))
            .filter(dosage::Column::MedicationId.eq(found.id))
            .one(db)
            .await
            .map_err(database_error)?
            .is_some();
        if scalar.original_etag != current.etag || has_options {
            return Err(conflict());
        }
    }
    let quantity = Decimal::from_str(&input.new_quantity)
        .map_err(|_| validation("Quantity must be a valid nonnegative number"))?;
    if quantity < Decimal::ZERO
        || quantity.normalize().scale() > 2
        || quantity >= Decimal::from(100_000_000)
    {
        return Err(validation("Quantity must be a valid nonnegative number"));
    }
    lock_row(db, "medications", found.id).await?;
    let medication = visible_medication(tenant, &input.medication_id)
        .await?
        .ok_or(OperationError::NotFound)?;
    let before = audit::medication_snapshot(&medication);
    let mut active: medication::ActiveModel = medication.into();
    active.current_supply = Set(Some(quantity));
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active.update(db).await.map_err(database_error)?;
    audit::record_inventory_adjustment(
        db,
        context,
        &tenant.scope().request_id,
        &updated,
        before,
        quantity,
        input.reason.as_deref(),
    )
    .await?;
    let now = Utc::now().naive_utc();
    api_change_event::ActiveModel {
        household_id:Set(tenant.scope().household_id),
        household_membership_id:Set(Some(tenant.membership().id)),
        account_id:Set(Some(tenant.scope().actor.account_id)),
        action:Set("update".into()),record_type:Set("Medication".into()),record_id:Set(updated.id),
        record_portable_id:Set(Some(updated.portable_id.clone())),request_id:Set(Some(tenant.scope().request_id.clone())),
        metadata:Set(json!({"record_type":"Medication","record_id":updated.id,"portable_id":updated.portable_id})),
        occurred_at:Set(now),created_at:Set(now),updated_at:Set(now),..Default::default()
    }.insert(db).await.map_err(database_error)?;
    Ok(updated)
}
