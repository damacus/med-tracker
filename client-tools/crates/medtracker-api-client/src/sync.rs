use serde_json::Value;

use crate::client::ApiClient;
use crate::error::ApiError;
use crate::models::BatchMutationRequest;

impl ApiClient {
    pub async fn sync_snapshot(&self, household_id: &str) -> Result<Value, ApiError> {
        self.require_capability("sync snapshot", &["sync", "mobile_snapshot"])
            .await?;
        self.get_data(&format!("/api/v1/households/{household_id}/sync/snapshot"))
            .await
    }

    pub async fn sync_changes(
        &self,
        household_id: &str,
        since: Option<&str>,
    ) -> Result<Value, ApiError> {
        self.require_capability("sync change feed", &["sync", "change_feed"])
            .await?;
        let suffix = since.map_or_else(String::new, |value| format!("?since={value}"));
        self.get_data(&format!(
            "/api/v1/households/{household_id}/sync/changes{suffix}"
        ))
        .await
    }

    pub async fn sync_batch(
        &self,
        household_id: &str,
        operations: Vec<Value>,
    ) -> Result<Value, ApiError> {
        self.require_capability("sync batch mutations", &["sync", "batch_mutations"])
            .await?;
        let request = BatchMutationRequest { operations };
        self.post_json_data(
            &format!("/api/v1/households/{household_id}/sync/batches"),
            &request,
        )
        .await
    }
}
