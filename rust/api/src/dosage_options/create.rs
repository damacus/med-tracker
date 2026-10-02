use super::*;

pub(crate) async fn create(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    let (db, context) =
        match write_context(&state, &headers, household_id, "POST", "create").await? {
            Ok(value) => value,
            Err(response) => return Ok(response),
        };
    let Json(body) = match payload {
        Ok(value) => value,
        Err(_) => {
            return failure(
                db,
                &context,
                "POST",
                "create",
                StatusCode::BAD_REQUEST,
                "bad_request",
                "Invalid JSON request body",
            )
            .await
        }
    };
    let Some(attrs) = attributes(&body, true) else {
        return validation(db, &context, "POST", "create").await;
    };
    let Some(medication) = medication_row(
        &db,
        household_id,
        attrs
            .medication_id
            .as_deref()
            .expect("create medication ID"),
    )
    .await?
    else {
        return failure(
            db,
            &context,
            "POST",
            "create",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
        )
        .await;
    };
    lock_household(&db, household_id).await?;
    lock_medication(&db, medication.id).await?;
    let medication = medication::Entity::find_by_id(medication.id)
        .one(&db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?;
    let now = Utc::now().naive_utc();
    let active = dosage::ActiveModel {
        household_id: Set(household_id),
        medication_id: Set(medication.id),
        amount: Set(attrs.amount.expect("validated create amount")),
        unit: Set(attrs.unit.expect("validated create unit")),
        frequency: Set(attrs.frequency.expect("validated create frequency")),
        description: Set(attrs.description),
        default_for_adults: Set(attrs.default_for_adults.unwrap_or(false)),
        default_for_children: Set(attrs.default_for_children.unwrap_or(false)),
        default_max_daily_doses: Set(attrs
            .default_max_daily_doses
            .expect("validated create maximum doses")),
        default_min_hours_between_doses: Set(attrs
            .default_min_hours_between_doses
            .expect("validated create minimum hours")),
        default_dose_cycle: Set(attrs
            .default_dose_cycle
            .expect("validated create dose cycle")),
        current_supply: Set(attrs.current_supply.unwrap_or(None)),
        reorder_threshold: Set(attrs.reorder_threshold.unwrap_or(None)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    let savepoint = db.begin().await.map_err(database_error)?;
    let record = match active.insert(&savepoint).await {
        Ok(record) => record,
        Err(error)
            if matches!(
                error.sql_err(),
                Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
            ) =>
        {
            savepoint.rollback().await.map_err(database_error)?;
            return validation(db, &context, "POST", "create").await;
        }
        Err(error) => return Err(database_error(error)),
    };
    let parent_before = medication_snapshot(&medication);
    let tracked_inventory = record.current_supply.is_some();
    let parent = if tracked_inventory {
        match synchronize_inventory(&savepoint, medication.clone(), now, true).await? {
            Some(parent) => parent,
            None => {
                savepoint.rollback().await.map_err(database_error)?;
                return validation(db, &context, "POST", "create").await;
            }
        }
    } else {
        let mut active: medication::ActiveModel = medication.into();
        active.dose_amount = Set(None);
        active.updated_at = Set(now);
        active.update(&savepoint).await.map_err(database_error)?
    };
    savepoint.commit().await.map_err(database_error)?;
    let request_id = Uuid::new_v4().to_string();
    record_version(
        &db,
        &context,
        &request_id,
        "MedicationDosageOption",
        record.id,
        "api_create",
        None,
        Some(dosage_snapshot(&record)),
    )
    .await?;
    if tracked_inventory {
        record_version(
            &db,
            &context,
            &request_id,
            "Medication",
            parent.id,
            "api_update",
            Some(parent_before),
            Some(medication_snapshot(&parent)),
        )
        .await?;
    }
    record_sync(
        &db,
        &context,
        &request_id,
        "MedicationDosageOption",
        record.id,
        &record.portable_id,
        "create",
    )
    .await?;
    record_sync(
        &db,
        &context,
        &request_id,
        "Medication",
        parent.id,
        &parent.portable_id,
        "update",
    )
    .await?;
    let (body, etag) = representation(&db, record).await?;
    finish_with_request_id(
        db,
        &context,
        &request_id,
        "POST",
        "api/v1/dosage_options",
        "DosageOptionPolicy",
        "create",
        StatusCode::CREATED,
        true,
        body,
        Some(&etag),
    )
    .await
}
