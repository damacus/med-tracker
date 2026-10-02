use super::*;

pub(super) fn sync_error(status: StatusCode) -> ApiError {
    let (code, message) = match status {
        StatusCode::NOT_FOUND => ("not_found", "Record not found"),
        StatusCode::FORBIDDEN => (
            "forbidden",
            "You are not authorized to perform this action.",
        ),
        StatusCode::PRECONDITION_REQUIRED => (
            "precondition_required",
            "A current resource version is required",
        ),
        StatusCode::CONFLICT => ("sync_conflict", "Record has changed since it was last read"),
        StatusCode::BAD_REQUEST => ("bad_request", "Invalid request body"),
        _ => ("unprocessable_content", "Person medication is invalid"),
    };
    ApiError {
        status,
        code,
        message,
        preserve_activity: false,
    }
}

pub(super) fn sync_precondition(actual: &str, expected: Option<&str>) -> Result<(), ApiError> {
    let expected = expected.ok_or_else(|| sync_error(StatusCode::PRECONDITION_REQUIRED))?;
    if actual != expected {
        return Err(sync_error(StatusCode::CONFLICT));
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
    let (record, changed) = if operation.action == "create" {
        let body = json!({"person_medication": operation.attributes});
        let attrs = Attributes::parse(&body, true).map_err(sync_error)?;
        let person = find_person(db, context, attrs.person_id.as_deref().unwrap())
            .await?
            .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?;
        if !can_manage_person(db, context, person.id).await? {
            return Err(sync_error(StatusCode::FORBIDDEN));
        }
        let medication = find_medication(db, context, attrs.medication_id.as_deref().unwrap())
            .await?
            .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?;
        if person_medication::Entity::find()
            .filter(person_medication::Column::PersonId.eq(person.id))
            .filter(person_medication::Column::MedicationId.eq(medication.id))
            .filter(person_medication::Column::RetiredAt.is_null())
            .one(db)
            .await
            .map_err(database_error)?
            .is_some()
        {
            return Err(sync_error(StatusCode::UNPROCESSABLE_ENTITY));
        }
        let selected_option = if let Some(id) = attrs.source_dosage_option_id.as_deref() {
            Some(
                find_visible_option(db, context, id)
                    .await?
                    .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?,
            )
        } else {
            None
        };
        let (amount, unit, source_option) = resolved_dose(
            db,
            context,
            &attrs,
            &person,
            &medication,
            None,
            selected_option.as_ref(),
        )
        .await?
        .ok_or_else(|| sync_error(StatusCode::UNPROCESSABLE_ENTITY))?;
        let position = person_medication::Entity::find()
            .filter(person_medication::Column::PersonId.eq(person.id))
            .order_by_desc(person_medication::Column::Position)
            .one(db)
            .await
            .map_err(database_error)?
            .map_or(1, |row| row.position + 1);
        let now = Utc::now().naive_utc();
        let kind = attrs.administration_kind.unwrap_or(1);
        let mut minimum = attrs.min_hours_between_doses.unwrap_or(None);
        if kind == 0 && attrs.max_daily_doses == Some(1) && minimum == Some(24) {
            minimum = None;
        }
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
            min_hours_between_doses: Set(minimum),
            dose_cycle: Set(attrs.dose_cycle),
            position: Set(position),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(database_error)?;
        let (body, _) = representation(db, context, record.id).await?;
        record_version(
            db,
            context,
            request_id,
            "PersonMedication",
            record.id,
            "create",
            None,
            Some(body["data"].clone()),
        )
        .await?;
        record_change(
            db,
            context,
            request_id,
            SyncRecord {
                record_type: "PersonMedication",
                record_id: record.id,
                portable_id: &record.portable_id,
                action: "create",
                person_portable_id: Some(&person.portable_id),
            },
        )
        .await?;
        (record, true)
    } else {
        let id = operation
            .id
            .as_deref()
            .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?;
        let found = find_assignment(db, context, id)
            .await?
            .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?;
        if !can_manage_person(db, context, found.person_id).await? {
            return Err(sync_error(StatusCode::FORBIDDEN));
        }
        let (before, current_etag) = representation(db, context, found.id).await?;
        sync_precondition(&current_etag, operation.if_match.as_deref())?;
        match operation.action.as_str() {
            "update" => {
                let body = json!({"person_medication": operation.attributes});
                let attrs = Attributes::parse(&body, false).map_err(sync_error)?;
                let person = person::Entity::find_by_id(found.person_id)
                    .one(db)
                    .await
                    .map_err(database_error)?
                    .ok_or_else(ApiError::not_found)?;
                if let Some(id) = attrs.person_id.as_deref() {
                    let requested = find_person(db, context, id)
                        .await?
                        .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?;
                    if requested.id != found.person_id {
                        return Err(sync_error(StatusCode::UNPROCESSABLE_ENTITY));
                    }
                }
                let medication = if let Some(id) = attrs.medication_id.as_deref() {
                    find_medication(db, context, id)
                        .await?
                        .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?
                } else {
                    medication::Entity::find_by_id(found.medication_id)
                        .one(db)
                        .await
                        .map_err(database_error)?
                        .ok_or_else(ApiError::not_found)?
                };
                if medication.id != found.medication_id
                    && person_medication::Entity::find()
                        .filter(person_medication::Column::PersonId.eq(found.person_id))
                        .filter(person_medication::Column::MedicationId.eq(medication.id))
                        .filter(person_medication::Column::RetiredAt.is_null())
                        .one(db)
                        .await
                        .map_err(database_error)?
                        .is_some()
                {
                    return Err(sync_error(StatusCode::UNPROCESSABLE_ENTITY));
                }
                let selected_option = if let Some(id) = attrs.source_dosage_option_id.as_deref() {
                    Some(
                        find_visible_option(db, context, id)
                            .await?
                            .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?,
                    )
                } else if let Some(id) = found.source_dosage_option_id {
                    dosage::Entity::find_by_id(id)
                        .one(db)
                        .await
                        .map_err(database_error)?
                } else {
                    None
                };
                let (amount, unit, source_option) = resolved_dose(
                    db,
                    context,
                    &attrs,
                    &person,
                    &medication,
                    Some(&found),
                    selected_option.as_ref(),
                )
                .await?
                .ok_or_else(|| sync_error(StatusCode::UNPROCESSABLE_ENTITY))?;
                let kind = attrs
                    .administration_kind
                    .unwrap_or(found.administration_kind);
                let maximum = attrs.max_daily_doses.or(found.max_daily_doses);
                let mut minimum = attrs
                    .min_hours_between_doses
                    .unwrap_or(found.min_hours_between_doses);
                if kind == 0 && maximum == Some(1) && minimum == Some(24) {
                    minimum = None;
                }
                let unchanged = medication.id == found.medication_id
                    && found.dose_amount == Some(amount)
                    && found.dose_unit.as_deref() == Some(unit.as_str())
                    && found.source_dosage_option_id == source_option
                    && found.administration_kind == kind
                    && found.notes == attrs.notes.clone().or(found.notes.clone())
                    && found.max_daily_doses == maximum
                    && found.min_hours_between_doses == minimum
                    && found.dose_cycle == attrs.dose_cycle.or(found.dose_cycle);
                if unchanged {
                    (found, false)
                } else {
                    let mut active = found.clone().into_active_model();
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
                    if let Some(value) = attrs.dose_cycle {
                        active.dose_cycle = Set(Some(value));
                    }
                    active.min_hours_between_doses = Set(minimum);
                    active.updated_at = Set(Utc::now().naive_utc());
                    let updated = active.update(db).await.map_err(database_error)?;
                    let (after, _) = representation(db, context, updated.id).await?;
                    record_version(
                        db,
                        context,
                        request_id,
                        "PersonMedication",
                        updated.id,
                        "update",
                        Some(before["data"].clone()),
                        Some(after["data"].clone()),
                    )
                    .await?;
                    record_change(
                        db,
                        context,
                        request_id,
                        SyncRecord {
                            record_type: "PersonMedication",
                            record_id: updated.id,
                            portable_id: &updated.portable_id,
                            action: "update",
                            person_portable_id: Some(&person.portable_id),
                        },
                    )
                    .await?;
                    (updated, true)
                }
            }
            "delete" => {
                if !operation.attributes.is_empty() {
                    return Err(sync_error(StatusCode::UNPROCESSABLE_ENTITY));
                }
                let mut active = found.clone().into_active_model();
                let now = Utc::now().naive_utc();
                active.retired_at = Set(Some(now));
                active.active = Set(false);
                active.updated_at = Set(now);
                let retired = active.update(db).await.map_err(database_error)?;
                record_version(
                    db,
                    context,
                    request_id,
                    "PersonMedication",
                    retired.id,
                    "destroy",
                    Some(before["data"].clone()),
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
                        record_type: "PersonMedication",
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
                    record_type: Set("PersonMedication".to_owned()),
                    record_portable_id: Set(retired.portable_id.clone()),
                    metadata: Set(json!({"record_type": "PersonMedication", "record_id": retired.id,
                        "portable_id": retired.portable_id, "person_portable_id": person.portable_id})),
                    deleted_at: Set(now), created_at: Set(now), updated_at: Set(now),
                    ..Default::default()
                }.insert(db).await.map_err(database_error)?;
                return Ok(SyncResult {
                    record_type: "PersonMedication",
                    record_id: Some(retired.id),
                    record_portable_id: Some(retired.portable_id),
                    etag: None,
                    replayed: None,
                });
            }
            _ => return Err(sync_error(StatusCode::UNPROCESSABLE_ENTITY)),
        }
    };
    let (_, etag) = representation(db, context, record.id).await?;
    Ok(SyncResult {
        record_type: "PersonMedication",
        record_id: Some(record.id),
        record_portable_id: Some(record.portable_id),
        etag: Some(etag),
        replayed: Some(!changed),
    })
}

pub(crate) async fn authorize_sync_replay(
    db: &DatabaseTransaction,
    context: &AuthContext,
    operation: &SyncOperation,
    saved: &Value,
) -> Result<(), ApiError> {
    if saved.get("action").and_then(Value::as_str) != Some(operation.action.as_str())
        || saved.get("record_type").and_then(Value::as_str) != Some("PersonMedication")
    {
        return Err(ApiError::forbidden());
    }
    let portable_id = saved
        .get("record_portable_id")
        .and_then(Value::as_str)
        .ok_or_else(ApiError::forbidden)?;
    let record = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(context.membership.household_id))
        .filter(person_medication::Column::PortableId.eq(portable_id))
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::forbidden)?;
    if saved.get("record_id").and_then(Value::as_str) != Some(record.id.to_string().as_str())
        || operation
            .id
            .as_deref()
            .is_some_and(|id| id != record.portable_id && id != record.id.to_string())
        || !can_manage_person(db, context, record.person_id).await?
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
        if find_medication(db, context, id).await?.is_none() {
            return Err(ApiError::forbidden());
        }
    }
    if operation.action == "delete"
        && (record.retired_at.is_none()
            || api_tombstone::Entity::find()
                .filter(api_tombstone::Column::HouseholdId.eq(context.membership.household_id))
                .filter(api_tombstone::Column::RecordType.eq("PersonMedication"))
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
