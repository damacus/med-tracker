use super::*;

async fn update(
    state: AppState,
    household_id: i64,
    id: String,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
    method: &str,
) -> Result<Response, ApiError> {
    let (db, context) =
        match write_context(&state, &headers, household_id, method, "update").await? {
            Ok(value) => value,
            Err(response) => return Ok(response),
        };
    if !valid_identifier(&id) {
        return failure(
            db,
            &context,
            method,
            "update",
            StatusCode::BAD_REQUEST,
            "bad_request",
            "Invalid resource ID",
        )
        .await;
    }
    let Some(found) = dosage_row(&db, household_id, &id, false).await? else {
        return failure(
            db,
            &context,
            method,
            "update",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
        )
        .await;
    };
    lock_household(&db, household_id).await?;
    lock_medication(&db, found.medication_id).await?;
    let Some(record) = dosage_row(&db, household_id, &id, true).await? else {
        return failure(
            db,
            &context,
            method,
            "update",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
        )
        .await;
    };
    if let Some(if_match) = headers.get(header::IF_MATCH) {
        let (_, current) = representation(&db, record.clone()).await?;
        if if_match.to_str().ok() != Some(current.as_str()) {
            return failure(
                db,
                &context,
                method,
                "update",
                StatusCode::CONFLICT,
                "conflict",
                "Record has changed since it was last read",
            )
            .await;
        }
    }
    let Json(body) = match payload {
        Ok(value) => value,
        Err(_) => {
            return failure(
                db,
                &context,
                method,
                "update",
                StatusCode::BAD_REQUEST,
                "bad_request",
                "Invalid JSON request body",
            )
            .await
        }
    };
    let Some(attrs) = attributes(&body, false) else {
        return validation(db, &context, method, "update").await;
    };
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
    let now = Utc::now().naive_utc();
    active.updated_at = Set(now);
    let savepoint = db.begin().await.map_err(database_error)?;
    let record = match active.update(&savepoint).await {
        Ok(record) => record,
        Err(error)
            if matches!(
                error.sql_err(),
                Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
            ) =>
        {
            savepoint.rollback().await.map_err(database_error)?;
            return validation(db, &context, method, "update").await;
        }
        Err(error) => return Err(database_error(error)),
    };
    if !valid_persisted_dosage(&record) {
        savepoint.rollback().await.map_err(database_error)?;
        return validation(db, &context, method, "update").await;
    }
    let parent = if sync_inventory {
        let parent = medication::Entity::find_by_id(record.medication_id)
            .one(&savepoint)
            .await
            .map_err(database_error)?
            .ok_or_else(ApiError::not_found)?;
        let before = medication_snapshot(&parent);
        match synchronize_inventory(&savepoint, parent, now, false).await? {
            Some(parent) => Some((before, parent)),
            None => {
                savepoint.rollback().await.map_err(database_error)?;
                return validation(db, &context, method, "update").await;
            }
        }
    } else {
        None
    };
    savepoint.commit().await.map_err(database_error)?;
    let request_id = Uuid::new_v4().to_string();
    record_version(
        &db,
        &context,
        &request_id,
        "MedicationDosageOption",
        record.id,
        "api_update",
        Some(before),
        Some(dosage_snapshot(&record)),
    )
    .await?;
    record_sync(
        &db,
        &context,
        &request_id,
        "MedicationDosageOption",
        record.id,
        &record.portable_id,
        "update",
    )
    .await?;
    if let Some((before, parent)) = parent {
        record_version(
            &db,
            &context,
            &request_id,
            "Medication",
            parent.id,
            "api_update",
            Some(before),
            Some(medication_snapshot(&parent)),
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
    }
    let (body, etag) = representation(&db, record).await?;
    finish_with_request_id(
        db,
        &context,
        &request_id,
        method,
        "api/v1/dosage_options",
        "DosageOptionPolicy",
        "update",
        StatusCode::OK,
        true,
        body,
        Some(&etag),
    )
    .await
}

pub(crate) async fn patch(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    update(state, household_id, id, headers, payload, "PATCH").await
}

pub(crate) async fn put(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    update(state, household_id, id, headers, payload, "PUT").await
}
