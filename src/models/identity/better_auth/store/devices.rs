use async_trait::async_trait;
use better_auth_core::{
    AuthError, AuthRecordFields, AuthResult, CreateDeviceCode, DeviceCode, DeviceCodeOwnership,
    FieldMap, FieldValue, FromFieldMap, SchemaValue, UpdateDeviceCode,
    store::{DeviceCodeStore, JwksStore, WalletStore},
};
use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, FromQueryResult};

use super::{ClinicalStore, clinical_id, clinical_schema_id, context, database_error, statement};

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
        let bindings = FieldMap::from([
            ("id".into(), row.id.clone().into()),
            ("deviceCode".into(), row.device_code.clone().into()),
            (
                "clientId".into(),
                row.client_id
                    .clone()
                    .map(FieldValue::from)
                    .unwrap_or(FieldValue::Null),
            ),
            (
                "userId".into(),
                row.account_id
                    .map(|id| id.to_string())
                    .map(FieldValue::from)
                    .unwrap_or(FieldValue::Null),
            ),
            ("status".into(), row.status.clone().into()),
        ]);
        Self {
            field_order: Vec::new(),
            additional_fields: FieldMap::new(),
            id: row.id.into(),
            device_code: row.device_code.into(),
            user_code: row.user_code.into(),
            user_id: row.account_id.map(|id| id.to_string()).into(),
            expires_at: row.expires_at.into(),
            status: row.status.into(),
            last_polled_at: row.last_polled_at.map(Into::into).into(),
            polling_interval: row.polling_interval.map(|value| value as f64).into(),
            client_id: row.client_id.into(),
            scope: row.scope.into(),
            ..Self::default()
        }
        .with_storage_bindings(bindings)
    }
}

impl ClinicalStore {
    async fn device_update(
        &self,
        id: &str,
        expected: Option<&str>,
        update: UpdateDeviceCode,
    ) -> AuthResult<Option<DeviceCode>> {
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.identity_device_id", id).await?;
        if !update.additional_fields.is_empty() {
            return Err(AuthError::config(
                "Device-code extension fields are not configured",
            ));
        }
        let change_user = update.user_id.is_some();
        let user_id = update
            .user_id
            .as_ref()
            .map(SchemaValue::typed)
            .transpose()?
            .cloned()
            .flatten()
            .as_deref()
            .map(clinical_id)
            .transpose()?;
        let change_poll = update.last_polled_at.is_some();
        let poll = update
            .last_polled_at
            .flatten()
            .map(|value| {
                value.to_datetime().and_then(|value| {
                    value.ok_or_else(|| AuthError::bad_request("Invalid poll timestamp"))
                })
            })
            .transpose()?;
        let change_scope = !update.scope.is_undefined();
        let scope = if change_scope {
            update.scope.typed()?.clone()
        } else {
            None
        };
        let row = DeviceRow::find_by_statement(statement("UPDATE public.identity_device_codes SET status=COALESCE($3,status),account_id=CASE WHEN $4 THEN $5 ELSE account_id END,last_polled_at=CASE WHEN $6 THEN $7 ELSE last_polled_at END,scope=CASE WHEN $8 THEN $9 ELSE scope END WHERE id=$1 AND ($2::text IS NULL OR status=$2) AND ($3::text IS NULL OR expires_at>CURRENT_TIMESTAMP) AND (NOT $4 OR account_id IS NOT DISTINCT FROM $5) RETURNING *", [id.into(), expected.into(), update.status.into(), change_user.into(), user_id.into(), change_poll.into(), poll.into(), change_scope.into(), scope.into()])).one(&*transaction).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        Ok(row.map(Into::into))
    }
}

#[async_trait]
impl DeviceCodeStore for ClinicalStore {
    async fn create_device_code_record(&self, fields: FieldMap) -> AuthResult<Option<FieldMap>> {
        let record = DeviceCode::from_field_values(fields)?;
        self.create_device_code(CreateDeviceCode {
            additional_fields: record.additional_fields,
            device_code: record.device_code.typed()?.clone(),
            user_code: record.user_code.typed()?.clone(),
            user_id: record.user_id.typed()?.clone(),
            expires_at: record.expires_at.typed()?.clone(),
            status: record.status.typed()?.clone(),
            last_polled_at: record.last_polled_at.typed()?.clone(),
            polling_interval: record.polling_interval.typed()?.clone(),
            client_id: record.client_id.typed()?.clone(),
            scope: record.scope,
        })
        .await?
        .field_values()
        .map(Some)
    }

    async fn get_device_code_record(
        &self,
        id: &SchemaValue<String>,
    ) -> AuthResult<Option<FieldMap>> {
        let id = id.typed()?;
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.identity_device_id", id).await?;
        let row = DeviceRow::find_by_statement(statement(
            "SELECT * FROM public.identity_device_codes WHERE id=$1",
            [id.into()],
        ))
        .one(&*transaction)
        .await
        .map_err(database_error)?;
        let result = row
            .map(DeviceCode::from)
            .map(|row| row.field_values())
            .transpose()?;
        transaction.commit().await.map_err(database_error)?;
        Ok(result)
    }

    async fn update_device_code_record(
        &self,
        id: &SchemaValue<String>,
        fields: FieldMap,
    ) -> AuthResult<Option<FieldMap>> {
        let record = DeviceCode::from_field_values(fields)?;
        self.device_update(
            id.typed()?,
            None,
            UpdateDeviceCode {
                additional_fields: record.additional_fields,
                status: record.status.typed().ok().cloned(),
                user_id: if record.user_id.is_undefined() {
                    None
                } else {
                    Some(record.user_id)
                },
                last_polled_at: if record.last_polled_at.is_undefined() {
                    None
                } else {
                    Some(record.last_polled_at.typed()?.clone())
                },
                scope: record.scope,
            },
        )
        .await?
        .map(|row| row.field_values())
        .transpose()
    }

    async fn create_device_code(&self, input: CreateDeviceCode) -> AuthResult<DeviceCode> {
        if !input.additional_fields.is_empty() {
            return Err(AuthError::config(
                "Device-code extension fields are not configured",
            ));
        }
        let transaction = self.transaction().await?;
        context(
            &transaction,
            "med_tracker.identity_device_code",
            &input.device_code,
        )
        .await?;
        let user_id = input.user_id.as_deref().map(clinical_id).transpose()?;
        let expiry = input
            .expires_at
            .to_datetime()?
            .ok_or_else(|| AuthError::bad_request("Invalid device expiry"))?;
        let poll = input
            .last_polled_at
            .map(|value| {
                value.to_datetime().and_then(|value| {
                    value.ok_or_else(|| AuthError::bad_request("Invalid poll timestamp"))
                })
            })
            .transpose()?;
        let interval = input
            .polling_interval
            .map(|value| {
                if value >= 0.0 && value.fract() == 0.0 {
                    Ok(value as i64)
                } else {
                    Err(AuthError::bad_request("Invalid polling interval"))
                }
            })
            .transpose()?;
        let row = DeviceRow::find_by_statement(statement("INSERT INTO public.identity_device_codes(id,device_code,user_code,account_id,expires_at,status,last_polled_at,polling_interval,client_id,scope) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) RETURNING *", [uuid::Uuid::new_v4().to_string().into(), input.device_code.into(), input.user_code.into(), user_id.into(), expiry.into(), input.status.into(), poll.into(), interval.into(), input.client_id.into(), input.scope.typed()?.clone().into()])).one(&*transaction).await.map_err(database_error)?.ok_or_else(|| AuthError::internal("Device request was not stored"))?;
        transaction.commit().await.map_err(database_error)?;
        Ok(row.into())
    }

    async fn get_device_code_by_device_code(&self, code: &str) -> AuthResult<Option<DeviceCode>> {
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.identity_device_code", code).await?;
        let row = DeviceRow::find_by_statement(statement(
            "SELECT * FROM public.identity_device_codes WHERE device_code=$1",
            [code.into()],
        ))
        .one(&*transaction)
        .await
        .map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        Ok(row.map(Into::into))
    }

    async fn get_device_code_by_user_code(&self, code: &str) -> AuthResult<Option<DeviceCode>> {
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.identity_device_user_code", code).await?;
        let row = DeviceRow::find_by_statement(statement(
            "SELECT * FROM public.identity_device_codes WHERE user_code=$1",
            [code.into()],
        ))
        .one(&*transaction)
        .await
        .map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        Ok(row.map(Into::into))
    }

    async fn update_device_code(
        &self,
        id: &SchemaValue<String>,
        update: UpdateDeviceCode,
    ) -> AuthResult<DeviceCode> {
        self.device_update(id.typed()?, None, update)
            .await?
            .ok_or_else(|| AuthError::not_found("Device request not found"))
    }

    async fn update_device_code_if_status(
        &self,
        id: &SchemaValue<String>,
        expected: &str,
        update: UpdateDeviceCode,
    ) -> AuthResult<bool> {
        Ok(self
            .device_update(id.typed()?, Some(expected), update)
            .await?
            .is_some())
    }

    async fn claim_device_code(
        &self,
        id: &SchemaValue<String>,
        user_id: &SchemaValue<String>,
    ) -> AuthResult<bool> {
        let id = id.typed()?;
        let account_id = clinical_schema_id(user_id)?;
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.identity_device_id", id).await?;
        context(
            &transaction,
            "med_tracker.current_account_id",
            &account_id.to_string(),
        )
        .await?;
        let account = transaction
            .query_one_raw(statement(
                "SELECT id FROM public.accounts WHERE id=$1 AND status=2 FOR UPDATE",
                [account_id.into()],
            ))
            .await
            .map_err(database_error)?;
        if account.is_none() {
            return Err(AuthError::Unauthenticated);
        }
        let changed = transaction.execute_raw(statement("UPDATE public.identity_device_codes SET account_id=$2 WHERE id=$1 AND account_id IS NULL AND status='pending' AND expires_at>CURRENT_TIMESTAMP", [id.into(), account_id.into()])).await.map_err(database_error)?.rows_affected() == 1;
        transaction.commit().await.map_err(database_error)?;
        Ok(changed)
    }

    async fn consume_device_code(
        &self,
        expected: &DeviceCode,
        ownership: &DeviceCodeOwnership,
    ) -> AuthResult<Option<DeviceCode>> {
        let (bindings, unchanged) = expected.consumption_bindings()?;
        if !unchanged {
            return Ok(None);
        }
        let client = match ownership {
            DeviceCodeOwnership::ClientId(client) => client,
            _ => {
                return Err(AuthError::config(
                    "Device-code ownership query is not configured",
                ));
            }
        };
        if bindings.get("clientId").and_then(|value| value.as_str()) != Some(client.as_str()) {
            return Ok(None);
        }
        let string = |name: &str| -> AuthResult<&str> {
            bindings
                .get(name)
                .and_then(|value| value.as_str())
                .ok_or_else(|| AuthError::bad_request("Invalid device-code consumption binding"))
        };
        let id = string("id")?;
        let device_code = string("deviceCode")?;
        let user_id = string("userId")?;
        let account_id = clinical_id(user_id)?;
        if string("status")? != "approved" {
            return Ok(None);
        }
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.identity_device_id", id).await?;
        context(&transaction, "med_tracker.current_account_id", user_id).await?;
        let row = DeviceRow::find_by_statement(statement(
            "DELETE FROM public.identity_device_codes WHERE id=$1 AND device_code=$2 AND client_id=$3 AND account_id=$4 AND status='approved' AND expires_at>CURRENT_TIMESTAMP RETURNING *",
            [id.into(), device_code.into(), client.as_str().into(), account_id.into()],
        )).one(&*transaction).await.map_err(database_error)?;
        let consumed = row.map(Into::into);
        transaction.commit().await.map_err(database_error)?;
        Ok(consumed)
    }

    async fn delete_device_code(&self, id: &SchemaValue<String>) -> AuthResult<()> {
        let id = id.typed()?;
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.identity_device_id", id).await?;
        transaction
            .execute_raw(statement(
                "DELETE FROM public.identity_device_codes WHERE id=$1",
                [id.into()],
            ))
            .await
            .map_err(database_error)?;
        transaction.commit().await.map_err(database_error)
    }

    async fn delete_device_code_if_status(
        &self,
        id: &SchemaValue<String>,
        status: &str,
    ) -> AuthResult<bool> {
        let id = id.typed()?;
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.identity_device_id", id).await?;
        let removed = transaction
            .execute_raw(statement(
                "DELETE FROM public.identity_device_codes WHERE id=$1 AND status=$2",
                [id.into(), status.into()],
            ))
            .await
            .map_err(database_error)?
            .rows_affected()
            == 1;
        transaction.commit().await.map_err(database_error)?;
        Ok(removed)
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
