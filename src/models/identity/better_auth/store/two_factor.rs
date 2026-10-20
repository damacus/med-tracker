use async_trait::async_trait;
use better_auth_core::{
    AuthError, AuthRecordFields, AuthResult, CreateTwoFactor, FieldDate, FieldMap, FieldValue,
    FromFieldMap, SchemaValue, TwoFactor, UpdateTwoFactor, store::TwoFactorStore,
};
use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, FromQueryResult};

use super::{ClinicalStore, clinical_id, clinical_schema_id, context, database_error, statement};

#[derive(FromQueryResult)]
struct FactorRow {
    id: String,
    account_id: i64,
    secret: String,
    backup_codes: String,
    verified: bool,
    failed_verification_count: i64,
    locked_until: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<FactorRow> for TwoFactor {
    fn from(row: FactorRow) -> Self {
        Self {
            field_order: Vec::new(),
            additional_fields: FieldMap::new(),
            id: row.id.into(),
            user_id: row.account_id.to_string().into(),
            secret: row.secret.into(),
            backup_codes: row.backup_codes.into(),
            verified: Some(row.verified).into(),
            failed_verification_count: Some(row.failed_verification_count as f64).into(),
            locked_until: row.locked_until.map(Into::into).into(),
            created_at: row.created_at.into(),
            updated_at: row.updated_at.into(),
        }
    }
}

impl ClinicalStore {
    async fn factor_account(&self, user_id: &str) -> AuthResult<super::Transaction> {
        let id = clinical_id(user_id)?;
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.current_account_id", user_id).await?;
        let account = transaction
            .query_one_raw(statement(
                "SELECT id FROM public.accounts WHERE id=$1 AND status=2 FOR UPDATE",
                [id.into()],
            ))
            .await
            .map_err(database_error)?;
        if account.is_none() {
            return Err(AuthError::Unauthenticated);
        }
        Ok(transaction)
    }

    async fn factor_by_id(&self, id: &str) -> AuthResult<(super::Transaction, FactorRow)> {
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.identity_factor_id", id).await?;
        let row = FactorRow::find_by_statement(statement(
            "SELECT * FROM public.identity_two_factors WHERE id=$1",
            [id.into()],
        ))
        .one(&*transaction)
        .await
        .map_err(database_error)?
        .ok_or_else(|| AuthError::not_found("Authenticator not found"))?;
        context(
            &transaction,
            "med_tracker.current_account_id",
            &row.account_id.to_string(),
        )
        .await?;
        let account = transaction
            .query_one_raw(statement(
                "SELECT id FROM public.accounts WHERE id=$1 AND status=2 FOR UPDATE",
                [row.account_id.into()],
            ))
            .await
            .map_err(database_error)?;
        if account.is_none() {
            return Err(AuthError::Unauthenticated);
        }
        Ok((transaction, row))
    }
}

#[async_trait]
impl TwoFactorStore for ClinicalStore {
    async fn create_two_factor_record(&self, fields: FieldMap) -> AuthResult<Option<FieldMap>> {
        let record = TwoFactor::from_field_values(fields)?;
        self.create_two_factor(CreateTwoFactor {
            additional_fields: record.additional_fields,
            user_id: record.user_id.typed()?.clone(),
            secret: record.secret.typed()?.clone(),
            backup_codes: record.backup_codes.typed()?.clone(),
            verified: if record.verified.is_undefined() {
                false
            } else {
                record.verified.typed()?.unwrap_or(false)
            },
        })
        .await?
        .field_values()
        .map(Some)
    }

    async fn get_two_factor_record(
        &self,
        id: &SchemaValue<String>,
    ) -> AuthResult<Option<FieldMap>> {
        let id = id.typed()?;
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.identity_factor_id", id).await?;
        let row = FactorRow::find_by_statement(statement(
            "SELECT * FROM public.identity_two_factors WHERE id=$1",
            [id.into()],
        ))
        .one(&*transaction)
        .await
        .map_err(database_error)?;
        let result = row
            .map(TwoFactor::from)
            .map(|row| row.field_values())
            .transpose()?;
        transaction.commit().await.map_err(database_error)?;
        Ok(result)
    }

    async fn update_two_factor_record(
        &self,
        id: &SchemaValue<String>,
        fields: FieldMap,
    ) -> AuthResult<Option<FieldMap>> {
        let record = TwoFactor::from_field_values(fields)?;
        if self.get_two_factor_record(id).await?.is_none() {
            return Ok(None);
        }
        self.update_two_factor(
            id,
            UpdateTwoFactor {
                additional_fields: record.additional_fields,
                secret: record.secret.typed().ok().cloned(),
                backup_codes: record.backup_codes.typed().ok().cloned(),
                verified: record.verified.typed().ok().copied().flatten(),
            },
        )
        .await?
        .field_values()
        .map(Some)
    }

    async fn create_two_factor(&self, input: CreateTwoFactor) -> AuthResult<TwoFactor> {
        if !input.additional_fields.is_empty() {
            return Err(AuthError::config(
                "Two-factor extension fields are not configured",
            ));
        }
        let transaction = self.factor_account(&input.user_id).await?;
        let row = FactorRow::find_by_statement(statement("INSERT INTO public.identity_two_factors(id,account_id,secret,backup_codes,verified) VALUES($1,$2,$3,$4,$5) RETURNING *", [uuid::Uuid::new_v4().to_string().into(), clinical_id(&input.user_id)?.into(), input.secret.into(), input.backup_codes.into(), input.verified.into()])).one(&*transaction).await.map_err(database_error)?.ok_or_else(|| AuthError::internal("Authenticator was not stored"))?;
        transaction.commit().await.map_err(database_error)?;
        Ok(row.into())
    }

    async fn get_two_factor_by_user_id_value(
        &self,
        user_id: &SchemaValue<String>,
    ) -> AuthResult<Option<TwoFactor>> {
        let id = clinical_schema_id(user_id)?;
        let transaction = self.transaction().await?;
        context(
            &transaction,
            "med_tracker.current_account_id",
            &id.to_string(),
        )
        .await?;
        let row = FactorRow::find_by_statement(statement("SELECT f.* FROM public.identity_two_factors f JOIN public.accounts a ON a.id=f.account_id WHERE f.account_id=$1 AND a.status=2", [id.into()])).one(&*transaction).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        Ok(row.map(Into::into))
    }

    async fn update_two_factor_backup_codes(
        &self,
        user_id: &str,
        codes: &str,
    ) -> AuthResult<TwoFactor> {
        let transaction = self.factor_account(user_id).await?;
        let row = FactorRow::find_by_statement(statement("UPDATE public.identity_two_factors SET backup_codes=$2,updated_at=CURRENT_TIMESTAMP WHERE account_id=$1 RETURNING *", [clinical_id(user_id)?.into(), codes.into()])).one(&*transaction).await.map_err(database_error)?.ok_or_else(|| AuthError::not_found("Authenticator not found"))?;
        transaction.commit().await.map_err(database_error)?;
        Ok(row.into())
    }

    async fn update_two_factor(
        &self,
        id: &SchemaValue<String>,
        update: UpdateTwoFactor,
    ) -> AuthResult<TwoFactor> {
        if !update.additional_fields.is_empty() {
            return Err(AuthError::config(
                "Two-factor extension fields are not configured",
            ));
        }
        let id = id.typed()?;
        let (transaction, _) = self.factor_by_id(id).await?;
        let row = FactorRow::find_by_statement(statement("UPDATE public.identity_two_factors SET secret=COALESCE($2,secret),backup_codes=COALESCE($3,backup_codes),verified=COALESCE($4,verified),updated_at=CURRENT_TIMESTAMP WHERE id=$1 RETURNING *", [id.into(), update.secret.into(), update.backup_codes.into(), update.verified.into()])).one(&*transaction).await.map_err(database_error)?.ok_or_else(|| AuthError::not_found("Authenticator not found"))?;
        transaction.commit().await.map_err(database_error)?;
        Ok(row.into())
    }

    async fn compare_exchange_two_factor_backup_codes(
        &self,
        id: &SchemaValue<String>,
        previous: &FieldValue,
        replacement: FieldValue,
    ) -> AuthResult<bool> {
        let id = id.typed()?;
        let previous = previous
            .as_str()
            .ok_or_else(|| AuthError::bad_request("Invalid backup codes"))?;
        let replacement = replacement
            .as_str()
            .ok_or_else(|| AuthError::bad_request("Invalid backup codes"))?;
        let (transaction, _) = self.factor_by_id(id).await?;
        let changed = transaction.execute_raw(statement("UPDATE public.identity_two_factors SET backup_codes=$3,updated_at=CURRENT_TIMESTAMP WHERE id=$1 AND backup_codes=$2 AND (locked_until IS NULL OR locked_until<=CURRENT_TIMESTAMP)", [id.into(), previous.into(), replacement.into()])).await.map_err(database_error)?.rows_affected() == 1;
        transaction.commit().await.map_err(database_error)?;
        Ok(changed)
    }

    async fn record_two_factor_failure(
        &self,
        id: &SchemaValue<String>,
        max_attempts: i64,
        locked_until: &(dyn Fn() -> AuthResult<FieldDate> + Send + Sync),
    ) -> AuthResult<()> {
        let id = id.typed()?;
        if max_attempts <= 0 {
            return Err(AuthError::bad_request(
                "Invalid verification attempt budget",
            ));
        }
        let (transaction, _) = self.factor_by_id(id).await?;
        let next = transaction.query_one_raw(statement("UPDATE public.identity_two_factors SET failed_verification_count=failed_verification_count+1,updated_at=CURRENT_TIMESTAMP WHERE id=$1 RETURNING failed_verification_count", [id.into()])).await.map_err(database_error)?
            .ok_or_else(|| AuthError::not_found("Authenticator not found"))?;
        let count: i64 = next
            .try_get("", "failed_verification_count")
            .map_err(database_error)?;
        if count >= max_attempts {
            let lock = locked_until()?
                .to_datetime()?
                .ok_or_else(|| AuthError::bad_request("Invalid verification lock time"))?;
            transaction.execute_raw(statement("UPDATE public.identity_two_factors SET locked_until=GREATEST(locked_until,$2) WHERE id=$1", [id.into(), lock.into()])).await.map_err(database_error)?;
        }
        transaction.commit().await.map_err(database_error)
    }

    async fn reset_two_factor_failures(
        &self,
        id: &SchemaValue<String>,
        locked_before: Option<FieldDate>,
    ) -> AuthResult<()> {
        let id = id.typed()?;
        let locked_before = locked_before
            .map(|value| {
                value.to_datetime().and_then(|value| {
                    value.ok_or_else(|| AuthError::bad_request("Invalid verification lock time"))
                })
            })
            .transpose()?;
        let (transaction, _) = self.factor_by_id(id).await?;
        transaction.execute_raw(statement("UPDATE public.identity_two_factors SET failed_verification_count=0,locked_until=NULL,updated_at=CURRENT_TIMESTAMP WHERE id=$1 AND ($2::timestamptz IS NULL OR locked_until<=$2)", [id.into(), locked_before.into()])).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)
    }

    async fn delete_two_factor_by_user_id_value(
        &self,
        user_id: &SchemaValue<String>,
    ) -> AuthResult<()> {
        let id = clinical_schema_id(user_id)?;
        let transaction = self.factor_account(&id.to_string()).await?;
        transaction
            .execute_raw(statement(
                "DELETE FROM public.identity_two_factors WHERE account_id=$1",
                [id.into()],
            ))
            .await
            .map_err(database_error)?;
        transaction.commit().await.map_err(database_error)
    }
}
