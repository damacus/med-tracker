use super::*;
use crate::models::{
    care::treatment_command::{DoseCycle, ScheduleType},
    entities::shared_schedule,
};
use serde::Serialize;

#[derive(Clone, Serialize)]
pub(crate) struct SharedPlan {
    pub(crate) id: String,
    pub(crate) person_id: i64,
    pub(crate) clinical_revision: i64,
    pub(crate) etag: String,
    pub(crate) medicine_name: Option<String>,
    pub(crate) dmd_system: Option<String>,
    pub(crate) dmd_code: Option<String>,
    pub(crate) dmd_concept_class: Option<String>,
    pub(crate) stock_unit: Option<String>,
    pub(crate) origin_time_zone: String,
    pub(crate) active: bool,
    pub(crate) retired_at: Option<chrono::DateTime<Utc>>,
    pub(crate) pause_started_at: Option<chrono::DateTime<Utc>>,
    pub(crate) latest_taken_at: Option<chrono::DateTime<Utc>>,
    pub(crate) dose_amount: Option<String>,
    pub(crate) dose_unit: Option<String>,
    pub(crate) dose_cycle: Option<DoseCycle>,
    pub(crate) frequency: Option<String>,
    pub(crate) start_date: Option<chrono::NaiveDate>,
    pub(crate) end_date: Option<chrono::NaiveDate>,
    pub(crate) max_daily_doses: Option<i32>,
    pub(crate) min_hours_between_doses: Option<String>,
    pub(crate) schedule_type: ScheduleType,
    pub(crate) schedule_config: serde_json::Value,
}

impl TryFrom<&shared_schedule::Model> for SharedPlan {
    type Error = OperationError;
    fn try_from(row: &shared_schedule::Model) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id.clone(),
            person_id: row.person_id,
            clinical_revision: row.clinical_revision,
            etag: format!("\"person-schedule:{}:{}\"", row.id, row.clinical_revision),
            medicine_name: row.medicine_name.clone(),
            dmd_system: row.dmd_system.clone(),
            dmd_code: row.dmd_code.clone(),
            dmd_concept_class: row.dmd_concept_class.clone(),
            stock_unit: row.stock_unit.clone(),
            origin_time_zone: row.origin_time_zone.clone(),
            active: row.active,
            retired_at: row.retired_at.map(|time| time.and_utc()),
            pause_started_at: row.pause_started_at.map(|time| time.and_utc()),
            latest_taken_at: row.latest_taken_at.map(|time| time.and_utc()),
            dose_amount: row.dose_amount.map(|amount| amount.normalize().to_string()),
            dose_unit: row.dose_unit.clone(),
            dose_cycle: row
                .dose_cycle
                .map(|cycle| DoseCycle::from_database(cycle).ok_or(OperationError::Unavailable))
                .transpose()?,
            frequency: row.frequency.clone(),
            start_date: row.start_date,
            end_date: row.end_date,
            max_daily_doses: row.max_daily_doses,
            min_hours_between_doses: row.min_hours_between_doses.map(|hours| hours.to_string()),
            schedule_type: ScheduleType::from_database(row.schedule_type)
                .ok_or(OperationError::Unavailable)?,
            schedule_config: row.schedule_config.clone(),
        })
    }
}

pub(super) fn clinical_changes(
    before: &shared_schedule::Model,
    after: &shared_schedule::Model,
) -> Result<serde_json::Value, OperationError> {
    let before = serde_json::to_value(SharedPlan::try_from(before)?)
        .map_err(|_| OperationError::Unavailable)?;
    let after = serde_json::to_value(SharedPlan::try_from(after)?)
        .map_err(|_| OperationError::Unavailable)?;
    let fields = after.as_object().ok_or(OperationError::Unavailable)?;
    let changes: serde_json::Map<_, _> = fields
        .iter()
        .filter(|(key, _)| {
            ![
                "id",
                "person_id",
                "clinical_revision",
                "etag",
                "latest_taken_at",
            ]
            .contains(&key.as_str())
        })
        .filter(|(key, value)| before.get(*key) != Some(*value))
        .map(|(key, value)| (key.clone(), serde_json::json!([before.get(key), value])))
        .collect();
    Ok(serde_json::Value::Object(changes))
}
