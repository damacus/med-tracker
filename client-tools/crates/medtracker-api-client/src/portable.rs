use serde_json::Value;

use crate::client::ApiClient;
use crate::error::ApiError;
use crate::models::{PortableImportMode, PortableImportRequest};

impl ApiClient {
    pub async fn portable_export(&self, household_id: &str) -> Result<Value, ApiError> {
        self.get_data(&format!(
            "/api/v1/households/{household_id}/portable_export"
        ))
        .await
    }

    pub async fn portable_import(
        &self,
        household_id: &str,
        data: Value,
        passphrase: Option<String>,
        mode: PortableImportMode,
    ) -> Result<Value, ApiError> {
        let endpoint = match mode {
            PortableImportMode::DryRun => {
                format!("/api/v1/households/{household_id}/portable_imports/dry_run")
            }
            PortableImportMode::Apply => {
                format!("/api/v1/households/{household_id}/portable_imports")
            }
        };
        let request = PortableImportRequest {
            data,
            mode,
            passphrase,
        };

        self.post_json_data(&endpoint, &request).await
    }
}
