use super::*;
use loco_rs::model::TenantQueryExt;

#[cfg(test)]
mod tests {
    use super::super::source::Reference;
    use super::*;

    fn request() -> TakeRequest {
        TakeRequest {
            client_uuid: Some("006B49D91DA242F1800B8C80867AEE1C".into()),
            source_type: "schedule".into(),
            source_id: "42".into(),
            taken_at: "2026-10-25T01:30:00+01:00".into(),
            dose_amount: Some("0.10".into()),
            dose_unit: Some("tablet".into()),
            taken_from_medication_id: Some(7),
            expected_effective_amount: None,
            expected_effective_unit: None,
        }
    }

    #[test]
    fn dose_boundary_parses_exact_amount_time_uuid_and_distinct_source()
    -> Result<(), OperationError> {
        let parsed = prepare_request(request(), chrono_tz::Europe::London)?;
        assert!(
            matches!(parsed.source, SourceReference::Schedule(Reference::Id(id)) if id.get() == 42)
        );
        assert_eq!(parsed.amount?, Some(Decimal::new(10, 2)));
        assert_eq!(parsed.taken_at?.to_string(), "2026-10-25 00:30:00");
        assert_eq!(parsed.zone, chrono_tz::Europe::London);
        assert_eq!(
            parsed.client_uuid.map(|id| id.to_string()).as_deref(),
            Some("006b49d9-1da2-42f1-800b-8c80867aee1c")
        );
        assert_eq!(
            parsed.selected.map(crate::models::ids::MedicationId::get),
            Some(7)
        );
        Ok(())
    }

    #[test]
    fn dose_boundary_keeps_invalid_semantics_for_replay_conflict_precedence()
    -> Result<(), OperationError> {
        let mut request = request();
        request.taken_at = "invalid".into();
        request.dose_amount = Some("1e2".into());
        let parsed = prepare_request(request, chrono_tz::UTC)?;
        assert!(parsed.taken_at.is_err());
        assert!(parsed.amount.is_err());
        Ok(())
    }

    #[test]
    fn dose_boundary_keeps_portable_sources_and_rejects_invalid_ids() -> Result<(), OperationError>
    {
        let mut request = request();
        request.source_type = "person_medication".into();
        request.source_id = "006b49d9-1da2-42f1-800b-8c80867aee1c".into();
        let parsed = prepare_request(request.clone(), chrono_tz::UTC)?;
        assert!(matches!(
            parsed.source,
            SourceReference::Assignment(Reference::Portable(_))
        ));
        for invalid in ["0", "-1", "9223372036854775808", "not-an-id"] {
            request.source_id = invalid.into();
            assert!(prepare_request(request.clone(), chrono_tz::UTC).is_err());
        }
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn dose_configuration_numeric_fraction_never_rounds_to_storage_precision()
-> Result<(), Box<dyn std::error::Error>> {
    let config: Value = serde_json::from_str(r#"{"amount":0.1000000000000000000000000001}"#)?;
    assert!(
        !config_decimal(&config, &["amount"]).is_some_and(valid_numeric_10_2),
        "The JSON numeric fraction rounded into an accepted two-decimal dose"
    );
    Ok(())
}

pub(super) struct TakeCandidate {
    pub(super) source: Source,
    pub(super) taken_at: NaiveDateTime,
    pub(super) zone: chrono_tz::Tz,
    pub(super) amount: Decimal,
    pub(super) unit: String,
    pub(super) selected: medication::Model,
}

pub(super) struct DoseCommand {
    pub(super) source: DosePrescription,
    pub(super) taken_at: NaiveDateTime,
    pub(super) amount: Decimal,
    pub(super) unit: String,
    pub(super) selected: StockSelection,
}

pub(super) struct StockSelection {
    pub(super) id: MedicationId,
    pub(super) location_id: Option<crate::models::ids::LocationId>,
    pub(super) current_supply: Option<Decimal>,
}

pub(super) struct ValidatedTake(DoseCommand);

impl std::ops::Deref for ValidatedTake {
    type Target = DoseCommand;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub(super) fn valid_numeric_10_2(amount: Decimal) -> bool {
    amount > Decimal::ZERO && amount.normalize().scale() <= 2 && amount < Decimal::from(100_000_000)
}

#[derive(Clone, Debug)]
pub struct PreparedRequest {
    pub(super) source: SourceReference,
    pub(super) client_uuid: Option<Uuid>,
    pub(super) taken_at: Result<NaiveDateTime, ApiError>,
    pub(super) amount: Result<Option<Decimal>, ApiError>,
    pub(super) unit: Option<String>,
    pub(super) selected: Option<MedicationId>,
    pub(super) expected_amount: Option<Option<Decimal>>,
    pub(super) expected_unit: Option<String>,
    pub(super) zone: chrono_tz::Tz,
}

pub fn prepare_request(
    request: TakeRequest,
    zone: chrono_tz::Tz,
) -> Result<PreparedRequest, ApiError> {
    let source = SourceReference::parse(&request.source_type, &request.source_id)?;
    let client_uuid = request
        .client_uuid
        .as_deref()
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|_| error(ErrorKind::Validation, "invalid client_uuid"))?;
    if request.dose_unit.as_ref().is_some_and(String::is_empty) {
        return Err(error(ErrorKind::Validation, "invalid dose_unit"));
    }
    let selected = request
        .taken_from_medication_id
        .map(|id| {
            if id > 0 {
                Ok(MedicationId::from_database(id))
            } else {
                Err(error(
                    ErrorKind::Validation,
                    "invalid taken_from_medication_id",
                ))
            }
        })
        .transpose()?;
    let taken_at = DateTime::parse_from_rfc3339(&request.taken_at)
        .ok()
        .and_then(|value| DateTime::<Utc>::from_timestamp_micros(value.timestamp_micros()))
        .map(|value| value.naive_utc())
        .ok_or_else(|| error(ErrorKind::Validation, "taken_at is invalid"));
    let amount = request
        .dose_amount
        .as_deref()
        .map(|text| {
            parse_decimal(text)
                .filter(|amount| valid_numeric_10_2(*amount))
                .ok_or_else(|| error(ErrorKind::Validation, "Invalid dose configured"))
        })
        .transpose();
    Ok(PreparedRequest {
        source,
        client_uuid,
        taken_at,
        amount,
        unit: request.dose_unit,
        selected,
        expected_amount: request
            .expected_effective_amount
            .as_deref()
            .map(|value| Decimal::from_str_exact(value).ok()),
        expected_unit: request.expected_effective_unit,
        zone,
    })
}

pub(super) async fn prepare(
    db: &DatabaseTransaction,
    context: &DoseContext<'_>,
    household_id: HouseholdId,
    request: &PreparedRequest,
) -> Result<TakeCandidate, TakeFailure> {
    let mut source = source(db, context, household_id, &request.source).await?;
    if source.retired {
        return Err(OperationError::NotFound.into());
    }
    if !allowed_person(db, context, source.person_id).await? {
        return Err(OperationError::Forbidden.into());
    }
    let taken_at = request.taken_at.clone()?;
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
    if source.invalid_configured_amount {
        return Err(error(ErrorKind::Validation, "Invalid dose configured").into());
    }
    if request.expected_amount.is_some() || request.expected_unit.is_some() {
        if request.expected_amount.flatten() != source.dose_amount
            || request.expected_unit.as_deref() != source.dose_unit.as_deref()
        {
            return Err(DoseConflict::DoseChanged.into_error().into());
        }
    }
    let amount = request
        .amount
        .clone()?
        .or(source.dose_amount)
        .ok_or_else(|| error(ErrorKind::Validation, "Invalid dose configured"))?;
    if !valid_numeric_10_2(amount) {
        return Err(error(ErrorKind::Validation, "Invalid dose configured").into());
    }
    let unit = source
        .dose_unit
        .clone()
        .ok_or_else(|| error(ErrorKind::Validation, "Invalid dose configured"))?;
    let selected = select_stock(
        db,
        context,
        household_id,
        source.medication_id,
        request.selected,
    )
    .await?;
    Ok(TakeCandidate {
        source,
        taken_at,
        zone: request.zone,
        amount,
        unit,
        selected,
    })
}

pub(super) async fn validate(
    db: &DatabaseTransaction,
    candidate: TakeCandidate,
    requested_unit: Option<&str>,
) -> Result<ValidatedTake, TakeFailure> {
    if requested_unit.is_some_and(|unit| unit != candidate.unit) {
        return Err(error(ErrorKind::Validation, "dose_unit does not match the source").into());
    }
    if !timing_allowed(db, &candidate).await? {
        return Err(error(
            ErrorKind::Validation,
            "Cannot take medication: timing restrictions not met",
        )
        .into());
    }
    Ok(ValidatedTake(DoseCommand {
        source: candidate.source.into_prescription(),
        taken_at: candidate.taken_at,
        amount: candidate.amount,
        unit: candidate.unit,
        selected: StockSelection {
            id: MedicationId::from_database(candidate.selected.id),
            location_id: candidate
                .selected
                .location_id
                .map(crate::models::ids::LocationId::from_database),
            current_supply: candidate.selected.current_supply,
        },
    }))
}

async fn select_stock(
    db: &DatabaseTransaction,
    context: &DoseContext<'_>,
    household_id: HouseholdId,
    source_medication_id: MedicationId,
    requested_stock: Option<MedicationId>,
) -> Result<medication::Model, TakeFailure> {
    let original = medication::Entity::find_by_id(source_medication_id.get())
        .in_tenant(household_id.get())
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or(OperationError::NotFound)?;
    let matching = access::medication_scope(context.tenant)
        .filter(original.name.map_or_else(
            || medication::Column::Name.is_null(),
            |name| medication::Column::Name.eq(name),
        ))
        .filter(original.dose_amount.map_or_else(
            || medication::Column::DoseAmount.is_null(),
            |amount| medication::Column::DoseAmount.eq(amount),
        ))
        .filter(original.dose_unit.map_or_else(
            || medication::Column::DoseUnit.is_null(),
            |unit| medication::Column::DoseUnit.eq(unit),
        ))
        .filter(
            Condition::any()
                .add(medication::Column::CurrentSupply.is_null())
                .add(medication::Column::CurrentSupply.gt(Decimal::ZERO)),
        )
        .all(db)
        .await
        .map_err(database_error)?;
    if matching.is_empty() {
        return Err(TakeFailure {
            error: error(
                ErrorKind::Validation,
                "Cannot take medication: out of stock",
            ),
            cause: Some(TakeFailureCause::OutOfStock),
        });
    }
    let selected_id = requested_stock;
    let selected = if let Some(id) = selected_id {
        matching
            .into_iter()
            .find(|value| value.id == id.get())
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
    Ok(selected)
}
