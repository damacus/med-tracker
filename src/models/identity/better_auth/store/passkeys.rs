use async_trait::async_trait;
use better_auth_core::{
    AuthError, AuthResult, CreatePasskey, Passkey, store::PasskeyStore,
    types::UpdatePasskeyAuthentication,
};
use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, FromQueryResult};

use super::{ClinicalStore, clinical_id, context, database_error, statement};

const COLUMNS: &str = "id,account_id,name,public_key,credential_id,counter::text AS counter,device_type,backed_up,transports,credential,aaguid,created_at,updated_at";

#[derive(FromQueryResult)]
struct PasskeyRow {
    id: String,
    account_id: i64,
    name: Option<String>,
    public_key: String,
    credential_id: String,
    counter: String,
    device_type: String,
    backed_up: bool,
    transports: Option<String>,
    credential: String,
    aaguid: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<PasskeyRow> for Passkey {
    type Error = AuthError;
    fn try_from(row: PasskeyRow) -> AuthResult<Self> {
        Ok(Self {
            id: row.id,
            user_id: row.account_id.to_string(),
            name: row.name,
            public_key: row.public_key,
            credential_id: row.credential_id,
            counter: row
                .counter
                .parse()
                .map_err(|_| AuthError::internal("Invalid stored passkey counter"))?,
            device_type: row.device_type,
            backed_up: row.backed_up,
            transports: row.transports,
            credential: row.credential,
            aaguid: row.aaguid,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

impl ClinicalStore {
    pub(super) async fn create_passkey_in(
        &self,
        transaction: &sea_orm::DatabaseTransaction,
        input: CreatePasskey,
    ) -> AuthResult<Passkey> {
        let account_id = clinical_id(&input.user_id)?;
        context(
            transaction,
            "med_tracker.current_account_id",
            &input.user_id,
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
        let row = PasskeyRow::find_by_statement(statement(&format!("INSERT INTO public.identity_passkeys(id,account_id,name,public_key,credential_id,counter,device_type,backed_up,transports,credential,aaguid) VALUES($1,$2,$3,$4,$5,$6::text::numeric,$7,$8,$9,$10,$11) RETURNING {COLUMNS}"),[uuid::Uuid::new_v4().to_string().into(),account_id.into(),input.name.into(),input.public_key.into(),input.credential_id.into(),input.counter.to_string().into(),input.device_type.into(),input.backed_up.into(),input.transports.into(),input.credential.into(),input.aaguid.into()])).one(transaction).await.map_err(database_error)?.ok_or_else(||AuthError::internal("Passkey was not stored"))?;
        let passkey = row.try_into()?;
        self.audit(transaction, account_id, "webauthn_credential", "created")
            .await?;
        Ok(passkey)
    }

    async fn passkey_transaction(
        &self,
        id: &str,
    ) -> AuthResult<(super::Transaction, Option<PasskeyRow>)> {
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.identity_passkey_id", id).await?;
        let row = PasskeyRow::find_by_statement(statement(
            &format!("SELECT {COLUMNS} FROM public.identity_passkeys WHERE id=$1"),
            [id.into()],
        ))
        .one(&*transaction)
        .await
        .map_err(database_error)?;
        if let Some(row) = &row {
            context(
                &transaction,
                "med_tracker.current_account_id",
                &row.account_id.to_string(),
            )
            .await?;
        }
        Ok((transaction, row))
    }
}

#[async_trait]
impl PasskeyStore for ClinicalStore {
    async fn create_passkey(&self, input: CreatePasskey) -> AuthResult<Passkey> {
        let transaction = self.transaction().await?;
        let passkey = self.create_passkey_in(&transaction, input).await?;
        transaction.commit().await.map_err(database_error)?;
        Ok(passkey)
    }

    async fn get_passkey_by_id(&self, id: &str) -> AuthResult<Option<Passkey>> {
        let (transaction, row) = self.passkey_transaction(id).await?;
        let passkey = row.map(TryInto::try_into).transpose()?;
        transaction.commit().await.map_err(database_error)?;
        Ok(passkey)
    }

    async fn get_passkey_by_credential_id(&self, id: &str) -> AuthResult<Option<Passkey>> {
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.identity_credential_id", id).await?;
        if let Some(owner) = transaction
            .query_one_raw(statement(
                "SELECT account_id FROM public.identity_passkeys WHERE credential_id=$1",
                [id.into()],
            ))
            .await
            .map_err(database_error)?
        {
            let account_id: i64 = owner.try_get("", "account_id").map_err(database_error)?;
            context(
                &transaction,
                "med_tracker.current_account_id",
                &account_id.to_string(),
            )
            .await?;
            transaction
                .query_one_raw(statement(
                    "SELECT id FROM public.accounts WHERE id=$1 FOR UPDATE",
                    [account_id.into()],
                ))
                .await
                .map_err(database_error)?;
        }
        let row = PasskeyRow::find_by_statement(statement(
            &format!("SELECT {COLUMNS} FROM public.identity_passkeys WHERE credential_id=$1"),
            [id.into()],
        ))
        .one(&*transaction)
        .await
        .map_err(database_error)?;
        let passkey = row.map(TryInto::try_into).transpose()?;
        transaction.commit().await.map_err(database_error)?;
        Ok(passkey)
    }

    async fn list_passkeys_by_user(&self, user_id: &str) -> AuthResult<Vec<Passkey>> {
        let id = clinical_id(user_id)?;
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.current_account_id", user_id).await?;
        let rows=PasskeyRow::find_by_statement(statement(&format!("SELECT {COLUMNS} FROM public.identity_passkeys WHERE account_id=$1 ORDER BY created_at,id"),[id.into()])).all(&*transaction).await.map_err(database_error)?;
        let keys = rows
            .into_iter()
            .map(TryInto::try_into)
            .collect::<AuthResult<Vec<_>>>()?;
        transaction.commit().await.map_err(database_error)?;
        Ok(keys)
    }

    async fn update_passkey_authentication(
        &self,
        id: &str,
        input: UpdatePasskeyAuthentication,
    ) -> AuthResult<Passkey> {
        let (transaction, existing) = self.passkey_transaction(id).await?;
        let existing = existing.ok_or_else(|| AuthError::not_found("Passkey not found"))?;
        let account = transaction
            .query_one_raw(statement(
                "SELECT id FROM public.accounts WHERE id=$1 AND status=2 FOR UPDATE",
                [existing.account_id.into()],
            ))
            .await
            .map_err(database_error)?;
        if account.is_none() {
            return Err(AuthError::Unauthenticated);
        }
        let row=PasskeyRow::find_by_statement(statement(&format!("UPDATE public.identity_passkeys SET credential=$2,counter=$3::text::numeric,backed_up=$4,device_type=$5,updated_at=CURRENT_TIMESTAMP WHERE id=$1 RETURNING {COLUMNS}"),[id.into(),input.credential.into(),input.counter.to_string().into(),input.backed_up.into(),input.device_type.into()])).one(&*transaction).await.map_err(database_error)?.ok_or_else(||AuthError::not_found("Passkey not found"))?;
        let key = row.try_into()?;
        transaction.commit().await.map_err(database_error)?;
        Ok(key)
    }

    async fn update_passkey_name(&self, id: &str, name: &str) -> AuthResult<Passkey> {
        let (transaction, existing) = self.passkey_transaction(id).await?;
        existing.ok_or_else(|| AuthError::not_found("Passkey not found"))?;
        let row=PasskeyRow::find_by_statement(statement(&format!("UPDATE public.identity_passkeys SET name=$2,updated_at=CURRENT_TIMESTAMP WHERE id=$1 RETURNING {COLUMNS}"),[id.into(),name.into()])).one(&*transaction).await.map_err(database_error)?.ok_or_else(||AuthError::not_found("Passkey not found"))?;
        let key = row.try_into()?;
        transaction.commit().await.map_err(database_error)?;
        Ok(key)
    }

    async fn delete_passkey(&self, id: &str) -> AuthResult<()> {
        let (transaction, existing) = self.passkey_transaction(id).await?;
        if let Some(existing) = existing {
            transaction
                .query_one_raw(statement(
                    "SELECT id FROM public.accounts WHERE id=$1 FOR UPDATE",
                    [existing.account_id.into()],
                ))
                .await
                .map_err(database_error)?;
            transaction
                .execute_raw(statement(
                    "DELETE FROM public.identity_passkeys WHERE id=$1",
                    [id.into()],
                ))
                .await
                .map_err(database_error)?;
        }
        transaction.commit().await.map_err(database_error)
    }
}
