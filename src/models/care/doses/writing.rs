use super::*;

pub(crate) async fn create_with_failure(
    db: &DatabaseTransaction,
    context: &DoseContext<'_>,
    household_id: HouseholdId,
    request: &PreparedRequest,
    request_id: &str,
) -> Result<(bool, medication_take::Model), TakeFailure> {
    access::lock_household(context.tenant).await?;
    access::recheck(context.tenant).await?;
    let client_uuid = request.client_uuid;
    if let Some(client_uuid) = client_uuid {
        lock_client_uuid(db, client_uuid).await?;
    }
    if let Some(client_uuid) = client_uuid
        && let Some(existing) = existing_take(db, household_id, client_uuid).await?
    {
        let replay = match replay_matches(db, context, household_id, &existing, request).await {
            Ok(replay) => replay,
            Err(OperationError::Forbidden) => {
                prepare(db, context, household_id, request).await?;
                return Err(DoseConflict::IdempotencyKeyUnavailable.into_error().into());
            }
            Err(problem) => return Err(problem.into()),
        };
        if !replay {
            return Err(DoseConflict::PayloadMismatch.into_error().into());
        }
        return Ok((true, existing));
    }
    let candidate = prepare(db, context, household_id, request).await?;
    let validated = input::validate(db, candidate, request.unit.as_deref()).await?;
    let take = apply(
        db,
        context,
        household_id,
        &validated,
        client_uuid,
        request_id,
    )
    .await?;
    Ok((false, take))
}

async fn apply(
    db: &DatabaseTransaction,
    context: &DoseContext<'_>,
    household_id: HouseholdId,
    proposed: &ValidatedTake,
    client_uuid: Option<Uuid>,
    request_id: &str,
) -> Result<medication_take::Model, TakeFailure> {
    let take = insert_take(db, household_id, proposed, client_uuid)
        .await?
        .ok_or_else(|| DoseConflict::IdempotencyKeyUnavailable.into_error())?;
    crate::models::care::take_history::append(
        context.tenant,
        crate::models::ids::TakeId::from_database(take.id),
        proposed.source.medication_id,
        context.provenance,
    )
    .await?;
    decrement_stock(db, context, request_id, proposed).await?;
    crate::models::care::browser_push::low_stock::record_crossing(
        context.tenant,
        proposed.selected.id,
        proposed.selected.current_supply,
        take.id,
    )
    .await?;
    record_domain_audit(db, context, request_id, &take, proposed).await?;
    Ok(take)
}
