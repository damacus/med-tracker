use std::{ops::Deref, sync::{Arc, atomic::{AtomicBool, Ordering}}};

use sea_orm::{DatabaseTransaction, DbErr};

pub(super) struct Scope {
    pub transaction: DatabaseTransaction,
    pub rollback_only: AtomicBool,
}

tokio::task_local! {
    pub(super) static TRANSACTION: Arc<Scope>;
}

enum Inner {
    Owned(DatabaseTransaction),
    Shared(Arc<Scope>),
}

pub(crate) struct Transaction {
    inner: Option<Inner>,
    finished: bool,
}

impl Transaction {
    pub(super) fn owned(transaction: DatabaseTransaction) -> Self {
        Self { inner: Some(Inner::Owned(transaction)), finished: false }
    }

    pub(super) fn shared(scope: Arc<Scope>) -> Self {
        Self { inner: Some(Inner::Shared(scope)), finished: false }
    }

    pub(crate) async fn commit(mut self) -> Result<(), DbErr> {
        self.finished = true;
        match self.inner.take().expect("transaction exists until completion") {
            Inner::Owned(transaction) => transaction.commit().await,
            Inner::Shared(_) => Ok(()),
        }
    }

    pub(crate) async fn rollback(mut self) -> Result<(), DbErr> {
        self.finished = true;
        match self.inner.take().expect("transaction exists until completion") {
            Inner::Owned(transaction) => transaction.rollback().await,
            Inner::Shared(scope) => {
                scope.rollback_only.store(true, Ordering::Release);
                Ok(())
            }
        }
    }
}

impl Deref for Transaction {
    type Target = DatabaseTransaction;

    fn deref(&self) -> &Self::Target {
        match self.inner.as_ref().expect("transaction exists until completion") {
            Inner::Owned(transaction) => transaction,
            Inner::Shared(scope) => &scope.transaction,
        }
    }
}

impl Drop for Transaction {
    fn drop(&mut self) {
        if !self.finished && let Some(Inner::Shared(scope)) = &self.inner {
            scope.rollback_only.store(true, Ordering::Release);
        }
    }
}
