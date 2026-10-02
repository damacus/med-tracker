use super::*;

pub(crate) async fn create(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let Json(body) = match payload {
        Ok(value) => value,
        Err(_) => return failure(db, &context, "POST", "create", StatusCode::BAD_REQUEST).await,
    };
    let parsed = Attributes::parse(&body, true);
    let person_identifier = body
        .get("person_medication")
        .and_then(Value::as_object)
        .and_then(|inner| inner.get("person_id"))
        .and_then(Value::as_str)
        .filter(|value| valid_identifier(value));
    let medication_identifier = body
        .get("person_medication")
        .and_then(Value::as_object)
        .and_then(|inner| inner.get("medication_id"))
        .and_then(Value::as_str)
        .filter(|value| valid_identifier(value));
    let (Some(person_identifier), Some(medication_identifier)) =
        (person_identifier, medication_identifier)
    else {
        let status = parsed.err().unwrap_or(StatusCode::UNPROCESSABLE_ENTITY);
        return failure(db, &context, "POST", "create", status).await;
    };
    let (_, context) = mutation_idempotency::lock_household_and_reauthenticate(
        &state,
        &db,
        &headers,
        household_id,
    )
    .await?;
    let person = match find_person(&db, &context, person_identifier).await? {
        Some(value) => value,
        None => return failure(db, &context, "POST", "create", StatusCode::NOT_FOUND).await,
    };
    if !can_manage_person(&db, &context, person.id).await? {
        return failure(db, &context, "POST", "create", StatusCode::FORBIDDEN).await;
    }
    let medication = match find_medication(&db, &context, medication_identifier).await? {
        Some(value) => value,
        None => return failure(db, &context, "POST", "create", StatusCode::NOT_FOUND).await,
    };
    let request_path = path(household_id, None);
    let request_digest = mutation_idempotency::digest("POST", &request_path, &body);
    if let Some(response) = replay_or_conflict(
        &db,
        &context,
        &headers,
        "POST",
        "create",
        &request_path,
        &request_digest,
    )
    .await?
    {
        db.commit().await.map_err(database_error)?;
        return Ok(response);
    }
    let attrs = match parsed {
        Ok(value) => value,
        Err(StatusCode::UNPROCESSABLE_ENTITY) => {
            return validation_failure(
                db,
                &context,
                &headers,
                "POST",
                "create",
                &request_path,
                &request_digest,
            )
            .await;
        }
        Err(status) => return failure(db, &context, "POST", "create", status).await,
    };
    if person_medication::Entity::find()
        .filter(person_medication::Column::PersonId.eq(person.id))
        .filter(person_medication::Column::MedicationId.eq(medication.id))
        .filter(person_medication::Column::RetiredAt.is_null())
        .one(&db)
        .await
        .map_err(database_error)?
        .is_some()
    {
        return validation_failure(
            db,
            &context,
            &headers,
            "POST",
            "create",
            &request_path,
            &request_digest,
        )
        .await;
    }
    let selected_option = if let Some(identifier) = attrs.source_dosage_option_id.as_deref() {
        match find_visible_option(&db, &context, identifier).await? {
            Some(option) => Some(option),
            None => return failure(db, &context, "POST", "create", StatusCode::NOT_FOUND).await,
        }
    } else {
        None
    };
    let Some((amount, unit, source_option)) = resolved_dose(
        &db,
        &context,
        &attrs,
        &person,
        &medication,
        None,
        selected_option.as_ref(),
    )
    .await?
    else {
        return validation_failure(
            db,
            &context,
            &headers,
            "POST",
            "create",
            &request_path,
            &request_digest,
        )
        .await;
    };
    let position = person_medication::Entity::find()
        .filter(person_medication::Column::PersonId.eq(person.id))
        .order_by_desc(person_medication::Column::Position)
        .one(&db)
        .await
        .map_err(database_error)?
        .map_or(1, |record| record.position + 1);
    let now = Utc::now().naive_utc();
    let kind = attrs.administration_kind.unwrap_or(1);
    let min_hours = attrs.min_hours_between_doses.unwrap_or(None);
    let min_hours = if kind == 0 && attrs.max_daily_doses == Some(1) && min_hours == Some(24) {
        None
    } else {
        min_hours
    };
    let record = person_medication::ActiveModel {
        household_id: Set(household_id),
        person_id: Set(person.id),
        medication_id: Set(medication.id),
        active: Set(true),
        administration_kind: Set(kind),
        dose_amount: Set(Some(amount)),
        dose_unit: Set(Some(unit)),
        source_dosage_option_id: Set(source_option),
        notes: Set(attrs.notes),
        max_daily_doses: Set(attrs.max_daily_doses),
        min_hours_between_doses: Set(min_hours),
        dose_cycle: Set(attrs.dose_cycle),
        position: Set(position),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .map_err(database_error)?;
    let (response_body, etag) = representation(&db, &context, record.id).await?;
    let request_id = Uuid::new_v4().to_string();
    record_version(
        &db,
        &context,
        &request_id,
        "PersonMedication",
        record.id,
        "create",
        None,
        Some(response_body["data"].clone()),
    )
    .await?;
    record_change(
        &db,
        &context,
        &request_id,
        SyncRecord {
            record_type: "PersonMedication",
            record_id: record.id,
            portable_id: &record.portable_id,
            action: "create",
            person_portable_id: Some(&person.portable_id),
        },
    )
    .await?;
    finish_write(
        db,
        WriteCompletion {
            context: &context,
            headers: &headers,
            method: "POST",
            action: "create",
            request_path: &request_path,
            request_digest: &request_digest,
            request_id: &request_id,
            status: StatusCode::CREATED,
            body: response_body,
            etag: &etag,
        },
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

pub(super) async fn update(
    state: AppState,
    household_id: i64,
    id: String,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
    method: &str,
) -> Result<Response, ApiError> {
    let (db, _) = request_context(&state, &headers, household_id).await?;
    let (_, context) = mutation_idempotency::lock_household_and_reauthenticate(
        &state,
        &db,
        &headers,
        household_id,
    )
    .await?;
    let record = match find_assignment(&db, &context, &id).await? {
        Some(value) => value,
        None => return failure(db, &context, method, "update", StatusCode::NOT_FOUND).await,
    };
    if !can_manage_person(&db, &context, record.person_id).await? {
        return failure(db, &context, method, "update", StatusCode::FORBIDDEN).await;
    }
    let Json(body) = match payload {
        Ok(value) => value,
        Err(_) => return failure(db, &context, method, "update", StatusCode::BAD_REQUEST).await,
    };
    let request_path = path(household_id, Some(&id));
    let request_digest = mutation_idempotency::digest(method, &request_path, &body);
    if let Some(response) = replay_or_conflict(
        &db,
        &context,
        &headers,
        method,
        "update",
        &request_path,
        &request_digest,
    )
    .await?
    {
        db.commit().await.map_err(database_error)?;
        return Ok(response);
    }
    let attrs = match Attributes::parse(&body, false) {
        Ok(value) => value,
        Err(StatusCode::UNPROCESSABLE_ENTITY) => {
            return validation_failure(
                db,
                &context,
                &headers,
                method,
                "update",
                &request_path,
                &request_digest,
            )
            .await;
        }
        Err(status) => return failure(db, &context, method, "update", status).await,
    };
    let (before_body, current_etag) = representation(&db, &context, record.id).await?;
    if headers
        .get(header::IF_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| !value.is_empty() && value != current_etag)
    {
        return failure(db, &context, method, "update", StatusCode::CONFLICT).await;
    }
    let person = person::Entity::find_by_id(record.person_id)
        .one(&db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?;
    if let Some(identifier) = attrs.person_id.as_deref() {
        let Some(requested) = find_person(&db, &context, identifier).await? else {
            return failure(db, &context, method, "update", StatusCode::NOT_FOUND).await;
        };
        if requested.id != record.person_id {
            return validation_failure(
                db,
                &context,
                &headers,
                method,
                "update",
                &request_path,
                &request_digest,
            )
            .await;
        }
    }
    let medication = if let Some(identifier) = attrs.medication_id.as_deref() {
        match find_medication(&db, &context, identifier).await? {
            Some(value) => value,
            None => return failure(db, &context, method, "update", StatusCode::NOT_FOUND).await,
        }
    } else {
        medication::Entity::find_by_id(record.medication_id)
            .one(&db)
            .await
            .map_err(database_error)?
            .ok_or_else(ApiError::not_found)?
    };
    if medication.id != record.medication_id
        && person_medication::Entity::find()
            .filter(person_medication::Column::PersonId.eq(record.person_id))
            .filter(person_medication::Column::MedicationId.eq(medication.id))
            .filter(person_medication::Column::RetiredAt.is_null())
            .one(&db)
            .await
            .map_err(database_error)?
            .is_some()
    {
        return validation_failure(
            db,
            &context,
            &headers,
            method,
            "update",
            &request_path,
            &request_digest,
        )
        .await;
    }
    let selected_option = if let Some(identifier) = attrs.source_dosage_option_id.as_deref() {
        match find_visible_option(&db, &context, identifier).await? {
            Some(option) => Some(option),
            None => return failure(db, &context, method, "update", StatusCode::NOT_FOUND).await,
        }
    } else if let Some(option_id) = record.source_dosage_option_id {
        dosage::Entity::find_by_id(option_id)
            .one(&db)
            .await
            .map_err(database_error)?
    } else {
        None
    };
    let Some((amount, unit, source_option)) = resolved_dose(
        &db,
        &context,
        &attrs,
        &person,
        &medication,
        Some(&record),
        selected_option.as_ref(),
    )
    .await?
    else {
        return validation_failure(
            db,
            &context,
            &headers,
            method,
            "update",
            &request_path,
            &request_digest,
        )
        .await;
    };
    let kind = attrs
        .administration_kind
        .unwrap_or(record.administration_kind);
    let maximum = attrs.max_daily_doses.or(record.max_daily_doses);
    let mut minimum = attrs
        .min_hours_between_doses
        .unwrap_or(record.min_hours_between_doses);
    if kind == 0 && maximum == Some(1) && minimum == Some(24) {
        minimum = None;
    }
    let unchanged = medication.id == record.medication_id
        && record.dose_amount == Some(amount)
        && record.dose_unit.as_deref() == Some(unit.as_str())
        && record.source_dosage_option_id == source_option
        && record.administration_kind == kind
        && record.notes == attrs.notes.clone().or(record.notes.clone())
        && record.max_daily_doses == maximum
        && record.min_hours_between_doses == minimum
        && record.dose_cycle == attrs.dose_cycle.or(record.dose_cycle);
    if unchanged {
        let request_id = Uuid::new_v4().to_string();
        return finish_write(
            db,
            WriteCompletion {
                context: &context,
                headers: &headers,
                method,
                action: "update",
                request_path: &request_path,
                request_digest: &request_digest,
                request_id: &request_id,
                status: StatusCode::OK,
                body: before_body,
                etag: &current_etag,
            },
        )
        .await;
    }
    let mut active = record.clone().into_active_model();
    active.medication_id = Set(medication.id);
    active.dose_amount = Set(Some(amount));
    active.dose_unit = Set(Some(unit));
    active.source_dosage_option_id = Set(source_option);
    if let Some(value) = attrs.administration_kind {
        active.administration_kind = Set(value);
    }
    if let Some(value) = attrs.notes {
        active.notes = Set(Some(value));
    }
    if let Some(value) = attrs.max_daily_doses {
        active.max_daily_doses = Set(Some(value));
    }
    if let Some(value) = attrs.min_hours_between_doses {
        active.min_hours_between_doses = Set(value);
    }
    if let Some(value) = attrs.dose_cycle {
        active.dose_cycle = Set(Some(value));
    }
    active.min_hours_between_doses = Set(minimum);
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active.update(&db).await.map_err(database_error)?;
    let (response_body, etag) = representation(&db, &context, updated.id).await?;
    let request_id = Uuid::new_v4().to_string();
    record_version(
        &db,
        &context,
        &request_id,
        "PersonMedication",
        updated.id,
        "update",
        Some(before_body["data"].clone()),
        Some(response_body["data"].clone()),
    )
    .await?;
    record_change(
        &db,
        &context,
        &request_id,
        SyncRecord {
            record_type: "PersonMedication",
            record_id: updated.id,
            portable_id: &updated.portable_id,
            action: "update",
            person_portable_id: Some(&person.portable_id),
        },
    )
    .await?;
    finish_write(
        db,
        WriteCompletion {
            context: &context,
            headers: &headers,
            method,
            action: "update",
            request_path: &request_path,
            request_digest: &request_digest,
            request_id: &request_id,
            status: StatusCode::OK,
            body: response_body,
            etag: &etag,
        },
    )
    .await
}
