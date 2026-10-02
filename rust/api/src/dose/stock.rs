use super::*;

pub(crate) fn same_stock_signature(left: &medication::Model, right: &medication::Model) -> bool {
    left.name == right.name
        && left.dose_amount == right.dose_amount
        && left.dose_unit == right.dose_unit
}

pub(super) async fn decrement_stock(
    db: &DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    proposed: &ProposedTake,
) -> Result<(), ApiError> {
    lock_row(db, "medications", proposed.selected.id).await?;
    let selected = medication::Entity::find_by_id(proposed.selected.id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?;
    let tracked = dosage::Entity::find()
        .filter(dosage::Column::MedicationId.eq(selected.id))
        .filter(dosage::Column::CurrentSupply.is_not_null())
        .all(db)
        .await
        .map_err(database_error)?;
    if tracked.is_empty() {
        if let Some(supply) = selected.current_supply {
            let needed = quantity(proposed.amount, &proposed.unit);
            if !sufficient_stock(Some(supply), proposed.amount, &proposed.unit) {
                return Err(error(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Cannot take medication: out of stock",
                ));
            }
            let selected_id = selected.id;
            let selected_portable_id = selected.portable_id.clone();
            let mut update: medication::ActiveModel = selected.into();
            update.current_supply = Set(Some(supply - needed));
            update.updated_at = Set(Utc::now().naive_utc());
            update.update(db).await.map_err(database_error)?;
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
                .one(db)
                .await
                .map_err(database_error)?
                .ok_or_else(ApiError::not_found)?,
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
            StatusCode::UNPROCESSABLE_ENTITY,
            "Selected location is unavailable for this medication.",
        )
    })?;
    lock_row(db, "dosages", selected_option.id).await?;
    let selected_option = dosage::Entity::find_by_id(selected_option.id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?;
    let supply = selected_option
        .current_supply
        .ok_or_else(ApiError::internal)?;
    let needed = quantity(proposed.amount, &selected_option.unit);
    if !sufficient_stock(Some(supply), proposed.amount, &selected_option.unit) {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Cannot take medication: out of stock",
        ));
    }
    let option_id = selected_option.id;
    let option_portable_id = selected_option.portable_id.clone();
    let mut update: dosage::ActiveModel = selected_option.into();
    update.current_supply = Set(Some(supply - needed));
    update.update(db).await.map_err(database_error)?;
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
        .filter(dosage::Column::MedicationId.eq(proposed.selected.id))
        .filter(dosage::Column::CurrentSupply.is_not_null())
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .filter_map(|v| v.current_supply)
        .sum::<Decimal>();
    if total >= Decimal::from(100_000_000) {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Invalid dose configured",
        ));
    }
    let previous = selected.current_supply;
    let selected_id = selected.id;
    let selected_portable_id = selected.portable_id.clone();
    let mut inventory: medication::ActiveModel = selected.into();
    inventory.current_supply = Set(Some(total));
    inventory.updated_at = Set(Utc::now().naive_utc());
    inventory.update(db).await.map_err(database_error)?;
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
