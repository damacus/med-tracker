use serde::Serialize;

#[derive(Serialize)]
pub struct DoseOccurrence {
    pub key: String,
    pub source_type: &'static str,
    pub source_id: i64,
    pub source_portable_id: String,
    pub window_starts_on: chrono::NaiveDate,
    pub window_ends_on: chrono::NaiveDate,
    pub position: i32,
    pub scheduled_at: Option<String>,
    pub outcome: String,
    pub expected: bool,
    pub due: bool,
    pub reason: Option<String>,
    pub note: Option<String>,
    pub resolved_at: Option<String>,
    pub medication_take_id: Option<i64>,
    pub etag: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct BrowserDoseOccurrence {
    #[serde(flatten)]
    pub occurrence: DoseOccurrence,
    pub prescribed_amount: Option<String>,
    pub prescribed_unit: Option<String>,
}
