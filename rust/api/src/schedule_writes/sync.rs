use super::*;

pub(super) fn sync_error(failure: InputFailure) -> ApiError {
    let (status, code, message) = match failure {
        InputFailure::Malformed => (
            StatusCode::BAD_REQUEST,
            "bad_request",
            "Invalid request body",
        ),
        InputFailure::NotFound => (StatusCode::NOT_FOUND, "not_found", "Record not found"),
        InputFailure::Forbidden => (
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
        ),
        InputFailure::Invalid(_, _) | InputFailure::InvalidFields(_) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "unprocessable_content",
            "Schedule is invalid",
        ),
    };
    ApiError {
        status,
        code,
        message,
        preserve_activity: false,
    }
}

pub(super) fn sync_precondition(actual: &str, expected: Option<&str>) -> Result<(), ApiError> {
    let expected = expected.ok_or(ApiError {
        status: StatusCode::PRECONDITION_REQUIRED,
        code: "precondition_required",
        message: "A current resource version is required",
        preserve_activity: false,
    })?;
    if actual != expected {
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
    let household_id = context.membership.household_id;
    let record = if operation.action == "create" {
        let body = json!({"schedule": operation.attributes});
        let attributes = attributes(&body).map_err(sync_error)?;
        let raw_person_id = attributes
            .get("person_id")
            .ok_or_else(|| sync_error(InputFailure::Invalid("person_id", "can't be blank")))?;
        let person_id = identifier(raw_person_id, "person_id").map_err(sync_error)?;
        let person = find_person(db, context, person_id)
            .await?
            .ok_or_else(|| sync_error(InputFailure::NotFound))?;
        if !person_access(db, context, person.id, false).await? {
            return Err(sync_error(InputFailure::NotFound));
        }
        if !person_access(db, context, person.id, true).await? {
            return Err(sync_error(InputFailure::Forbidden));
        }
        let mut fields = ScheduleFields::new();
        apply_attributes(db, context, attributes, &mut fields, None)
            .await?
            .map_err(sync_error)?;
        fields.validate().map_err(sync_error)?;
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
        let record = active.insert(db).await.map_err(database_error)?;
        record_version(
            db,
            context,
            request_id,
            "Schedule",
            record.id,
            "create",
            None,
            Some(snapshot(&record)),
        )
        .await?;
        record_change(
            db,
            context,
            request_id,
            SyncRecord {
                record_type: "Schedule",
                record_id: record.id,
                portable_id: &record.portable_id,
                action: "create",
                person_portable_id: Some(&person.portable_id),
            },
        )
        .await?;
        record
    } else {
        let id = operation
            .id
            .as_deref()
            .ok_or_else(|| sync_error(InputFailure::NotFound))?;
        let found = find_schedule(db, context, id)
            .await?
            .ok_or_else(|| sync_error(InputFailure::NotFound))?;
        if !person_access(db, context, found.person_id, true).await? {
            return Err(sync_error(InputFailure::Forbidden));
        }
        let (_, current_etag) = schedule_body(db, context, found.id).await?;
        sync_precondition(&current_etag, operation.if_match.as_deref())?;
        match operation.action.as_str() {
            "update" => {
                let body = json!({"schedule": operation.attributes});
                let attributes = attributes(&body).map_err(sync_error)?;
                let mut fields = ScheduleFields::from_record(&found);
                apply_attributes(db, context, attributes, &mut fields, Some(&found))
                    .await?
                    .map_err(sync_error)?;
                fields.validate().map_err(sync_error)?;
                if fields == ScheduleFields::from_record(&found) {
                    found
                } else {
                    let before = snapshot(&found);
                    let mut active: schedule::ActiveModel = found.clone().into();
                    fields.assign(&mut active);
                    active.updated_at = Set(Utc::now().naive_utc());
                    let updated = active.update(db).await.map_err(database_error)?;
                    record_version(
                        db,
                        context,
                        request_id,
                        "Schedule",
                        updated.id,
                        "update",
                        Some(before),
                        Some(snapshot(&updated)),
                    )
                    .await?;
                    let person = person::Entity::find_by_id(updated.person_id)
                        .one(db)
                        .await
                        .map_err(database_error)?
                        .ok_or_else(ApiError::not_found)?;
                    record_change(
                        db,
                        context,
                        request_id,
                        SyncRecord {
                            record_type: "Schedule",
                            record_id: updated.id,
                            portable_id: &updated.portable_id,
                            action: "update",
                            person_portable_id: Some(&person.portable_id),
                        },
                    )
                    .await?;
                    updated
                }
            }
            "delete" => {
                if !operation.attributes.is_empty() {
                    return Err(sync_error(InputFailure::Malformed));
                }
                let before = snapshot(&found);
                let mut active: schedule::ActiveModel = found.clone().into();
                let now = Utc::now().naive_utc();
                active.retired_at = Set(Some(now));
                active.active = Set(false);
                active.updated_at = Set(now);
                let retired = active.update(db).await.map_err(database_error)?;
                record_version(
                    db,
                    context,
                    request_id,
                    "Schedule",
                    retired.id,
                    "destroy",
                    Some(before),
                    None,
                )
                .await?;
                let person = person::Entity::find_by_id(retired.person_id)
                    .one(db)
                    .await
                    .map_err(database_error)?
                    .ok_or_else(ApiError::not_found)?;
                record_change(
                    db,
                    context,
                    request_id,
                    SyncRecord {
                        record_type: "Schedule",
                        record_id: retired.id,
                        portable_id: &retired.portable_id,
                        action: "update",
                        person_portable_id: Some(&person.portable_id),
                    },
                )
                .await?;
                api_tombstone::ActiveModel {
                    household_id: Set(household_id),
                    household_membership_id: Set(Some(context.membership.id)),
                    account_id: Set(Some(context.account_id)),
                    action: Set("delete".to_owned()),
                    record_type: Set("Schedule".to_owned()),
                    record_portable_id: Set(retired.portable_id.clone()),
                    metadata: Set(json!({"record_type": "Schedule", "record_id": retired.id,
                        "portable_id": retired.portable_id, "person_portable_id": person.portable_id})),
                    deleted_at: Set(now), created_at: Set(now), updated_at: Set(now),
                    ..Default::default()
                }.insert(db).await.map_err(database_error)?;
                return Ok(SyncResult {
                    record_type: "Schedule",
                    record_id: Some(retired.id),
                    record_portable_id: Some(retired.portable_id),
                    etag: None,
                    replayed: None,
                });
            }
            _ => return Err(sync_error(InputFailure::Malformed)),
        }
    };
    let (_, etag) = schedule_body(db, context, record.id).await?;
    Ok(SyncResult {
        record_type: "Schedule",
        record_id: Some(record.id),
        record_portable_id: Some(record.portable_id),
        etag: Some(etag),
        replayed: None,
    })
}

pub(crate) async fn authorize_sync_replay(
    db: &DatabaseTransaction,
    context: &AuthContext,
    operation: &SyncOperation,
    saved: &Value,
) -> Result<(), ApiError> {
    if saved.get("action").and_then(Value::as_str) != Some(operation.action.as_str())
        || saved.get("record_type").and_then(Value::as_str) != Some("Schedule")
    {
        return Err(ApiError::forbidden());
    }
    let portable_id = saved
        .get("record_portable_id")
        .and_then(Value::as_str)
        .ok_or_else(ApiError::forbidden)?;
    let record = schedule::Entity::find()
        .filter(schedule::Column::HouseholdId.eq(context.membership.household_id))
        .filter(schedule::Column::PortableId.eq(portable_id))
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::forbidden)?;
    if saved.get("record_id").and_then(Value::as_str) != Some(record.id.to_string().as_str())
        || operation
            .id
            .as_deref()
            .is_some_and(|id| id != record.portable_id && id != record.id.to_string())
        || !person_access(db, context, record.person_id, true).await?
    {
        return Err(ApiError::forbidden());
    }
    if let Some(id) = operation
        .attributes
        .get("person_id")
        .and_then(Value::as_str)
    {
        if find_person(db, context, id)
            .await?
            .is_none_or(|person| person.id != record.person_id)
        {
            return Err(ApiError::forbidden());
        }
    }
    if let Some(id) = operation
        .attributes
        .get("medication_id")
        .and_then(Value::as_str)
    {
        if visible_medication(db, context, id).await?.is_none() {
            return Err(ApiError::forbidden());
        }
    }
    if operation.action == "delete"
        && (record.retired_at.is_none()
            || api_tombstone::Entity::find()
                .filter(api_tombstone::Column::HouseholdId.eq(context.membership.household_id))
                .filter(api_tombstone::Column::RecordType.eq("Schedule"))
                .filter(api_tombstone::Column::RecordPortableId.eq(&record.portable_id))
                .one(db)
                .await
                .map_err(database_error)?
                .is_none())
    {
        return Err(ApiError::forbidden());
    }
    Ok(())
}
