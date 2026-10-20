use async_trait::async_trait;
use better_auth_core::{
    AuthError, AuthResult, CreateAccount, FieldDate, SchemaValue, UpdateAccount,
    store::AccountStore, wire::AccountView,
};
use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, DatabaseTransaction, FromQueryResult};

use super::super::ClinicalAuthSchema;
use super::{ClinicalStore, clinical_id, context, database_error, statement};

#[derive(FromQueryResult)]
struct ProviderRow {
    id: String,
    account_id: i64,
    provider_id: String,
    provider_account_id: String,
    access_token: Option<String>,
    refresh_token: Option<String>,
    id_token: Option<String>,
    access_token_expires_at: Option<DateTime<Utc>>,
    refresh_token_expires_at: Option<DateTime<Utc>>,
    scope: Option<String>,
    password: Option<String>,
    disabled_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<ProviderRow> for AccountView {
    fn from(row: ProviderRow) -> Self {
        Self {
            field_order: Vec::new(),
            id: row.id.into(),
            account_id: row.provider_account_id.into(),
            provider_id: row.provider_id.into(),
            user_id: row.account_id.to_string().into(),
            access_token: row.access_token.into(),
            refresh_token: row.refresh_token.into(),
            id_token: row.id_token.into(),
            access_token_expires_at: row.access_token_expires_at.map(FieldDate::from).into(),
            refresh_token_expires_at: row.refresh_token_expires_at.map(FieldDate::from).into(),
            scope: row.scope.into(),
            password: row.password.into(),
            created_at: row.created_at.into(),
            updated_at: row.updated_at.into(),
            additional_fields: Default::default(),
        }
    }
}

fn optional_date(value: &SchemaValue<Option<FieldDate>>) -> AuthResult<Option<DateTime<Utc>>> {
    match value {
        SchemaValue::Undefined => Ok(None),
        SchemaValue::Typed(value) => value
            .as_ref()
            .map(|date| {
                date.to_datetime().and_then(|date| {
                    date.ok_or_else(|| AuthError::validation("Invalid account expiry"))
                })
            })
            .transpose(),
        SchemaValue::Dynamic(_) => Err(AuthError::validation("Unsupported account expiry")),
    }
}

fn optional_text(value: &SchemaValue<Option<String>>) -> AuthResult<Option<String>> {
    match value {
        SchemaValue::Undefined => Ok(None),
        SchemaValue::Typed(value) => Ok(value.clone()),
        SchemaValue::Dynamic(_) => Err(AuthError::validation("Unsupported account field value")),
    }
}

impl ClinicalStore {
    pub(crate) async fn provider_enabled(&self, id: &str) -> AuthResult<bool> {
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.identity_provider_row_id", id).await?;
        let row = transaction.query_one_raw(statement("SELECT disabled_at IS NULL AS enabled FROM public.identity_provider_accounts WHERE id=$1", [id.into()])).await.map_err(database_error)?.ok_or(AuthError::InvalidCredentials)?;
        let enabled = row.try_get("", "enabled").map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        Ok(enabled)
    }

    pub(crate) async fn set_provider_enabled(&self, id: &str, enabled: bool) -> AuthResult<()> {
        let actor = super::super::request::authenticated()?;
        let transaction = self.account_transaction().await?;
        let changed = transaction.execute_raw(statement("UPDATE public.identity_provider_accounts SET disabled_at=CASE WHEN $3 THEN NULL ELSE clock_timestamp() END,updated_at=clock_timestamp() WHERE id=$1 AND account_id=$2 AND provider_id='zitadel'", [id.into(), actor.account_id.into(), enabled.into()])).await.map_err(database_error)?.rows_affected();
        if changed != 1 {
            return Err(AuthError::InvalidCredentials);
        }
        transaction.commit().await.map_err(database_error)
    }

    pub(crate) async fn provider_identity(
        &self,
        provider: &str,
        identity: &str,
    ) -> AuthResult<Option<AccountView>> {
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.identity_provider_id", provider).await?;
        context(
            &transaction,
            "med_tracker.identity_provider_account_id",
            identity,
        )
        .await?;
        let row = ProviderRow::find_by_statement(statement("SELECT * FROM public.identity_provider_accounts WHERE provider_id=$1 AND provider_account_id=$2", [provider.into(), identity.into()])).one(&*transaction).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        Ok(row.map(Into::into))
    }

    pub(super) async fn create_provider_in(
        &self,
        transaction: &DatabaseTransaction,
        input: CreateAccount,
    ) -> AuthResult<AccountView> {
        let user_id = input.user_id.typed()?.as_str();
        let account_id = clinical_id(user_id)?;
        context(transaction, "med_tracker.current_account_id", user_id).await?;
        let account = transaction
            .query_one_raw(statement(
                "SELECT id FROM public.accounts WHERE id=$1 AND status IN (1,2) FOR UPDATE",
                [account_id.into()],
            ))
            .await
            .map_err(database_error)?;
        if account.is_none() {
            return Err(AuthError::Unauthenticated);
        }
        let row = ProviderRow::find_by_statement(statement("INSERT INTO public.identity_provider_accounts(id,account_id,provider_id,provider_account_id,access_token,refresh_token,id_token,access_token_expires_at,refresh_token_expires_at,scope,password) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) RETURNING *", [uuid::Uuid::new_v4().to_string().into(), account_id.into(), input.provider_id.typed()?.clone().into(), input.account_id.typed()?.clone().into(), optional_text(&input.access_token)?.into(), optional_text(&input.refresh_token)?.into(), optional_text(&input.id_token)?.into(), optional_date(&input.access_token_expires_at)?.into(), optional_date(&input.refresh_token_expires_at)?.into(), optional_text(&input.scope)?.into(), optional_text(&input.password)?.into()])).one(transaction).await.map_err(database_error)?.ok_or_else(|| AuthError::internal("Credential was not stored"))?;
        Ok(row.into())
    }

    async fn provider_transaction(&self, id: &str) -> AuthResult<super::Transaction> {
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.identity_provider_row_id", id).await?;
        let row = ProviderRow::find_by_statement(statement(
            "SELECT * FROM public.identity_provider_accounts WHERE id=$1",
            [id.into()],
        ))
        .one(&*transaction)
        .await
        .map_err(database_error)?
        .ok_or_else(|| AuthError::not_found("Credential not found"))?;
        context(
            &transaction,
            "med_tracker.current_account_id",
            &row.account_id.to_string(),
        )
        .await?;
        let account = transaction
            .query_one_raw(statement(
                "SELECT id FROM public.accounts WHERE id=$1 AND status IN (1,2) FOR UPDATE",
                [row.account_id.into()],
            ))
            .await
            .map_err(database_error)?;
        if account.is_none() {
            return Err(AuthError::Unauthenticated);
        }
        Ok(transaction)
    }
}

#[async_trait]
impl AccountStore<ClinicalAuthSchema> for ClinicalStore {
    async fn create_account(&self, input: CreateAccount) -> AuthResult<AccountView> {
        let transaction = self.transaction().await?;
        let account = self.create_provider_in(&transaction, input).await?;
        transaction.commit().await.map_err(database_error)?;
        Ok(account)
    }

    async fn get_account(
        &self,
        provider: &str,
        provider_account_id: &str,
    ) -> AuthResult<Option<AccountView>> {
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.identity_provider_id", provider).await?;
        context(
            &transaction,
            "med_tracker.identity_provider_account_id",
            provider_account_id,
        )
        .await?;
        let row = ProviderRow::find_by_statement(statement("SELECT p.* FROM public.identity_provider_accounts p JOIN public.accounts a ON a.id=p.account_id WHERE p.provider_id=$1 AND p.provider_account_id=$2 AND p.disabled_at IS NULL AND a.status IN (1,2)", [provider.into(), provider_account_id.into()])).one(&*transaction).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        Ok(row.map(Into::into))
    }

    async fn get_user_accounts(&self, user_id: &str) -> AuthResult<Vec<AccountView>> {
        let account_id = clinical_id(user_id)?;
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.current_account_id", user_id).await?;
        let rows = ProviderRow::find_by_statement(statement("SELECT p.* FROM public.identity_provider_accounts p JOIN public.accounts a ON a.id=p.account_id WHERE p.account_id=$1 AND p.disabled_at IS NULL AND a.status IN (1,2) ORDER BY p.created_at DESC,p.id", [account_id.into()])).all(&*transaction).await.map_err(database_error)?;
        let mut accounts: Vec<AccountView> = rows.into_iter().map(Into::into).collect();
        if !accounts
            .iter()
            .any(|account| account.provider_id == "credential")
        {
            let row = transaction
                .query_one_raw(statement(
                    "SELECT password_hash FROM public.accounts WHERE id=$1 AND status=2 FOR UPDATE",
                    [account_id.into()],
                ))
                .await
                .map_err(database_error)?;
            if let Some(row) = row
                && let Some(password) = row
                    .try_get::<Option<String>>("", "password_hash")
                    .map_err(database_error)?
            {
                let existing = ProviderRow::find_by_statement(statement("SELECT * FROM public.identity_provider_accounts WHERE account_id=$1 AND provider_id='credential' AND provider_account_id=$2", [account_id.into(), user_id.into()])).one(&*transaction).await.map_err(database_error)?;
                let credential = if let Some(existing) = existing {
                    (existing.disabled_at.is_none()).then(|| existing.into())
                } else {
                    Some(
                        self.create_provider_in(
                            &transaction,
                            CreateAccount {
                                user_id: user_id.into(),
                                account_id: user_id.into(),
                                provider_id: "credential".into(),
                                password: Some(password).into(),
                                ..Default::default()
                            },
                        )
                        .await?,
                    )
                };
                if let Some(credential) = credential {
                    accounts.push(credential);
                }
            }
        }
        transaction.commit().await.map_err(database_error)?;
        Ok(accounts)
    }

    async fn get_credential_account(&self, user_id: &str) -> AuthResult<Option<AccountView>> {
        let account_id = clinical_id(user_id)?;
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.current_account_id", user_id).await?;
        let existing = ProviderRow::find_by_statement(statement("SELECT p.* FROM public.identity_provider_accounts p JOIN public.accounts a ON a.id=p.account_id WHERE p.account_id=$1 AND p.provider_id='credential' AND p.provider_account_id=$2 AND a.status=2 LIMIT 1", [account_id.into(), user_id.into()])).one(&*transaction).await.map_err(database_error)?;
        let account = if let Some(existing) = existing {
            (existing.disabled_at.is_none()).then(|| existing.into())
        } else {
            let row = transaction
                .query_one_raw(statement(
                    "SELECT password_hash FROM public.accounts WHERE id=$1 AND status=2 FOR UPDATE",
                    [account_id.into()],
                ))
                .await
                .map_err(database_error)?;
            if let Some(Some(password)) = row
                .map(|row| row.try_get::<Option<String>>("", "password_hash"))
                .transpose()
                .map_err(database_error)?
            {
                Some(
                    self.create_provider_in(
                        &transaction,
                        CreateAccount {
                            user_id: user_id.into(),
                            account_id: user_id.into(),
                            provider_id: "credential".into(),
                            password: Some(password).into(),
                            ..Default::default()
                        },
                    )
                    .await?,
                )
            } else {
                None
            }
        };
        transaction.commit().await.map_err(database_error)?;
        Ok(account)
    }

    async fn update_account(&self, id: &str, input: UpdateAccount) -> AuthResult<AccountView> {
        if !input.id.is_undefined()
            || !input.account_id.is_undefined()
            || !input.provider_id.is_undefined()
            || !input.user_id.is_undefined()
            || !input.created_at.is_undefined()
            || !input.updated_at.is_undefined()
            || !input.additional_fields.is_empty()
        {
            return Err(AuthError::forbidden(
                "Credential ownership and identity cannot be changed",
            ));
        }
        let transaction = self.provider_transaction(id).await?;
        let row = ProviderRow::find_by_statement(statement("UPDATE public.identity_provider_accounts SET access_token=CASE WHEN $2 THEN $3 ELSE access_token END,refresh_token=CASE WHEN $4 THEN $5 ELSE refresh_token END,id_token=CASE WHEN $6 THEN $7 ELSE id_token END,access_token_expires_at=CASE WHEN $8 THEN $9 ELSE access_token_expires_at END,refresh_token_expires_at=CASE WHEN $10 THEN $11 ELSE refresh_token_expires_at END,scope=CASE WHEN $12 THEN $13 ELSE scope END,password=CASE WHEN $14 THEN $15 ELSE password END,updated_at=CURRENT_TIMESTAMP WHERE id=$1 RETURNING *", [id.into(), (!input.access_token.is_undefined()).into(), optional_text(&input.access_token)?.into(), (!input.refresh_token.is_undefined()).into(), optional_text(&input.refresh_token)?.into(), (!input.id_token.is_undefined()).into(), optional_text(&input.id_token)?.into(), (!input.access_token_expires_at.is_undefined()).into(), optional_date(&input.access_token_expires_at)?.into(), (!input.refresh_token_expires_at.is_undefined()).into(), optional_date(&input.refresh_token_expires_at)?.into(), (!input.scope.is_undefined()).into(), optional_text(&input.scope)?.into(), (!input.password.is_undefined()).into(), optional_text(&input.password)?.into()])).one(&*transaction).await.map_err(database_error)?.ok_or_else(|| AuthError::not_found("Credential not found"))?;
        transaction.commit().await.map_err(database_error)?;
        Ok(row.into())
    }

    async fn delete_account(&self, id: &str) -> AuthResult<()> {
        let transaction = self.provider_transaction(id).await?;
        transaction
            .execute_raw(statement(
                "DELETE FROM public.identity_provider_accounts WHERE id=$1",
                [id.into()],
            ))
            .await
            .map_err(database_error)?;
        transaction.commit().await.map_err(database_error)
    }
}
