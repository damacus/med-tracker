use serde::Serialize;

#[derive(Serialize)]
pub struct AuditEntry {
    pub id: i64,
    pub event_type: String,
    pub actor_account_id: Option<i64>,
    pub actor_membership_id: Option<i64>,
    pub request_id: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: String,
}
