use serde_json::Value;

use crate::client::ApiClient;
use crate::error::ApiError;

impl ApiClient {
    pub async fn backup_export(&self, household_id: &str, mode: &str) -> Result<Value, ApiError> {
        self.require_capability("backup export", &["backups", "health_data_json"])
            .await?;
        self.get_data(&format!(
            "/api/v1/households/{household_id}/data_exports/{mode}"
        ))
        .await
    }
}
