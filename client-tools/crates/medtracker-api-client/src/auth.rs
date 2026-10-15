use serde_json::Value;

use crate::client::ApiClient;
use crate::error::ApiError;

impl ApiClient {
    #[doc = "# Errors\nThe logout request fails or the server rejects it."]
    pub async fn logout(&self) -> Result<(), ApiError> {
        self.delete_empty("/api/v1/auth/logout").await
    }

    #[doc = "# Errors\nThe households request fails, is rejected, or cannot be decoded."]
    pub async fn households(&self) -> Result<Value, ApiError> {
        self.get_data("/api/v1/auth/households").await
    }
}
