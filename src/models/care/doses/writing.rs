use super::*;

pub(crate) async fn create_with_failure(
    db: &DatabaseTransaction,
    context: &DoseContext<'_>,
    household_id: i64,
    command: &Take,
    request_id: &str,
) -> Result<(bool, medication_take::Model), TakeFailure> {
    lock_household(db, household_id).await?;
    access::recheck(context.tenant).await?;
    let before = source(db, context, household_id, &command.source).await?;
    if before.person_schedule_id.is_some() {
        crate::models::care::person_schedules::projection::refresh_one(context.tenant, before.id)
            .await?;
    }
    let canonical_uuid = command.client_uuid.map(|id| id.hyphenated().to_string());
    let client_uuid = canonical_uuid.as_deref();
    if let Some(client_uuid) = command.client_uuid {
        lock_client_uuid(db, client_uuid).await?;
    }
    if let Some(client_uuid) = command.client_uuid
        && let Some(existing) = existing_take(db, household_id, client_uuid).await?
    {
        let replay = match replay_matches(db, context, household_id, &existing, command).await {
            Ok(replay) => replay,
            Err(OperationError::Forbidden) => {
                prepare(db, context, household_id, command).await?;
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
    let proposed = prepare(db, context, household_id, command).await?;
    if command
        .dose_unit
        .as_deref()
        .is_some_and(|unit| unit != proposed.unit)
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
    if let Some(id) = proposed.source.person_schedule_id.as_ref() {
        crate::models::entities::shared_administration::Entity::insert(
            crate::models::entities::shared_administration::ActiveModel {
                person_schedule_id: Set(id.clone()),
                household_id: Set(household_id),
                medication_take_id: Set(take.id),
                taken_at: Set(proposed.taken_at),
                occurrence_starts_on: Set(None),
                occurrence_position: Set(None),
                ..Default::default()
            },
        )
        .exec(db)
        .await
        .map_err(|problem| match problem {
            sea_orm::DbErr::RecordNotInserted => TakeFailure::from(error(
                ErrorKind::Validation,
                "Cannot take medication: timing restrictions not met",
            )),
            problem => TakeFailure::from(problem),
        })?;
        let state = crate::models::entities::shared_schedule::Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or(OperationError::NotFound)?;
        if state
            .latest_taken_at
            .is_none_or(|latest| latest < proposed.taken_at)
        {
            let mut current: crate::models::entities::shared_schedule::ActiveModel = state.into();
            current.latest_taken_at = Set(Some(proposed.taken_at));
            current.update(db).await?;
        }
    }
    decrement_stock(db, context, request_id, &proposed).await?;
    crate::models::care::browser_push::low_stock::record_crossing(
        context.tenant,
        &proposed.selected,
        take.id,
    )
    .await?;
    record_domain_audit(db, context, request_id, &take, &proposed).await?;
    Ok((false, take))
}
