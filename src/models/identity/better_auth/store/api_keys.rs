use async_trait::async_trait;
use better_auth_core::{
    ApiKey, AuthError, AuthRecordFields, AuthResult, CreateApiKey, FieldDate, FieldMap, FieldValue,
    FromFieldMap, SchemaValue, UpdateApiKey,
    store::{ApiKeyStore, ApiKeyUsageWrite},
};
use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, DatabaseTransaction, FromQueryResult};
use serde_json::Value;

use super::super::request;
use super::{
    ClinicalStore, clinical_field_id, clinical_id, clinical_schema_id, context, database_error,
    statement,
};

#[derive(FromQueryResult)]
struct KeyRow {
    payload: Value,
}

fn decoded(row: KeyRow) -> AuthResult<ApiKey> {
    let object = row
        .payload
        .as_object()
        .cloned()
        .ok_or_else(|| AuthError::internal("Invalid stored API credential"))?;
    let mut fields = FieldMap::from_json(object)?;
    for name in [
        "createdAt",
        "updatedAt",
        "lastRefillAt",
        "lastRequest",
        "expiresAt",
    ] {
        if let Some(FieldValue::String(value)) = fields.get(name) {
            let date = timestamp(value)?.into();
            fields.insert(name.into(), FieldValue::Date(date));
        }
    }
    ApiKey::from_field_values(fields)
}

fn timestamp(value: &str) -> AuthResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| AuthError::bad_request("Invalid API credential timestamp"))
}

fn metadata_text(value: FieldValue) -> AuthResult<Option<String>> {
    if value.is_null() || value.is_undefined() {
        return Ok(None);
    }
    if let Some(value) = value.as_str() {
        return Ok(Some(value.to_owned()));
    }
    value
        .json()?
        .map(|value| value.to_string())
        .ok_or_else(|| AuthError::bad_request("Invalid API credential metadata"))
        .map(Some)
}

fn patch_number(value: &SchemaValue<Option<f64>>) -> AuthResult<Option<f64>> {
    if value.is_undefined() {
        return Ok(None);
    }
    value
        .typed()?
        .as_ref()
        .copied()
        .ok_or_else(|| AuthError::bad_request("Cannot clear numeric API credential field"))
        .map(Some)
}

fn patch_string(value: &SchemaValue<Option<String>>) -> AuthResult<Option<String>> {
    if value.is_undefined() {
        return Ok(None);
    }
    value
        .typed()?
        .clone()
        .ok_or_else(|| AuthError::bad_request("Cannot clear API credential field"))
        .map(Some)
}

fn patch_date(value: &SchemaValue<Option<FieldDate>>) -> AuthResult<Option<Option<FieldDate>>> {
    if value.is_undefined() {
        Ok(None)
    } else {
        Ok(Some(value.typed()?.clone()))
    }
}

fn apply(key: ApiKey, update: UpdateApiKey) -> AuthResult<ApiKey> {
    let mut fields = key.field_values()?;
    for (name, value) in update.into_adapter_fields()? {
        fields.insert(name, value);
    }
    fields.insert("updatedAt".into(), Utc::now().into());
    ApiKey::from_field_values(fields)
}

impl ClinicalStore {
    async fn key_transaction(&self, id: &str) -> AuthResult<(super::Transaction, ApiKey)> {
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.identity_api_key_id", id).await?;
        let owner = transaction
            .query_one_raw(statement(
                "SELECT account_id FROM public.identity_api_keys WHERE id=$1",
                [id.into()],
            ))
            .await
            .map_err(database_error)?
            .ok_or_else(|| AuthError::not_found("API credential not found"))?;
        let account_id: i64 = owner.try_get("", "account_id").map_err(database_error)?;
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
        crate::models::access::verify_account(&transaction, account_id)
            .await
            .map_err(super::tenant::operation_error)?;
        let row = KeyRow::find_by_statement(statement(
            "SELECT payload FROM public.identity_api_keys WHERE id=$1 FOR UPDATE",
            [id.into()],
        ))
        .one(&*transaction)
        .await
        .map_err(database_error)?
        .ok_or_else(|| AuthError::not_found("API credential not found"))?;
        Ok((transaction, decoded(row)?))
    }

    async fn save_key(&self, transaction: &DatabaseTransaction, key: &ApiKey) -> AuthResult<()> {
        let expiry = key
            .expires_at
            .typed()?
            .as_ref()
            .map(|date| {
                date.to_datetime().and_then(|value| {
                    value.ok_or_else(|| AuthError::bad_request("Invalid API credential expiry"))
                })
            })
            .transpose()?;
        let payload = Value::Object(key.field_values()?.json()?);
        transaction
            .execute_raw(statement(
                "UPDATE public.identity_api_keys SET payload=$2,expires_at=$3 WHERE id=$1",
                [
                    key.id.typed()?.clone().into(),
                    payload.into(),
                    expiry.into(),
                ],
            ))
            .await
            .map_err(database_error)?;
        Ok(())
    }

    async fn authorize_key_reference(&self, reference: &str) -> AuthResult<()> {
        let actor = request::authenticated()?;
        if actor.account_id != clinical_id(reference)? {
            return Err(AuthError::forbidden("API credential access denied"));
        }
        let transaction = self.account_transaction().await?;
        transaction.commit().await.map_err(database_error)
    }
}

#[async_trait]
impl ApiKeyStore for ClinicalStore {
    async fn create_api_key_record(&self, fields: FieldMap) -> AuthResult<Option<FieldMap>> {
        let record = ApiKey::from_field_values(fields)?;
        self.create_api_key(CreateApiKey {
            additional_fields: record.additional_fields,
            reference_id: record.reference_id,
            config_id: record.config_id.typed()?.clone(),
            name: record.name,
            prefix: record.prefix.typed()?.clone(),
            key_hash: record.key_hash.typed()?.clone(),
            start: record.start.typed()?.clone(),
            expires_at: record.expires_at.typed()?.clone(),
            remaining: record.remaining.typed()?.clone(),
            rate_limit_enabled: *record.rate_limit_enabled.typed()?,
            rate_limit_time_window: record.rate_limit_time_window.typed()?.clone(),
            rate_limit_max: record.rate_limit_max.typed()?.clone(),
            refill_interval: record.refill_interval.typed()?.clone(),
            refill_amount: record.refill_amount.typed()?.clone(),
            permissions: record.permissions,
            metadata: metadata_text(record.metadata.field_value())?,
            enabled: record.enabled,
        })
        .await?
        .field_values()
        .map(Some)
    }

    async fn get_api_key_record(&self, id: &SchemaValue<String>) -> AuthResult<Option<FieldMap>> {
        self.get_api_key_by_id(id.typed()?)
            .await?
            .map(|row| row.field_values())
            .transpose()
    }

    async fn update_api_key_record(
        &self,
        id: &SchemaValue<String>,
        fields: FieldMap,
    ) -> AuthResult<Option<FieldMap>> {
        let record = ApiKey::from_field_values(fields)?;
        if !record.id.is_undefined()
            || !record.start.is_undefined()
            || !record.prefix.is_undefined()
            || !record.key_hash.is_undefined()
            || !record.reference_id.is_undefined()
            || !record.config_id.is_undefined()
            || !record.created_at.is_undefined()
            || !record.updated_at.is_undefined()
        {
            return Err(AuthError::bad_request(
                "API credential identity fields cannot be updated",
            ));
        }
        if !record.additional_fields.is_empty() {
            return Err(AuthError::config(
                "API-key extension fields are not configured",
            ));
        }
        self.update_api_key_optional(
            id,
            UpdateApiKey {
                additional_fields: record.additional_fields,
                name: if record.name.is_undefined() {
                    None
                } else {
                    let _ = record.name.typed()?;
                    Some(record.name)
                },
                enabled: if record.enabled.is_undefined() {
                    None
                } else {
                    let _ = record.enabled.typed()?;
                    Some(record.enabled)
                },
                remaining: patch_number(&record.remaining)?,
                rate_limit_enabled: if record.rate_limit_enabled.is_undefined() {
                    None
                } else {
                    Some(*record.rate_limit_enabled.typed()?)
                },
                rate_limit_time_window: patch_number(&record.rate_limit_time_window)?,
                rate_limit_max: patch_number(&record.rate_limit_max)?,
                refill_interval: patch_number(&record.refill_interval)?,
                refill_amount: patch_number(&record.refill_amount)?,
                permissions: patch_string(&record.permissions)?,
                metadata: if record.metadata.is_undefined() {
                    None
                } else {
                    Some(
                        metadata_text(record.metadata.field_value())?.ok_or_else(|| {
                            AuthError::bad_request(
                                "Clearing API credential metadata is unsupported",
                            )
                        })?,
                    )
                },
                expires_at: patch_date(&record.expires_at)?,
                last_request: patch_date(&record.last_request)?,
                request_count: patch_number(&record.request_count)?,
                last_refill_at: patch_date(&record.last_refill_at)?,
            },
        )
        .await?
        .map(|row| row.field_values())
        .transpose()
    }

    async fn create_api_key(&self, input: CreateApiKey) -> AuthResult<ApiKey> {
        if !input.additional_fields.is_empty() {
            return Err(AuthError::config(
                "API-key extension fields are not configured",
            ));
        }
        let recovery = input.config_id == super::super::onboarding::RECOVERY_CONFIG;
        if !recovery {
            self.authorize_key_reference(input.reference_id.typed()?)
                .await?;
        }
        let account_id = clinical_schema_id(&input.reference_id)?;
        if recovery && !request::recovery_issuance_allowed(account_id) {
            return Err(AuthError::forbidden("Recovery issuance is unavailable"));
        }
        let now = Utc::now();
        let expiry = input
            .expires_at
            .as_ref()
            .map(|date| {
                date.to_datetime().and_then(|value| {
                    value.ok_or_else(|| AuthError::bad_request("Invalid API credential expiry"))
                })
            })
            .transpose()?;
        let key = ApiKey {
            field_order: Vec::new(),
            additional_fields: FieldMap::new(),
            id: uuid::Uuid::new_v4().to_string().into(),
            name: input.name,
            start: input.start.into(),
            prefix: input.prefix.into(),
            key_hash: input.key_hash.into(),
            reference_id: input.reference_id,
            config_id: input.config_id.into(),
            refill_interval: input.refill_interval.into(),
            refill_amount: input.refill_amount.into(),
            last_refill_at: None.into(),
            enabled: input.enabled,
            rate_limit_enabled: input.rate_limit_enabled.into(),
            rate_limit_time_window: input.rate_limit_time_window.into(),
            rate_limit_max: input.rate_limit_max.into(),
            request_count: Some(0.0).into(),
            remaining: input.remaining.into(),
            last_request: None.into(),
            expires_at: input.expires_at.into(),
            created_at: now.into(),
            updated_at: now.into(),
            permissions: input.permissions,
            metadata: match input.metadata {
                Some(metadata) => SchemaValue::from_field(FieldValue::parse_json(&metadata)?),
                None => SchemaValue::Typed(None),
            },
        };
        let transaction = if recovery {
            self.transaction().await?
        } else {
            self.account_transaction().await?
        };
        context(
            &transaction,
            "med_tracker.current_account_id",
            &account_id.to_string(),
        )
        .await?;
        let payload = Value::Object(key.field_values()?.json()?);
        transaction.execute_raw(statement("INSERT INTO public.identity_api_keys(id,account_id,key_hash,expires_at,payload) VALUES($1,$2,$3,$4,$5)", [key.id.typed()?.clone().into(), account_id.into(), key.key_hash.typed()?.clone().into(), expiry.into(), payload.into()])).await.map_err(database_error)?;
        self.audit(&transaction, account_id, "api_key", "created")
            .await?;
        transaction.commit().await.map_err(database_error)?;
        Ok(key)
    }

    async fn get_api_key_by_id(&self, id: &str) -> AuthResult<Option<ApiKey>> {
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.identity_api_key_id", id).await?;
        let row = KeyRow::find_by_statement(statement(
            "SELECT payload FROM public.identity_api_keys WHERE id=$1",
            [id.into()],
        ))
        .one(&*transaction)
        .await
        .map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        row.map(decoded).transpose()
    }

    async fn get_api_key_by_hash(&self, hash: &str) -> AuthResult<Option<ApiKey>> {
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.identity_api_key_hash", hash).await?;
        let row = KeyRow::find_by_statement(statement("SELECT k.payload FROM public.identity_api_keys k JOIN public.accounts a ON a.id=k.account_id WHERE k.key_hash=$1 AND a.status=2", [hash.into()])).one(&*transaction).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        row.map(decoded).transpose()
    }

    async fn find_api_keys_by_reference_value(
        &self,
        reference: &FieldValue,
        sort: Option<(&str, &str)>,
    ) -> AuthResult<Vec<ApiKey>> {
        let account_id = clinical_field_id(reference)?;
        self.authorize_key_reference(&account_id.to_string())
            .await?;
        let transaction = self.account_transaction().await?;
        let order = match sort {
            None | Some(("createdAt", "asc")) => "payload->>'createdAt' ASC, id ASC",
            Some(("createdAt", "desc")) => "payload->>'createdAt' DESC, id DESC",
            Some(("name", "asc")) => "payload->>'name' ASC, id ASC",
            Some(("name", "desc")) => "payload->>'name' DESC, id DESC",
            _ => return Err(AuthError::bad_request("Unsupported API credential sort")),
        };
        let sql = format!(
            "SELECT payload FROM public.identity_api_keys WHERE account_id=$1 ORDER BY {order} LIMIT 100"
        );
        let rows = KeyRow::find_by_statement(statement(&sql, [account_id.into()]))
            .all(&*transaction)
            .await
            .map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        rows.into_iter().map(decoded).collect()
    }

    async fn count_api_keys_by_reference_value(&self, reference: &FieldValue) -> AuthResult<u64> {
        let account_id = clinical_field_id(reference)?;
        self.authorize_key_reference(&account_id.to_string())
            .await?;
        let transaction = self.account_transaction().await?;
        let row = transaction
            .query_one_raw(statement(
                "SELECT count(*) AS count FROM public.identity_api_keys WHERE account_id=$1",
                [account_id.into()],
            ))
            .await
            .map_err(database_error)?
            .ok_or_else(|| AuthError::internal("API credential count unavailable"))?;
        let count: i64 = row.try_get("", "count").map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        u64::try_from(count).map_err(|_| AuthError::internal("Invalid API credential count"))
    }

    async fn update_api_key(
        &self,
        id: &SchemaValue<String>,
        update: UpdateApiKey,
    ) -> AuthResult<ApiKey> {
        if !update.additional_fields.is_empty() {
            return Err(AuthError::config(
                "API-key extension fields are not configured",
            ));
        }
        let id = id.typed()?;
        let current = self
            .get_api_key_by_id(id)
            .await?
            .ok_or_else(|| AuthError::not_found("API credential not found"))?;
        if current.config_id.typed()? == super::super::personal_keys::CONFIG
            && (update.name.is_some()
                || update.permissions.is_some()
                || update.metadata.is_some()
                || update.expires_at.is_some()
                || update.remaining.is_some()
                || update.refill_interval.is_some()
                || update.refill_amount.is_some()
                || update
                    .enabled
                    .as_ref()
                    .is_some_and(|value| value.typed().is_ok_and(|enabled| *enabled)))
        {
            return Err(AuthError::forbidden(
                "Personal API keys are immutable; revoke and replace them",
            ));
        }
        self.authorize_key_reference(current.reference_id.typed()?)
            .await?;
        let (transaction, key) = self.key_transaction(id).await?;
        let key = apply(key, update)?;
        self.save_key(&transaction, &key).await?;
        self.audit(
            &transaction,
            clinical_schema_id(&key.reference_id)?,
            "api_key",
            "updated",
        )
        .await?;
        transaction.commit().await.map_err(database_error)?;
        Ok(key)
    }

    async fn delete_api_key(&self, id: &SchemaValue<String>) -> AuthResult<()> {
        let id = id.typed()?;
        let current = self
            .get_api_key_by_id(id)
            .await?
            .ok_or_else(|| AuthError::not_found("API credential not found"))?;
        self.authorize_key_reference(current.reference_id.typed()?)
            .await?;
        let (transaction, key) = self.key_transaction(id).await?;
        transaction
            .execute_raw(statement(
                "DELETE FROM public.identity_api_keys WHERE id=$1",
                [id.into()],
            ))
            .await
            .map_err(database_error)?;
        self.audit(
            &transaction,
            clinical_schema_id(&key.reference_id)?,
            "api_key",
            "revoked",
        )
        .await?;
        transaction.commit().await.map_err(database_error)
    }

    async fn delete_expired_api_keys_before(&self, cutoff: FieldDate) -> AuthResult<usize> {
        let cutoff = cutoff
            .to_datetime()?
            .ok_or_else(|| AuthError::bad_request("Invalid API credential cutoff"))?;
        if cutoff > Utc::now() {
            return Err(AuthError::bad_request(
                "API credential cutoff is in the future",
            ));
        }
        let transaction = self.transaction().await?;
        let row = transaction
            .query_one_raw(statement(
                "SELECT public.identity_delete_expired_api_keys($1) AS count",
                [cutoff.into()],
            ))
            .await
            .map_err(database_error)?
            .ok_or_else(|| AuthError::internal("Expiry result unavailable"))?;
        let count: i64 = row.try_get("", "count").map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        usize::try_from(count).map_err(|_| AuthError::internal("Invalid expiry result"))
    }

    async fn write_api_key_usage(
        &self,
        id: &SchemaValue<String>,
        write: ApiKeyUsageWrite,
    ) -> AuthResult<Option<ApiKey>> {
        let id = id.typed()?;
        let (transaction, key) = self.key_transaction(id).await?;
        let now = Utc::now();
        if !*key.enabled.typed()?
            || key.expires_at.typed()?.as_ref().is_some_and(|expiry| {
                expiry
                    .to_datetime()
                    .ok()
                    .flatten()
                    .is_none_or(|expiry| expiry <= now)
            })
        {
            return Err(AuthError::InvalidCredentials);
        }
        let mut update = UpdateApiKey::default();
        let guard = match write {
            ApiKeyUsageWrite::Refill {
                previous,
                remaining,
                at,
            } => {
                let expected = if previous.is_null() || previous.is_undefined() {
                    None
                } else {
                    Some(
                        previous
                            .as_date()
                            .ok_or_else(|| AuthError::bad_request("Invalid refill timestamp"))?
                            .clone(),
                    )
                };
                if key.last_refill_at.typed()?.as_ref() != expected.as_ref() {
                    false
                } else {
                    update.remaining = Some(remaining);
                    update.last_refill_at = Some(Some(at.into()));
                    true
                }
            }
            ApiKeyUsageWrite::Decrement => {
                if let Some(remaining) = key.remaining.typed()? {
                    if *remaining > 0.0 {
                        update.remaining = Some(remaining - 1.0);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            ApiKeyUsageWrite::StartWindow {
                previous_before,
                at,
            } => {
                let last = key.last_request.typed()?;
                let matches = match (last, previous_before) {
                    (None, None) => true,
                    (Some(last), Some(cutoff)) => last.milliseconds() < cutoff.milliseconds(),
                    _ => false,
                };
                if matches {
                    update.request_count = Some(1.0);
                    update.last_request = Some(Some(at.into()));
                }
                matches
            }
            ApiKeyUsageWrite::IncrementWindow {
                previous_after,
                maximum,
                at,
            } => {
                let maximum = maximum
                    .as_f64()
                    .ok_or_else(|| AuthError::bad_request("Invalid rate limit"))?;
                let matches = key
                    .last_request
                    .typed()?
                    .as_ref()
                    .is_some_and(|last| last.milliseconds() >= previous_after.milliseconds())
                    && key.request_count.typed()?.unwrap_or(0.0) < maximum;
                if matches {
                    update.request_count = Some(key.request_count.typed()?.unwrap_or(0.0) + 1.0);
                    update.last_request = Some(Some(at.into()));
                }
                matches
            }
            ApiKeyUsageWrite::LastRequest(at) => {
                update.last_request = Some(Some(at.into()));
                true
            }
            ApiKeyUsageWrite::UpdatedAt(at) => {
                let mut fields = key.field_values()?;
                fields.insert("updatedAt".into(), at.into());
                let key = ApiKey::from_field_values(fields)?;
                self.save_key(&transaction, &key).await?;
                transaction.commit().await.map_err(database_error)?;
                return Ok(Some(key));
            }
        };
        if !guard {
            transaction.commit().await.map_err(database_error)?;
            return Ok(None);
        }
        let updated = apply(key, update)?;
        self.save_key(&transaction, &updated).await?;
        transaction.commit().await.map_err(database_error)?;
        Ok(Some(updated))
    }
}
