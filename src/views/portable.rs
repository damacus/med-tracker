use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Serialize)]
pub(crate) struct ExportStatus {
    pub id: String,
    pub status: String,
}

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum ExportRead {
    Status(ExportStatus),
    Artifact(Value),
}

#[derive(Serialize)]
pub(crate) struct ImportResult {
    pub applied: bool,
    pub counts: Map<String, Value>,
    pub conflicts: Vec<ImportConflict>,
    pub errors: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct ImportConflict {
    pub record_type: &'static str,
    pub portable_id: String,
    pub field: &'static str,
    pub existing_portable_id: String,
}
