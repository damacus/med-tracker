use async_trait::async_trait;
use better_auth_core::{
    AuthError, AuthResult, CreateDeviceCode, DeviceCode, DeviceCodeOwnership, FieldMap, FieldValue,
    SchemaValue, UpdateDeviceCode,
    store::{DeviceCodeStore, JwksStore, WalletStore},
};

use super::ClinicalStore;

#[async_trait]
impl DeviceCodeStore for ClinicalStore {
    async fn create_device_code_record(&self, _fields: FieldMap) -> AuthResult<Option<FieldMap>> {
        Err(AuthError::config("Device authorization is not configured"))
    }

    async fn get_device_code_record(
        &self,
        _id: &SchemaValue<String>,
    ) -> AuthResult<Option<FieldMap>> {
        Err(AuthError::config("Device authorization is not configured"))
    }

    async fn update_device_code_record(
        &self,
        _id: &SchemaValue<String>,
        _fields: FieldMap,
    ) -> AuthResult<Option<FieldMap>> {
        Err(AuthError::config("Device authorization is not configured"))
    }

    async fn create_device_code(&self, _input: CreateDeviceCode) -> AuthResult<DeviceCode> {
        Err(AuthError::config("Device authorization is not configured"))
    }

    async fn get_device_code_by_device_code(&self, _code: &str) -> AuthResult<Option<DeviceCode>> {
        Err(AuthError::config("Device authorization is not configured"))
    }

    async fn get_device_code_by_user_code(&self, _code: &str) -> AuthResult<Option<DeviceCode>> {
        Err(AuthError::config("Device authorization is not configured"))
    }

    async fn update_device_code(
        &self,
        _id: &SchemaValue<String>,
        _update: UpdateDeviceCode,
    ) -> AuthResult<DeviceCode> {
        Err(AuthError::config("Device authorization is not configured"))
    }

    async fn update_device_code_if_status(
        &self,
        _id: &SchemaValue<String>,
        _expected: &str,
        _update: UpdateDeviceCode,
    ) -> AuthResult<bool> {
        Err(AuthError::config("Device authorization is not configured"))
    }

    async fn claim_device_code(
        &self,
        _id: &SchemaValue<String>,
        _user_id: &SchemaValue<String>,
    ) -> AuthResult<bool> {
        Err(AuthError::config("Device authorization is not configured"))
    }

    async fn consume_device_code(
        &self,
        _expected: &DeviceCode,
        _ownership: &DeviceCodeOwnership,
    ) -> AuthResult<Option<DeviceCode>> {
        Err(AuthError::config("Device authorization is not configured"))
    }

    async fn delete_device_code(&self, _id: &SchemaValue<String>) -> AuthResult<()> {
        Err(AuthError::config("Device authorization is not configured"))
    }

    async fn delete_device_code_if_status(
        &self,
        _id: &SchemaValue<String>,
        _status: &str,
    ) -> AuthResult<bool> {
        Err(AuthError::config("Device authorization is not configured"))
    }
}

#[async_trait]
impl WalletStore for ClinicalStore {
    async fn create_wallet_address_record(&self, _input: FieldMap) -> AuthResult<Option<FieldMap>> {
        Err(AuthError::config("Wallet authentication is not configured"))
    }
    async fn get_wallet_address_record(
        &self,
        _id: &SchemaValue<String>,
    ) -> AuthResult<Option<FieldMap>> {
        Err(AuthError::config("Wallet authentication is not configured"))
    }
    async fn update_wallet_address_record(
        &self,
        _id: &SchemaValue<String>,
        _input: FieldMap,
    ) -> AuthResult<Option<FieldMap>> {
        Err(AuthError::config("Wallet authentication is not configured"))
    }
    async fn delete_wallet_address_record(&self, _id: &SchemaValue<String>) -> AuthResult<()> {
        Err(AuthError::config("Wallet authentication is not configured"))
    }
    async fn get_wallet_address_value(
        &self,
        _address: &FieldValue,
        _chain_id: Option<&FieldValue>,
    ) -> AuthResult<Option<better_auth_core::WalletAddress>> {
        Err(AuthError::config("Wallet authentication is not configured"))
    }
}

#[async_trait]
impl JwksStore for ClinicalStore {
    async fn create_jwk_record(&self, _input: FieldMap) -> AuthResult<Option<FieldMap>> {
        Err(AuthError::config("JWT signing keys are not configured"))
    }
    async fn get_jwk_record(&self, _id: &SchemaValue<String>) -> AuthResult<Option<FieldMap>> {
        Err(AuthError::config("JWT signing keys are not configured"))
    }
    async fn update_jwk_record(
        &self,
        _id: &SchemaValue<String>,
        _input: FieldMap,
    ) -> AuthResult<Option<FieldMap>> {
        Err(AuthError::config("JWT signing keys are not configured"))
    }
    async fn delete_jwk_record(&self, _id: &SchemaValue<String>) -> AuthResult<()> {
        Err(AuthError::config("JWT signing keys are not configured"))
    }
    async fn list_jwk_records(&self) -> AuthResult<Vec<FieldMap>> {
        Err(AuthError::config("JWT signing keys are not configured"))
    }
}
