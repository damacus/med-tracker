use super::*;
use crate::models::entities::{pause_period, schedule};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

pub(super) enum RestoreConflict {
    Clinical { source: String, field: &'static str },
    Pause { source: String },
}

impl std::fmt::Display for RestoreConflict {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Clinical { source, field } => write!(
                formatter,
                "schedules[{source}] {field} conflicts with a person-owned plan; use its treatment edit instead of historical restore"
            ),
            Self::Pause { source } => write!(
                formatter,
                "schedules[{source}] pause state conflicts with the retained person-owned execution; restore cannot change its current pause"
            ),
        }
    }
}

fn same_field<T: serde::de::DeserializeOwned + PartialEq>(
    fields: &Map<String, Value>,
    name: &'static str,
    current: &T,
) -> bool {
    fields.get(name).is_none_or(|value| {
        serde_json::from_value::<T>(value.clone()).is_ok_and(|incoming| incoming == *current)
    })
}

fn clinical_mismatch(fields: &Map<String, Value>, row: &schedule::Model) -> Option<&'static str> {
    macro_rules! matches {
        ($name:literal,$current:expr) => {
            if !same_field(fields, $name, &$current) {
                return Some($name);
            }
        };
    }
    matches!("person_id", row.person_id);
    matches!("medication_id", row.medication_id);
    matches!("source_dosage_option_id", row.source_dosage_option_id);
    matches!("active", row.active);
    matches!("dose_amount", row.dose_amount);
    matches!("dose_unit", row.dose_unit);
    matches!("dose_cycle", row.dose_cycle);
    matches!("frequency", row.frequency);
    matches!("start_date", row.start_date);
    matches!("end_date", row.end_date);
    matches!("notes", row.notes);
    matches!("max_daily_doses", row.max_daily_doses);
    matches!(
        "min_hours_between_doses",
        row.min_hours_between_doses.map(Decimal::from)
    );
    matches!("schedule_type", row.schedule_type);
    matches!("schedule_config", row.schedule_config);
    matches!("retired_at", row.retired_at.map(|time| time.and_utc()));
    None
}

pub(super) async fn conflicts(
    db: &DatabaseTransaction,
    tenant: &TenantTransaction,
    plan: &ImportPlan,
    index: &ExistingIndex,
) -> Result<Vec<RestoreConflict>, OperationError> {
    let mut ids: HashSet<String> = plan
        .rows("schedules")
        .iter()
        .filter_map(|row| row["portable_id"].as_str().map(str::to_owned))
        .collect();
    ids.extend(
        plan.rows("medication_pause_periods")
            .iter()
            .filter(|row| row["source_type"] == "schedule")
            .filter_map(|row| row["source_portable_id"].as_str().map(str::to_owned)),
    );
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let sources = schedule::Entity::find()
        .filter(schedule::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(schedule::Column::PortableId.is_in(ids))
        .filter(schedule::Column::PersonScheduleId.is_not_null())
        .all(db)
        .await?;
    if sources.is_empty() {
        return Ok(Vec::new());
    }
    let periods = pause_period::Entity::find()
        .filter(pause_period::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(pause_period::Column::ScheduleId.is_in(sources.iter().map(|row| row.id)))
        .filter(pause_period::Column::EndedAt.is_null())
        .all(db)
        .await?;
    let mut conflicts = Vec::new();
    for source in sources {
        if let Some(incoming) = plan
            .rows("schedules")
            .iter()
            .find(|row| row["portable_id"] == source.portable_id)
            && let Ok(fields) = mapped_row("schedules", incoming, index, tenant)
            && let Some(field) = clinical_mismatch(&fields, &source)
        {
            conflicts.push(RestoreConflict::Clinical {
                source: source.portable_id.clone(),
                field,
            });
        }
        let current: HashSet<String> = periods
            .iter()
            .filter(|row| row.schedule_id == Some(source.id))
            .map(|row| row.portable_id.clone())
            .collect();
        let mut projected = current.clone();
        for incoming in plan.rows("medication_pause_periods").iter().filter(|row| {
            row["source_type"] == "schedule" && row["source_portable_id"] == source.portable_id
        }) {
            let Some(id) = incoming["portable_id"].as_str() else {
                continue;
            };
            if incoming["ended_at"].is_null() {
                projected.insert(id.into());
            } else {
                projected.remove(id);
            }
        }
        if current != projected {
            conflicts.push(RestoreConflict::Pause {
                source: source.portable_id,
            });
        }
    }
    Ok(conflicts)
}
