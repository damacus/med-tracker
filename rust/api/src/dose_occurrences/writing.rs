use super::*;

#[allow(clippy::too_many_arguments)]
async fn mutate(
    state: AppState,
    household_id: i64,
    id: String,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
    kind: Kind,
    action: &'static str,
    method: &'static str,
) -> Result<Response, ApiError> {
    let (db, _) = request_context(&state, &headers, household_id).await?;
    let (_, context) = mutation_idempotency::lock_household_and_reauthenticate(
        &state,
        &db,
        &headers,
        household_id,
    )
    .await?;
    let Some(source) = find_source(&db, &context, kind, &id).await? else {
        return fail(db, &context, kind, method, action, Failure::NotFound).await;
    };
    if !person_access(&db, &context, source.person_id(), action).await? {
        return fail(db, &context, kind, method, action, Failure::Forbidden).await;
    }
    let Json(body) = match payload {
        Ok(body) => body,
        Err(_) => return fail(db, &context, kind, method, action, Failure::Malformed).await,
    };
    let path = format!(
        "/api/v1/households/{household_id}/{}/{id}/dose_occurrences/{action}",
        kind.path_segment(),
    );
    match keyed_replay(&db, &context, &headers, kind, method, action, &path, &body).await? {
        Replay::New => {}
        Replay::Saved(response) => {
            db.commit().await.map_err(database_error)?;
            return Ok(response);
        }
        Replay::Conflict => {
            return fail(db, &context, kind, method, action, Failure::KeyConflict).await
        }
    }
    let attributes = match attributes(&body, action) {
        Ok(attributes) => attributes,
        Err(failure) => {
            return fail_mutation(
                db, &context, &headers, kind, method, action, &path, &body, failure,
            )
            .await
        }
    };
    let secret = state.oauth.occurrence_key_secret();
    let Some(mut row) =
        find_row(&db, &secret, &source, attributes["key"].as_str().unwrap()).await?
    else {
        return fail_mutation(
            db,
            &context,
            &headers,
            kind,
            method,
            action,
            &path,
            &body,
            Failure::InvalidOccurrence,
        )
        .await;
    };
    let request_id = Uuid::new_v4().to_string();
    match action {
        "not_taken" => {
            let (reason, note) = match parse_not_taken(attributes) {
                Ok(values) => values,
                Err(failure) => {
                    return fail_mutation(
                        db, &context, &headers, kind, method, action, &path, &body, failure,
                    )
                    .await
                }
            };
            if let Some(record) = row.record.as_ref() {
                if record.outcome != "open" {
                    if record.outcome == "not_taken"
                        && record.reason == reason
                        && record.note == note
                    {
                        let data = row_value(&secret, &source, &row);
                        let etag = record_etag(record);
                        let response_body = json!({"data": data});
                        store_key(
                            &db,
                            &context,
                            &headers,
                            method,
                            &path,
                            &body,
                            &response_body,
                            &request_id,
                            &etag,
                        )
                        .await?;
                        return finish(
                            db,
                            &context,
                            kind,
                            method,
                            action,
                            response_body,
                            Some(&etag),
                            &request_id,
                        )
                        .await;
                    }
                    return fail(db, &context, kind, method, action, Failure::AlreadyResolved)
                        .await;
                }
            }
            if row.legacy_take_id.is_some() {
                return fail(db, &context, kind, method, action, Failure::AlreadyResolved).await;
            }
            if !actionable(&source, &row) {
                return fail_mutation(
                    db,
                    &context,
                    &headers,
                    kind,
                    method,
                    action,
                    &path,
                    &body,
                    Failure::InvalidOccurrence,
                )
                .await;
            }
            row.record =
                Some(save_decision(&db, &context, &source, &row, reason, note, &request_id).await?);
        }
        "reopen" => {
            let Some(record) = row.record.as_ref() else {
                return fail_mutation(
                    db,
                    &context,
                    &headers,
                    kind,
                    method,
                    action,
                    &path,
                    &body,
                    Failure::InvalidOccurrence,
                )
                .await;
            };
            if record.outcome != "not_taken" {
                return fail_mutation(
                    db,
                    &context,
                    &headers,
                    kind,
                    method,
                    action,
                    &path,
                    &body,
                    Failure::InvalidOccurrence,
                )
                .await;
            }
            let Some(if_match) = headers
                .get(header::IF_MATCH)
                .and_then(|value| value.to_str().ok())
                .filter(|value| !value.is_empty())
            else {
                return fail_mutation(
                    db,
                    &context,
                    &headers,
                    kind,
                    method,
                    action,
                    &path,
                    &body,
                    Failure::PreconditionRequired,
                )
                .await;
            };
            if if_match != record_etag(record) {
                return fail(db, &context, kind, method, action, Failure::SyncConflict).await;
            }
            row.record = Some(reopen_decision(&db, &context, &source, record, &request_id).await?);
        }
        "take" => {
            let taken_at = match parse_take(attributes) {
                Ok(value) => value,
                Err(failure) => {
                    return fail_mutation(
                        db, &context, &headers, kind, method, action, &path, &body, failure,
                    )
                    .await
                }
            };
            if row.legacy_take_id.is_some() {
                return fail(db, &context, kind, method, action, Failure::AlreadyResolved).await;
            }
            if let Some(record) = row.record.as_ref() {
                if record.outcome == "not_taken" {
                    let Some(if_match) = headers
                        .get(header::IF_MATCH)
                        .and_then(|value| value.to_str().ok())
                        .filter(|value| !value.is_empty())
                    else {
                        return fail_mutation(
                            db,
                            &context,
                            &headers,
                            kind,
                            method,
                            action,
                            &path,
                            &body,
                            Failure::PreconditionRequired,
                        )
                        .await;
                    };
                    if if_match != record_etag(record) {
                        return fail(db, &context, kind, method, action, Failure::SyncConflict)
                            .await;
                    }
                } else if record.outcome == "taken" {
                    let supplied_uuid = attributes.get("client_uuid").and_then(Value::as_str);
                    let linked_uuid = match record.medication_take_id {
                        Some(id) => medication_take::Entity::find_by_id(id)
                            .one(&db)
                            .await
                            .map_err(database_error)?
                            .and_then(|take| take.client_uuid),
                        None => None,
                    };
                    if supplied_uuid.is_none() || supplied_uuid != linked_uuid.as_deref() {
                        return fail(db, &context, kind, method, action, Failure::AlreadyResolved)
                            .await;
                    }
                }
            }
            if row
                .record
                .as_ref()
                .is_none_or(|record| record.outcome != "taken")
            {
                if !actionable(&source, &row) {
                    return fail_mutation(
                        db,
                        &context,
                        &headers,
                        kind,
                        method,
                        action,
                        &path,
                        &body,
                        Failure::InvalidOccurrence,
                    )
                    .await;
                }
                let taken_day = dose::local_date(taken_at);
                let in_window = if kind == Kind::Schedule {
                    taken_day == row.window_start
                } else {
                    taken_day >= row.window_start
                        && taken_day <= row.window_end
                        && taken_at >= source.created_at()
                };
                if !in_window {
                    return fail_mutation(
                        db,
                        &context,
                        &headers,
                        kind,
                        method,
                        action,
                        &path,
                        &body,
                        Failure::Invalid("taken_at", "does not match the occurrence window"),
                    )
                    .await;
                }
            }
            let mut take_attributes = Map::new();
            take_attributes.insert("source_type".to_owned(), json!(kind.name()));
            take_attributes.insert("source_id".to_owned(), json!(source.portable_id()));
            for field in [
                "taken_at",
                "client_uuid",
                "dose_amount",
                "taken_from_medication_id",
            ] {
                if let Some(value) = attributes.get(field) {
                    take_attributes.insert(field.to_owned(), value.clone());
                }
            }
            let take_body = json!({"medication_take": take_attributes});
            let take = match dose::create_with_failure(
                &db,
                &context,
                household_id,
                &take_body,
                &request_id,
            )
            .await
            {
                Ok((_, take)) => take,
                Err(error) => {
                    let error = error.into_occurrence_error();
                    db.rollback().await.map_err(database_error)?;
                    let (audit_db, _) = request_context(&state, &headers, household_id).await?;
                    let (_, audit_context) =
                        mutation_idempotency::lock_household_and_reauthenticate(
                            &state,
                            &audit_db,
                            &headers,
                            household_id,
                        )
                        .await?;
                    if error.status.is_server_error() || error.status == StatusCode::CONFLICT {
                        return fail_api(audit_db, &audit_context, kind, method, action, error)
                            .await;
                    }
                    return finish_cached_error(
                        audit_db,
                        &audit_context,
                        &headers,
                        kind,
                        method,
                        action,
                        &path,
                        &body,
                        error.status,
                        error.code,
                        error.message,
                        None,
                    )
                    .await;
                }
            };
            if let Some(record) = row
                .record
                .as_ref()
                .filter(|record| record.outcome == "taken")
            {
                if record.medication_take_id != Some(take.id) {
                    return fail(db, &context, kind, method, action, Failure::AlreadyResolved)
                        .await;
                }
            } else {
                row.record =
                    Some(link_take(&db, &context, &source, &row, &take, &request_id).await?);
            }
        }
        _ => return Err(ApiError::internal()),
    }
    let data = row_value(&secret, &source, &row);
    let etag = row
        .record
        .as_ref()
        .map(record_etag)
        .ok_or_else(ApiError::internal)?;
    let response_body = json!({"data": data});
    store_key(
        &db,
        &context,
        &headers,
        method,
        &path,
        &body,
        &response_body,
        &request_id,
        &etag,
    )
    .await?;
    finish(
        db,
        &context,
        kind,
        method,
        action,
        response_body,
        Some(&etag),
        &request_id,
    )
    .await
}

pub(crate) async fn not_taken_schedule(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    mutate(
        state,
        household_id,
        id,
        headers,
        payload,
        Kind::Schedule,
        "not_taken",
        "POST",
    )
    .await
}

pub(crate) async fn not_taken_assignment(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    mutate(
        state,
        household_id,
        id,
        headers,
        payload,
        Kind::Assignment,
        "not_taken",
        "POST",
    )
    .await
}

pub(crate) async fn reopen_schedule(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    mutate(
        state,
        household_id,
        id,
        headers,
        payload,
        Kind::Schedule,
        "reopen",
        "PATCH",
    )
    .await
}

pub(crate) async fn reopen_assignment(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    mutate(
        state,
        household_id,
        id,
        headers,
        payload,
        Kind::Assignment,
        "reopen",
        "PATCH",
    )
    .await
}

pub(crate) async fn take_schedule(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    mutate(
        state,
        household_id,
        id,
        headers,
        payload,
        Kind::Schedule,
        "take",
        "POST",
    )
    .await
}

pub(crate) async fn take_assignment(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    mutate(
        state,
        household_id,
        id,
        headers,
        payload,
        Kind::Assignment,
        "take",
        "POST",
    )
    .await
}
