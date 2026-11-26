use crate::models::care::treatment_command::DoseCycle;
use serde::Serialize;

#[derive(Default, Serialize)]
pub(crate) struct Suggestion {
    pub medication: MedicationFields,
    pub doses: Vec<DoseSuggestion>,
    pub sources: Vec<Source>,
    pub errors: Vec<String>,
}

#[derive(Default, Serialize)]
pub(crate) struct MedicationFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct Source {
    pub url: String,
    pub title: String,
}

#[derive(Serialize)]
pub(crate) struct DoseSuggestion {
    pub amount: String,
    pub unit: String,
    pub default_max_daily_doses: u64,
    pub default_min_hours_between_doses: String,
    pub default_dose_cycle: DoseCycle,
    pub evidence: Evidence,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct Evidence {
    pub url: String,
    pub title: String,
    pub text: String,
}
