use better_auth_core::{AuthError, AuthResult};
use sea_orm::{ConnectionTrait};

use super::{ClinicalStore, context, database_error, statement};
use super::super::request;
use crate::models::{access::{self, Actor, HouseholdScope, TenantTransaction}, errors::OperationError};

pub(super) fn operation_error(error: OperationError) -> AuthError {
    match error {
        OperationError::Unauthenticated => AuthError::Unauthenticated,
        OperationError::Forbidden => AuthError::forbidden("Household access denied"),
        OperationError::NotFound => AuthError::not_found("Household record not found"),
        _ => AuthError::internal("Household operation failed"),
    }
}

impl ClinicalStore {
    pub(super) async fn account_transaction(&self) -> AuthResult<super::Transaction> {
        let request = request::authenticated()?;
        let transaction = self.transaction().await?;
        context(&*transaction, "med_tracker.identity_session_token", &request.session_token).await?;
        context(&*transaction, "med_tracker.current_account_id", &request.account_id.to_string()).await?;
        let account = transaction.query_one_raw(statement("SELECT id FROM public.accounts WHERE id=$1 AND status=2 FOR UPDATE", [request.account_id.into()])).await.map_err(database_error)?;
        if account.is_none() { return Err(AuthError::Unauthenticated); }
        access::verify_account_actor(&*transaction, request.account_id).await.map_err(operation_error)?;
        let session = transaction.query_one_raw(statement("SELECT s.id FROM public.identity_sessions s JOIN public.accounts a ON a.id=s.account_id WHERE s.account_id=$1 AND s.token=$2 AND s.active AND s.purpose='authenticated' AND s.expires_at>CURRENT_TIMESTAMP AND a.status=2 FOR SHARE OF s", [request.account_id.into(), request.session_token.into()])).await.map_err(database_error)?;
        if session.is_none() { return Err(AuthError::Unauthenticated); }
        Ok(transaction)
    }

    pub(super) async fn tenant(&self, household_id: i64) -> AuthResult<TenantTransaction> {
        let request = request::authenticated()?;
        let tenant = access::begin(&self.db, &HouseholdScope {
            actor: Actor { account_id: request.account_id },
            household_id,
            request_id: request.request_id,
        }).await.map_err(operation_error)?;
        let account = tenant.transaction().query_one_raw(statement("SELECT id FROM public.accounts WHERE id=$1 AND status=2 FOR UPDATE", [request.account_id.into()])).await.map_err(database_error)?;
        if account.is_none() { return Err(AuthError::Unauthenticated); }
        let session = tenant.transaction().query_one_raw(statement("SELECT id FROM public.identity_sessions WHERE account_id=$1 AND token=$2 AND active AND purpose='authenticated' AND expires_at>CURRENT_TIMESTAMP FOR SHARE",[request.account_id.into(),request.session_token.into()])).await.map_err(database_error)?;
        if session.is_none() { return Err(AuthError::Unauthenticated); }
        Ok(tenant)
    }
}
