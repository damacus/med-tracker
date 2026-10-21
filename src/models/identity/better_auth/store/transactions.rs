use async_trait::async_trait;
use better_auth_core::{
    AuthError, AuthResult,
    store::{BoxedTransactionValue, TransactionStore, TransactionWork},
};

use super::{super::ClinicalAuthSchema, ClinicalStore};

#[async_trait]
impl TransactionStore<ClinicalAuthSchema> for ClinicalStore {
    async fn transaction_boxed(
        &self,
        _work: Box<TransactionWork<ClinicalAuthSchema>>,
    ) -> AuthResult<BoxedTransactionValue> {
        Err(AuthError::config(
            "Native transactional plugin operations are not configured",
        ))
    }
}
