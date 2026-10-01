use super::*;

fn sync_batch_error(status: StatusCode, code: &'static str, message: &'static str) -> ApiError {
    ApiError {
        status,
        code,
        message,
        preserve_activity: false,
    }
}

fn sync_batch_invalid() -> ApiError {
    sync_batch_error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "unprocessable_content",
        "Dosage option attributes are invalid",
    )
}

async fn sync_batch_result(
    db: &DatabaseTransaction,
    record: dosage::Model,
) -> Result<crate::sync_batch::SyncResult, ApiError> {
    let (_, etag) = representation(db, record.clone()).await?;
    Ok(crate::sync_batch::SyncResult {
        record_type: "MedicationDosageOption",
        record_id: Some(record.id),
        record_portable_id: Some(record.portable_id),
        etag: Some(etag),
        replayed: None,
    })
}

pub(crate) async fn apply_sync_operation(
    db: &DatabaseTransaction,
    context: &AuthContext,
    operation: &crate::sync_batch::SyncOperation,
    request_id: &str,
) -> Result<crate::sync_batch::SyncResult, ApiError> {
    if !household_manager(context) {
        return Err(ApiError::forbidden());
    }
    let household_id = context.membership.household_id;
    let creating = operation.action == "create";
    let body = json!({"dosage_option": operation.attributes});
    let attrs = attributes(&body, creating).ok_or_else(sync_batch_invalid)?;
    if creating {
        let medication_id = attrs
            .medication_id
            .as_deref()
            .ok_or_else(sync_batch_invalid)?;
        let medication = medication_row(db, household_id, medication_id)
            .await?
            .ok_or_else(ApiError::not_found)?;
        lock_medication(db, medication.id).await?;
        let medication = medication::Entity::find_by_id(medication.id)
            .one(db)
            .await
            .map_err(database_error)?
            .ok_or_else(ApiError::not_found)?;
        let now = Utc::now().naive_utc();
        let active = dosage::ActiveModel {
            household_id: Set(household_id),
            medication_id: Set(medication.id),
            amount: Set(attrs.amount.ok_or_else(sync_batch_invalid)?),
            unit: Set(attrs.unit.ok_or_else(sync_batch_invalid)?),
            frequency: Set(attrs.frequency.ok_or_else(sync_batch_invalid)?),
            description: Set(attrs.description),
            default_for_adults: Set(attrs.default_for_adults.unwrap_or(false)),
            default_for_children: Set(attrs.default_for_children.unwrap_or(false)),
            default_max_daily_doses: Set(attrs
                .default_max_daily_doses
                .ok_or_else(sync_batch_invalid)?),
            default_min_hours_between_doses: Set(attrs
                .default_min_hours_between_doses
                .ok_or_else(sync_batch_invalid)?),
            default_dose_cycle: Set(attrs.default_dose_cycle.ok_or_else(sync_batch_invalid)?),
            current_supply: Set(attrs.current_supply.unwrap_or(None)),
            reorder_threshold: Set(attrs.reorder_threshold.unwrap_or(None)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        let record = active.insert(db).await.map_err(|error| {
            if matches!(
                error.sql_err(),
                Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
            ) {
                sync_batch_invalid()
            } else {
                database_error(error)
            }
        })?;
        let parent_before = medication_snapshot(&medication);
        let tracked_inventory = record.current_supply.is_some();
        let parent = if tracked_inventory {
            synchronize_inventory(db, medication.clone(), now, true)
                .await?
                .ok_or_else(sync_batch_invalid)?
        } else {
            let mut active: medication::ActiveModel = medication.into();
            active.dose_amount = Set(None);
            active.updated_at = Set(now);
            active.update(db).await.map_err(database_error)?
        };
        record_version(
            db,
            context,
            request_id,
            "MedicationDosageOption",
            record.id,
            "api_create",
            None,
            Some(dosage_snapshot(&record)),
        )
        .await?;
        if tracked_inventory {
            record_version(
                db,
                context,
                request_id,
                "Medication",
                parent.id,
                "api_update",
                Some(parent_before),
                Some(medication_snapshot(&parent)),
            )
            .await?;
        }
        record_sync(
            db,
            context,
            request_id,
            "MedicationDosageOption",
            record.id,
            &record.portable_id,
            "create",
        )
        .await?;
        record_sync(
            db,
            context,
            request_id,
            "Medication",
            parent.id,
            &parent.portable_id,
            "update",
        )
        .await?;
        return sync_batch_result(db, record).await;
    }
    let id = operation.id.as_deref().ok_or_else(sync_batch_invalid)?;
    let found = dosage_row(db, household_id, id, false)
        .await?
        .ok_or_else(ApiError::not_found)?;
    lock_medication(db, found.medication_id).await?;
    let record = dosage_row(db, household_id, id, true)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let if_match = operation
        .if_match
        .as_deref()
        .filter(|tag| !tag.is_empty())
        .ok_or_else(|| {
            sync_batch_error(
                StatusCode::PRECONDITION_REQUIRED,
                "precondition_required",
                "if_match is required",
            )
        })?;
    let (_, current) = representation(db, record.clone()).await?;
    if if_match != current {
        return Err(sync_batch_error(
            StatusCode::CONFLICT,
            "sync_conflict",
            "Record has changed since it was last read",
        ));
    }
    let new_supply = attrs.current_supply.unwrap_or(record.current_supply);
    let sync_inventory = new_supply.is_some() || new_supply != record.current_supply;
    let before = dosage_snapshot(&record);
    let mut active: dosage::ActiveModel = record.into();
    if let Some(value) = attrs.amount {
        active.amount = Set(value);
    }
    if let Some(value) = attrs.unit {
        active.unit = Set(value);
    }
    if let Some(value) = attrs.frequency {
        active.frequency = Set(value);
    }
    if let Some(value) = attrs.description {
        active.description = Set(Some(value));
    }
    if let Some(value) = attrs.default_for_adults {
        active.default_for_adults = Set(value);
    }
    if let Some(value) = attrs.default_for_children {
        active.default_for_children = Set(value);
    }
    if let Some(value) = attrs.default_max_daily_doses {
        active.default_max_daily_doses = Set(value);
    }
    if let Some(value) = attrs.default_min_hours_between_doses {
        active.default_min_hours_between_doses = Set(value);
    }
    if let Some(value) = attrs.default_dose_cycle {
        active.default_dose_cycle = Set(value);
    }
    if let Some(value) = attrs.current_supply {
        active.current_supply = Set(value);
    }
    if let Some(value) = attrs.reorder_threshold {
        active.reorder_threshold = Set(value);
    }
    active.updated_at = Set(Utc::now().naive_utc());
    let record = active.update(db).await.map_err(|error| {
        if matches!(
            error.sql_err(),
            Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
        ) {
            sync_batch_invalid()
        } else {
            database_error(error)
        }
    })?;
    if !valid_persisted_dosage(&record) {
        return Err(sync_batch_invalid());
    }
    let parent = if sync_inventory {
        let parent = medication::Entity::find_by_id(record.medication_id)
            .one(db)
            .await
            .map_err(database_error)?
            .ok_or_else(ApiError::not_found)?;
        let before = medication_snapshot(&parent);
        let parent = synchronize_inventory(db, parent, Utc::now().naive_utc(), false)
            .await?
            .ok_or_else(sync_batch_invalid)?;
        Some((before, parent))
    } else {
        None
    };
    record_version(
        db,
        context,
        request_id,
        "MedicationDosageOption",
        record.id,
        "api_update",
        Some(before),
        Some(dosage_snapshot(&record)),
    )
    .await?;
    record_sync(
        db,
        context,
        request_id,
        "MedicationDosageOption",
        record.id,
        &record.portable_id,
        "update",
    )
    .await?;
    if let Some((before, parent)) = parent {
        record_version(
            db,
            context,
            request_id,
            "Medication",
            parent.id,
            "api_update",
            Some(before),
            Some(medication_snapshot(&parent)),
        )
        .await?;
        record_sync(
            db,
            context,
            request_id,
            "Medication",
            parent.id,
            &parent.portable_id,
            "update",
        )
        .await?;
    }
    sync_batch_result(db, record).await
}

pub(crate) async fn authorize_sync_replay(
    db: &DatabaseTransaction,
    context: &AuthContext,
    _operation: &crate::sync_batch::SyncOperation,
    saved: &Value,
) -> Result<(), ApiError> {
    if !household_manager(context) {
        return Err(ApiError::forbidden());
    }
    let portable_id = saved
        .get("record_portable_id")
        .and_then(Value::as_str)
        .ok_or_else(ApiError::forbidden)?;
    if dosage_row(db, context.membership.household_id, portable_id, false)
        .await?
        .is_none()
    {
        return Err(ApiError::forbidden());
    }
    Ok(())
}
