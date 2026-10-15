use crate::client::ApiClient;
use crate::error::ApiError;
use crate::models::Capabilities;

impl ApiClient {
    #[doc = "# Errors\nThe capability request fails, is rejected, or cannot be decoded."]
    pub async fn capabilities(&self) -> Result<Capabilities, ApiError> {
        self.get_data("/api/v1/capabilities").await
    }

    #[doc = "# Errors\nCapability discovery fails or the server does not support the requested feature."]
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
