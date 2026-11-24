use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct AppliedBatch {
    pub applied: bool,
    pub results: Vec<BatchOperation>,
}

#[derive(Serialize)]
pub(crate) struct BatchOperation {
    #[serde(flatten)]
    pub result: SyncResult,
    pub index: usize,
    pub action: String,
}

#[derive(Serialize)]
pub(crate) struct SyncResult {
    pub record_type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_portable_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replayed: Option<bool>,
}

#[derive(Serialize)]
pub(crate) struct Changes {
    pub cursor: String,
    pub changes: Vec<Change>,
    pub tombstones: Vec<Tombstone>,
}

#[derive(Serialize)]
pub(crate) struct Change {
    pub id: i64,
    pub record_type: String,
    pub record_id: i64,
    pub record_portable_id: String,
    pub action: String,
    pub occurred_at: String,
    pub metadata: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record: Option<serde_json::Value>,
}

#[derive(Serialize)]
pub(crate) struct Tombstone {
    pub id: i64,
    pub record_type: String,
    pub record_portable_id: String,
    pub action: String,
    pub deleted_at: String,
    pub metadata: serde_json::Value,
}
