use super::*;
use loco_rs::model::TenantQueryExt;

pub(super) struct ProposedTake {
    pub(super) source: Source,
    pub(super) taken_at: NaiveDateTime,
    pub(super) zone: chrono_tz::Tz,
    pub(super) amount: Decimal,
    pub(super) unit: String,
    pub(super) selected: medication::Model,
}

pub(super) fn valid_numeric_10_2(amount: Decimal) -> bool {
    amount > Decimal::ZERO && amount.normalize().scale() <= 2 && amount < Decimal::from(100_000_000)
}

pub(super) fn parse_input_time(raw: &str) -> Result<NaiveDateTime, ApiError> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .and_then(|value| DateTime::<Utc>::from_timestamp_micros(value.timestamp_micros()))
        .map(|value| value.naive_utc())
        .ok_or_else(|| error(ErrorKind::Validation, "taken_at is invalid"))
}

pub(super) async fn prepare(
    db: &DatabaseTransaction,
    context: &DoseContext<'_>,
    household_id: i64,
    command: &Take,
) -> Result<ProposedTake, TakeFailure> {
    let mut source = source(db, context, household_id, &command.source).await?;
    if source.retired {
        return Err(OperationError::NotFound.into());
    }
    if !allowed_person(db, context, source.person_id, true).await? {
        return Err(OperationError::Forbidden.into());
    }
    let taken_at = command.taken_at;
    if taken_at > Utc::now().naive_utc() + Duration::hours(1) {
        return Err(error(
            ErrorKind::Validation,
            "Cannot record a dose more than one hour in the future",
        )
        .into());
    }
    if !source.active {
        return Err(TakeFailure {
            error: error(ErrorKind::Validation, "Cannot take medication: paused"),
            cause: Some(TakeFailureCause::Paused),
        });
    }
    let effective_date = local_date_in_zone(taken_at, context.zone);
    if !applies_on(&source, effective_date) {
        return Err(error(
            ErrorKind::Validation,
            "Cannot take medication: schedule does not apply on this date",
        )
        .into());
    }
    effective_source(&mut source, effective_date);
    if command.expected_effective_amount.is_some() || command.expected_effective_unit.is_some() {
        let expected_amount = command.expected_effective_amount;
        let expected_unit = command.expected_effective_unit.as_deref();
        if expected_amount != source.dose_amount || expected_unit != source.dose_unit.as_deref() {
            return Err(OperationError::Conflict {
                code: "dose_changed".into(),
                details: json!({"error":"Dose changed. Review the updated dose before recording."}),
            }
            .into());
        }
    }
    let amount = command
        .dose_amount
        .or(source.dose_amount)
        .ok_or_else(|| error(ErrorKind::Validation, "Invalid dose configured"))?;
    if !valid_numeric_10_2(amount) {
        return Err(error(ErrorKind::Validation, "Invalid dose configured").into());
    }
    let unit = source
        .dose_unit
        .clone()
        .ok_or_else(|| error(ErrorKind::Validation, "Invalid dose configured"))?;
    let original = medication::Entity::find_by_id(source.medication_id)
        .in_tenant(household_id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or(OperationError::NotFound)?;
    let candidates = access::medication_scope(context.tenant)
        .all(db)
        .await
        .map_err(database_error)?;
    let candidate_ids: HashSet<i64> = if access::can_manage_household(context.tenant) {
        candidates.iter().map(|value| value.id).collect()
    } else {
        let schedule_ids = schedule::Entity::find()
            .in_tenant(household_id)
            .filter(schedule::Column::PersonId.eq(source.person_id))
            .all(db)
            .await
            .map_err(database_error)?
            .into_iter()
            .map(|value| value.medication_id);
        let assignment_ids = person_medication::Entity::find()
            .in_tenant(household_id)
            .filter(person_medication::Column::PersonId.eq(source.person_id))
            .all(db)
            .await
            .map_err(database_error)?
            .into_iter()
            .map(|value| value.medication_id);
        schedule_ids
            .chain(assignment_ids)
            .chain(std::iter::once(source.medication_id))
            .collect()
    };
    let matching: Vec<_> = candidates
        .into_iter()
        .filter(|value| {
            candidate_ids.contains(&value.id)
                && same_stock_signature(value, &original)
                && value
                    .current_supply
                    .is_none_or(|supply| supply > Decimal::ZERO)
        })
        .collect();
    if matching.is_empty() {
        return Err(TakeFailure {
            error: error(
                ErrorKind::Validation,
                "Cannot take medication: out of stock",
            ),
            cause: Some(TakeFailureCause::OutOfStock),
        });
    }
    let selected_id = command.taken_from_medication_id.map(|id| id.0);
    let selected = if let Some(id) = selected_id {
        matching
            .into_iter()
            .find(|value| value.id == id)
            .ok_or_else(|| {
                error(
                    ErrorKind::Validation,
                    "Selected location is unavailable for this medication.",
                )
            })?
    } else if matching.len() == 1 {
        matching
            .into_iter()
            .next()
            .ok_or(OperationError::Unavailable)?
    } else if matching.len() > 1 {
        return Err(error(
            ErrorKind::Validation,
            "Choose a location to record this dose.",
        )
        .into());
    } else {
        return Err(error(
            ErrorKind::Validation,
            "Cannot take medication: out of stock",
        )
        .into());
    };
    Ok(ProposedTake {
        source,
        taken_at,
        zone: context.zone,
        amount,
        unit,
        selected,
    })
}
