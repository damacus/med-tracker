use super::*;

pub(super) fn sync_failure(failure: Failure) -> ApiError {
    let (status, code, message) = match failure {
        Failure::PreconditionRequired => (
            StatusCode::PRECONDITION_REQUIRED,
            "precondition_required",
            "If-Match is required",
        ),
        Failure::Malformed => (
            StatusCode::BAD_REQUEST,
            "bad_request",
            "Invalid request body",
        ),
        Failure::Invalid(_, _) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "unprocessable_content",
            "Attributes are invalid",
        ),
        Failure::NotFound => (StatusCode::NOT_FOUND, "not_found", "Record not found"),
        Failure::Forbidden => (
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
        ),
        Failure::Conflict => (
            StatusCode::CONFLICT,
            "conflict",
            "Record has changed since it was last read",
        ),
        Failure::KeyConflict => (
            StatusCode::CONFLICT,
            "idempotency_key_reused",
            "Idempotency key has already been used for a different request",
        ),
    };
    ApiError {
        status,
        code,
        message,
        preserve_activity: false,
    }
}

pub(super) fn sync_result(source: &Source, etag: String) -> SyncResult {
    SyncResult {
        record_type: source.kind().record_type(),
        record_id: Some(source.id()),
        record_portable_id: Some(source.portable_id().to_owned()),
        etag: Some(etag),
        replayed: None,
    }
}

pub(super) fn sync_precondition(actual: &str, expected: Option<&str>) -> Result<(), ApiError> {
    let expected = expected.ok_or(ApiError {
        status: StatusCode::PRECONDITION_REQUIRED,
        code: "precondition_required",
        message: "A current resource version is required",
        preserve_activity: false,
    })?;
    if expected != actual {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            code: "sync_conflict",
            message: "Record has changed since it was last read",
            preserve_activity: false,
        });
    }
    Ok(())
}

pub(crate) async fn apply_sync_operation(
    db: &DatabaseTransaction,
    context: &AuthContext,
    operation: &SyncOperation,
    request_id: &str,
) -> Result<SyncResult, ApiError> {
    match operation.resource_type.as_str() {
        "schedule" | "person_medication" => {
            let kind = if operation.resource_type == "schedule" {
                Kind::Schedule
            } else {
                Kind::Assignment
            };
            let id = operation
                .id
                .as_deref()
                .ok_or_else(|| sync_failure(Failure::NotFound))?;
            let source = find_source_path(db, context, kind, id)
                .await?
                .ok_or_else(|| sync_failure(Failure::NotFound))?;
            if !person_access(db, context, source.person_id(), true).await? {
                return Err(sync_failure(Failure::Forbidden));
            }
            let current_etag = representation_etag(&source_body(db, context, &source).await?);
            sync_precondition(&current_etag, operation.if_match.as_deref())?;
            if operation.action == "reorder" {
                let Source::Assignment(source) = source else {
                    return Err(sync_failure(Failure::NotFound));
                };
                return reorder_sync(db, context, source, operation, request_id).await;
            }
            let source = match operation.action.as_str() {
                "pause" => {
                    if operation
                        .attributes
                        .keys()
                        .any(|key| !matches!(key.as_str(), "reason" | "note"))
                    {
                        return Err(sync_failure(Failure::Invalid(
                            "attributes",
                            "contains an unsupported field",
                        )));
                    }
                    let reason = operation
                        .attributes
                        .get("reason")
                        .and_then(Value::as_str)
                        .filter(|reason| REASONS.contains(reason))
                        .ok_or_else(|| sync_failure(Failure::Invalid("reason", "is invalid")))?;
                    let note = match operation.attributes.get("note") {
                        None | Some(Value::Null) => None,
                        Some(Value::String(value)) => Some(value.clone()),
                        _ => {
                            return Err(sync_failure(Failure::Invalid("note", "must be a string")))
                        }
                    };
                    pause_source(db, context, source, reason, note, request_id)
                        .await?
                        .0
                }
                "resume" => {
                    if !operation.attributes.is_empty() {
                        return Err(sync_failure(Failure::Malformed));
                    }
                    close_period(db, context, source, None, request_id).await?.0
                }
                _ => return Err(sync_failure(Failure::Malformed)),
            };
            let etag = representation_etag(&source_body(db, context, &source).await?);
            Ok(sync_result(&source, etag))
        }
        "medication_pause_period" => apply_period_sync(db, context, operation, request_id).await,
        _ => Err(sync_failure(Failure::Malformed)),
    }
}

pub(super) async fn apply_period_sync(
    db: &DatabaseTransaction,
    context: &AuthContext,
    operation: &SyncOperation,
    request_id: &str,
) -> Result<SyncResult, ApiError> {
    let (source, period, replayed) = if operation.action == "create" {
        let body = json!({"medication_pause_period": operation.attributes});
        if operation
            .attributes
            .get("source_id")
            .and_then(Value::as_str)
            .is_some_and(|id| id.parse::<i64>().is_ok())
        {
            return Err(sync_failure(Failure::NotFound));
        }
        let (kind, id, reason, note) = create_attributes(&body).map_err(sync_failure)?;
        let source = find_source(db, context, kind, id, false)
            .await?
            .ok_or_else(|| sync_failure(Failure::NotFound))?;
        if !person_access(db, context, source.person_id(), true).await? {
            return Err(sync_failure(Failure::Forbidden));
        }
        let existing = open_period(db, &source).await?.is_some();
        let (source, period) = pause_source(db, context, source, reason, note, request_id).await?;
        (source, period, existing)
    } else if operation.action == "close" {
        if !operation.attributes.is_empty() {
            return Err(sync_failure(Failure::Invalid(
                "attributes",
                "must be empty",
            )));
        }
        let id = operation
            .id
            .as_deref()
            .ok_or_else(|| sync_failure(Failure::NotFound))?;
        let period = pause_period::Entity::find()
            .filter(pause_period::Column::HouseholdId.eq(context.membership.household_id))
            .filter(pause_period::Column::PortableId.eq(id))
            .one(db)
            .await
            .map_err(database_error)?
            .ok_or_else(|| sync_failure(Failure::NotFound))?;
        let source = source_for_period(db, context, &period, false)
            .await?
            .ok_or_else(|| sync_failure(Failure::NotFound))?;
        if !person_access(db, context, source.person_id(), true).await? {
            return Err(sync_failure(Failure::Forbidden));
        }
        let (_, etag) = period_body(db, &period, &source).await?;
        sync_precondition(&etag, operation.if_match.as_deref())?;
        let replayed = period.ended_at.is_some();
        let (source, period) = close_period(db, context, source, Some(period), request_id).await?;
        (source, period.ok_or_else(ApiError::not_found)?, replayed)
    } else {
        return Err(sync_failure(Failure::Malformed));
    };
    let (_, etag) = period_body(db, &period, &source).await?;
    Ok(SyncResult {
        record_type: "MedicationPausePeriod",
        record_id: Some(period.id),
        record_portable_id: Some(period.portable_id),
        etag: Some(etag),
        replayed: Some(replayed),
    })
}

pub(super) async fn reorder_sync(
    db: &DatabaseTransaction,
    context: &AuthContext,
    source: person_medication::Model,
    operation: &SyncOperation,
    request_id: &str,
) -> Result<SyncResult, ApiError> {
    let direction = operation
        .attributes
        .get("direction")
        .and_then(Value::as_str);
    if operation.attributes.len() != 1 || !matches!(direction, Some("up" | "down")) {
        return Err(ApiError {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: "unprocessable_content",
            message: "Direction must be up or down",
            preserve_activity: false,
        });
    }
    let direction = direction.unwrap();
    let mut query = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(context.membership.household_id))
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
    let adjacent = query.one(db).await.map_err(database_error)?;
    let updated = if let Some(adjacent) = adjacent {
        let mut moving: person_medication::ActiveModel = source.clone().into();
        moving.position = Set(adjacent.position);
        moving.updated_at = Set(Utc::now().naive_utc());
        let moving = moving.update(db).await.map_err(database_error)?;
        let mut swapping: person_medication::ActiveModel = adjacent.clone().into();
        swapping.position = Set(source.position);
        swapping.updated_at = Set(Utc::now().naive_utc());
        let swapping = swapping.update(db).await.map_err(database_error)?;
        let person_portable_id = person::Entity::find_by_id(source.person_id)
            .one(db)
            .await
            .map_err(database_error)?
            .ok_or_else(ApiError::not_found)?
            .portable_id;
        for (before, after) in [(&source, &moving), (&adjacent, &swapping)] {
            record_version(
                db,
                context,
                request_id,
                "PersonMedication",
                after.id,
                "update",
                Some(json!({"position": before.position})),
                Some(json!({"position": after.position})),
            )
            .await?;
            record_change(
                db,
                context,
                request_id,
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
    let source = Source::Assignment(updated);
    let etag = representation_etag(&source_body(db, context, &source).await?);
    Ok(sync_result(&source, etag))
}

pub(crate) async fn authorize_sync_replay(
    db: &DatabaseTransaction,
    context: &AuthContext,
    operation: &SyncOperation,
    saved: &Value,
) -> Result<(), ApiError> {
    if saved.get("action").and_then(Value::as_str) != Some(operation.action.as_str()) {
        return Err(ApiError::forbidden());
    }
    let portable_id = saved
        .get("record_portable_id")
        .and_then(Value::as_str)
        .ok_or_else(ApiError::forbidden)?;
    if operation.resource_type == "medication_pause_period" {
        if saved.get("record_type").and_then(Value::as_str) != Some("MedicationPausePeriod") {
            return Err(ApiError::forbidden());
        }
        let period = pause_period::Entity::find()
            .filter(pause_period::Column::HouseholdId.eq(context.membership.household_id))
            .filter(pause_period::Column::PortableId.eq(portable_id))
            .one(db)
            .await
            .map_err(database_error)?
            .ok_or_else(ApiError::forbidden)?;
        if saved.get("record_id").and_then(Value::as_str) != Some(period.id.to_string().as_str())
            || operation
                .id
                .as_deref()
                .is_some_and(|id| id != period.portable_id && id != period.id.to_string())
        {
            return Err(ApiError::forbidden());
        }
        let source = source_for_period(db, context, &period, true)
            .await?
            .ok_or_else(ApiError::forbidden)?;
        if !person_access(db, context, source.person_id(), true).await? {
            return Err(ApiError::forbidden());
        }
        if operation.action == "create"
            && (operation
                .attributes
                .get("source_type")
                .and_then(Value::as_str)
                != Some(source.kind().name())
                || operation
                    .attributes
                    .get("source_id")
                    .and_then(Value::as_str)
                    != Some(source.portable_id()))
        {
            return Err(ApiError::forbidden());
        }
        return Ok(());
    }
    let kind = match operation.resource_type.as_str() {
        "schedule" => Kind::Schedule,
        "person_medication" => Kind::Assignment,
        _ => return Err(ApiError::forbidden()),
    };
    if saved.get("record_type").and_then(Value::as_str) != Some(kind.record_type()) {
        return Err(ApiError::forbidden());
    }
    let source = find_source(db, context, kind, portable_id, true)
        .await?
        .ok_or_else(ApiError::forbidden)?;
    if saved.get("record_id").and_then(Value::as_str) != Some(source.id().to_string().as_str())
        || operation
            .id
            .as_deref()
            .is_none_or(|id| id != source.portable_id() && id != source.id().to_string())
        || !person_access(db, context, source.person_id(), true).await?
    {
        return Err(ApiError::forbidden());
    }
    Ok(())
}
