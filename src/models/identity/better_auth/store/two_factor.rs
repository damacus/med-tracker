use async_trait::async_trait;
use better_auth_core::{
    AuthError, AuthResult, CreateTwoFactor, TwoFactor, UpdateTwoFactor, store::TwoFactorStore,
};
use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, FromQueryResult};

use super::{ClinicalStore, clinical_id, context, database_error, statement};

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
            id: row.id,
            user_id: row.account_id.to_string(),
            secret: row.secret,
            backup_codes: row.backup_codes,
            verified: row.verified,
            failed_verification_count: row.failed_verification_count,
            locked_until: row.locked_until,
            created_at: row.created_at,
            updated_at: row.updated_at,
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
    async fn create_two_factor(&self, input: CreateTwoFactor) -> AuthResult<TwoFactor> {
        let transaction = self.factor_account(&input.user_id).await?;
        let row = FactorRow::find_by_statement(statement("INSERT INTO public.identity_two_factors(id,account_id,secret,backup_codes,verified) VALUES($1,$2,$3,$4,$5) RETURNING *", [uuid::Uuid::new_v4().to_string().into(), clinical_id(&input.user_id)?.into(), input.secret.into(), input.backup_codes.into(), input.verified.into()])).one(&*transaction).await.map_err(database_error)?.ok_or_else(|| AuthError::internal("Authenticator was not stored"))?;
        transaction.commit().await.map_err(database_error)?;
        Ok(row.into())
    }

    async fn get_two_factor_by_user_id(&self, user_id: &str) -> AuthResult<Option<TwoFactor>> {
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.current_account_id", user_id).await?;
        let row = FactorRow::find_by_statement(statement("SELECT f.* FROM public.identity_two_factors f JOIN public.accounts a ON a.id=f.account_id WHERE f.account_id=$1 AND a.status=2", [clinical_id(user_id)?.into()])).one(&*transaction).await.map_err(database_error)?;
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

    async fn update_two_factor(&self, id: &str, update: UpdateTwoFactor) -> AuthResult<TwoFactor> {
        let (transaction, _) = self.factor_by_id(id).await?;
        let row = FactorRow::find_by_statement(statement("UPDATE public.identity_two_factors SET secret=COALESCE($2,secret),backup_codes=COALESCE($3,backup_codes),verified=COALESCE($4,verified),updated_at=CURRENT_TIMESTAMP WHERE id=$1 RETURNING *", [id.into(), update.secret.into(), update.backup_codes.into(), update.verified.into()])).one(&*transaction).await.map_err(database_error)?.ok_or_else(|| AuthError::not_found("Authenticator not found"))?;
        transaction.commit().await.map_err(database_error)?;
        Ok(row.into())
    }

    async fn compare_exchange_two_factor_backup_codes(
        &self,
        id: &str,
        previous: &str,
        replacement: &str,
    ) -> AuthResult<bool> {
        let (transaction, _) = self.factor_by_id(id).await?;
        let changed = transaction.execute_raw(statement("UPDATE public.identity_two_factors SET backup_codes=$3,updated_at=CURRENT_TIMESTAMP WHERE id=$1 AND backup_codes=$2 AND (locked_until IS NULL OR locked_until<=CURRENT_TIMESTAMP)", [id.into(), previous.into(), replacement.into()])).await.map_err(database_error)?.rows_affected() == 1;
        transaction.commit().await.map_err(database_error)?;
        Ok(changed)
    }

    async fn record_two_factor_failure(
        &self,
        id: &str,
        max_attempts: i64,
        locked_until: DateTime<Utc>,
    ) -> AuthResult<()> {
        let (transaction, _) = self.factor_by_id(id).await?;
        transaction.execute_raw(statement("UPDATE public.identity_two_factors SET failed_verification_count=failed_verification_count+1,locked_until=CASE WHEN failed_verification_count+1 >= $2 THEN GREATEST(locked_until,$3) ELSE locked_until END,updated_at=CURRENT_TIMESTAMP WHERE id=$1", [id.into(), max_attempts.into(), locked_until.into()])).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)
    }

    async fn reset_two_factor_failures(
        &self,
        id: &str,
        locked_before: Option<DateTime<Utc>>,
    ) -> AuthResult<()> {
        let (transaction, _) = self.factor_by_id(id).await?;
        transaction.execute_raw(statement("UPDATE public.identity_two_factors SET failed_verification_count=0,locked_until=NULL,updated_at=CURRENT_TIMESTAMP WHERE id=$1 AND ($2::timestamptz IS NULL OR locked_until<=$2)", [id.into(), locked_before.into()])).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)
    }

    async fn delete_two_factor(&self, user_id: &str) -> AuthResult<()> {
        let transaction = self.factor_account(user_id).await?;
        transaction
            .execute_raw(statement(
                "DELETE FROM public.identity_two_factors WHERE account_id=$1",
                [clinical_id(user_id)?.into()],
            ))
            .await
            .map_err(database_error)?;
        transaction.commit().await.map_err(database_error)
    }
}
