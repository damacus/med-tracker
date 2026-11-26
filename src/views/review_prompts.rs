use super::DataResponse;
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct ReviewPrompt {
    pub id: String,
    pub person_id: String,
    pub primary_medication_id: String,
    pub interacting_medication_id: String,
    pub evidence_record_id: String,
    pub primary_medication_name: String,
    pub interacting_medication_name: String,
    pub evidence_source_name: String,
    pub evidence_source_url: String,
    pub evidence_source_version: String,
    pub matched_term: String,
    pub match_type: String,
    pub source_instruction: String,
    pub match_reason: String,
    pub evidence_text: String,
    pub etag: String,
    pub evidence_source_checked_on: chrono::NaiveDate,
    pub evidence_source_effective_on: chrono::NaiveDate,
    pub risk_level: String,
    pub match_confidence: String,
    pub status: String,
    pub practitioner_name: Option<String>,
    pub practitioner_role: Option<String>,
    pub review_note: Option<String>,
    pub reviewed_on: Option<chrono::NaiveDate>,
    pub reviewed_by_membership_id: Option<String>,
    pub updated_at: String,
}

#[derive(Serialize)]
pub(crate) struct ReviewCounts {
    pub needs_review: u64,
    pub reviewed: u64,
    pub all: u64,
}

#[derive(Serialize)]
pub(crate) struct ReviewMeta {
    pub page: u64,
    pub per_page: u64,
    pub total_count: u64,
    pub hidden_count: u64,
    pub review_counts: ReviewCounts,
    pub active_status: String,
    pub active_priority: String,
    pub show_hidden: bool,
}

#[derive(Serialize)]
pub(crate) struct ReviewCollection {
    pub data: Vec<ReviewPrompt>,
    pub meta: ReviewMeta,
}

#[derive(Serialize)]
pub(crate) struct ReviewDetail {
    pub data: ReviewPrompt,
    pub history: Vec<ReviewHistory>,
}

#[derive(Serialize)]
pub(crate) struct ReviewHistory {
    pub event_type: String,
    pub snapshot: serde_json::Value,
    pub created_at: String,
}

#[derive(Serialize)]
pub(crate) struct EditLease {
    pub token: String,
    pub etag: String,
    pub expires_at: String,
}

#[derive(Serialize)]
pub(crate) struct RenewedLease {
    pub expires_at: String,
}

#[derive(Serialize)]
pub(crate) struct ReleasedLease {
    pub released: bool,
}

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum LeaseResponse {
    Acquired(DataResponse<EditLease>),
    Renewed(DataResponse<RenewedLease>),
    Released(DataResponse<ReleasedLease>),
}
