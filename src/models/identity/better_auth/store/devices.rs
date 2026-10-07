use async_trait::async_trait;
use better_auth_core::{AuthError, AuthResult, CreateDeviceCode, DeviceCode, UpdateDeviceCode, store::DeviceCodeStore};
use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, FromQueryResult};

use super::{ClinicalStore, clinical_id, context, database_error, statement};

#[derive(FromQueryResult)]
struct DeviceRow {
    id: String,
    device_code: String,
    user_code: String,
    account_id: Option<i64>,
    expires_at: DateTime<Utc>,
    status: String,
    last_polled_at: Option<DateTime<Utc>>,
    polling_interval: Option<i64>,
    client_id: Option<String>,
    scope: Option<String>,
}

impl From<DeviceRow> for DeviceCode {
    fn from(row: DeviceRow) -> Self {
        Self { id: row.id, device_code: row.device_code, user_code: row.user_code, user_id: row.account_id.map(|id| id.to_string()), expires_at: row.expires_at, status: row.status, last_polled_at: row.last_polled_at, polling_interval: row.polling_interval, client_id: row.client_id, scope: row.scope }
    }
}

impl ClinicalStore {
    async fn device_update(&self, id: &str, expected: Option<&str>, update: UpdateDeviceCode) -> AuthResult<Option<DeviceCode>> {
        let transaction = self.transaction().await?;
        context(&*transaction, "med_tracker.identity_device_id", id).await?;
        let change_user = update.user_id.is_some();
        let user_id = update.user_id.flatten().as_deref().map(clinical_id).transpose()?;
        let change_poll = update.last_polled_at.is_some();
        let row = DeviceRow::find_by_statement(statement("UPDATE public.identity_device_codes SET status=COALESCE($3,status),account_id=CASE WHEN $4 THEN $5 ELSE account_id END,last_polled_at=CASE WHEN $6 THEN $7 ELSE last_polled_at END WHERE id=$1 AND ($2::text IS NULL OR status=$2) RETURNING *", [id.into(), expected.into(), update.status.into(), change_user.into(), user_id.into(), change_poll.into(), update.last_polled_at.flatten().into()])).one(&*transaction).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        Ok(row.map(Into::into))
    }
}

#[async_trait]
impl DeviceCodeStore for ClinicalStore {
    async fn create_device_code(&self, input: CreateDeviceCode) -> AuthResult<DeviceCode> {
        let transaction = self.transaction().await?;
        context(&*transaction, "med_tracker.identity_device_code", &input.device_code).await?;
        let user_id = input.user_id.as_deref().map(clinical_id).transpose()?;
        let row = DeviceRow::find_by_statement(statement("INSERT INTO public.identity_device_codes(id,device_code,user_code,account_id,expires_at,status,last_polled_at,polling_interval,client_id,scope) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) RETURNING *", [uuid::Uuid::new_v4().to_string().into(), input.device_code.into(), input.user_code.into(), user_id.into(), input.expires_at.into(), input.status.into(), input.last_polled_at.into(), input.polling_interval.into(), input.client_id.into(), input.scope.into()])).one(&*transaction).await.map_err(database_error)?.ok_or_else(|| AuthError::internal("Device request was not stored"))?;
        transaction.commit().await.map_err(database_error)?;
        Ok(row.into())
    }

    async fn get_device_code_by_device_code(&self, code: &str) -> AuthResult<Option<DeviceCode>> {
        let transaction = self.transaction().await?;
        context(&*transaction, "med_tracker.identity_device_code", code).await?;
        let row = DeviceRow::find_by_statement(statement("SELECT * FROM public.identity_device_codes WHERE device_code=$1", [code.into()])).one(&*transaction).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        Ok(row.map(Into::into))
    }

    async fn get_device_code_by_user_code(&self, code: &str) -> AuthResult<Option<DeviceCode>> {
        let transaction = self.transaction().await?;
        context(&*transaction, "med_tracker.identity_device_user_code", code).await?;
        let row = DeviceRow::find_by_statement(statement("SELECT * FROM public.identity_device_codes WHERE user_code=$1", [code.into()])).one(&*transaction).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        Ok(row.map(Into::into))
    }

    async fn update_device_code(&self, id: &str, update: UpdateDeviceCode) -> AuthResult<DeviceCode> {
        self.device_update(id, None, update).await?.ok_or_else(|| AuthError::not_found("Device request not found"))
    }

    async fn update_device_code_if_status(&self, id: &str, expected: &str, update: UpdateDeviceCode) -> AuthResult<bool> {
        Ok(self.device_update(id, Some(expected), update).await?.is_some())
    }

    async fn claim_device_code(&self, id: &str, user_id: &str) -> AuthResult<bool> {
        let account_id = clinical_id(user_id)?;
        let transaction = self.transaction().await?;
        context(&*transaction, "med_tracker.identity_device_id", id).await?;
        context(&*transaction, "med_tracker.current_account_id", user_id).await?;
        let account = transaction.query_one_raw(statement("SELECT id FROM public.accounts WHERE id=$1 AND status=2 FOR UPDATE", [account_id.into()])).await.map_err(database_error)?;
        if account.is_none() { return Err(AuthError::Unauthenticated); }
        let changed = transaction.execute_raw(statement("UPDATE public.identity_device_codes SET account_id=$2 WHERE id=$1 AND account_id IS NULL AND status='pending' AND expires_at>CURRENT_TIMESTAMP", [id.into(), account_id.into()])).await.map_err(database_error)?.rows_affected() == 1;
        transaction.commit().await.map_err(database_error)?;
        Ok(changed)
    }

    async fn delete_device_code(&self, id: &str) -> AuthResult<()> {
        let transaction = self.transaction().await?;
        context(&*transaction, "med_tracker.identity_device_id", id).await?;
        transaction.execute_raw(statement("DELETE FROM public.identity_device_codes WHERE id=$1", [id.into()])).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)
    }

    async fn delete_device_code_if_status(&self, id: &str, status: &str) -> AuthResult<bool> {
        let transaction = self.transaction().await?;
        context(&*transaction, "med_tracker.identity_device_id", id).await?;
        let removed = transaction.execute_raw(statement("DELETE FROM public.identity_device_codes WHERE id=$1 AND status=$2", [id.into(), status.into()])).await.map_err(database_error)?.rows_affected() == 1;
        transaction.commit().await.map_err(database_error)?;
        Ok(removed)
    }
}
