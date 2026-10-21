use std::{
    future::Future,
    sync::{
        Arc, Weak,
        atomic::{AtomicBool, Ordering},
    },
};

use async_trait::async_trait;
use better_auth_core::{
    AuthError, AuthResult, CreateAccount, CreateSession, CreateUser, CreateVerification, FieldMap,
    FieldValue, SchemaValue, UpdateUser,
    store::{
        AuthTransaction, BoxedTransactionValue, DeviceCodeStore, JwksStore, TransactionStore,
        TransactionWork, TypedTransactionFuture, UserStore, VerificationStore, WalletStore,
    },
    wire::{AccountView, SessionView, UserView, VerificationView},
};

use super::super::ClinicalAuthSchema;
use super::{ClinicalStore, database_error, scope};

pub(super) struct ClinicalTransaction {
    pub(super) store: ClinicalStore,
    scope: Weak<scope::Scope>,
}

impl ClinicalTransaction {
    pub(super) fn active_scope(&self) -> AuthResult<Arc<scope::Scope>> {
        self.scope
            .upgrade()
            .ok_or_else(|| AuthError::internal("Identity transaction has finished"))
    }

    pub(super) async fn in_scope<T>(
        &self,
        work: impl Future<Output = AuthResult<T>>,
    ) -> AuthResult<T> {
        scope::TRANSACTION.scope(self.active_scope()?, work).await
    }
}

#[async_trait]
impl AuthTransaction<ClinicalAuthSchema> for ClinicalTransaction {
    fn clone_handle(&self) -> Arc<dyn AuthTransaction<ClinicalAuthSchema>> {
        Arc::new(Self {
            store: self.store.clone(),
            scope: self.scope.clone(),
        })
    }

    fn queue_after_commit(&self, effect: TypedTransactionFuture<'static, ()>) -> AuthResult<()> {
        let scope = self.active_scope()?;
        if scope.rollback_only.load(Ordering::Acquire) {
            return Err(AuthError::internal(
                "Identity transaction has been rolled back",
            ));
        }
        scope
            .after_commit
            .lock()
            .map_err(|_| AuthError::internal("Identity after-commit queue unavailable"))?
            .push(effect);
        Ok(())
    }

    async fn create_user(&self, input: CreateUser) -> AuthResult<UserView> {
        let scope = self.active_scope()?;
        self.store.create_user_in(&scope.transaction, input).await
    }

    async fn create_user_fields_optional(&self, input: FieldMap) -> AuthResult<Option<UserView>> {
        self.in_scope(
            UserStore::<ClinicalAuthSchema>::create_user_fields_optional(&self.store, input),
        )
        .await
    }

    async fn get_user_by_id(&self, id: &str) -> AuthResult<Option<UserView>> {
        self.in_scope(UserStore::<ClinicalAuthSchema>::get_user_by_id(
            &self.store,
            id,
        ))
        .await
    }

    async fn get_user_by_email(&self, email: &str) -> AuthResult<Option<UserView>> {
        self.in_scope(UserStore::<ClinicalAuthSchema>::get_user_by_email(
            &self.store,
            email,
        ))
        .await
    }

    async fn update_user_optional(
        &self,
        id: &str,
        update: UpdateUser,
    ) -> AuthResult<Option<UserView>> {
        self.in_scope(UserStore::<ClinicalAuthSchema>::update_user_optional(
            &self.store,
            id,
            update,
        ))
        .await
    }

    async fn update_user(&self, id: &str, update: UpdateUser) -> AuthResult<UserView> {
        self.in_scope(UserStore::<ClinicalAuthSchema>::update_user(
            &self.store,
            id,
            update,
        ))
        .await
    }

    async fn delete_user(&self, id: &str) -> AuthResult<()> {
        self.in_scope(UserStore::<ClinicalAuthSchema>::delete_user(
            &self.store,
            id,
        ))
        .await
    }

    async fn create_account(&self, input: CreateAccount) -> AuthResult<AccountView> {
        let scope = self.active_scope()?;
        self.store
            .create_provider_in(&scope.transaction, input)
            .await
    }

    async fn create_account_optional(
        &self,
        input: CreateAccount,
    ) -> AuthResult<Option<AccountView>> {
        self.create_account(input).await.map(Some)
    }

    async fn create_session(&self, input: CreateSession) -> AuthResult<SessionView> {
        let scope = self.active_scope()?;
        self.store
            .create_session_in(&scope.transaction, input)
            .await
    }

    async fn create_session_optional(
        &self,
        input: CreateSession,
    ) -> AuthResult<Option<SessionView>> {
        self.create_session(input).await.map(Some)
    }

    async fn create_verification_optional(
        &self,
        verification: CreateVerification,
    ) -> AuthResult<Option<VerificationView>> {
        self.create_verification(verification).await.map(Some)
    }

    async fn create_verification(
        &self,
        verification: CreateVerification,
    ) -> AuthResult<VerificationView> {
        let scope = self.active_scope()?;
        self.store
            .create_verification_in(&scope.transaction, verification)
            .await
    }

    async fn update_verification(
        &self,
        identifier: &str,
        update: better_auth_core::store::database_hooks::VerificationUpdate,
    ) -> AuthResult<Option<VerificationView>> {
        self.in_scope(
            VerificationStore::<ClinicalAuthSchema>::update_verification(
                &self.store,
                identifier,
                update,
            ),
        )
        .await
    }

    async fn get_verification_including_expired(
        &self,
        identifier: &str,
    ) -> AuthResult<Option<VerificationView>> {
        self.in_scope(
            VerificationStore::<ClinicalAuthSchema>::get_verification_including_expired(
                &self.store,
                identifier,
            ),
        )
        .await
    }

    async fn delete_verification_by_identifier(&self, identifier: &str) -> AuthResult<()> {
        self.in_scope(
            VerificationStore::<ClinicalAuthSchema>::delete_verification_by_identifier(
                &self.store,
                identifier,
            ),
        )
        .await
    }

    async fn delete_expired_verifications(&self) -> AuthResult<usize> {
        self.in_scope(
            VerificationStore::<ClinicalAuthSchema>::delete_expired_verifications(&self.store),
        )
        .await
    }

    async fn create_passkey(
        &self,
        input: better_auth_core::CreatePasskey,
    ) -> AuthResult<better_auth_core::Passkey> {
        self.in_scope(better_auth_core::store::PasskeyStore::create_passkey(
            &self.store,
            input,
        ))
        .await
    }
}

#[async_trait]
impl DeviceCodeStore for ClinicalTransaction {
    async fn create_device_code_record(&self, fields: FieldMap) -> AuthResult<Option<FieldMap>> {
        self.in_scope(DeviceCodeStore::create_device_code_record(
            &self.store,
            fields,
        ))
        .await
    }
    async fn get_device_code_record(
        &self,
        id: &SchemaValue<String>,
    ) -> AuthResult<Option<FieldMap>> {
        self.in_scope(DeviceCodeStore::get_device_code_record(&self.store, id))
            .await
    }
    async fn update_device_code_record(
        &self,
        id: &SchemaValue<String>,
        fields: FieldMap,
    ) -> AuthResult<Option<FieldMap>> {
        self.in_scope(DeviceCodeStore::update_device_code_record(
            &self.store,
            id,
            fields,
        ))
        .await
    }
    async fn create_device_code(
        &self,
        input: better_auth_core::CreateDeviceCode,
    ) -> AuthResult<better_auth_core::DeviceCode> {
        self.in_scope(DeviceCodeStore::create_device_code(&self.store, input))
            .await
    }
    async fn get_device_code_by_device_code(
        &self,
        code: &str,
    ) -> AuthResult<Option<better_auth_core::DeviceCode>> {
        self.in_scope(DeviceCodeStore::get_device_code_by_device_code(
            &self.store,
            code,
        ))
        .await
    }
    async fn get_device_code_by_user_code(
        &self,
        code: &str,
    ) -> AuthResult<Option<better_auth_core::DeviceCode>> {
        self.in_scope(DeviceCodeStore::get_device_code_by_user_code(
            &self.store,
            code,
        ))
        .await
    }
    async fn update_device_code(
        &self,
        id: &SchemaValue<String>,
        update: better_auth_core::UpdateDeviceCode,
    ) -> AuthResult<better_auth_core::DeviceCode> {
        self.in_scope(DeviceCodeStore::update_device_code(&self.store, id, update))
            .await
    }
    async fn update_device_code_if_status(
        &self,
        id: &SchemaValue<String>,
        status: &str,
        update: better_auth_core::UpdateDeviceCode,
    ) -> AuthResult<bool> {
        self.in_scope(DeviceCodeStore::update_device_code_if_status(
            &self.store,
            id,
            status,
            update,
        ))
        .await
    }
    async fn claim_device_code(
        &self,
        id: &SchemaValue<String>,
        user_id: &SchemaValue<String>,
    ) -> AuthResult<bool> {
        self.in_scope(DeviceCodeStore::claim_device_code(&self.store, id, user_id))
            .await
    }
    async fn consume_device_code(
        &self,
        expected: &better_auth_core::DeviceCode,
        ownership: &better_auth_core::DeviceCodeOwnership,
    ) -> AuthResult<Option<better_auth_core::DeviceCode>> {
        self.in_scope(DeviceCodeStore::consume_device_code(
            &self.store,
            expected,
            ownership,
        ))
        .await
    }
    async fn delete_device_code(&self, id: &SchemaValue<String>) -> AuthResult<()> {
        self.in_scope(DeviceCodeStore::delete_device_code(&self.store, id))
            .await
    }
    async fn delete_device_code_if_status(
        &self,
        id: &SchemaValue<String>,
        status: &str,
    ) -> AuthResult<bool> {
        self.in_scope(DeviceCodeStore::delete_device_code_if_status(
            &self.store,
            id,
            status,
        ))
        .await
    }
}

#[async_trait]
impl WalletStore for ClinicalTransaction {
    async fn create_wallet_address_record(&self, fields: FieldMap) -> AuthResult<Option<FieldMap>> {
        self.in_scope(WalletStore::create_wallet_address_record(
            &self.store,
            fields,
        ))
        .await
    }
    async fn get_wallet_address_record(
        &self,
        id: &SchemaValue<String>,
    ) -> AuthResult<Option<FieldMap>> {
        self.in_scope(WalletStore::get_wallet_address_record(&self.store, id))
            .await
    }
    async fn update_wallet_address_record(
        &self,
        id: &SchemaValue<String>,
        fields: FieldMap,
    ) -> AuthResult<Option<FieldMap>> {
        self.in_scope(WalletStore::update_wallet_address_record(
            &self.store,
            id,
            fields,
        ))
        .await
    }
    async fn delete_wallet_address_record(&self, id: &SchemaValue<String>) -> AuthResult<()> {
        self.in_scope(WalletStore::delete_wallet_address_record(&self.store, id))
            .await
    }
    async fn get_wallet_address_value(
        &self,
        address: &FieldValue,
        chain_id: Option<&FieldValue>,
    ) -> AuthResult<Option<better_auth_core::WalletAddress>> {
        self.in_scope(WalletStore::get_wallet_address_value(
            &self.store,
            address,
            chain_id,
        ))
        .await
    }
}

#[async_trait]
impl JwksStore for ClinicalTransaction {
    async fn create_jwk_record(&self, fields: FieldMap) -> AuthResult<Option<FieldMap>> {
        self.in_scope(JwksStore::create_jwk_record(&self.store, fields))
            .await
    }
    async fn get_jwk_record(&self, id: &SchemaValue<String>) -> AuthResult<Option<FieldMap>> {
        self.in_scope(JwksStore::get_jwk_record(&self.store, id))
            .await
    }
    async fn update_jwk_record(
        &self,
        id: &SchemaValue<String>,
        fields: FieldMap,
    ) -> AuthResult<Option<FieldMap>> {
        self.in_scope(JwksStore::update_jwk_record(&self.store, id, fields))
            .await
    }
    async fn delete_jwk_record(&self, id: &SchemaValue<String>) -> AuthResult<()> {
        self.in_scope(JwksStore::delete_jwk_record(&self.store, id))
            .await
    }
    async fn list_jwk_records(&self) -> AuthResult<Vec<FieldMap>> {
        self.in_scope(JwksStore::list_jwk_records(&self.store))
            .await
    }
}

#[async_trait]
impl TransactionStore<ClinicalAuthSchema> for ClinicalStore {
    async fn transaction_boxed(
        &self,
        work: Box<TransactionWork<ClinicalAuthSchema>>,
    ) -> AuthResult<BoxedTransactionValue> {
        if let Ok(scope) = scope::TRANSACTION.try_with(Arc::clone) {
            let transaction = ClinicalTransaction {
                store: self.clone(),
                scope: Arc::downgrade(&scope),
            };
            let result = work(&transaction).await;
            if result.is_err() {
                scope.rollback_only.store(true, Ordering::Release);
            }
            return result;
        }
        let scope = Arc::new(scope::Scope {
            transaction: self.begin_transaction().await?,
            rollback_only: AtomicBool::new(false),
            after_commit: std::sync::Mutex::new(Vec::new()),
        });
        let transaction = ClinicalTransaction {
            store: self.clone(),
            scope: Arc::downgrade(&scope),
        };
        let result = scope::TRANSACTION
            .scope(Arc::clone(&scope), work(&transaction))
            .await;
        drop(transaction);
        let scope = Arc::try_unwrap(scope)
            .map_err(|_| AuthError::internal("Identity transaction handle still in use"))?;
        if result.is_ok() && !scope.rollback_only.load(Ordering::Acquire) {
            scope.transaction.commit().await.map_err(database_error)?;
            for effect in scope
                .after_commit
                .into_inner()
                .map_err(|_| AuthError::internal("Identity after-commit queue unavailable"))?
            {
                effect.await?;
            }
            result
        } else {
            scope.transaction.rollback().await.map_err(database_error)?;
            result.and_then(|_| Err(AuthError::internal("Identity transaction was rolled back")))
        }
    }
}
