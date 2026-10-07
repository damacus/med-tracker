use async_trait::async_trait;
use better_auth_core::{AuthResult, CreateVerification, store::VerificationStore, wire::VerificationView};
use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, FromQueryResult};

use super::{ClinicalStore, context, database_error, statement};
use super::super::ClinicalAuthSchema;

#[derive(FromQueryResult)]
struct VerificationRow {
    id: String,
    identifier: String,
    value: String,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<VerificationRow> for VerificationView {
    fn from(row: VerificationRow) -> Self {
        Self { id: row.id, identifier: row.identifier, value: row.value, expires_at: row.expires_at, created_at: row.created_at, updated_at: row.updated_at }
    }
}

impl ClinicalStore {
    pub(crate) async fn create_verification_in(&self, transaction: &sea_orm::DatabaseTransaction, verification: CreateVerification) -> AuthResult<VerificationView> {
        context(transaction, "med_tracker.identity_verification_identifier", &verification.identifier).await?;
        let row = VerificationRow::find_by_statement(statement("INSERT INTO public.identity_verifications (id, identifier, value, expires_at) VALUES ($1, $2, $3, $4) RETURNING *", [uuid::Uuid::new_v4().to_string().into(), verification.identifier.into(), verification.value.into(), verification.expires_at.into()])).one(transaction).await.map_err(database_error)?.ok_or_else(|| better_auth_core::AuthError::internal("Verification was not stored"))?;
        Ok(row.into())
    }

    async fn verification(&self, key: &str, value: &str, sql: &str) -> AuthResult<Option<VerificationView>> {
        let transaction = self.transaction().await?;
        context(&*transaction, key, value).await?;
        let row = VerificationRow::find_by_statement(statement(sql, [value.into()])).one(&*transaction).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        Ok(row.map(Into::into))
    }
}

#[async_trait]
impl VerificationStore<ClinicalAuthSchema> for ClinicalStore {
    async fn create_verification(&self, verification: CreateVerification) -> AuthResult<VerificationView> {
        let transaction = self.transaction().await?;
        let row = self.create_verification_in(&*transaction, verification).await?;
        transaction.commit().await.map_err(database_error)?;
        Ok(row)
    }

    async fn get_verification(&self, identifier: &str, value: &str) -> AuthResult<Option<VerificationView>> {
        let transaction = self.transaction().await?;
        context(&*transaction, "med_tracker.identity_verification_identifier", identifier).await?;
        let row = VerificationRow::find_by_statement(statement("SELECT * FROM public.identity_verifications WHERE identifier=$1 AND value=$2 AND expires_at>CURRENT_TIMESTAMP ORDER BY created_at DESC, id DESC LIMIT 1", [identifier.into(), value.into()])).one(&*transaction).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        Ok(row.map(Into::into))
    }

    async fn get_verification_by_value(&self, value: &str) -> AuthResult<Option<VerificationView>> {
        self.verification("med_tracker.identity_verification_value", value, "SELECT * FROM public.identity_verifications WHERE value=$1 AND expires_at>CURRENT_TIMESTAMP ORDER BY created_at DESC, id DESC LIMIT 1").await
    }

    async fn get_verification_by_identifier(&self, identifier: &str) -> AuthResult<Option<VerificationView>> {
        self.verification("med_tracker.identity_verification_identifier", identifier, "SELECT * FROM public.identity_verifications WHERE identifier=$1 AND expires_at>CURRENT_TIMESTAMP ORDER BY created_at DESC, id DESC LIMIT 1").await
    }

    async fn consume_verification(&self, identifier: &str, value: &str) -> AuthResult<Option<VerificationView>> {
        let transaction = self.transaction().await?;
        context(&*transaction, "med_tracker.identity_verification_identifier", identifier).await?;
        let row = VerificationRow::find_by_statement(statement("DELETE FROM public.identity_verifications WHERE id=(SELECT id FROM public.identity_verifications WHERE identifier=$1 AND value=$2 ORDER BY created_at DESC, id DESC LIMIT 1 FOR UPDATE) RETURNING *", [identifier.into(), value.into()])).one(&*transaction).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        Ok(row.filter(|row| row.expires_at>Utc::now()).map(Into::into))
    }

    async fn consume_verification_by_identifier(&self, identifier: &str) -> AuthResult<Option<VerificationView>> {
        let transaction = self.transaction().await?;
        context(&*transaction, "med_tracker.identity_verification_identifier", identifier).await?;
        let rows = VerificationRow::find_by_statement(statement("DELETE FROM public.identity_verifications WHERE identifier=$1 RETURNING *", [identifier.into()])).all(&*transaction).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        Ok(rows.into_iter().max_by(|left,right| (left.created_at, &left.id).cmp(&(right.created_at, &right.id))).filter(|row| row.expires_at>Utc::now()).map(Into::into))
    }

    async fn delete_verification(&self, id: &str) -> AuthResult<()> {
        let transaction = self.transaction().await?;
        context(&*transaction, "med_tracker.identity_verification_id", id).await?;
        transaction.execute_raw(statement("DELETE FROM public.identity_verifications WHERE id=$1", [id.into()])).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)
    }

    async fn delete_expired_verifications(&self) -> AuthResult<usize> {
        let transaction = self.transaction().await?;
        let deleted = transaction.query_one_raw(statement("SELECT public.identity_delete_expired_verifications() AS deleted", [])).await.map_err(database_error)?.ok_or_else(|| better_auth_core::AuthError::internal("Verification cleanup unavailable"))?;
        let count: i64 = deleted.try_get("", "deleted").map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        usize::try_from(count).map_err(|_| better_auth_core::AuthError::internal("Verification cleanup count overflow"))
    }
}
