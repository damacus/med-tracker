use serde::Serialize;
use serde_json::Value;

#[derive(Debug, PartialEq, Serialize)]
pub struct StockRemoval {
    pub id: String,
    pub medication_id: String,
    pub dosage_id: Option<String>,
    pub quantity: Option<Value>,
    pub reason: Option<Value>,
    pub note: Option<Value>,
    pub submission_id: Option<Value>,
    pub previous_quantity: Option<Value>,
    pub remaining_quantity: Option<Value>,
    pub unit: Option<Value>,
    pub created_at: Option<String>,
    pub actor_membership_id: Option<String>,
}

pub type StockRemovalCollection = super::PaginatedResponse<StockRemoval>;

#[derive(Debug, Serialize)]
pub struct Medication {
    pub id: i64,
    pub portable_id: String,
    pub name: Option<String>,
    pub friendly_name: Option<String>,
    pub barcode: Option<String>,
    pub warnings: Option<String>,
    pub display_name: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
    pub dose_amount: Option<String>,
    pub dose_unit: Option<String>,
    pub current_supply: Option<String>,
    pub reorder_threshold: String,
    pub reorder_status: Option<ReorderStatus>,
    pub location_id: i64,
    pub location_portable_id: Option<String>,
    pub updated_at: String,
    pub low_stock: bool,
    pub out_of_stock: bool,
    pub days_until_low_stock: Option<i64>,
    pub days_until_out_of_stock: Option<i64>,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReorderStatus {
    Ordered,
    Received,
}

pub type MedicationResponse = super::DataResponse<Medication>;

pub type MedicationCollectionResponse = super::PaginatedResponse<Medication>;
