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
    UpdateUser,
    store::{
        AuthTransaction, BoxedTransactionValue, TransactionStore, TransactionWork,
        TypedTransactionFuture, UserStore, VerificationStore,
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

    async fn create_session(&self, input: CreateSession) -> AuthResult<SessionView> {
        let scope = self.active_scope()?;
        self.store
            .create_session_in(&scope.transaction, input)
            .await
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
