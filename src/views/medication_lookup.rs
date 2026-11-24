use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct Lookup {
    pub results: Vec<Product>,
    pub permissions: Permissions,
    #[serde(flatten)]
    pub details: Option<Details>,
}

#[derive(Serialize)]
pub(crate) struct Permissions {
    pub can_create: bool,
    pub can_update: bool,
}

#[derive(Serialize)]
pub(crate) struct Details {
    pub query: String,
    pub barcode: Option<String>,
    pub barcode_resolution: Option<BarcodeResolution>,
    pub form: Option<String>,
    pub strength: Option<String>,
    pub review_guidance: ReviewGuidance,
}

#[derive(Serialize)]
pub(crate) struct BarcodeResolution {
    pub status: &'static str,
    pub source: String,
}

#[derive(Serialize)]
pub(crate) struct ReviewGuidance {
    pub status: &'static str,
}

#[derive(Default, Serialize)]
pub(crate) struct Product {
    pub barcode: Option<String>,
    pub code: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub display: String,
    pub system: Option<String>,
    pub concept_class: Option<String>,
    pub category: Option<String>,
    pub trade_family: Option<TradeFamily>,
    pub trade_family_group: Option<TradeFamily>,
    pub package_size: Option<String>,
    pub package_quantity: Option<String>,
    pub package_unit: Option<String>,
    pub directions: Option<String>,
    pub warnings: Option<String>,
    pub pil_url: Option<String>,
    pub spc_url: Option<String>,
    pub concept_class_label: Option<String>,
    pub source_label: Option<String>,
    pub match_reason: Option<String>,
    pub match_reason_label: Option<String>,
    pub related_medications: Vec<StockMatch>,
    pub review_prompts: Vec<ReviewPrompt>,
    pub review_prompt_filter: ReviewPromptFilter,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub existing_medication: Option<ExistingMedication>,
}

#[derive(Clone, Serialize)]
pub(crate) struct TradeFamily {
    pub code: String,
    pub name: Option<String>,
}

#[derive(Clone, Serialize)]
pub(crate) struct StockMatch {
    pub id: i64,
    pub name: String,
    pub location: String,
    pub path: String,
    pub current_supply: String,
}

#[derive(Serialize)]
pub(crate) struct ExistingMedication {
    #[serde(flatten)]
    pub medication: StockMatch,
    pub refill_path: String,
}

#[derive(Serialize)]
pub(crate) struct ReviewPrompt {
    pub evidence_record_id: i64,
    pub risk_level: String,
    pub risk_level_label: String,
    pub match_confidence: String,
    pub match_confidence_label: String,
    pub matched_term: String,
    pub match_type: &'static str,
    pub source_instruction: &'static str,
    pub match_reason: String,
    pub interacting_medication_name: String,
    pub description: &'static str,
    pub source_name: String,
    pub source_checked_on: chrono::NaiveDate,
    pub source_version: Option<String>,
    pub source_effective_on: Option<chrono::NaiveDate>,
    pub source_url: String,
    pub evidence_text: String,
}

#[derive(Default, Serialize)]
pub(crate) struct ReviewPromptFilter {
    pub hidden_count: usize,
}

#[derive(Serialize)]
pub(crate) struct LookupUnavailable {
    pub results: [(); 0],
    pub error: &'static str,
}
