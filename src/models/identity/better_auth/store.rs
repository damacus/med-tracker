use ::better_auth::AuthError;
use sea_orm::{ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, DbErr, Statement, Value};

mod verifications;
mod sessions;
mod passkeys;
mod tenant;
mod canonical;
mod organizations;
mod members;
mod users;
mod invitations;
mod two_factor;
mod accounts;
mod devices;
mod api_keys;
mod provisioning;
mod transactions;
mod scope;

use scope::Transaction;

#[derive(Clone)]
pub struct ClinicalStore {
    db: DatabaseConnection,
}

impl ClinicalStore {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub(super) async fn transaction(&self) -> Result<Transaction, AuthError> {
        if let Ok(transaction) = scope::TRANSACTION.try_with(std::sync::Arc::clone) {
            return Ok(Transaction::shared(transaction));
        }
        self.begin_transaction().await.map(Transaction::owned)
    }

    pub(super) async fn begin_transaction(&self) -> Result<DatabaseTransaction, AuthError> {
        let transaction = super::super::browser::transaction(&self.db)
            .await
            .map_err(|_| AuthError::internal("Identity database transaction unavailable"))?;
        transaction.execute_unprepared("SELECT set_config('med_tracker.identity_session_token', '', true), set_config('med_tracker.identity_credential_id', '', true), set_config('med_tracker.identity_passkey_id', '', true), set_config('med_tracker.identity_verification_identifier', '', true), set_config('med_tracker.identity_verification_id', '', true), set_config('med_tracker.identity_verification_value', '', true)").await.map_err(database_error)?;
        transaction.execute_unprepared("SELECT set_config('med_tracker.identity_provider_id', '', true), set_config('med_tracker.identity_provider_account_id', '', true), set_config('med_tracker.identity_provider_row_id', '', true), set_config('med_tracker.identity_factor_id', '', true)").await.map_err(database_error)?;
        transaction.execute_unprepared("SELECT set_config('med_tracker.identity_device_id', '', true), set_config('med_tracker.identity_device_code', '', true), set_config('med_tracker.identity_device_user_code', '', true)").await.map_err(database_error)?;
        transaction.execute_unprepared("SELECT set_config('med_tracker.identity_api_key_id', '', true), set_config('med_tracker.identity_api_key_hash', '', true)").await.map_err(database_error)?;
        Ok(transaction)
    }

    pub(super) async fn audit(&self, transaction: &DatabaseTransaction, account_id: i64, token_type: &str, action: &str) -> Result<(), AuthError> {
        let request_id = super::request::request_id();
        super::super::browser::record_auth_token(transaction, account_id, token_type, action, request_id.as_deref()).await.map_err(|_| AuthError::internal("Identity audit unavailable"))
    }

    pub(super) async fn atomic<T>(&self, work: impl std::future::Future<Output = Result<T, AuthError>>, accept: impl FnOnce(&T) -> bool) -> Result<T, AuthError> {
        let scope = std::sync::Arc::new(scope::Scope {
            transaction: self.begin_transaction().await?,
            rollback_only: std::sync::atomic::AtomicBool::new(false),
        });
        let result = scope::TRANSACTION.scope(scope.clone(), work).await;
        let scope = std::sync::Arc::try_unwrap(scope).map_err(|_| AuthError::internal("Identity transaction still in use"))?;
        let rollback_only = scope.rollback_only.load(std::sync::atomic::Ordering::Acquire);
        let acceptable = result.as_ref().is_ok_and(accept);
        let commit = acceptable && !rollback_only;
        if commit {
            scope.transaction.commit().await.map_err(database_error)?;
            result
        } else {
            scope.transaction.rollback().await.map_err(database_error)?;
            if rollback_only && acceptable { result.and_then(|_| Err(AuthError::internal("Identity transaction was rolled back"))) } else { result }
        }
    }
}

pub(super) fn statement(sql: &str, values: impl IntoIterator<Item = Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, values)
}

pub(super) fn database_error(_: DbErr) -> AuthError {
    AuthError::internal("Identity persistence unavailable")
}

pub(super) async fn context(transaction: &DatabaseTransaction, key: &str, value: &str) -> Result<(), AuthError> {
    transaction.execute_raw(statement("SELECT set_config($1, $2, true)", [key.into(), value.into()])).await.map_err(database_error)?;
    Ok(())
}

pub(super) fn clinical_id(value: &str) -> Result<i64, AuthError> {
    let id = value
        .parse::<i64>()
        .map_err(|_| AuthError::bad_request("Invalid identity identifier"))?;
    if id <= 0 || id.to_string() != value {
        return Err(AuthError::bad_request("Invalid identity identifier"));
    }
    Ok(id)
}
