use serde_json::Value;

use crate::client::ApiClient;
use crate::error::ApiError;
use crate::models::{BatchMutationPayload, BatchMutationRequest};

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
        let path = format!("/api/v1/households/{household_id}/sync/changes");
        match since {
            Some(cursor) => self.get_data_with_query(&path, &[("cursor", cursor)]).await,
            None => self.get_data(&path).await,
        }
    }

    pub async fn sync_batch(
        &self,
        household_id: &str,
        operations: Vec<Value>,
    ) -> Result<Value, ApiError> {
        self.require_capability("sync batch mutations", &["sync", "batch_mutations"])
            .await?;
        let request = BatchMutationRequest {
            batch: BatchMutationPayload { operations },
        };
        self.post_json_data(
            &format!("/api/v1/households/{household_id}/sync/batches"),
            &request,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    #[tokio::test]
    async fn sync_changes_sends_the_rails_cursor_parameter() {
        let server = MockServer::start().await;
        mount_sync_capabilities(&server).await;
        Mock::given(method("GET"))
            .and(path("/api/v1/households/household-1/sync/changes"))
            .and(query_param("cursor", "2026-07-14T12:00:00+05:00"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": { "cursor": "next", "changes": [], "tombstones": [] }
            })))
            .mount(&server)
            .await;

        let client = ApiClient::new(server.uri(), Some("test-token".to_string())).unwrap();
        let result = client
            .sync_changes("household-1", Some("2026-07-14T12:00:00+05:00"))
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn sync_batch_sends_the_rails_batch_envelope() {
        let server = MockServer::start().await;
        mount_sync_capabilities(&server).await;
        let operations = vec![json!({
            "action": "delete",
            "resource_type": "medications",
            "id": "medication-1"
        })];
        Mock::given(method("POST"))
            .and(path("/api/v1/households/household-1/sync/batches"))
            .and(body_json(json!({ "batch": { "operations": operations } })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": { "results": [] }
            })))
            .mount(&server)
            .await;

        let client = ApiClient::new(server.uri(), Some("test-token".to_string())).unwrap();
        let result = client.sync_batch("household-1", operations).await;

        assert!(result.is_ok());
    }

    async fn mount_sync_capabilities(server: &MockServer) {
        Mock::given(method("GET"))
            .and(path("/api/v1/capabilities"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": {
                    "format": "medtracker.api.capabilities.v1",
                    "api_version": "v1",
                    "client_tools": {
                        "cli": { "supported": true },
                        "mcp_server": { "supported": true }
                    },
                    "sync": {
                        "change_feed": true,
                        "batch_mutations": true
                    }
                }
            })))
            .mount(server)
            .await;
    }
}
