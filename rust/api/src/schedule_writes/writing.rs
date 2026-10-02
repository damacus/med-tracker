use super::*;

pub(crate) async fn create(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    let (db, _) = request_context(&state, &headers, household_id).await?;
    let (_, context) = mutation_idempotency::lock_household_and_reauthenticate(
        &state,
        &db,
        &headers,
        household_id,
    )
    .await?;
    let Json(body) = match payload {
        Ok(payload) => payload,
        Err(_) => {
            return input_error(db, &context, "POST", "create", InputFailure::Malformed).await
        }
    };
    let attributes = match attributes(&body) {
        Ok(attributes) => attributes,
        Err(failure) => return input_error(db, &context, "POST", "create", failure).await,
    };
    let Some(raw_person_id) = attributes.get("person_id") else {
        return input_error(
            db,
            &context,
            "POST",
            "create",
            InputFailure::Invalid("person_id", "can't be blank"),
        )
        .await;
    };
    let person_id = match identifier(raw_person_id, "person_id") {
        Ok(id) => id,
        Err(failure) => return input_error(db, &context, "POST", "create", failure).await,
    };
    let Some(person) = find_person(&db, &context, person_id).await? else {
        return input_error(db, &context, "POST", "create", InputFailure::NotFound).await;
    };
    if !person_access(&db, &context, person.id, false).await? {
        return input_error(db, &context, "POST", "create", InputFailure::NotFound).await;
    }
    if !person_access(&db, &context, person.id, true).await? {
        return input_error(db, &context, "POST", "create", InputFailure::Forbidden).await;
    }
    let path = format!("/api/v1/households/{household_id}/schedules");
    match keyed_replay(&db, &context, &headers, "POST", "create", &path, &body).await? {
        ReplayDecision::New => {}
        ReplayDecision::Replay(response) => {
            db.commit().await.map_err(database_error)?;
            return Ok(response);
        }
        ReplayDecision::Conflict => {
            return error_response(
                db,
                &context,
                "POST",
                CONTROLLER,
                POLICY,
                "create",
                StatusCode::CONFLICT,
                "idempotency_key_reused",
                "Idempotency key has already been used for a different request",
                None,
            )
            .await;
        }
    }
    let mut fields = ScheduleFields::new();
    if let Err(failure) = apply_attributes(&db, &context, attributes, &mut fields, None).await? {
        return keyed_validation(
            db, &context, &headers, "POST", "create", &path, &body, failure,
        )
        .await;
    }
    if let Err(failure) = fields.validate() {
        return keyed_validation(
            db, &context, &headers, "POST", "create", &path, &body, failure,
        )
        .await;
    }
    let now = Utc::now().naive_utc();
    let mut active = schedule::ActiveModel {
        household_id: Set(household_id),
        portable_id: Set(Uuid::new_v4().to_string()),
        active: Set(true),
        retired_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    fields.assign(&mut active);
    let created = active.insert(&db).await.map_err(database_error)?;
    let request_id = Uuid::new_v4().to_string();
    record_version(
        &db,
        &context,
        &request_id,
        "Schedule",
        created.id,
        "create",
        None,
        Some(snapshot(&created)),
    )
    .await?;
    let person = person::Entity::find_by_id(created.person_id)
        .one(&db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?;
    record_change(
        &db,
        &context,
        &request_id,
        SyncRecord {
            record_type: "Schedule",
            record_id: created.id,
            portable_id: &created.portable_id,
            action: "create",
            person_portable_id: Some(&person.portable_id),
        },
    )
    .await?;
    let (response_body, etag) = schedule_body(&db, &context, created.id).await?;
    store_key(
        &db,
        &context,
        &headers,
        "POST",
        &path,
        &body,
        StatusCode::CREATED,
        &response_body,
        &request_id,
        Some(&etag),
    )
    .await?;
    finish_with_request_id(
        db,
        &context,
        &request_id,
        "POST",
        CONTROLLER,
        POLICY,
        "create",
        StatusCode::CREATED,
        true,
        response_body,
        Some(&etag),
    )
    .await
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
    let Some(found) = find_schedule(&db, &context, &id).await? else {
        return input_error(db, &context, method, "update", InputFailure::NotFound).await;
    };
    if !person_access(&db, &context, found.person_id, true).await? {
        return input_error(db, &context, method, "update", InputFailure::Forbidden).await;
    }
    let Json(body) = match payload {
        Ok(payload) => payload,
        Err(_) => {
            return input_error(db, &context, method, "update", InputFailure::Malformed).await
        }
    };
    let path = format!("/api/v1/households/{household_id}/schedules/{id}");
    match keyed_replay(&db, &context, &headers, method, "update", &path, &body).await? {
        ReplayDecision::New => {}
        ReplayDecision::Replay(response) => {
            db.commit().await.map_err(database_error)?;
            return Ok(response);
        }
        ReplayDecision::Conflict => {
            return error_response(
                db,
                &context,
                method,
                CONTROLLER,
                POLICY,
                "update",
                StatusCode::CONFLICT,
                "idempotency_key_reused",
                "Idempotency key has already been used for a different request",
                None,
            )
            .await;
        }
    }
    let (_, current_etag) = schedule_body(&db, &context, found.id).await?;
    if headers
        .get(header::IF_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| !value.is_empty() && value != current_etag)
    {
        return error_response(
            db,
            &context,
            method,
            CONTROLLER,
            POLICY,
            "update",
            StatusCode::CONFLICT,
            "conflict",
            "Record has changed since it was last read",
            None,
        )
        .await;
    }
    let attributes = match attributes(&body) {
        Ok(attributes) => attributes,
        Err(failure) => {
            return keyed_validation(
                db, &context, &headers, method, "update", &path, &body, failure,
            )
            .await
        }
    };
    let mut fields = ScheduleFields::from_record(&found);
    if let Err(failure) =
        apply_attributes(&db, &context, attributes, &mut fields, Some(&found)).await?
    {
        return keyed_validation(
            db, &context, &headers, method, "update", &path, &body, failure,
        )
        .await;
    }
    if let Err(failure) = fields.validate() {
        return keyed_validation(
            db, &context, &headers, method, "update", &path, &body, failure,
        )
        .await;
    }
    if fields == ScheduleFields::from_record(&found) {
        let (response_body, etag) = schedule_body(&db, &context, found.id).await?;
        let request_id = Uuid::new_v4().to_string();
        store_key(
            &db,
            &context,
            &headers,
            method,
            &path,
            &body,
            StatusCode::OK,
            &response_body,
            &request_id,
            Some(&etag),
        )
        .await?;
        return finish_with_request_id(
            db,
            &context,
            &request_id,
            method,
            CONTROLLER,
            POLICY,
            "update",
            StatusCode::OK,
            true,
            response_body,
            Some(&etag),
        )
        .await;
    }
    let before = snapshot(&found);
    let mut active: schedule::ActiveModel = found.into();
    fields.assign(&mut active);
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active.update(&db).await.map_err(database_error)?;
    let request_id = Uuid::new_v4().to_string();
    record_version(
        &db,
        &context,
        &request_id,
        "Schedule",
        updated.id,
        "update",
        Some(before),
        Some(snapshot(&updated)),
    )
    .await?;
    let person = person::Entity::find_by_id(updated.person_id)
        .one(&db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?;
    record_change(
        &db,
        &context,
        &request_id,
        SyncRecord {
            record_type: "Schedule",
            record_id: updated.id,
            portable_id: &updated.portable_id,
            action: "update",
            person_portable_id: Some(&person.portable_id),
        },
    )
    .await?;
    let (response_body, etag) = schedule_body(&db, &context, updated.id).await?;
    store_key(
        &db,
        &context,
        &headers,
        method,
        &path,
        &body,
        StatusCode::OK,
        &response_body,
        &request_id,
        Some(&etag),
    )
    .await?;
    finish_with_request_id(
        db,
        &context,
        &request_id,
        method,
        CONTROLLER,
        POLICY,
        "update",
        StatusCode::OK,
        true,
        response_body,
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
