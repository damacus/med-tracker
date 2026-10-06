use super::*;

async fn record_transition(
    db: &DatabaseTransaction,
    context: &AuthContext<'_>,
    source: &Source,
    record: &dose_occurrence::Model,
    before: Option<Value>,
    request_id: &str,
) -> Result<(), ApiError> {
    let action = if before.is_some() { "update" } else { "create" };
    record_version(
        context,
        "MedicationDoseOccurrence",
        record.id,
        action,
        before,
        Some(snapshot(record)),
    )
    .await?;
    let person_portable_id = person::Entity::find_by_id(source.person_id())
        .filter(person::Column::HouseholdId.eq(context.tenant.scope().household_id))
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(not_found)?
        .portable_id;
    record_change(
        db,
        context,
        request_id,
        SyncRecord {
            record_type: "MedicationDoseOccurrence",
            record_id: record.id,
            portable_id: &record.portable_id,
            action,
            person_portable_id: Some(&person_portable_id),
        },
    )
    .await?;
    Ok(())
}

pub(super) async fn find_row(
    db: &DatabaseTransaction,
    secret: &Arc<[u8]>,
    source: &Source,
    key_value: &str,
) -> Result<Option<Occurrence>, ApiError> {
    let Some((date, position)) = decode_key(secret, source, key_value) else {
        return Ok(None);
    };
    Ok(projected(db, source, date, date)
        .await?
        .into_iter()
        .find(|row| row.window_start == date && row.position == position))
}

pub(super) fn actionable(source: &Source, row: &Occurrence) -> bool {
    let due_time = row
        .scheduled_at
        .unwrap_or_else(|| local_midnight(row.window_start));
    row.expected
        && row.legacy_take_id.is_none()
        && due_time <= Utc::now().naive_utc()
        && (source.kind() == Kind::Schedule || source.created_at() <= Utc::now().naive_utc())
}

pub(super) async fn save_decision(
    db: &DatabaseTransaction,
    context: &AuthContext<'_>,
    source: &Source,
    row: &Occurrence,
    reason: Option<String>,
    note: Option<String>,
    request_id: &str,
) -> Result<dose_occurrence::Model, ApiError> {
    let now = Utc::now().naive_utc();
    let before = row.record.as_ref().map(snapshot);
    let model = if let Some(record) = &row.record {
        let mut model: dose_occurrence::ActiveModel = record.clone().into();
        model.outcome = Set("not_taken".to_owned());
        model.reason = Set(reason);
        model.note = Set(note);
        model.resolved_at = Set(Some(now));
        model.resolved_by_membership_id = Set(Some(context.tenant.membership().id));
        model.updated_at = Set(now);
        model.update(db).await.map_err(database_error)?
    } else {
        dose_occurrence::ActiveModel {
            household_id: Set(context.tenant.scope().household_id),
            portable_id: Set(Uuid::new_v4().to_string()),
            schedule_id: Set((source.kind() == Kind::Schedule).then_some(source.id())),
            person_medication_id: Set((source.kind() == Kind::Assignment).then_some(source.id())),
            medication_take_id: Set(None),
            resolved_by_membership_id: Set(Some(context.tenant.membership().id)),
            window_starts_on: Set(row.window_start),
            window_ends_on: Set(Some(row.window_end)),
            position: Set(row.position),
            scheduled_at: Set(row.scheduled_at),
            outcome: Set("not_taken".to_owned()),
            reason: Set(reason),
            note: Set(note),
            resolved_at: Set(Some(now)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(database_error)?
    };
    record_transition(db, context, source, &model, before, request_id).await?;
    Ok(model)
}

pub(super) async fn reopen_decision(
    db: &DatabaseTransaction,
    context: &AuthContext<'_>,
    source: &Source,
    record: &dose_occurrence::Model,
    request_id: &str,
) -> Result<dose_occurrence::Model, ApiError> {
    let before = snapshot(record);
    let mut model: dose_occurrence::ActiveModel = record.clone().into();
    model.outcome = Set("open".to_owned());
    model.reason = Set(None);
    model.note = Set(None);
    model.resolved_at = Set(None);
    model.resolved_by_membership_id = Set(None);
    model.updated_at = Set(Utc::now().naive_utc());
    let model = model.update(db).await.map_err(database_error)?;
    record_transition(db, context, source, &model, Some(before), request_id).await?;
    Ok(model)
}

pub(super) async fn link_take(
    db: &DatabaseTransaction,
    context: &AuthContext<'_>,
    source: &Source,
    row: &Occurrence,
    take: &medication_take::Model,
    request_id: &str,
) -> Result<dose_occurrence::Model, ApiError> {
    let now = Utc::now().naive_utc();
    let before = row.record.as_ref().map(snapshot);
    let model = if let Some(record) = &row.record {
        let mut model: dose_occurrence::ActiveModel = record.clone().into();
        model.outcome = Set("taken".to_owned());
        model.medication_take_id = Set(Some(take.id));
        model.reason = Set(None);
        model.note = Set(None);
        model.resolved_at = Set(Some(now));
        model.resolved_by_membership_id = Set(Some(context.tenant.membership().id));
        model.updated_at = Set(now);
        model.update(db).await.map_err(database_error)?
    } else {
        dose_occurrence::ActiveModel {
            household_id: Set(context.tenant.scope().household_id),
            portable_id: Set(Uuid::new_v4().to_string()),
            schedule_id: Set((source.kind() == Kind::Schedule).then_some(source.id())),
            person_medication_id: Set((source.kind() == Kind::Assignment).then_some(source.id())),
            medication_take_id: Set(Some(take.id)),
            resolved_by_membership_id: Set(Some(context.tenant.membership().id)),
            window_starts_on: Set(row.window_start),
            window_ends_on: Set(Some(row.window_end)),
            position: Set(row.position),
            scheduled_at: Set(row.scheduled_at),
            outcome: Set("taken".to_owned()),
            reason: Set(None),
            note: Set(None),
            resolved_at: Set(Some(now)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(database_error)?
    };
    record_transition(db, context, source, &model, before, request_id).await?;
    Ok(model)
}
