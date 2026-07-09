use serde_json::Value;

use crate::client::ApiClient;
use crate::error::ApiError;
use crate::models::{LoginRequest, RefreshRequest};

impl ApiClient {
    pub async fn login(
        &self,
        email: &str,
        password: &str,
        device_name: &str,
    ) -> Result<Value, ApiError> {
        let request = LoginRequest {
            email,
            password,
            device_name,
        };

        self.post_json_data("/api/v1/auth/login", &request).await
    }

    pub async fn refresh(&self, refresh_token: &str) -> Result<Value, ApiError> {
        let request = RefreshRequest { refresh_token };

        self.post_json_data("/api/v1/auth/refresh", &request).await
    }

    pub async fn logout(&self) -> Result<(), ApiError> {
        self.delete_empty("/api/v1/auth/logout").await
    }

    pub async fn households(&self) -> Result<Value, ApiError> {
        self.get_data("/api/v1/auth/households").await
    }
}
