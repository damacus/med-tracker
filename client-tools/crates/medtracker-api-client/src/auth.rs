use serde_json::Value;

use crate::client::ApiClient;
use crate::error::ApiError;

impl ApiClient {
    pub async fn logout(&self) -> Result<(), ApiError> {
        self.delete_empty("/api/v1/auth/logout").await
    }

    pub async fn households(&self) -> Result<Value, ApiError> {
        self.get_data("/api/v1/auth/households").await
    }
}
