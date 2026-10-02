use super::*;

pub(crate) async fn apply_sync_operation(
    db: &DatabaseTransaction,
    context: &AuthContext,
    operation: &SyncOperation,
    request_id: &str,
    secret: &Arc<[u8]>,
) -> Result<SyncResult, ApiError> {
    let (source, mut row) = if operation.action == "create" {
        let attrs = &operation.attributes;
        if attrs.keys().any(|key| {
            !matches!(
                key.as_str(),
                "source_type" | "source_id" | "occurrence_key" | "outcome" | "reason" | "note"
            )
        }) {
            return Err(sync_error(Failure::Invalid(
                "attributes",
                "contains an unsupported field",
            )));
        }
        if attrs.get("outcome").and_then(Value::as_str) != Some("not_taken") {
            return Err(sync_error(Failure::InvalidOccurrence));
        }
        let kind = match attrs.get("source_type").and_then(Value::as_str) {
            Some("schedule") => Kind::Schedule,
            Some("person_medication") => Kind::Assignment,
            _ => return Err(sync_error(Failure::InvalidOccurrence)),
        };
        let source_id = attrs
            .get("source_id")
            .and_then(Value::as_str)
            .ok_or_else(|| sync_error(Failure::InvalidOccurrence))?;
        let source = find_source(db, context, kind, source_id)
            .await?
            .ok_or_else(|| sync_error(Failure::InvalidOccurrence))?;
        if !person_access(db, context, source.person_id(), "not_taken").await? {
            return Err(sync_error(Failure::Forbidden));
        }
        let occurrence_key = attrs
            .get("occurrence_key")
            .and_then(Value::as_str)
            .ok_or_else(|| sync_error(Failure::InvalidOccurrence))?;
        let row = find_row(db, secret, &source, occurrence_key)
            .await?
            .ok_or_else(|| sync_error(Failure::InvalidOccurrence))?;
        (source, row)
    } else if operation.action == "update" {
        if operation.attributes.get("outcome").and_then(Value::as_str) != Some("open")
            || operation.attributes.len() != 1
        {
            return Err(sync_error(Failure::InvalidOccurrence));
        }
        let id = operation
            .id
            .as_deref()
            .ok_or_else(|| sync_error(Failure::NotFound))?;
        let mut query = dose_occurrence::Entity::find()
            .filter(dose_occurrence::Column::HouseholdId.eq(context.membership.household_id));
        query = match id.parse::<i64>() {
            Ok(id) => query.filter(dose_occurrence::Column::Id.eq(id)),
            Err(_) => query.filter(dose_occurrence::Column::PortableId.eq(id)),
        };
        let record = query
            .one(db)
            .await
            .map_err(database_error)?
            .ok_or_else(|| sync_error(Failure::NotFound))?;
        let (kind, source_id) = if let Some(id) = record.schedule_id {
            (Kind::Schedule, id)
        } else if let Some(id) = record.person_medication_id {
            (Kind::Assignment, id)
        } else {
            return Err(sync_error(Failure::InvalidOccurrence));
        };
        let source = find_source(db, context, kind, &source_id.to_string())
            .await?
            .ok_or_else(|| sync_error(Failure::InvalidOccurrence))?;
        if !person_access(db, context, source.person_id(), "reopen").await? {
            return Err(sync_error(Failure::Forbidden));
        }
        let occurrence_key = key(secret, &source, record.window_starts_on, record.position);
        let row = find_row(db, secret, &source, &occurrence_key)
            .await?
            .ok_or_else(|| sync_error(Failure::InvalidOccurrence))?;
        if row
            .record
            .as_ref()
            .is_none_or(|row_record| row_record.id != record.id)
        {
            return Err(sync_error(Failure::InvalidOccurrence));
        }
        (source, row)
    } else {
        return Err(sync_error(Failure::InvalidOccurrence));
    };
    let replayed = if operation.action == "create" {
        let (reason, note) = parse_not_taken(&operation.attributes).map_err(sync_error)?;
        if let Some(record) = row.record.as_ref() {
            if record.outcome != "open" {
                if record.outcome == "not_taken" && record.reason == reason && record.note == note {
                    true
                } else {
                    return Err(sync_error(Failure::AlreadyResolved));
                }
            } else {
                if !actionable(&source, &row) {
                    return Err(sync_error(Failure::InvalidOccurrence));
                }
                row.record = Some(
                    save_decision(db, context, &source, &row, reason, note, request_id).await?,
                );
                false
            }
        } else {
            if !actionable(&source, &row) {
                return Err(sync_error(Failure::InvalidOccurrence));
            }
            row.record =
                Some(save_decision(db, context, &source, &row, reason, note, request_id).await?);
            false
        }
    } else {
        let record = row
            .record
            .as_ref()
            .ok_or_else(|| sync_error(Failure::InvalidOccurrence))?;
        if record.outcome != "not_taken" {
            return Err(sync_error(Failure::InvalidOccurrence));
        }
        let expected = operation
            .if_match
            .as_deref()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| sync_error(Failure::PreconditionRequired))?;
        if expected != record_etag(record) {
            return Err(sync_error(Failure::SyncConflict));
        }
        row.record = Some(reopen_decision(db, context, &source, record, request_id).await?);
        false
    };
    let record = row.record.ok_or_else(ApiError::internal)?;
    Ok(SyncResult {
        record_type: "MedicationDoseOccurrence",
        record_id: Some(record.id),
        record_portable_id: Some(record.portable_id.clone()),
        etag: Some(record_etag(&record)),
        replayed: Some(replayed),
    })
}

pub(crate) async fn authorize_sync_replay(
    db: &DatabaseTransaction,
    context: &AuthContext,
    operation: &SyncOperation,
    saved: &Value,
) -> Result<(), ApiError> {
    if saved.get("action").and_then(Value::as_str) != Some(operation.action.as_str())
        || saved.get("record_type").and_then(Value::as_str) != Some("MedicationDoseOccurrence")
    {
        return Err(ApiError::forbidden());
    }
    let portable_id = saved
        .get("record_portable_id")
        .and_then(Value::as_str)
        .ok_or_else(ApiError::forbidden)?;
    let record = dose_occurrence::Entity::find()
        .filter(dose_occurrence::Column::HouseholdId.eq(context.membership.household_id))
        .filter(dose_occurrence::Column::PortableId.eq(portable_id))
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::forbidden)?;
    if saved.get("record_id").and_then(Value::as_str) != Some(record.id.to_string().as_str())
        || operation
            .id
            .as_deref()
            .is_some_and(|id| id != record.portable_id && id != record.id.to_string())
    {
        return Err(ApiError::forbidden());
    }
    let source = if let Some(id) = record.schedule_id {
        schedule::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(database_error)?
            .filter(|source| source.household_id == context.membership.household_id)
            .map(Source::Schedule)
    } else if let Some(id) = record.person_medication_id {
        person_medication::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(database_error)?
            .filter(|source| source.household_id == context.membership.household_id)
            .map(Source::Assignment)
    } else {
        None
    }
    .ok_or_else(ApiError::forbidden)?;
    let action = if operation.action == "create" {
        "not_taken"
    } else {
        "reopen"
    };
    if !person_access(db, context, source.person_id(), action).await? {
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
                .is_none_or(|id| id != source.portable_id() && id != source.id().to_string()))
    {
        return Err(ApiError::forbidden());
    }
    Ok(())
}
