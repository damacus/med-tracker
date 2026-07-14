use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize, Serialize)]
pub struct DataEnvelope<T> {
    pub data: T,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginRequest<'a> {
    pub email: &'a str,
    pub password: &'a str,
    pub device_name: &'a str,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RefreshRequest<'a> {
    pub refresh_token: &'a str,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PortableImportRequest {
    pub data: Value,
    pub mode: PortableImportMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passphrase: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PortableImportMode {
    DryRun,
    Apply,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BatchMutationRequest {
    pub batch: BatchMutationPayload,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BatchMutationPayload {
    pub operations: Vec<Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Capabilities {
    pub format: String,
    pub api_version: String,
    pub client_tools: ClientTools,
    #[serde(default)]
    pub backups: Value,
    #[serde(default)]
    pub sync: Value,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClientTools {
    pub cli: ToolCapability,
    pub mcp_server: ToolCapability,
    #[serde(default)]
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ToolCapability {
    pub supported: bool,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(flatten)]
    pub extra: Value,
}

impl Capabilities {
    pub fn supports_path(&self, path: &[&str]) -> bool {
        let mut value = match path.first().copied() {
            Some("backups") => &self.backups,
            Some("sync") => &self.sync,
            _ => return false,
        };

        for segment in &path[1..] {
            value = match value.get(*segment) {
                Some(next) => next,
                None => return false,
            };
        }

        value.as_bool().unwrap_or(false)
    }
}
