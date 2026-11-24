use super::PaginationMeta;
use serde::Serialize;

#[derive(Serialize)]
pub struct MedicationTake {
    pub id: i64,
    pub portable_id: String,
    pub client_uuid: Option<String>,
    pub schedule_id: Option<i64>,
    pub schedule_portable_id: Option<String>,
    pub person_medication_id: Option<i64>,
    pub person_medication_portable_id: Option<String>,
    pub taken_from_medication_id: Option<i64>,
    pub taken_from_medication_portable_id: Option<String>,
    pub taken_from_location_id: Option<i64>,
    pub taken_from_location_portable_id: Option<String>,
    pub dose_amount: Option<String>,
    pub dose_unit: Option<String>,
    pub taken_at: Option<String>,
    pub updated_at: String,
    pub person_id: Option<i64>,
    pub person_portable_id: Option<String>,
    pub medication_id: Option<i64>,
    pub medication_portable_id: Option<String>,
}

#[derive(Serialize)]
pub struct MedicationTakeResponse {
    pub data: MedicationTake,
}

#[derive(Serialize)]
pub struct MedicationTakeCollectionResponse {
    pub data: Vec<MedicationTake>,
    pub meta: PaginationMeta,
}
