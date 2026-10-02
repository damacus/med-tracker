use super::*;

pub(super) async fn legacy_action(
    state: AppState,
    household_id: i64,
    id: String,
    headers: HeaderMap,
    payload: Bytes,
    kind: Kind,
    pause: bool,
) -> Result<Response, ApiError> {
    let (db, _) = request_context(&state, &headers, household_id).await?;
    let (_, context) = mutation_idempotency::lock_household_and_reauthenticate(
        &state,
        &db,
        &headers,
        household_id,
    )
    .await?;
    let action = if pause { "pause" } else { "resume" };
    let Some(source) = find_source_path(&db, &context, kind, &id).await? else {
        return fail(
            db,
            &context,
            "PATCH",
            kind.controller(),
            kind.policy(),
            action,
            Failure::NotFound,
        )
        .await;
    };
    if !person_access(&db, &context, source.person_id(), true).await? {
        return fail(
            db,
            &context,
            "PATCH",
            kind.controller(),
            kind.policy(),
            action,
            Failure::Forbidden,
        )
        .await;
    }
    let _body = match empty_request(&payload) {
        Ok(body) => body,
        Err(failure) => {
            return fail(
                db,
                &context,
                "PATCH",
                kind.controller(),
                kind.policy(),
                action,
                failure,
            )
            .await
        }
    };
    let request_id = Uuid::new_v4().to_string();
    let source = if pause {
        pause_source(
            &db,
            &context,
            source,
            "reason_not_recorded",
            None,
            &request_id,
        )
        .await?
        .0
    } else {
        close_period(&db, &context, source, None, &request_id)
            .await?
            .0
    };
    let response_body = source_body(&db, &context, &source).await?;
    let etag = representation_etag(&response_body);
    finish(
        db,
        &context,
        "PATCH",
        kind.controller(),
        kind.policy(),
        action,
        StatusCode::OK,
        response_body,
        Some(&etag),
        &request_id,
    )
    .await
}

pub(crate) async fn pause_schedule(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Bytes,
) -> Result<Response, ApiError> {
    legacy_action(
        state,
        household_id,
        id,
        headers,
        payload,
        Kind::Schedule,
        true,
    )
    .await
}

pub(crate) async fn resume_schedule(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Bytes,
) -> Result<Response, ApiError> {
    legacy_action(
        state,
        household_id,
        id,
        headers,
        payload,
        Kind::Schedule,
        false,
    )
    .await
}

pub(crate) async fn pause_assignment(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Bytes,
) -> Result<Response, ApiError> {
    legacy_action(
        state,
        household_id,
        id,
        headers,
        payload,
        Kind::Assignment,
        true,
    )
    .await
}

pub(crate) async fn resume_assignment(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Bytes,
) -> Result<Response, ApiError> {
    legacy_action(
        state,
        household_id,
        id,
        headers,
        payload,
        Kind::Assignment,
        false,
    )
    .await
}

pub(crate) async fn reorder_assignment(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
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
    let Some(Source::Assignment(source)) =
        find_source_path(&db, &context, Kind::Assignment, &id).await?
    else {
        return fail(
            db,
            &context,
            "PATCH",
            Kind::Assignment.controller(),
            Kind::Assignment.policy(),
            "reorder",
            Failure::NotFound,
        )
        .await;
    };
    if !person_access(&db, &context, source.person_id, true).await? {
        return fail(
            db,
            &context,
            "PATCH",
            Kind::Assignment.controller(),
            Kind::Assignment.policy(),
            "reorder",
            Failure::Forbidden,
        )
        .await;
    }
    let Json(body) = match payload {
        Ok(body) => body,
        Err(_) => {
            return fail(
                db,
                &context,
                "PATCH",
                Kind::Assignment.controller(),
                Kind::Assignment.policy(),
                "reorder",
                Failure::Malformed,
            )
            .await
        }
    };
    let Some(map) = body.as_object() else {
        return fail(
            db,
            &context,
            "PATCH",
            Kind::Assignment.controller(),
            Kind::Assignment.policy(),
            "reorder",
            Failure::Malformed,
        )
        .await;
    };
    let direction = map.get("direction").and_then(Value::as_str);
    if map.len() != 1 || !matches!(direction, Some("up" | "down")) {
        return fail(
            db,
            &context,
            "PATCH",
            Kind::Assignment.controller(),
            Kind::Assignment.policy(),
            "reorder",
            Failure::Invalid("direction", "must be up or down"),
        )
        .await;
    }
    let direction = direction.unwrap();
    let mut query = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(household_id))
        .filter(person_medication::Column::PersonId.eq(source.person_id))
        .filter(person_medication::Column::RetiredAt.is_null())
        .filter(if direction == "up" {
            person_medication::Column::Position.lt(source.position)
        } else {
            person_medication::Column::Position.gt(source.position)
        });
    query = if direction == "up" {
        query
            .order_by_desc(person_medication::Column::Position)
            .order_by_desc(person_medication::Column::Id)
    } else {
        query
            .order_by_asc(person_medication::Column::Position)
            .order_by_asc(person_medication::Column::Id)
    };
    let adjacent = query.one(&db).await.map_err(database_error)?;
    let request_id = Uuid::new_v4().to_string();
    let updated = if let Some(adjacent) = adjacent {
        let original_position = source.position;
        let adjacent_position = adjacent.position;
        let mut moving: person_medication::ActiveModel = source.clone().into();
        moving.position = Set(adjacent_position);
        moving.updated_at = Set(Utc::now().naive_utc());
        let moving = moving.update(&db).await.map_err(database_error)?;
        let mut swapping: person_medication::ActiveModel = adjacent.clone().into();
        swapping.position = Set(original_position);
        swapping.updated_at = Set(Utc::now().naive_utc());
        let swapping = swapping.update(&db).await.map_err(database_error)?;
        let person_portable_id = person::Entity::find_by_id(source.person_id)
            .one(&db)
            .await
            .map_err(database_error)?
            .ok_or_else(ApiError::not_found)?
            .portable_id;
        for (before, after) in [(&source, &moving), (&adjacent, &swapping)] {
            record_version(
                &db,
                &context,
                &request_id,
                "PersonMedication",
                after.id,
                "update",
                Some(json!({"position": before.position})),
                Some(json!({"position": after.position})),
            )
            .await?;
            record_change(
                &db,
                &context,
                &request_id,
                SyncRecord {
                    record_type: "PersonMedication",
                    record_id: after.id,
                    portable_id: &after.portable_id,
                    action: "update",
                    person_portable_id: Some(&person_portable_id),
                },
            )
            .await?;
        }
        moving
    } else {
        source
    };
    let response_body = source_body(&db, &context, &Source::Assignment(updated)).await?;
    let etag = representation_etag(&response_body);
    finish(
        db,
        &context,
        "PATCH",
        Kind::Assignment.controller(),
        Kind::Assignment.policy(),
        "reorder",
        StatusCode::OK,
        response_body,
        Some(&etag),
        &request_id,
    )
    .await
}
