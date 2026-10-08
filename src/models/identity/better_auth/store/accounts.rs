use async_trait::async_trait;
use better_auth_core::{
    AuthError, AuthResult, CreateAccount, UpdateAccount, store::AccountStore, wire::AccountView,
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
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<ProviderRow> for AccountView {
    fn from(row: ProviderRow) -> Self {
        Self {
            id: row.id,
            account_id: row.provider_account_id,
            provider_id: row.provider_id,
            user_id: row.account_id.to_string(),
            access_token: row.access_token,
            refresh_token: row.refresh_token,
            id_token: row.id_token,
            access_token_expires_at: row.access_token_expires_at,
            refresh_token_expires_at: row.refresh_token_expires_at,
            scope: row.scope,
            password: row.password,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
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
        let account_id = clinical_id(&input.user_id)?;
        context(
            transaction,
            "med_tracker.current_account_id",
            &input.user_id,
        )
        .await?;
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
        let row = ProviderRow::find_by_statement(statement("INSERT INTO public.identity_provider_accounts(id,account_id,provider_id,provider_account_id,access_token,refresh_token,id_token,access_token_expires_at,refresh_token_expires_at,scope,password) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) RETURNING *", [uuid::Uuid::new_v4().to_string().into(), account_id.into(), input.provider_id.into(), input.account_id.into(), input.access_token.into(), input.refresh_token.into(), input.id_token.into(), input.access_token_expires_at.into(), input.refresh_token_expires_at.into(), input.scope.into(), input.password.into()])).one(transaction).await.map_err(database_error)?.ok_or_else(|| AuthError::internal("Credential was not stored"))?;
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
                let existing = ProviderRow::find_by_statement(statement("SELECT * FROM public.identity_provider_accounts WHERE account_id=$1 AND provider_id='credential'", [account_id.into()])).one(&*transaction).await.map_err(database_error)?;
                let credential = if let Some(existing) = existing {
                    existing.into()
                } else {
                    self.create_provider_in(
                        &transaction,
                        CreateAccount {
                            user_id: user_id.into(),
                            account_id: user_id.into(),
                            provider_id: "credential".into(),
                            password: Some(password),
                            access_token: None,
                            refresh_token: None,
                            id_token: None,
                            access_token_expires_at: None,
                            refresh_token_expires_at: None,
                            scope: None,
                        },
                    )
                    .await?
                };
                accounts.push(credential);
            }
        }
        transaction.commit().await.map_err(database_error)?;
        Ok(accounts)
    }

    async fn update_account(&self, id: &str, input: UpdateAccount) -> AuthResult<AccountView> {
        let transaction = self.provider_transaction(id).await?;
        let row = ProviderRow::find_by_statement(statement("UPDATE public.identity_provider_accounts SET access_token=COALESCE($2,access_token),refresh_token=COALESCE($3,refresh_token),id_token=COALESCE($4,id_token),access_token_expires_at=COALESCE($5,access_token_expires_at),refresh_token_expires_at=COALESCE($6,refresh_token_expires_at),scope=COALESCE($7,scope),password=COALESCE($8,password),updated_at=CURRENT_TIMESTAMP WHERE id=$1 RETURNING *", [id.into(), input.access_token.into(), input.refresh_token.into(), input.id_token.into(), input.access_token_expires_at.into(), input.refresh_token_expires_at.into(), input.scope.into(), input.password.into()])).one(&*transaction).await.map_err(database_error)?.ok_or_else(|| AuthError::not_found("Credential not found"))?;
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
