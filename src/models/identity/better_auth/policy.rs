use better_auth_core::{AuthError, AuthResult, wire::SessionView};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, DbErr, Statement, Value,
};

mod closure;

#[derive(Clone)]
pub struct IdentityPolicy {
    db: DatabaseConnection,
    settings: Option<serde_json::Value>,
}

impl IdentityPolicy {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db, settings: None }
    }
    pub fn with_settings(mut self, settings: Option<serde_json::Value>) -> Self {
        self.settings = settings;
        self
    }
    pub fn settings(&self) -> Option<&serde_json::Value> {
        self.settings.as_ref()
    }
    pub(super) fn database(&self) -> &DatabaseConnection {
        &self.db
    }

    pub(crate) async fn transaction(&self) -> AuthResult<DatabaseTransaction> {
        super::super::browser::transaction(&self.db)
            .await
            .map_err(|_| AuthError::internal("Identity database unavailable"))
    }

    pub(crate) async fn begin_transaction(&self) -> AuthResult<DatabaseTransaction> {
        self.transaction().await
    }

    pub(crate) async fn account_transaction(&self) -> AuthResult<DatabaseTransaction> {
        let actor = super::request::authenticated()?;
        let tx = self.transaction().await?;
        context(
            &tx,
            "med_tracker.current_account_id",
            &actor.account_id.to_string(),
        )
        .await?;
        context(
            &tx,
            "med_tracker.identity_session_token",
            &actor.session_token,
        )
        .await?;
        crate::models::access::verify_account(&tx, actor.account_id)
            .await
            .map_err(operation_error)?;
        tx.query_one_raw(statement("SELECT id FROM identity_sessions WHERE account_id=$1 AND token=$2 AND active AND purpose='authenticated' AND expires_at>clock_timestamp()", [actor.account_id.into(), actor.session_token.into()])).await.map_err(database_error)?.ok_or(AuthError::Unauthenticated)?;
        Ok(tx)
    }

    pub(crate) async fn admit_session(&self, session: &SessionView) -> AuthResult<()> {
        let account = clinical_id(&session.user_id.display_string()?)?;
        let transaction = self.transaction().await?;
        context(
            &transaction,
            "med_tracker.current_account_id",
            &account.to_string(),
        )
        .await?;
        crate::models::access::verify_account(&transaction, account)
            .await
            .map_err(operation_error)?;
        let lifetime = super::super::store::Lifetime::from_settings(self.settings())
            .map_err(|_| AuthError::internal("Session policy unavailable"))?;
        let maximum = lifetime.maximum_age_days();
        let row = transaction.query_one_raw(statement("SELECT s.id FROM identity_sessions s JOIN accounts a ON a.id=s.account_id JOIN people p ON p.id=a.person_id WHERE s.id=$1 AND s.account_id=$2 AND s.token=$3 AND s.active AND s.expires_at>clock_timestamp() AND s.updated_at>clock_timestamp()-($4::bigint*interval '1 day') AND ($5::bigint=0 OR s.created_at>clock_timestamp()-($5::bigint*interval '1 day')) AND a.status=2 AND p.date_of_birth<=(CURRENT_DATE-interval '18 years')::date AND NOT EXISTS(SELECT 1 FROM account_lockouts l WHERE l.account_id=a.id AND l.deadline>clock_timestamp())", [clinical_id(&session.id.display_string()?)?.into(), account.into(), session.token.typed()?.clone().into(), lifetime.inactivity_days().into(), maximum.into()])).await.map_err(database_error)?;
        if row.is_none() {
            return Err(AuthError::Unauthenticated);
        }
        transaction.commit().await.map_err(database_error)
    }

    pub(crate) async fn require_security_session(
        &self,
        tx: &DatabaseTransaction,
        session: &SessionView,
    ) -> AuthResult<()> {
        let account = clinical_id(&session.user_id.display_string()?)?;
        context(tx, "med_tracker.current_account_id", &account.to_string()).await?;
        context(
            tx,
            "med_tracker.identity_session_token",
            session.token.typed()?,
        )
        .await?;
        crate::models::access::verify_account(tx, account)
            .await
            .map_err(operation_error)?;
        tx.query_one_raw(statement("SELECT id FROM identity_sessions WHERE id=$1 AND account_id=$2 AND token=$3 AND active AND purpose='authenticated' AND expires_at>clock_timestamp()", [clinical_id(&session.id.display_string()?)?.into(), account.into(), session.token.typed()?.clone().into()])).await.map_err(database_error)?.ok_or(AuthError::Unauthenticated)?;
        Ok(())
    }

    pub(crate) async fn lock_security_session(
        &self,
        tx: &DatabaseTransaction,
        session: &SessionView,
    ) -> AuthResult<()> {
        self.require_security_session(tx, session).await?;
        tx.query_one_raw(statement(
            "SELECT id FROM accounts WHERE id=$1 AND status=2 FOR UPDATE",
            [clinical_id(&session.user_id.display_string()?)?.into()],
        ))
        .await
        .map_err(database_error)?
        .ok_or(AuthError::Unauthenticated)?;
        tx.query_one_raw(statement("SELECT id FROM identity_sessions WHERE id=$1 AND created_at>clock_timestamp()-interval '30 days'", [clinical_id(&session.id.display_string()?)?.into()])).await.map_err(database_error)?.ok_or(AuthError::Unauthenticated)?;
        Ok(())
    }

    pub(crate) async fn audit(
        &self,
        tx: &DatabaseTransaction,
        account: i64,
        token_type: &str,
        action: &str,
    ) -> AuthResult<()> {
        super::super::browser::record_auth_token(
            tx,
            account,
            token_type,
            action,
            super::request::request_id().as_deref(),
        )
        .await
        .map_err(|_| AuthError::internal("Identity audit unavailable"))
    }
}

pub(super) fn statement(sql: &str, values: impl IntoIterator<Item = Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, values)
}

pub(super) fn database_error(error: DbErr) -> AuthError {
    let category = match error {
        DbErr::Conn(_) => "connection",
        DbErr::Exec(_) => "execute",
        DbErr::Query(_) => "query",
        _ => "persistence",
    };
    tracing::error!(category, "Identity persistence operation failed");
    AuthError::internal("Identity persistence unavailable")
}

pub(super) async fn context(tx: &impl ConnectionTrait, key: &str, value: &str) -> AuthResult<()> {
    tx.execute_raw(statement(
        "SELECT set_config($1,$2,true)",
        [key.into(), value.into()],
    ))
    .await
    .map_err(database_error)?;
    Ok(())
}

pub(crate) fn clinical_id(value: &str) -> AuthResult<i64> {
    let id = value
        .parse::<i64>()
        .map_err(|_| AuthError::bad_request("Invalid identity identifier"))?;
    if id <= 0 || id.to_string() != value {
        return Err(AuthError::bad_request("Invalid identity identifier"));
    }
    Ok(id)
}

pub(super) fn operation_error(error: crate::models::errors::OperationError) -> AuthError {
    match error {
        crate::models::errors::OperationError::Unauthenticated => AuthError::Unauthenticated,
        crate::models::errors::OperationError::Forbidden => {
            AuthError::forbidden("Household access denied")
        }
        crate::models::errors::OperationError::NotFound => {
            AuthError::not_found("Household record unavailable")
        }
        _ => AuthError::internal("Household operation unavailable"),
    }
}
