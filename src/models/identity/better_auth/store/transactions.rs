use async_trait::async_trait;
use better_auth_core::{
    AuthResult, CreateAccount, CreateSession, CreateUser,
    store::{AuthTransaction, BoxedTransactionValue, TransactionStore, TransactionWork},
    wire::{AccountView, SessionView, UserView},
};
use sea_orm::DatabaseTransaction;

use super::super::ClinicalAuthSchema;
use super::{ClinicalStore, database_error};

struct ClinicalTransaction<'a> {
    store: &'a ClinicalStore,
    transaction: &'a DatabaseTransaction,
}

#[async_trait]
impl AuthTransaction<ClinicalAuthSchema> for ClinicalTransaction<'_> {
    async fn create_user(&self, input: CreateUser) -> AuthResult<UserView> {
        self.store.create_user_in(self.transaction, input).await
    }

    async fn create_account(&self, input: CreateAccount) -> AuthResult<AccountView> {
        self.store.create_provider_in(self.transaction, input).await
    }

    async fn create_session(&self, input: CreateSession) -> AuthResult<SessionView> {
        self.store.create_session_in(self.transaction, input).await
    }
}

#[async_trait]
impl TransactionStore<ClinicalAuthSchema> for ClinicalStore {
    async fn transaction_boxed(
        &self,
        work: Box<TransactionWork<ClinicalAuthSchema>>,
    ) -> AuthResult<BoxedTransactionValue> {
        let transaction = self.transaction().await?;
        let scoped = ClinicalTransaction {
            store: self,
            transaction: &transaction,
        };
        match work(&scoped).await {
            Ok(value) => {
                transaction.commit().await.map_err(database_error)?;
                Ok(value)
            }
            Err(error) => {
                transaction.rollback().await.map_err(database_error)?;
                Err(error)
            }
        }
    }
}
