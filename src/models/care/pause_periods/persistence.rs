use super::*;
fn not_found() -> OperationError {
    OperationError::NotFound
}

pub(super) async fn open_period(
    db: &DatabaseTransaction,
    source: &Source,
) -> Result<Option<pause_period::Model>, ApiError> {
    let query = pause_period::Entity::find().filter(pause_period::Column::EndedAt.is_null());
    let query = match source {
        Source::Schedule(row) => query.filter(pause_period::Column::ScheduleId.eq(row.id)),
        Source::Assignment(row) => {
            query.filter(pause_period::Column::PersonMedicationId.eq(row.id))
        }
    };
    query.one(db).await.map_err(database_error)
}

pub(super) async fn latest_completed(
    db: &DatabaseTransaction,
    source: &Source,
) -> Result<Option<pause_period::Model>, ApiError> {
    let query = pause_period::Entity::find()
        .filter(pause_period::Column::EndedAt.is_not_null())
        .order_by_desc(pause_period::Column::EndedAt)
        .order_by_desc(pause_period::Column::Id);
    let query = match source {
        Source::Schedule(row) => query.filter(pause_period::Column::ScheduleId.eq(row.id)),
        Source::Assignment(row) => {
            query.filter(pause_period::Column::PersonMedicationId.eq(row.id))
        }
    };
    query.one(db).await.map_err(database_error)
}

pub(super) async fn set_source_active(
    db: &DatabaseTransaction,
    context: &AuthContext<'_>,
    source: &Source,
    active: bool,
    request_id: &str,
) -> Result<Source, ApiError> {
    let person_portable_id = person::Entity::find_by_id(source.person_id())
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(not_found)?
        .portable_id;
    let updated = match source {
        Source::Schedule(row) => {
            let before = json!({"active": row.active});
            let mut model: schedule::ActiveModel = row.clone().into();
            model.active = Set(active);
            model.updated_at = Set(Utc::now().naive_utc());
            let row = model.update(db).await.map_err(database_error)?;
            record_version(
                context,
                "Schedule",
                row.id,
                "update",
                Some(before),
                Some(json!({"active": row.active})),
            )
            .await?;
            Source::Schedule(row)
        }
        Source::Assignment(row) => {
            let before = json!({"active": row.active});
            let mut model: person_medication::ActiveModel = row.clone().into();
            model.active = Set(active);
            model.updated_at = Set(Utc::now().naive_utc());
            let row = model.update(db).await.map_err(database_error)?;
            record_version(
                context,
                "PersonMedication",
                row.id,
                "update",
                Some(before),
                Some(json!({"active": row.active})),
            )
            .await?;
            Source::Assignment(row)
        }
    };
    record_change(
        db,
        context,
        request_id,
        SyncRecord {
            record_type: source.kind().record_type(),
            record_id: source.id(),
            portable_id: source.portable_id(),
            action: "update",
            person_portable_id: Some(&person_portable_id),
        },
    )
    .await?;
    Ok(updated)
}

pub(super) async fn insert_period(
    db: &DatabaseTransaction,
    context: &AuthContext<'_>,
    source: &Source,
    reason: &str,
    note: Option<String>,
    request_id: &str,
) -> Result<pause_period::Model, ApiError> {
    let now = Utc::now().naive_utc();
    let legacy = !source.active() || reason == "reason_not_recorded";
    let row = pause_period::ActiveModel {
        household_id: Set(context.tenant.scope().household_id),
        portable_id: Set(Uuid::new_v4().to_string()),
        schedule_id: Set((source.kind() == Kind::Schedule).then_some(source.id())),
        person_medication_id: Set((source.kind() == Kind::Assignment).then_some(source.id())),
        reason: Set(if source.active() {
            reason
        } else {
            "reason_not_recorded"
        }
        .to_owned()),
        note: Set(if source.active() { note } else { None }),
        legacy_context: Set(legacy),
        imported_context: Set(false),
        imported_actor_references: Set(json!({})),
        recorded_by_membership_id: Set(source.active().then_some(context.tenant.membership().id)),
        resumed_by_membership_id: Set(None),
        started_at: Set(source.active().then_some(now)),
        ended_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(database_error)?;
    record_version(
        context,
        "MedicationPausePeriod",
        row.id,
        "create",
        None,
        Some(period_snapshot(&row)),
    )
    .await?;
    record_period_change(db, context, source, &row, "create", request_id).await?;
    Ok(row)
}

pub(super) async fn record_period_change(
    db: &DatabaseTransaction,
    context: &AuthContext<'_>,
    source: &Source,
    period: &pause_period::Model,
    action: &str,
    request_id: &str,
) -> Result<(), ApiError> {
    let person = person::Entity::find_by_id(source.person_id())
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(not_found)?;
    record_change(
        db,
        context,
        request_id,
        SyncRecord {
            record_type: "MedicationPausePeriod",
            record_id: period.id,
            portable_id: &period.portable_id,
            action,
            person_portable_id: Some(&person.portable_id),
        },
    )
    .await
}

pub(super) async fn pause_source(
    db: &DatabaseTransaction,
    context: &AuthContext<'_>,
    source: Source,
    reason: &str,
    note: Option<String>,
    request_id: &str,
) -> Result<(Source, pause_period::Model), ApiError> {
    let period = match open_period(db, &source).await? {
        Some(period) => period,
        None => insert_period(db, context, &source, reason, note, request_id).await?,
    };
    let source = if source.active() {
        set_source_active(db, context, &source, false, request_id).await?
    } else {
        source
    };
    Ok((source, period))
}

pub(super) async fn close_period(
    db: &DatabaseTransaction,
    context: &AuthContext<'_>,
    source: Source,
    requested: Option<pause_period::Model>,
    request_id: &str,
) -> Result<(Source, Option<pause_period::Model>), ApiError> {
    let period = if let Some(requested) = requested {
        Some(requested)
    } else if let Some(open) = open_period(db, &source).await? {
        Some(open)
    } else if source.active() {
        latest_completed(db, &source).await?
    } else {
        Some(
            insert_period(
                db,
                context,
                &source,
                "reason_not_recorded",
                None,
                request_id,
            )
            .await?,
        )
    };
    let Some(period) = period else {
        return Ok((source, None));
    };
    if period.ended_at.is_some() {
        return Ok((source, Some(period)));
    }
    let before = period_snapshot(&period);
    let mut model: pause_period::ActiveModel = period.into();
    model.ended_at = Set(Some(Utc::now().naive_utc()));
    model.resumed_by_membership_id = Set(Some(context.tenant.membership().id));
    model.updated_at = Set(Utc::now().naive_utc());
    let period = model.update(db).await.map_err(database_error)?;
    record_version(
        context,
        "MedicationPausePeriod",
        period.id,
        "update",
        Some(before),
        Some(period_snapshot(&period)),
    )
    .await?;
    record_period_change(db, context, &source, &period, "update", request_id).await?;
    let source = if !source.active() {
        set_source_active(db, context, &source, true, request_id).await?
    } else {
        source
    };
    Ok((source, Some(period)))
}
