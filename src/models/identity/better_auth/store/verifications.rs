use async_trait::async_trait;
use better_auth_core::{
    AuthError, AuthResult, CreateVerification, SchemaValue,
    store::{VerificationStore, database_hooks::VerificationUpdate},
    wire::VerificationView,
};
use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, FromQueryResult};

use super::super::ClinicalAuthSchema;
use super::{ClinicalStore, context, database_error, statement};

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
        Self {
            field_order: Vec::new(),
            id: row.id.into(),
            identifier: row.identifier.into(),
            value: row.value.into(),
            expires_at: row.expires_at.into(),
            created_at: row.created_at.into(),
            updated_at: row.updated_at.into(),
            additional_fields: Default::default(),
        }
    }
}

impl ClinicalStore {
    pub(crate) async fn create_verification_in(
        &self,
        transaction: &sea_orm::DatabaseTransaction,
        verification: CreateVerification,
    ) -> AuthResult<VerificationView> {
        context(
            transaction,
            "med_tracker.identity_verification_identifier",
            verification.identifier.typed()?,
        )
        .await?;
        let id = match verification.id {
            SchemaValue::Undefined => uuid::Uuid::new_v4().to_string(),
            value => value.typed()?.clone(),
        };
        let expires_at = verification
            .expires_at
            .typed()?
            .to_datetime()?
            .ok_or_else(|| AuthError::validation("Invalid verification expiry"))?;
        let row = VerificationRow::find_by_statement(statement("INSERT INTO public.identity_verifications (id, identifier, value, expires_at) VALUES ($1, $2, $3, $4) RETURNING *", [id.into(), verification.identifier.typed()?.clone().into(), verification.value.typed()?.clone().into(), expires_at.into()])).one(transaction).await.map_err(database_error)?.ok_or_else(|| better_auth_core::AuthError::internal("Verification was not stored"))?;
        Ok(row.into())
    }

    async fn verification(
        &self,
        key: &str,
        value: &str,
        sql: &str,
    ) -> AuthResult<Option<VerificationView>> {
        let transaction = self.transaction().await?;
        context(&transaction, key, value).await?;
        let row = VerificationRow::find_by_statement(statement(sql, [value.into()]))
            .one(&*transaction)
            .await
            .map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        Ok(row.map(Into::into))
    }
}

#[async_trait]
impl VerificationStore<ClinicalAuthSchema> for ClinicalStore {
    async fn reserve_verification(
        &self,
        id: &str,
        verification: CreateVerification,
    ) -> AuthResult<bool> {
        let transaction = self.transaction().await?;
        let identifier = verification.identifier.typed()?.clone();
        let value = verification.value.typed()?.clone();
        let expiry = verification
            .expires_at
            .typed()?
            .to_datetime()?
            .ok_or_else(|| AuthError::validation("Invalid verification expiry"))?;
        context(
            &transaction,
            "med_tracker.identity_verification_identifier",
            &identifier,
        )
        .await?;
        let inserted = transaction.execute_raw(statement("INSERT INTO public.identity_verifications(id,identifier,value,expires_at) VALUES($1,$2,$3,$4) ON CONFLICT(id) DO NOTHING", [id.into(), identifier.clone().into(), value.into(), expiry.into()])).await.map_err(database_error)?.rows_affected() == 1;
        if !inserted
            && transaction
                .query_one_raw(statement(
                    "SELECT id FROM public.identity_verifications WHERE id=$1 AND identifier=$2",
                    [id.into(), identifier.into()],
                ))
                .await
                .map_err(database_error)?
                .is_none()
        {
            return Err(AuthError::Conflict(
                "Verification reservation collided with a different identifier".into(),
            ));
        }
        transaction.commit().await.map_err(database_error)?;
        Ok(inserted)
    }

    async fn get_verification_including_expired(
        &self,
        identifier: &str,
    ) -> AuthResult<Option<VerificationView>> {
        self.verification("med_tracker.identity_verification_identifier", identifier, "SELECT * FROM public.identity_verifications WHERE identifier=$1 ORDER BY created_at DESC,id DESC LIMIT 1").await
    }

    async fn update_verification(
        &self,
        identifier: &str,
        update: VerificationUpdate,
    ) -> AuthResult<Option<VerificationView>> {
        if !update.id.is_undefined()
            || !update.identifier.is_undefined()
            || !update.created_at.is_undefined()
            || !update.updated_at.is_undefined()
            || !update.additional_fields.is_empty()
        {
            return Err(AuthError::forbidden(
                "Verification identity cannot be changed",
            ));
        }
        let transaction = self.transaction().await?;
        context(
            &transaction,
            "med_tracker.identity_verification_identifier",
            identifier,
        )
        .await?;
        let value = match &update.value {
            SchemaValue::Undefined => None,
            other => Some(other.typed()?.clone()),
        };
        let expiry = match &update.expires_at {
            SchemaValue::Undefined => None,
            other => Some(
                other
                    .typed()?
                    .to_datetime()?
                    .ok_or_else(|| AuthError::validation("Invalid verification expiry"))?,
            ),
        };
        let row = VerificationRow::find_by_statement(statement("UPDATE public.identity_verifications SET value=COALESCE($2,value),expires_at=COALESCE($3,expires_at),updated_at=clock_timestamp() WHERE id=(SELECT id FROM public.identity_verifications WHERE identifier=$1 ORDER BY created_at DESC,id DESC LIMIT 1 FOR UPDATE) RETURNING *", [identifier.into(), value.into(), expiry.into()])).one(&*transaction).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        Ok(row.map(Into::into))
    }

    async fn update_verification_by_identifier(
        &self,
        identifier: &str,
        value: Option<String>,
        expires_at: Option<DateTime<Utc>>,
    ) -> AuthResult<()> {
        let transaction = self.transaction().await?;
        context(
            &transaction,
            "med_tracker.identity_verification_identifier",
            identifier,
        )
        .await?;
        transaction.execute_raw(statement("UPDATE public.identity_verifications SET value=COALESCE($2,value),expires_at=COALESCE($3,expires_at),updated_at=clock_timestamp() WHERE identifier=$1", [identifier.into(),value.into(),expires_at.into()])).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)
    }

    async fn delete_verification_by_identifier(&self, identifier: &str) -> AuthResult<()> {
        let transaction = self.transaction().await?;
        context(
            &transaction,
            "med_tracker.identity_verification_identifier",
            identifier,
        )
        .await?;
        transaction
            .execute_raw(statement(
                "DELETE FROM public.identity_verifications WHERE identifier=$1",
                [identifier.into()],
            ))
            .await
            .map_err(database_error)?;
        transaction.commit().await.map_err(database_error)
    }

    async fn create_verification(
        &self,
        verification: CreateVerification,
    ) -> AuthResult<VerificationView> {
        let transaction = self.transaction().await?;
        let row = self
            .create_verification_in(&transaction, verification)
            .await?;
        transaction.commit().await.map_err(database_error)?;
        Ok(row)
    }

    async fn get_verification(
        &self,
        identifier: &str,
        value: &str,
    ) -> AuthResult<Option<VerificationView>> {
        let transaction = self.transaction().await?;
        context(
            &transaction,
            "med_tracker.identity_verification_identifier",
            identifier,
        )
        .await?;
        let row = VerificationRow::find_by_statement(statement("SELECT * FROM public.identity_verifications WHERE identifier=$1 AND value=$2 AND expires_at>CURRENT_TIMESTAMP ORDER BY created_at DESC, id DESC LIMIT 1", [identifier.into(), value.into()])).one(&*transaction).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        Ok(row.map(Into::into))
    }

    async fn get_verification_by_value(&self, value: &str) -> AuthResult<Option<VerificationView>> {
        self.verification("med_tracker.identity_verification_value", value, "SELECT * FROM public.identity_verifications WHERE value=$1 AND expires_at>CURRENT_TIMESTAMP ORDER BY created_at DESC, id DESC LIMIT 1").await
    }

    async fn get_verification_by_identifier(
        &self,
        identifier: &str,
    ) -> AuthResult<Option<VerificationView>> {
        self.verification("med_tracker.identity_verification_identifier", identifier, "SELECT * FROM public.identity_verifications WHERE identifier=$1 AND expires_at>CURRENT_TIMESTAMP ORDER BY created_at DESC, id DESC LIMIT 1").await
    }

    async fn consume_verification(
        &self,
        identifier: &str,
        value: &str,
    ) -> AuthResult<Option<VerificationView>> {
        let transaction = self.transaction().await?;
        context(
            &transaction,
            "med_tracker.identity_verification_identifier",
            identifier,
        )
        .await?;
        let row = VerificationRow::find_by_statement(statement("DELETE FROM public.identity_verifications WHERE id=(SELECT id FROM public.identity_verifications WHERE identifier=$1 AND value=$2 ORDER BY created_at DESC, id DESC LIMIT 1 FOR UPDATE) RETURNING *", [identifier.into(), value.into()])).one(&*transaction).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        Ok(row
            .filter(|row| row.expires_at > Utc::now())
            .map(Into::into))
    }

    async fn consume_verification_by_identifier(
        &self,
        identifier: &str,
    ) -> AuthResult<Option<VerificationView>> {
        let transaction = self.transaction().await?;
        context(
            &transaction,
            "med_tracker.identity_verification_identifier",
            identifier,
        )
        .await?;
        let rows = VerificationRow::find_by_statement(statement(
            "DELETE FROM public.identity_verifications WHERE identifier=$1 RETURNING *",
            [identifier.into()],
        ))
        .all(&*transaction)
        .await
        .map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        Ok(rows
            .into_iter()
            .max_by(|left, right| (left.created_at, &left.id).cmp(&(right.created_at, &right.id)))
            .filter(|row| row.expires_at > Utc::now())
            .map(Into::into))
    }

    async fn consume_verification_including_expired(
        &self,
        identifier: &str,
    ) -> AuthResult<Option<VerificationView>> {
        let transaction = self.transaction().await?;
        context(
            &transaction,
            "med_tracker.identity_verification_identifier",
            identifier,
        )
        .await?;
        let rows = VerificationRow::find_by_statement(statement(
            "DELETE FROM public.identity_verifications WHERE identifier=$1 RETURNING *",
            [identifier.into()],
        ))
        .all(&*transaction)
        .await
        .map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        Ok(rows
            .into_iter()
            .max_by(|left, right| (left.created_at, &left.id).cmp(&(right.created_at, &right.id)))
            .map(Into::into))
    }

    async fn delete_verification(&self, id: &str) -> AuthResult<()> {
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.identity_verification_id", id).await?;
        transaction
            .execute_raw(statement(
                "DELETE FROM public.identity_verifications WHERE id=$1",
                [id.into()],
            ))
            .await
            .map_err(database_error)?;
        transaction.commit().await.map_err(database_error)
    }

    async fn delete_expired_verifications(&self) -> AuthResult<usize> {
        let transaction = self.transaction().await?;
        let deleted = transaction
            .query_one_raw(statement(
                "SELECT public.identity_delete_expired_verifications() AS deleted",
                [],
            ))
            .await
            .map_err(database_error)?
            .ok_or_else(|| {
                better_auth_core::AuthError::internal("Verification cleanup unavailable")
            })?;
        let count: i64 = deleted.try_get("", "deleted").map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        usize::try_from(count).map_err(|_| {
            better_auth_core::AuthError::internal("Verification cleanup count overflow")
        })
    }
}
