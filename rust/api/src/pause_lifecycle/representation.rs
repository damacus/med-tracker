use super::*;

pub(super) fn timestamp(value: chrono::NaiveDateTime) -> String {
    value
        .and_utc()
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

pub(super) fn period_row(
    period: &pause_period::Model,
    source: &Source,
    names: &HashMap<i64, String>,
) -> Value {
    json!({
        "id": period.portable_id,
        "portable_id": period.portable_id,
        "source_type": source.kind().name(),
        "source_id": source.portable_id(),
        "reason": period.reason,
        "note": period.note,
        "legacy_context": period.legacy_context,
        "started_at": period.started_at.map(timestamp),
        "ended_at": period.ended_at.map(timestamp),
        "recorded_by_membership_id": period.recorded_by_membership_id.map(|id| id.to_string()),
        "resumed_by_membership_id": period.resumed_by_membership_id.map(|id| id.to_string()),
        "recorded_by_name": period.recorded_by_membership_id.and_then(|id| names.get(&id)),
        "resumed_by_name": period.resumed_by_membership_id.and_then(|id| names.get(&id)),
        "created_at": timestamp(period.created_at),
        "updated_at": timestamp(period.updated_at),
    })
}

pub(super) fn period_snapshot(period: &pause_period::Model) -> Value {
    json!({
        "household_id": period.household_id,
        "portable_id": period.portable_id,
        "schedule_id": period.schedule_id,
        "person_medication_id": period.person_medication_id,
        "reason": period.reason,
        "note": period.note,
        "legacy_context": period.legacy_context,
        "recorded_by_membership_id": period.recorded_by_membership_id,
        "resumed_by_membership_id": period.resumed_by_membership_id,
        "started_at": period.started_at.map(timestamp),
        "ended_at": period.ended_at.map(timestamp),
    })
}

pub(super) async fn period_body(
    db: &DatabaseTransaction,
    period: &pause_period::Model,
    source: &Source,
) -> Result<(Value, String), ApiError> {
    let names = actor_names(db, std::slice::from_ref(period)).await?;
    let body = json!({"data": period_row(period, source, &names)});
    let etag = representation_etag(&body);
    Ok((body, etag))
}

pub(crate) async fn period_values(
    db: &DatabaseTransaction,
    periods: &[pause_period::Model],
    schedules: &HashMap<i64, schedule::Model>,
    assignments: &HashMap<i64, person_medication::Model>,
) -> Result<Vec<(Value, String)>, ApiError> {
    let names = actor_names(db, periods).await?;
    periods
        .iter()
        .map(|period| {
            let source = if let Some(id) = period.schedule_id {
                schedules.get(&id).cloned().map(Source::Schedule)
            } else {
                period
                    .person_medication_id
                    .and_then(|id| assignments.get(&id).cloned())
                    .map(Source::Assignment)
            }
            .ok_or_else(ApiError::not_found)?;
            let row = period_row(period, &source, &names);
            let etag = representation_etag(&json!({"data": row}));
            Ok((row, etag))
        })
        .collect()
}

pub(super) async fn source_body(
    db: &DatabaseTransaction,
    context: &AuthContext,
    source: &Source,
) -> Result<Value, ApiError> {
    let row = match source {
        Source::Schedule(source) => {
            let read = read_schedule::Entity::find_by_id(source.id)
                .one(db)
                .await
                .map_err(database_error)?
                .ok_or_else(ApiError::not_found)?;
            serialize_schedules(db, context, vec![read])
                .await?
                .remove(0)
        }
        Source::Assignment(source) => {
            let read = read_assignment::Entity::find_by_id(source.id)
                .one(db)
                .await
                .map_err(database_error)?
                .ok_or_else(ApiError::not_found)?;
            serialize_assignments(db, context, vec![read])
                .await?
                .remove(0)
        }
    };
    Ok(json!({"data": row}))
}
