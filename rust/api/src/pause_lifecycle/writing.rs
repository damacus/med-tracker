use super::*;

pub(crate) async fn create(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    guard: Option<axum::Extension<BrowserSourceGuard>>,
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
        Ok(body) => body,
        Err(_) => {
            return fail(
                db,
                &context,
                "POST",
                PERIOD_CONTROLLER,
                PERIOD_POLICY,
                "create",
                Failure::Malformed,
            )
            .await
        }
    };
    let (kind, source_id) = match source_identity(&body) {
        Ok(attributes) => attributes,
        Err(failure) => {
            return fail(
                db,
                &context,
                "POST",
                PERIOD_CONTROLLER,
                PERIOD_POLICY,
                "create",
                failure,
            )
            .await
        }
    };
    let Some(source) = find_source(&db, &context, kind, source_id, false).await? else {
        return fail(
            db,
            &context,
            "POST",
            PERIOD_CONTROLLER,
            PERIOD_POLICY,
            "create",
            Failure::NotFound,
        )
        .await;
    };
    if !person_access(&db, &context, source.person_id(), true).await? {
        return fail(
            db,
            &context,
            "POST",
            PERIOD_CONTROLLER,
            PERIOD_POLICY,
            "create",
            Failure::Forbidden,
        )
        .await;
    }
    let path = format!("/api/v1/households/{household_id}/medication_pause_periods");
    match keyed_replay(&db, &context, &headers, "POST", &path, &body, "create").await? {
        Replay::New => {}
        Replay::Saved(response) => {
            db.commit().await.map_err(database_error)?;
            return Ok(response);
        }
        Replay::Conflict => {
            return fail(
                db,
                &context,
                "POST",
                PERIOD_CONTROLLER,
                PERIOD_POLICY,
                "create",
                Failure::KeyConflict,
            )
            .await
        }
    }
    if let Some(axum::Extension(guard)) = guard {
        if let Some(failure) = browser_guard::validate(&db, &context, &source, &guard).await? {
            return fail(
                db,
                &context,
                "POST",
                PERIOD_CONTROLLER,
                PERIOD_POLICY,
                "create",
                failure,
            )
            .await;
        }
    }
    let (_, _, reason, note) = match create_attributes(&body) {
        Ok(attributes) => attributes,
        Err(Failure::Invalid(field, message)) => {
            return keyed_invalid(
                db, &context, &headers, "POST", "create", &path, &body, field, message,
            )
            .await;
        }
        Err(failure) => {
            return fail(
                db,
                &context,
                "POST",
                PERIOD_CONTROLLER,
                PERIOD_POLICY,
                "create",
                failure,
            )
            .await
        }
    };
    let request_id = Uuid::new_v4().to_string();
    let (_, period) = pause_source(&db, &context, source, reason, note, &request_id).await?;
    let source = source_for_period(&db, &context, &period, false)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let (response_body, etag) = period_body(&db, &period, &source).await?;
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
    finish(
        db,
        &context,
        "POST",
        PERIOD_CONTROLLER,
        PERIOD_POLICY,
        "create",
        StatusCode::CREATED,
        response_body,
        Some(&etag),
        &request_id,
    )
    .await
}

pub(crate) async fn resume(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    guard: Option<axum::Extension<BrowserSourceGuard>>,
    payload: Bytes,
) -> Result<Response, ApiError> {
    let (db, _) = request_context(&state, &headers, household_id).await?;
    let (_, context) = mutation_idempotency::lock_household_and_reauthenticate(
        &state,
        &db,
        &headers,
        household_id,
    )
    .await?;
    if Uuid::parse_str(&id).is_err() {
        return fail(
            db,
            &context,
            "POST",
            PERIOD_CONTROLLER,
            PERIOD_POLICY,
            "resume",
            Failure::NotFound,
        )
        .await;
    }
    let Some(period) = pause_period::Entity::find()
        .filter(pause_period::Column::HouseholdId.eq(household_id))
        .filter(pause_period::Column::PortableId.eq(&id))
        .one(&db)
        .await
        .map_err(database_error)?
    else {
        return fail(
            db,
            &context,
            "POST",
            PERIOD_CONTROLLER,
            PERIOD_POLICY,
            "resume",
            Failure::NotFound,
        )
        .await;
    };
    let Some(source) = source_for_period(&db, &context, &period, false).await? else {
        return fail(
            db,
            &context,
            "POST",
            PERIOD_CONTROLLER,
            PERIOD_POLICY,
            "resume",
            Failure::NotFound,
        )
        .await;
    };
    if !person_access(&db, &context, source.person_id(), true).await? {
        return fail(
            db,
            &context,
            "POST",
            PERIOD_CONTROLLER,
            PERIOD_POLICY,
            "resume",
            Failure::Forbidden,
        )
        .await;
    }
    let body = match empty_request(&payload) {
        Ok(body) => body,
        Err(failure) => {
            return fail(
                db,
                &context,
                "POST",
                PERIOD_CONTROLLER,
                PERIOD_POLICY,
                "resume",
                failure,
            )
            .await
        }
    };
    let path = format!("/api/v1/households/{household_id}/medication_pause_periods/{id}/resume");
    match keyed_replay(&db, &context, &headers, "POST", &path, &body, "resume").await? {
        Replay::New => {}
        Replay::Saved(response) => {
            db.commit().await.map_err(database_error)?;
            return Ok(response);
        }
        Replay::Conflict => {
            return fail(
                db,
                &context,
                "POST",
                PERIOD_CONTROLLER,
                PERIOD_POLICY,
                "resume",
                Failure::KeyConflict,
            )
            .await
        }
    }
    if let Some(axum::Extension(guard)) = guard {
        if let Some(failure) = browser_guard::validate(&db, &context, &source, &guard).await? {
            return fail(
                db,
                &context,
                "POST",
                PERIOD_CONTROLLER,
                PERIOD_POLICY,
                "resume",
                failure,
            )
            .await;
        }
        if headers
            .get(header::IF_MATCH)
            .and_then(|value| value.to_str().ok())
            .is_none_or(|value| value.trim().is_empty())
        {
            return fail(
                db,
                &context,
                "POST",
                PERIOD_CONTROLLER,
                PERIOD_POLICY,
                "resume",
                Failure::PreconditionRequired,
            )
            .await;
        }
    }
    let (_, etag) = period_body(&db, &period, &source).await?;
    if headers
        .get(header::IF_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| !value.is_empty() && value != etag)
    {
        return fail(
            db,
            &context,
            "POST",
            PERIOD_CONTROLLER,
            PERIOD_POLICY,
            "resume",
            Failure::Conflict,
        )
        .await;
    }
    let request_id = Uuid::new_v4().to_string();
    let (source, period) = close_period(&db, &context, source, Some(period), &request_id).await?;
    let period = period.ok_or_else(ApiError::not_found)?;
    let (response_body, etag) = period_body(&db, &period, &source).await?;
    store_key(
        &db,
        &context,
        &headers,
        "POST",
        &path,
        &body,
        StatusCode::OK,
        &response_body,
        &request_id,
        Some(&etag),
    )
    .await?;
    finish(
        db,
        &context,
        "POST",
        PERIOD_CONTROLLER,
        PERIOD_POLICY,
        "resume",
        StatusCode::OK,
        response_body,
        Some(&etag),
        &request_id,
    )
    .await
}
