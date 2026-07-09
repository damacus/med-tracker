use crate::client::ApiClient;
use crate::error::ApiError;
use crate::models::Capabilities;

impl ApiClient {
    pub async fn capabilities(&self) -> Result<Capabilities, ApiError> {
        self.get_data("/api/v1/capabilities").await
    }

    pub async fn require_capability(
        &self,
        feature: impl Into<String>,
        path: &[&str],
    ) -> Result<Capabilities, ApiError> {
        let capabilities = self.capabilities().await?;
        if capabilities.supports_path(path) {
            Ok(capabilities)
        } else {
            Err(ApiError::UnsupportedByServer {
                feature: feature.into(),
            })
        }
    }
}
