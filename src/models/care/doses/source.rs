use super::*;
use loco_rs::model::TenantQueryExt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DoseSource {
    Schedule(ScheduleId),
    Assignment(AssignmentId),
}

impl DoseSource {
    pub(super) fn schedule_id(self) -> Option<i64> {
        match self {
            Self::Schedule(id) => Some(id.get()),
            Self::Assignment(_) => None,
        }
    }
    pub(super) fn assignment_id(self) -> Option<i64> {
        match self {
            Self::Assignment(id) => Some(id.get()),
            Self::Schedule(_) => None,
        }
    }
}

#[derive(Clone, Debug)]
pub(super) enum Reference<I> {
    Id(I),
    Portable(PortableSourceId),
}

#[derive(Clone, Debug)]
pub(super) struct PortableSourceId(String);

impl PortableSourceId {
    fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub(super) enum SourceReference {
    Schedule(Reference<ScheduleId>),
    Assignment(Reference<AssignmentId>),
}

impl SourceReference {
    pub(super) fn parse(kind: &str, id: &str) -> Result<Self, ApiError> {
        if !valid_identifier(id) || kind.trim().is_empty() {
            return Err(error(ErrorKind::Validation, "invalid medication source"));
        }
        let numeric = if id.bytes().all(|byte| byte.is_ascii_digit()) {
            Some(
                id.parse::<i64>()
                    .map_err(|_| error(ErrorKind::Validation, "invalid medication source"))?,
            )
        } else {
            None
        };
        match kind {
            "schedule" => Ok(Self::Schedule(numeric.map_or_else(
                || Reference::Portable(PortableSourceId(id.to_owned())),
                |id| Reference::Id(ScheduleId::from_database(id)),
            ))),
            "person_medication" => Ok(Self::Assignment(numeric.map_or_else(
                || Reference::Portable(PortableSourceId(id.to_owned())),
                |id| Reference::Id(AssignmentId::from_database(id)),
            ))),
            _ => Err(OperationError::NotFound),
        }
    }

    pub(super) fn matches(&self, source: DoseSource, portable_id: &str) -> bool {
        match (self, source) {
            (Self::Schedule(Reference::Id(request)), DoseSource::Schedule(stored)) => {
                *request == stored
            }
            (Self::Assignment(Reference::Id(request)), DoseSource::Assignment(stored)) => {
                *request == stored
            }
            (Self::Schedule(Reference::Portable(request)), DoseSource::Schedule(_))
            | (Self::Assignment(Reference::Portable(request)), DoseSource::Assignment(_)) => {
                request.as_str() == portable_id
            }
            _ => false,
        }
    }
}

#[derive(Clone)]
pub(super) struct Source {
    pub(super) identity: DoseSource,
    pub(super) active: bool,
    pub(super) retired: bool,
    pub(super) invalid_configured_amount: bool,
    pub(super) person_id: PersonId,
    pub(super) medication_id: MedicationId,
    pub(super) dose_amount: Option<Decimal>,
    pub(super) dose_unit: Option<String>,
    pub(super) max_daily_doses: Option<i32>,
    pub(super) min_hours_between_doses: Option<Decimal>,
    pub(super) dose_cycle: Option<i32>,
    pub(super) source_dosage_option_id: Option<i64>,
    pub(super) schedule_type: Option<i32>,
    pub(super) schedule_config: Option<Value>,
    pub(super) start_date: Option<NaiveDate>,
    pub(super) end_date: Option<NaiveDate>,
}

pub(super) struct DosePrescription {
    pub(super) identity: DoseSource,
    pub(super) person_id: PersonId,
    pub(super) medication_id: MedicationId,
    pub(super) dose_amount: Option<Decimal>,
    pub(super) dose_unit: Option<String>,
    pub(super) source_dosage_option_id: Option<i64>,
}

impl Source {
    pub(super) fn into_prescription(self) -> DosePrescription {
        DosePrescription {
            identity: self.identity,
            person_id: self.person_id,
            medication_id: self.medication_id,
            dose_amount: self.dose_amount,
            dose_unit: self.dose_unit,
            source_dosage_option_id: self.source_dosage_option_id,
        }
    }
}

pub(super) fn source_from_assignment(value: person_medication::Model) -> Source {
    Source {
        identity: DoseSource::Assignment(AssignmentId::from_database(value.id)),
        active: value.active,
        retired: value.retired_at.is_some(),
        invalid_configured_amount: false,
        person_id: PersonId::from_database(value.person_id),
        medication_id: MedicationId::from_database(value.medication_id),
        dose_amount: value.dose_amount,
        dose_unit: value.dose_unit,
        max_daily_doses: value.max_daily_doses,
        min_hours_between_doses: value.min_hours_between_doses.map(Decimal::from),
        dose_cycle: value.dose_cycle,
        source_dosage_option_id: value.source_dosage_option_id,
        schedule_type: None,
        schedule_config: None,
        start_date: None,
        end_date: None,
    }
}

pub(super) fn source_from_schedule(value: schedule::Model) -> Source {
    Source {
        identity: DoseSource::Schedule(ScheduleId::from_database(value.id)),
        active: value.active,
        retired: value.retired_at.is_some(),
        invalid_configured_amount: false,
        person_id: PersonId::from_database(value.person_id),
        medication_id: MedicationId::from_database(value.medication_id),
        dose_amount: value.dose_amount,
        dose_unit: value.dose_unit,
        max_daily_doses: value.max_daily_doses,
        min_hours_between_doses: value.min_hours_between_doses.map(Decimal::from),
        dose_cycle: value.dose_cycle,
        source_dosage_option_id: value.source_dosage_option_id,
        schedule_type: Some(value.schedule_type),
        schedule_config: Some(value.schedule_config),
        start_date: value.start_date,
        end_date: value.end_date,
    }
}

pub(crate) fn app_zone(settings: Option<&Value>) -> chrono_tz::Tz {
    crate::models::settings::text(settings, &["time_zone"])
        .and_then(|value| value.parse().ok())
        .unwrap_or(chrono_tz::UTC)
}

pub(super) fn local_date_in_zone(time: NaiveDateTime, zone: chrono_tz::Tz) -> NaiveDate {
    Utc.from_utc_datetime(&time)
        .with_timezone(&zone)
        .date_naive()
}

pub(crate) fn config_value<'a>(config: &'a Value, names: &[&str]) -> Option<&'a Value> {
    names.iter().find_map(|name| config.get(*name))
}

pub(crate) fn config_decimal(config: &Value, names: &[&str]) -> Option<Decimal> {
    match config_value(config, names)? {
        Value::String(text) => Decimal::from_str_exact(text).ok(),
        Value::Number(number) => {
            let text = number.to_string();
            let mantissa = text.split(['e', 'E']).next()?;
            Decimal::from_str_exact(mantissa).ok()?;
            if text.contains(['e', 'E']) {
                Decimal::from_scientific(&text).ok()
            } else {
                Decimal::from_str_exact(&text).ok()
            }
        }
        _ => None,
    }
}

pub(super) fn effective_source(source: &mut Source, date: NaiveDate) {
    let Some(config) = source.schedule_config.as_ref() else {
        return;
    };
    let effective = if source.schedule_type == Some(5) {
        config
            .get("taper_steps")
            .and_then(Value::as_array)
            .and_then(|steps| {
                steps.iter().find(|step| {
                    let start = step
                        .get("start_date")
                        .and_then(Value::as_str)
                        .and_then(|v| NaiveDate::parse_from_str(v, "%Y-%m-%d").ok());
                    let end = step
                        .get("end_date")
                        .and_then(Value::as_str)
                        .and_then(|v| NaiveDate::parse_from_str(v, "%Y-%m-%d").ok());
                    start
                        .zip(end)
                        .is_some_and(|(start, end)| date >= start && date <= end)
                })
            })
            .unwrap_or(config)
    } else {
        config
    };
    if config_value(effective, &["amount", "dose_amount"]).is_some() {
        let amount = config_decimal(effective, &["amount", "dose_amount"]);
        source.invalid_configured_amount =
            amount.is_none_or(|amount| !input::valid_numeric_10_2(amount));
        source.dose_amount = amount;
    }
    source.dose_unit = config_value(effective, &["unit", "dose_unit"])
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| source.dose_unit.clone());
    source.max_daily_doses = config_decimal(effective, &["max_daily_doses", "max_doses", "max"])
        .and_then(|value| value.to_string().parse::<i32>().ok())
        .or(source.max_daily_doses);
    source.min_hours_between_doses = config_decimal(
        effective,
        &["min_hours_between_doses", "min_hours", "minimum_hours"],
    )
    .or(source.min_hours_between_doses);
    if source.schedule_type == Some(0)
        && effective
            .get("times")
            .and_then(Value::as_array)
            .is_some_and(|times| {
                times
                    .iter()
                    .filter(|value| !value.is_null() && value.as_str() != Some(""))
                    .count()
                    == 1
            })
    {
        source.min_hours_between_doses = None;
    }
}

pub(super) async fn source(
    db: &DatabaseTransaction,
    context: &DoseContext<'_>,
    household_id: HouseholdId,
    reference: &SourceReference,
) -> Result<Source, ApiError> {
    match reference {
        SourceReference::Assignment(reference) => {
            let mut query = person_medication::Entity::find().in_tenant(household_id.get());
            query = match reference {
                Reference::Id(id) => query.filter(person_medication::Column::Id.eq(id.get())),
                Reference::Portable(id) => {
                    query.filter(person_medication::Column::PortableId.eq(id.as_str()))
                }
            };
            let value = query
                .one(db)
                .await
                .map_err(database_error)?
                .ok_or(OperationError::NotFound)?;
            if !allowed_person(db, context, PersonId::from_database(value.person_id)).await? {
                return Err(OperationError::NotFound);
            }
            Ok(source_from_assignment(value))
        }
        SourceReference::Schedule(reference) => {
            let mut query = schedule::Entity::find().in_tenant(household_id.get());
            query = match reference {
                Reference::Id(id) => query.filter(schedule::Column::Id.eq(id.get())),
                Reference::Portable(id) => {
                    query.filter(schedule::Column::PortableId.eq(id.as_str()))
                }
            };
            let value = query
                .one(db)
                .await
                .map_err(database_error)?
                .ok_or(OperationError::NotFound)?;
            if !allowed_person(db, context, PersonId::from_database(value.person_id)).await? {
                return Err(OperationError::NotFound);
            }
            Ok(source_from_schedule(value))
        }
    }
}
