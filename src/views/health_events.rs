use serde::Serialize;

#[derive(Serialize)]
pub struct HealthEvent {
    pub id: i64,
    pub portable_id: String,
    pub person_id: i64,
    pub person_portable_id: Option<String>,
    pub event_kind: HealthEventKind,
    pub severity: Option<HealthEventSeverity>,
    pub title: String,
    pub notes: Option<String>,
    pub started_on: String,
    pub ended_on: Option<String>,
    pub updated_at: String,
    pub medication_ids: Vec<i64>,
    pub medication_portable_ids: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthEventKind {
    Illness,
    SuspectedSideEffect,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthEventSeverity {
    Mild,
    Moderate,
    Severe,
}

pub type HealthEventResponse = super::DataResponse<HealthEvent>;

pub type HealthEventCollectionResponse = super::PaginatedResponse<HealthEvent>;
