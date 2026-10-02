use super::*;

pub(super) struct ProposedTake {
    pub(super) source: Source,
    pub(super) taken_at: NaiveDateTime,
    pub(super) amount: Decimal,
    pub(super) unit: String,
    pub(super) selected: medication::Model,
}

pub(super) fn valid_numeric_10_2(amount: Decimal) -> bool {
    amount > Decimal::ZERO && amount.normalize().scale() <= 2 && amount < Decimal::from(100_000_000)
}

pub(super) fn decimal_from_json(value: Option<&Value>) -> Result<Option<Decimal>, ApiError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.as_str().is_none() {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "dose_amount must be a string",
        ));
    }
    let amount = parse_decimal(value)
        .ok_or_else(|| error(StatusCode::UNPROCESSABLE_ENTITY, "Invalid dose configured"))?;
    if !valid_numeric_10_2(amount) {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Invalid dose configured",
        ));
    }
    Ok(Some(amount))
}

pub(super) fn parse_input_time(value: Option<&Value>) -> Result<NaiveDateTime, ApiError> {
    let raw = value
        .and_then(Value::as_str)
        .ok_or_else(|| error(StatusCode::UNPROCESSABLE_ENTITY, "taken_at is invalid"))?;
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .and_then(|value| DateTime::<Utc>::from_timestamp_micros(value.timestamp_micros()))
        .map(|value| value.naive_utc())
        .ok_or_else(|| error(StatusCode::UNPROCESSABLE_ENTITY, "taken_at is invalid"))
}

fn string_field<'a>(value: &'a Value, field: &str) -> Result<&'a str, ApiError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(ApiError::not_found)
}

pub(super) async fn prepare(
    db: &DatabaseTransaction,
    context: &AuthContext,
    household_id: i64,
    attributes: &Value,
) -> Result<ProposedTake, TakeFailure> {
    let kind = string_field(attributes, "source_type")?;
    let source_id = string_field(attributes, "source_id")?;
    let mut source = source(db, context, household_id, kind, source_id).await?;
    if source.retired {
        return Err(ApiError::not_found().into());
    }
    if !allowed_person(db, context, source.person_id, true).await? {
        return Err(ApiError::forbidden().into());
    }
    let taken_at = parse_input_time(attributes.get("taken_at"))?;
    if taken_at > Utc::now().naive_utc() + Duration::hours(1) {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Cannot record a dose more than one hour in the future",
        )
        .into());
    }
    if !source.active {
        return Err(TakeFailure {
            error: error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Cannot take medication: paused",
            ),
            cause: Some(TakeFailureCause::Paused),
        });
    }
    let effective_date = local_date(taken_at);
    if !applies_on(&source, effective_date) {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Cannot take medication: schedule does not apply on this date",
        )
        .into());
    }
    effective_source(&mut source, effective_date);
    if attributes.get("dose_amount").is_some_and(Value::is_number) {
        return Err(TakeFailure {
            error: error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "dose_amount must be a string",
            ),
            cause: Some(TakeFailureCause::NumericDoseAmount),
        });
    }
    let amount = decimal_from_json(attributes.get("dose_amount"))?
        .or(source.dose_amount)
        .ok_or_else(|| error(StatusCode::UNPROCESSABLE_ENTITY, "Invalid dose configured"))?;
    if !valid_numeric_10_2(amount) {
        return Err(error(StatusCode::UNPROCESSABLE_ENTITY, "Invalid dose configured").into());
    }
    let unit = source
        .dose_unit
        .clone()
        .ok_or_else(|| error(StatusCode::UNPROCESSABLE_ENTITY, "Invalid dose configured"))?;
    let original = medication::Entity::find_by_id(source.medication_id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?;
    let candidates = scope(household_id, &context.membership)
        .all(db)
        .await
        .map_err(database_error)?;
    let candidate_ids: HashSet<i64> =
        if context.membership.role == "owner" || context.membership.role == "administrator" {
            candidates.iter().map(|value| value.id).collect()
        } else {
            let schedule_ids = schedule::Entity::find()
                .filter(schedule::Column::HouseholdId.eq(household_id))
                .filter(schedule::Column::PersonId.eq(source.person_id))
                .all(db)
                .await
                .map_err(database_error)?
                .into_iter()
                .map(|value| value.medication_id);
            let assignment_ids = person_medication::Entity::find()
                .filter(person_medication::Column::HouseholdId.eq(household_id))
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
                StatusCode::UNPROCESSABLE_ENTITY,
                "Cannot take medication: out of stock",
            ),
            cause: Some(TakeFailureCause::OutOfStock),
        });
    }
    let selected_id = attributes
        .get("taken_from_medication_id")
        .and_then(Value::as_i64);
    if attributes.get("taken_from_medication_id").is_some() && selected_id.is_none() {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Selected location is unavailable for this medication.",
        )
        .into());
    }
    let selected = if let Some(id) = selected_id {
        matching
            .into_iter()
            .find(|value| value.id == id)
            .ok_or_else(|| {
                error(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Selected location is unavailable for this medication.",
                )
            })?
    } else if matching.len() == 1 {
        matching.into_iter().next().ok_or_else(ApiError::internal)?
    } else if matching.len() > 1 {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Choose a location to record this dose.",
        )
        .into());
    } else {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Cannot take medication: out of stock",
        )
        .into());
    };
    Ok(ProposedTake {
        source,
        taken_at,
        amount,
        unit,
        selected,
    })
}
