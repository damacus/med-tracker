use crate::models::care::treatment_command::{AdministrationKind, DoseCycle, ScheduleType};
use serde::Serialize;

#[derive(PartialEq, Serialize)]
pub struct Schedule {
    #[serde(flatten)]
    pub source: TreatmentSource,
    pub person_schedule_id: Option<String>,
    pub clinical_revision: Option<i64>,
    pub origin_time_zone: Option<String>,
    pub frequency: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub pause_started_at: Option<String>,
    pub(crate) schedule_type: ScheduleType,
    pub schedule_config: serde_json::Value,
}

#[derive(PartialEq, Serialize)]
pub struct Assignment {
    #[serde(flatten)]
    pub source: TreatmentSource,
    pub(crate) administration_kind: AdministrationKind,
    pub position: i32,
}

#[derive(PartialEq, Serialize)]
pub struct TreatmentSource {
    pub id: i64,
    pub portable_id: String,
    pub person_id: i64,
    pub person_portable_id: Option<String>,
    pub medication_id: i64,
    pub medication_portable_id: Option<String>,
    pub dose_amount: Option<String>,
    pub dose_unit: Option<String>,
    pub(crate) dose_cycle: Option<DoseCycle>,
    pub active: bool,
    pub paused: bool,
    pub can_manage: bool,
    pub can_record: bool,
    pub eligible_stock_medication_ids: Vec<i64>,
    pub notes: Option<String>,
    pub updated_at: String,
    pub max_daily_doses: Option<i32>,
    pub min_hours_between_doses: Option<String>,
    pub current_pause_period: Option<PauseSummary>,
}

#[derive(Clone, PartialEq, Serialize)]
pub struct PauseSummary {
    pub id: String,
    pub portable_id: String,
    pub source_type: PauseSourceKind,
    pub source_id: String,
    pub reason: String,
    pub note: Option<String>,
    pub legacy_context: bool,
    pub recorded_by_membership_id: Option<String>,
    pub resumed_by_membership_id: Option<String>,
    pub recorded_by_name: Option<String>,
    pub resumed_by_name: Option<String>,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PauseSourceKind {
    Schedule,
    PersonMedication,
}

pub type TreatmentResponse<T> = super::DataResponse<T>;

pub type TreatmentCollection<T> = super::PaginatedResponse<T>;

pub type ScheduleResponse = TreatmentResponse<Schedule>;
pub type AssignmentResponse = TreatmentResponse<Assignment>;

#[derive(Serialize)]
pub(crate) struct StockBindingSchedule {
    pub id: i64,
    pub portable_id: String,
    pub person_id: i64,
    pub person_schedule_id: Option<String>,
    pub clinical_revision: Option<i64>,
    pub origin_time_zone: Option<String>,
    pub execution_ended_at: Option<chrono::NaiveDateTime>,
    pub retired_at: Option<chrono::NaiveDateTime>,
    pub medication_id: i64,
    pub source_dosage_option_id: Option<i64>,
    pub active: bool,
    pub dose_amount: Option<String>,
    pub dose_unit: Option<String>,
    pub frequency: Option<String>,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub notes: Option<String>,
    pub max_daily_doses: Option<i32>,
    pub min_hours_between_doses: Option<String>,
    pub dose_cycle: Option<DoseCycle>,
    pub schedule_type: ScheduleType,
    pub schedule_config: serde_json::Value,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(PartialEq, Serialize)]
#[serde(untagged)]
pub enum Treatment {
    Schedule(Box<ScheduleResponse>),
    Assignment(Box<AssignmentResponse>),
}

impl Treatment {
    pub fn source(&self) -> &TreatmentSource {
        match self {
            Self::Schedule(response) => &response.data.source,
            Self::Assignment(response) => &response.data.source,
        }
    }
}
