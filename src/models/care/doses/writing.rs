use super::*;

pub(crate) async fn create_with_failure(
    db: &DatabaseTransaction,
    context: &DoseContext<'_>,
    household_id: i64,
    body: &Value,
    request_id: &str,
) -> Result<(bool, medication_take::Model), TakeFailure> {
    let attributes = body
        .get("medication_take")
        .filter(|value| value.is_object())
        .ok_or_else(|| error(ErrorKind::Validation, "medication_take is required"))?;
    if body.as_object().is_none_or(|object| object.len() != 1) {
        return Err(error(ErrorKind::Validation, "unknown request field").into());
    }
    let allowed = [
        "client_uuid",
        "source_type",
        "source_id",
        "taken_at",
        "dose_amount",
        "dose_unit",
        "taken_from_medication_id",
    ];
    if attributes
        .as_object()
        .is_some_and(|object| object.keys().any(|key| !allowed.contains(&key.as_str())))
    {
        return Err(error(ErrorKind::Validation, "unknown medication_take field").into());
    }
    if !attributes
        .get("source_id")
        .and_then(Value::as_str)
        .is_some_and(valid_identifier)
        || !attributes
            .get("source_type")
            .and_then(Value::as_str)
            .is_some_and(|kind| !kind.trim().is_empty())
    {
        return Err(error(ErrorKind::Validation, "invalid medication source").into());
    }
    if !attributes
        .get("source_type")
        .and_then(Value::as_str)
        .is_some_and(|kind| matches!(kind, "schedule" | "person_medication"))
    {
        return Err(OperationError::NotFound.into());
    }
    if attributes
        .get("client_uuid")
        .is_some_and(|value| !value.as_str().is_some_and(|id| Uuid::parse_str(id).is_ok()))
    {
        return Err(error(ErrorKind::Validation, "invalid client_uuid").into());
    }
    if attributes
        .get("dose_unit")
        .is_some_and(|value| !value.as_str().is_some_and(|unit| !unit.is_empty()))
    {
        return Err(error(ErrorKind::Validation, "invalid dose_unit").into());
    }
    if attributes
        .get("taken_from_medication_id")
        .is_some_and(|value| !value.as_i64().is_some_and(|id| id > 0))
    {
        return Err(error(ErrorKind::Validation, "invalid taken_from_medication_id").into());
    }
    lock_row(db, "households", household_id).await?;
    access::recheck(context.tenant).await?;
    let canonical_uuid = attributes
        .get("client_uuid")
        .and_then(Value::as_str)
        .map(|value| Uuid::parse_str(value).map(|id| id.hyphenated().to_string()))
        .transpose()
        .map_err(|_| error(ErrorKind::Validation, "invalid client_uuid"))?;
    let client_uuid = canonical_uuid.as_deref();
    if let Some(client_uuid) = client_uuid {
        lock_client_uuid(db, client_uuid).await?;
    }
    if let Some(client_uuid) = client_uuid
        && let Some(existing) = existing_take(db, household_id, client_uuid).await?
    {
        let replay = match replay_matches(db, context, household_id, &existing, attributes).await {
            Ok(replay) => replay,
            Err(OperationError::Forbidden) => {
                prepare(db, context, household_id, attributes).await?;
                return Err(OperationError::Conflict {
                    code: "idempotency_key_unavailable".into(),
                    details: json!({"message": "Medication take idempotency key is unavailable"}),
                }
                .into());
            }
            Err(problem) => return Err(problem.into()),
        };
        if !replay {
            return Err(error(
                ErrorKind::Conflict,
                "client_uuid was already used for a different dose",
            )
            .into());
        }
        return Ok((true, existing));
    }
    let proposed = prepare(db, context, household_id, attributes).await?;
    if attributes
        .get("dose_unit")
        .is_some_and(|unit| !unit.is_null() && unit.as_str() != Some(proposed.unit.as_str()))
    {
        return Err(error(ErrorKind::Validation, "dose_unit does not match the source").into());
    }
    if !timing_allowed(db, &proposed).await? {
        return Err(error(
            ErrorKind::Validation,
            "Cannot take medication: timing restrictions not met",
        )
        .into());
    }
    let take = insert_take(db, household_id, &proposed, client_uuid)
        .await?
        .ok_or_else(|| OperationError::Conflict {
            code: "idempotency_key_unavailable".into(),
            details: json!({"message": "Medication take idempotency key is unavailable"}),
        })?;
    decrement_stock(db, context, request_id, &proposed).await?;
    record_domain_audit(db, context, request_id, &take, &proposed).await?;
    Ok((false, take))
}
