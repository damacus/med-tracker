use async_trait::async_trait;
use better_auth_core::{
    AuthError, AuthResult, CreatePasskey, FieldMap, FieldValue, Passkey, PasskeyCredentialState,
    SchemaValue, UpdatePasskey, store::PasskeyStore, types::UpdatePasskeyAuthentication,
};
use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, FromQueryResult};

use super::{
    ClinicalStore, clinical_field_id, clinical_schema_id, context, database_error, statement,
};

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
            field_order: Vec::new(),
            additional_fields: FieldMap::new(),
            id: row.id.into(),
            user_id: row.account_id.to_string().into(),
            name: row.name.into(),
            public_key: row.public_key.into(),
            credential_id: row.credential_id.into(),
            counter: row
                .counter
                .parse::<u64>()
                .map_err(|_| AuthError::internal("Invalid stored passkey counter"))?
                .into(),
            device_type: row.device_type.into(),
            backed_up: row.backed_up.into(),
            transports: row.transports.into(),
            credential: row.credential.into(),
            aaguid: row.aaguid.into(),
            created_at: Some(row.created_at.into()).into(),
            updated_at: row.updated_at.into(),
        })
    }
}

impl ClinicalStore {
    pub(super) async fn create_passkey_in(
        &self,
        transaction: &sea_orm::DatabaseTransaction,
        input: CreatePasskey,
    ) -> AuthResult<Passkey> {
        if !input.additional_fields.is_empty() {
            return Err(AuthError::config(
                "Passkey extension fields are not configured",
            ));
        }
        let account_id = clinical_schema_id(&input.user_id)?;
        let credential = match input.credential {
            PasskeyCredentialState::Legacy(value) => value,
            PasskeyCredentialState::Native => {
                return Err(AuthError::config(
                    "Native passkey storage is not configured",
                ));
            }
        };
        context(
            transaction,
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
        let row = PasskeyRow::find_by_statement(statement(&format!("INSERT INTO public.identity_passkeys(id,account_id,name,public_key,credential_id,counter,device_type,backed_up,transports,credential,aaguid) VALUES($1,$2,$3,$4,$5,$6::text::numeric,$7,$8,$9,$10,$11) RETURNING {COLUMNS}"),[uuid::Uuid::new_v4().to_string().into(),account_id.into(),input.name.typed()?.clone().into(),input.public_key.into(),input.credential_id.into(),input.counter.to_string().into(),input.device_type.into(),input.backed_up.into(),input.transports.into(),credential.into(),input.aaguid.typed()?.clone().into()])).one(transaction).await.map_err(database_error)?.ok_or_else(||AuthError::internal("Passkey was not stored"))?;
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

    async fn get_passkey_by_credential_id_value(
        &self,
        value: &FieldValue,
    ) -> AuthResult<Option<Passkey>> {
        let id = value
            .as_str()
            .ok_or_else(|| AuthError::bad_request("Invalid passkey credential identifier"))?;
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

    async fn list_passkeys_by_user_value(&self, user_id: &FieldValue) -> AuthResult<Vec<Passkey>> {
        let id = clinical_field_id(user_id)?;
        let transaction = self.transaction().await?;
        context(
            &transaction,
            "med_tracker.current_account_id",
            &id.to_string(),
        )
        .await?;
        let rows=PasskeyRow::find_by_statement(statement(&format!("SELECT {COLUMNS} FROM public.identity_passkeys WHERE account_id=$1 ORDER BY created_at,id"),[id.into()])).all(&*transaction).await.map_err(database_error)?;
        let keys = rows
            .into_iter()
            .map(TryInto::try_into)
            .collect::<AuthResult<Vec<_>>>()?;
        transaction.commit().await.map_err(database_error)?;
        Ok(keys)
    }

    async fn update_passkey_authentication_optional(
        &self,
        id: &SchemaValue<String>,
        input: UpdatePasskeyAuthentication,
    ) -> AuthResult<Option<Passkey>> {
        let UpdatePasskeyAuthentication::Legacy {
            credential,
            counter,
            backed_up,
            device_type,
        } = input
        else {
            return Err(AuthError::config(
                "Native passkey authentication is not configured",
            ));
        };
        let id = id.typed()?;
        let (transaction, existing) = self.passkey_transaction(id).await?;
        let Some(existing) = existing else {
            return Ok(None);
        };
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
        let row = PasskeyRow::find_by_statement(statement(&format!("UPDATE public.identity_passkeys SET credential=$2,counter=$3::text::numeric,backed_up=$4,device_type=$5,updated_at=CURRENT_TIMESTAMP WHERE id=$1 AND (counter<$3::text::numeric OR (counter=0 AND $3::text::numeric=0)) RETURNING {COLUMNS}"),[id.into(),credential.into(),counter.to_string().into(),backed_up.into(),device_type.into()])).one(&*transaction).await.map_err(database_error)?;
        let key = row.map(TryInto::try_into).transpose()?;
        transaction.commit().await.map_err(database_error)?;
        Ok(key)
    }

    async fn update_passkey_optional(
        &self,
        _id: &SchemaValue<String>,
        _update: UpdatePasskey,
    ) -> AuthResult<Option<Passkey>> {
        Err(AuthError::config(
            "Native passkey display updates are not configured",
        ))
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
