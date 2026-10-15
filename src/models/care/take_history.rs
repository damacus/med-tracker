use chrono::NaiveDateTime;
use sea_orm::{DatabaseTransaction, DbBackend, FromQueryResult, Statement, prelude::Decimal};
use std::collections::HashMap;

pub(crate) mod self_history;

use crate::models::{
    access::{self, TenantTransaction},
    care::doses::{CredentialProvenance, DoseSource},
    errors::OperationError,
    ids::{
        AssignmentId, HouseholdId, HouseholdMembershipId, MedicationId, PersonId, ScheduleId,
        TakeId,
    },
    person_identity::PersonSummary,
};

#[derive(Clone, Debug)]
pub enum HistoricalActor {
    Known {
        person_id: PersonId,
        membership_id: Option<HouseholdMembershipId>,
        name: String,
    },
    Unknown,
    Imported,
}

#[derive(Clone, Copy, Debug)]
pub enum Administration {
    Scheduled,
    Routine,
    AsNeeded,
}

impl Administration {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Scheduled => "scheduled",
            Self::Routine => "routine",
            Self::AsNeeded => "as_needed",
        }
    }
}

#[derive(Clone, Debug)]
pub struct RecordedTake {
    pub id: TakeId,
    pub portable_id: String,
    pub client_uuid: Option<String>,
    pub updated_at: NaiveDateTime,
    pub taken_at: Option<NaiveDateTime>,
    pub dose_amount: Option<Decimal>,
    pub dose_unit: Option<String>,
    pub household_id: HouseholdId,
    pub person_id: PersonId,
    pub person_portable_id: String,
    pub subject_name: String,
    pub source: DoseSource,
    pub source_portable_id: String,
    pub medication_id: MedicationId,
    pub medication_portable_id: String,
    pub medication_name: Option<String>,
    pub administration: Administration,
    pub taken_from_medication_id: Option<MedicationId>,
    pub taken_from_medication_portable_id: Option<String>,
    pub taken_from_location_id: Option<i64>,
    pub taken_from_location_portable_id: Option<String>,
    pub location_name: Option<String>,
    pub actor: HistoricalActor,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct TimingWindow {
    pub person_id: PersonId,
    pub medication_id: MedicationId,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct TimingFacts {
    pub cycle_count: u64,
    pub latest_at: Option<NaiveDateTime>,
}

#[derive(FromQueryResult)]
struct TimingProjection {
    window_index: i64,
    cycle_count: i64,
    latest_at: Option<NaiveDateTime>,
}

/// # Errors
/// Returns an error when live local authority is unavailable or timing facts
/// cannot be read and matched to every requested window.
pub(crate) async fn timing_facts(
    db: &DatabaseTransaction,
    household: HouseholdId,
    windows: &[TimingWindow],
    candidate: NaiveDateTime,
) -> Result<HashMap<TimingWindow, TimingFacts>, OperationError> {
    let rows = TimingProjection::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT * FROM med_tracker.local_dose_timing($1,$2,$3,$4,$5,$6)",
        [
            household.get().into(),
            windows
                .iter()
                .map(|window| window.person_id.get())
                .collect::<Vec<_>>()
                .into(),
            windows
                .iter()
                .map(|window| window.medication_id.get())
                .collect::<Vec<_>>()
                .into(),
            windows
                .iter()
                .map(|window| window.start)
                .collect::<Vec<_>>()
                .into(),
            windows
                .iter()
                .map(|window| window.end)
                .collect::<Vec<_>>()
                .into(),
            candidate.into(),
        ],
    ))
    .all(db)
    .await?;
    let mut facts = HashMap::with_capacity(windows.len());
    for row in rows {
        let index = row
            .window_index
            .checked_sub(1)
            .and_then(|index| usize::try_from(index).ok())
            .ok_or(OperationError::Unavailable)?;
        let window = windows
            .get(index)
            .copied()
            .ok_or(OperationError::Unavailable)?;
        let value = TimingFacts {
            cycle_count: u64::try_from(row.cycle_count).map_err(|_| OperationError::Unavailable)?,
            latest_at: row.latest_at,
        };
        if facts.insert(window, value).is_some() {
            return Err(OperationError::Unavailable);
        }
    }
    if facts.len() != windows.len() {
        return Err(OperationError::Unavailable);
    }
    Ok(facts)
}

pub(crate) struct DashboardTake {
    pub id: TakeId,
    pub person_id: PersonId,
    pub subject_name: String,
    pub medication_id: MedicationId,
    pub medication_name: String,
    pub source: DoseSource,
    pub administration: Administration,
    pub dose_amount: Decimal,
    pub dose_unit: String,
    pub taken_at: NaiveDateTime,
}

#[derive(FromQueryResult)]
struct DashboardProjection {
    id: i64,
    person_id: i64,
    subject_name: String,
    medication_id: i64,
    medication_name: Option<String>,
    schedule_id: Option<i64>,
    person_medication_id: Option<i64>,
    administration: String,
    dose_amount: Decimal,
    dose_unit: String,
    taken_at: NaiveDateTime,
}

/// # Errors
/// Returns an error when live local authority or original administration facts
/// are unavailable for the requested people and day.
pub(crate) async fn dashboard(
    tenant: &TenantTransaction,
    people: &[PersonId],
    start: NaiveDateTime,
    end: NaiveDateTime,
) -> Result<Vec<DashboardTake>, OperationError> {
    access::recheck(tenant).await?;
    let rows = DashboardProjection::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT * FROM med_tracker.local_dashboard_take_history($1,$2,$3,$4)",
        [
            tenant.scope().household_id.into(),
            people.iter().map(|id| id.get()).collect::<Vec<_>>().into(),
            start.into(),
            end.into(),
        ],
    ))
    .all(tenant.transaction())
    .await?;
    rows.into_iter()
        .map(|row| {
            let source = match (row.schedule_id, row.person_medication_id) {
                (Some(id), None) => DoseSource::Schedule(ScheduleId::from_database(id)),
                (None, Some(id)) => DoseSource::Assignment(AssignmentId::from_database(id)),
                _ => return Err(OperationError::Unavailable),
            };
            let administration = match row.administration.as_str() {
                "scheduled" => Administration::Scheduled,
                "routine" => Administration::Routine,
                "as_needed" => Administration::AsNeeded,
                _ => return Err(OperationError::Unavailable),
            };
            Ok(DashboardTake {
                id: TakeId::from_database(row.id),
                person_id: PersonId::from_database(row.person_id),
                subject_name: row.subject_name,
                medication_id: MedicationId::from_database(row.medication_id),
                medication_name: row.medication_name.unwrap_or_default(),
                source,
                administration,
                dose_amount: row.dose_amount,
                dose_unit: row.dose_unit,
                taken_at: row.taken_at,
            })
        })
        .collect()
}

#[derive(FromQueryResult)]
struct Projection {
    id: i64,
    portable_id: String,
    client_uuid: Option<String>,
    updated_at: NaiveDateTime,
    taken_at: Option<NaiveDateTime>,
    dose_amount: Option<Decimal>,
    dose_unit: Option<String>,
    household_id: i64,
    person_id: i64,
    person_portable_id: String,
    subject_name: String,
    schedule_id: Option<i64>,
    schedule_portable_id: Option<String>,
    person_medication_id: Option<i64>,
    person_medication_portable_id: Option<String>,
    medication_id: i64,
    medication_portable_id: String,
    medication_name: Option<String>,
    administration: String,
    taken_from_medication_id: Option<i64>,
    taken_from_medication_portable_id: Option<String>,
    taken_from_location_id: Option<i64>,
    taken_from_location_portable_id: Option<String>,
    location_name: Option<String>,
    actor_kind: String,
    actor_person_id: Option<i64>,
    actor_membership_id: Option<i64>,
    actor_name: Option<String>,
}

impl TryFrom<Projection> for RecordedTake {
    type Error = OperationError;
    fn try_from(row: Projection) -> Result<Self, Self::Error> {
        let (source, portable) = match (row.schedule_id, row.person_medication_id) {
            (Some(id), None) => (
                DoseSource::Schedule(ScheduleId::from_database(id)),
                row.schedule_portable_id,
            ),
            (None, Some(id)) => (
                DoseSource::Assignment(AssignmentId::from_database(id)),
                row.person_medication_portable_id,
            ),
            _ => return Err(OperationError::Unavailable),
        };
        let actor = match row.actor_kind.as_str() {
            "known" => HistoricalActor::Known {
                person_id: PersonId::from_database(
                    row.actor_person_id.ok_or(OperationError::Unavailable)?,
                ),
                membership_id: row
                    .actor_membership_id
                    .map(HouseholdMembershipId::from_database),
                name: row.actor_name.ok_or(OperationError::Unavailable)?,
            },
            "unknown" => HistoricalActor::Unknown,
            "imported" => HistoricalActor::Imported,
            _ => return Err(OperationError::Unavailable),
        };
        let administration = match row.administration.as_str() {
            "scheduled" => Administration::Scheduled,
            "routine" => Administration::Routine,
            "as_needed" => Administration::AsNeeded,
            _ => return Err(OperationError::Unavailable),
        };
        Ok(Self {
            id: TakeId::from_database(row.id),
            portable_id: row.portable_id,
            client_uuid: row.client_uuid,
            updated_at: row.updated_at,
            taken_at: row.taken_at,
            dose_amount: row.dose_amount,
            dose_unit: row.dose_unit,
            household_id: HouseholdId::from_database(row.household_id),
            person_id: PersonId::from_database(row.person_id),
            person_portable_id: row.person_portable_id,
            subject_name: row.subject_name,
            source,
            source_portable_id: portable.ok_or(OperationError::Unavailable)?,
            medication_id: MedicationId::from_database(row.medication_id),
            medication_portable_id: row.medication_portable_id,
            medication_name: row.medication_name,
            administration,
            taken_from_medication_id: row
                .taken_from_medication_id
                .map(MedicationId::from_database),
            taken_from_medication_portable_id: row.taken_from_medication_portable_id,
            taken_from_location_id: row.taken_from_location_id,
            taken_from_location_portable_id: row.taken_from_location_portable_id,
            location_name: row.location_name,
            actor,
        })
    }
}

pub(crate) async fn append(
    tenant: &TenantTransaction,
    take: TakeId,
    expected_medication: MedicationId,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    use sea_orm::ConnectionTrait;
    access::recheck(tenant).await?;
    tenant
        .transaction()
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT med_tracker.append_take_history($1,$2,$3,$4)",
            [
                take.get().into(),
                expected_medication.get().into(),
                provenance.map(|p| p.method.label().to_owned()).into(),
                provenance.map(|p| p.reference.clone()).into(),
            ],
        ))
        .await?;
    Ok(())
}

pub async fn by_ids(
    db: &DatabaseTransaction,
    ids: &[TakeId],
) -> Result<Vec<RecordedTake>, OperationError> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    if ids.len() > 100 {
        return Err(OperationError::Unavailable);
    }
    let rows = Projection::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT * FROM med_tracker.local_take_history($1)",
        [ids.iter().map(|id| id.get()).collect::<Vec<_>>().into()],
    ))
    .all(db)
    .await?;
    rows.into_iter().map(RecordedTake::try_from).collect()
}

pub async fn subject(
    tenant: &TenantTransaction,
    identifier: &str,
) -> Result<Option<PersonSummary>, OperationError> {
    use sea_orm::ConnectionTrait;
    access::recheck(tenant).await?;
    let row = tenant
        .transaction()
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT * FROM med_tracker.local_historical_subject($1)",
            [identifier.into()],
        ))
        .await?;
    row.map(|row| {
        Ok(PersonSummary {
            id: PersonId::from_database(row.try_get("", "person_id")?),
            name: row.try_get("", "name")?,
            date_of_birth: None,
        })
    })
    .transpose()
}

pub async fn report(
    tenant: &TenantTransaction,
    person: PersonId,
    start: NaiveDateTime,
    end: NaiveDateTime,
) -> Result<Vec<RecordedTake>, OperationError> {
    access::recheck(tenant).await?;
    let mut rows = Vec::new();
    let mut after = 0;
    loop {
        let page = Projection::find_by_statement(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT * FROM med_tracker.local_take_report($1,$2,$3,$4,100)",
            [person.get().into(), start.into(), end.into(), after.into()],
        ))
        .all(tenant.transaction())
        .await?;
        let count = page.len();
        if let Some(last) = page.last() {
            after = last.id;
        }
        for row in page {
            rows.push(RecordedTake::try_from(row)?);
        }
        if count < 100 {
            return Ok(rows);
        }
        if rows.len() >= 10_000 {
            return Err(OperationError::Validation {
                details: serde_json::json!({"error":"Report contains too many records; select a shorter date range"}),
            });
        }
    }
}

pub fn utc_timestamp(time: NaiveDateTime) -> String {
    time.and_utc()
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}
