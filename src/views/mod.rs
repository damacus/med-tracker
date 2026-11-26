//! Response types for the MedTracker API.
//!
//! Keep fixed API fields typed through the HTTP response boundary. Preserve
//! the OpenAPI field types, nullability, envelopes and headers.
//!
//! ETags retain their existing JSON encoding and key order. Idempotency
//! storage retains the original wire body for replay after current access
//! checks. Validation errors can contain variable fields. The response
//! boundary adds trace IDs to error bodies in their existing locations.
//!
//! Versioned portable documents and stored export artifacts retain their
//! JSON formats. Stored audit and review snapshots, fields from stock-removal
//! audit history, and user-defined schedule configuration also retain their
//! JSON representation. The OAuth library supplies its token and protocol
//! error bodies. Browser controllers can convert typed projections to JSON
//! for Tera templates or Finder-only fields.

pub(crate) mod account_sessions;
pub mod administration;
pub(crate) mod ai_suggestions;
pub mod audit_logs;
pub mod capabilities;
pub mod dosages;
pub mod dose_occurrences;
pub(crate) mod guardian_declarations;
pub mod health_events;
pub mod invitations;
pub mod locations;
pub(crate) mod medication_lookup;
pub mod medication_takes;
pub mod medications;
pub(crate) mod oauth;
pub mod people;
pub(crate) mod portable;
pub(crate) mod profile;
pub(crate) mod profile_security;
pub mod rate_limit;
pub(crate) mod review_prompts;
pub(crate) mod sync;
pub mod treatments;

#[derive(serde::Serialize)]
pub struct PaginationMeta {
    pub page: i64,
    pub per_page: i64,
    pub total_count: u64,
}

#[derive(Debug, PartialEq, serde::Serialize)]
pub struct DataResponse<T> {
    pub data: T,
}

#[derive(serde::Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub meta: PaginationMeta,
}

pub(crate) fn canonical_etag(
    response: &impl serde::Serialize,
) -> Result<String, crate::models::errors::OperationError> {
    use sha2::{Digest, Sha256};

    let mut canonical = serde_json::to_value(response)
        .map_err(|_| crate::models::errors::OperationError::Unavailable)?;
    canonical.sort_all_objects();
    Ok(format!(
        "\"{}\"",
        hex::encode(Sha256::digest(canonical.to_string().as_bytes()))
    ))
}
