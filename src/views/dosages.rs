use crate::models::care::dosages::DoseCycle;
use serde::Serialize;

#[derive(Serialize)]
pub struct DosageOption {
    pub id: i64,
    pub portable_id: String,
    pub medication_id: i64,
    pub medication_portable_id: String,
    pub amount: String,
    pub unit: String,
    pub frequency: String,
    pub description: Option<String>,
    pub default_for_adults: bool,
    pub default_for_children: bool,
    pub default_max_daily_doses: i32,
    pub default_min_hours_between_doses: String,
    pub default_dose_cycle: DoseCycle,
    pub current_supply: Option<String>,
    pub reorder_threshold: Option<String>,
    pub updated_at: String,
}

#[derive(Serialize)]
pub struct DosageOptionResponse {
    pub data: DosageOption,
}

#[derive(Serialize)]
pub struct DosageOptionCollectionResponse {
    pub data: Vec<DosageOption>,
    pub meta: PaginationMeta,
}

#[derive(Serialize)]
pub struct PaginationMeta {
    pub page: i64,
    pub per_page: i64,
    pub total_count: u64,
}
