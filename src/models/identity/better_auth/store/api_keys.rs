use async_trait::async_trait;
use better_auth_core::{
    ApiKey, AuthError, AuthResult, CreateApiKey, UpdateApiKey,
    store::{ApiKeyStore, ConsumeApiKeyResult},
};
use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, DatabaseTransaction, FromQueryResult};
use serde_json::Value;

use super::super::request;
use super::{ClinicalStore, clinical_id, context, database_error, statement};

#[derive(FromQueryResult)]
struct KeyRow {
    payload: Value,
}

fn decoded(row: KeyRow) -> AuthResult<ApiKey> {
    serde_json::from_value(row.payload)
        .map_err(|_| AuthError::internal("Invalid stored API credential"))
}

fn timestamp(value: &str) -> AuthResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| AuthError::bad_request("Invalid API credential timestamp"))
}

fn apply(key: &mut ApiKey, update: UpdateApiKey) -> AuthResult<()> {
    macro_rules! optional { ($($field:ident),*) => { $(if let Some(value) = update.$field { key.$field = Some(value); })* }; }
    optional!(
        name,
        remaining,
        rate_limit_time_window,
        rate_limit_max,
        refill_interval,
        refill_amount,
        permissions,
        metadata,
        request_count
    );
    if let Some(value) = update.enabled {
        key.enabled = value;
    }
    if let Some(value) = update.rate_limit_enabled {
        key.rate_limit_enabled = value;
    }
    if let Some(value) = update.expires_at {
        key.expires_at = value
            .map(|value| timestamp(&value).map(|value| value.to_rfc3339()))
            .transpose()?;
    }
    if let Some(value) = update.last_request {
        key.last_request = value
            .map(|value| timestamp(&value).map(|value| value.to_rfc3339()))
            .transpose()?;
    }
    if let Some(value) = update.last_refill_at {
        key.last_refill_at = value
            .map(|value| timestamp(&value).map(|value| value.to_rfc3339()))
            .transpose()?;
    }
    key.updated_at = Utc::now().to_rfc3339();
    Ok(())
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
        crate::models::access::verify_account_actor(&transaction, account_id)
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
        let expiry = key.expires_at.as_deref().map(timestamp).transpose()?;
        let payload = serde_json::to_value(key)
            .map_err(|_| AuthError::internal("Invalid API credential state"))?;
        transaction
            .execute_raw(statement(
                "UPDATE public.identity_api_keys SET payload=$2,expires_at=$3 WHERE id=$1",
                [key.id.clone().into(), payload.into(), expiry.into()],
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
    async fn create_api_key(&self, input: CreateApiKey) -> AuthResult<ApiKey> {
        let recovery = input.config_id == super::super::onboarding::RECOVERY_CONFIG;
        if !recovery {
            self.authorize_key_reference(&input.reference_id).await?;
        }
        let account_id = clinical_id(&input.reference_id)?;
        if recovery && !request::recovery_issuance_allowed(account_id) {
            return Err(AuthError::forbidden("Recovery issuance is unavailable"));
        }
        let now = Utc::now().to_rfc3339();
        let expiry = input.expires_at.as_deref().map(timestamp).transpose()?;
        let key = ApiKey {
            id: uuid::Uuid::new_v4().to_string(),
            name: input.name,
            start: input.start,
            prefix: input.prefix,
            key_hash: input.key_hash,
            reference_id: input.reference_id,
            config_id: input.config_id,
            refill_interval: input.refill_interval,
            refill_amount: input.refill_amount,
            last_refill_at: None,
            enabled: input.enabled,
            rate_limit_enabled: input.rate_limit_enabled,
            rate_limit_time_window: input.rate_limit_time_window,
            rate_limit_max: input.rate_limit_max,
            request_count: Some(0.0),
            remaining: input.remaining,
            last_request: None,
            expires_at: expiry.map(|value| value.to_rfc3339()),
            created_at: now.clone(),
            updated_at: now,
            permissions: input.permissions,
            metadata: input.metadata,
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
        let payload = serde_json::to_value(&key)
            .map_err(|_| AuthError::internal("Invalid API credential state"))?;
        transaction.execute_raw(statement("INSERT INTO public.identity_api_keys(id,account_id,key_hash,expires_at,payload) VALUES($1,$2,$3,$4,$5)", [key.id.clone().into(), account_id.into(), key.key_hash.clone().into(), expiry.into(), payload.into()])).await.map_err(database_error)?;
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

    async fn list_api_keys_by_reference(&self, reference: &str) -> AuthResult<Vec<ApiKey>> {
        self.authorize_key_reference(reference).await?;
        let transaction = self.account_transaction().await?;
        let rows = KeyRow::find_by_statement(statement("SELECT payload FROM public.identity_api_keys WHERE account_id=$1 ORDER BY payload->>'createdAt',id", [clinical_id(reference)?.into()])).all(&*transaction).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        rows.into_iter().map(decoded).collect()
    }

    async fn update_api_key(&self, id: &str, update: UpdateApiKey) -> AuthResult<ApiKey> {
        let current = self
            .get_api_key_by_id(id)
            .await?
            .ok_or_else(|| AuthError::not_found("API credential not found"))?;
        if current.config_id == super::super::personal_keys::CONFIG
            && (update.name.is_some()
                || update.permissions.is_some()
                || update.metadata.is_some()
                || update.expires_at.is_some()
                || update.remaining.is_some()
                || update.refill_interval.is_some()
                || update.refill_amount.is_some()
                || update.enabled == Some(true))
        {
            return Err(AuthError::forbidden(
                "Personal API keys are immutable; revoke and replace them",
            ));
        }
        self.authorize_key_reference(&current.reference_id).await?;
        let (transaction, mut key) = self.key_transaction(id).await?;
        apply(&mut key, update)?;
        self.save_key(&transaction, &key).await?;
        self.audit(
            &transaction,
            clinical_id(&key.reference_id)?,
            "api_key",
            "updated",
        )
        .await?;
        transaction.commit().await.map_err(database_error)?;
        Ok(key)
    }

    async fn delete_api_key(&self, id: &str) -> AuthResult<()> {
        let current = self
            .get_api_key_by_id(id)
            .await?
            .ok_or_else(|| AuthError::not_found("API credential not found"))?;
        self.authorize_key_reference(&current.reference_id).await?;
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
            clinical_id(&key.reference_id)?,
            "api_key",
            "revoked",
        )
        .await?;
        transaction.commit().await.map_err(database_error)
    }

    async fn delete_expired_api_keys(&self) -> AuthResult<usize> {
        let transaction = self.transaction().await?;
        let row = transaction
            .query_one_raw(statement(
                "SELECT public.identity_delete_expired_api_keys() AS count",
                [],
            ))
            .await
            .map_err(database_error)?
            .ok_or_else(|| AuthError::internal("Expiry result unavailable"))?;
        let count: i64 = row.try_get("", "count").map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        usize::try_from(count).map_err(|_| AuthError::internal("Invalid expiry result"))
    }

    async fn consume_api_key_usage(
        &self,
        id: &str,
        global_rate_limit_enabled: bool,
    ) -> AuthResult<ConsumeApiKeyResult> {
        let (transaction, mut key) = self.key_transaction(id).await?;
        let now = Utc::now();
        if !key.enabled
            || key
                .expires_at
                .as_deref()
                .map(timestamp)
                .transpose()?
                .is_some_and(|expiry| expiry <= now)
        {
            return Err(AuthError::InvalidCredentials);
        }
        let mut update = UpdateApiKey::default();
        if let Some(remaining) = key.remaining {
            if remaining == 0.0 && key.refill_amount.is_none() {
                transaction
                    .execute_raw(statement(
                        "DELETE FROM public.identity_api_keys WHERE id=$1",
                        [id.into()],
                    ))
                    .await
                    .map_err(database_error)?;
                transaction.commit().await.map_err(database_error)?;
                return Ok(ConsumeApiKeyResult::UsageExhausted);
            }
            let last_refill = timestamp(key.last_refill_at.as_deref().unwrap_or(&key.created_at))?;
            if let (Some(interval), Some(amount)) = (key.refill_interval, key.refill_amount)
                && interval != 0.0
                && amount != 0.0
                && (now.timestamp_millis() - last_refill.timestamp_millis()) as f64 > interval
            {
                update.remaining = Some(amount - 1.0);
                update.last_refill_at = Some(Some(now.to_rfc3339()));
            } else if remaining > 0.0 {
                update.remaining = Some(remaining - 1.0);
            } else {
                transaction.commit().await.map_err(database_error)?;
                return Ok(ConsumeApiKeyResult::UsageExhausted);
            }
        }
        if global_rate_limit_enabled && key.rate_limit_enabled {
            if let (Some(window), Some(maximum)) = (key.rate_limit_time_window, key.rate_limit_max)
            {
                let elapsed = key
                    .last_request
                    .as_deref()
                    .map(timestamp)
                    .transpose()?
                    .map(|last| (now.timestamp_millis() - last.timestamp_millis()) as f64);
                if let Some(elapsed) = elapsed
                    && elapsed <= window
                    && key.request_count.unwrap_or(0.0) >= maximum
                {
                    if update.remaining.is_some() {
                        let modified = key.updated_at.clone();
                        apply(&mut key, update)?;
                        key.updated_at = modified;
                        self.save_key(&transaction, &key).await?;
                    }
                    transaction.commit().await.map_err(database_error)?;
                    return Ok(ConsumeApiKeyResult::RateLimited {
                        try_again_in: (window - elapsed).ceil(),
                    });
                }
                update.request_count = Some(if elapsed.is_none_or(|elapsed| elapsed > window) {
                    1.0
                } else {
                    key.request_count.unwrap_or(0.0) + 1.0
                });
                update.last_request = Some(Some(now.to_rfc3339()));
            }
        } else {
            update.last_request = Some(Some(now.to_rfc3339()));
        }
        apply(&mut key, update)?;
        self.save_key(&transaction, &key).await?;
        transaction.commit().await.map_err(database_error)?;
        Ok(ConsumeApiKeyResult::Allowed(Box::new(key)))
    }
}
