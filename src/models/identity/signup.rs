use super::browser;
use crate::models::{care::invitations, errors::OperationError};
use chrono::{Months, NaiveDate, Utc};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use std::collections::BTreeMap;

mod bootstrap;
mod native;
mod policy;
mod provisioning;
pub(crate) use native::{complete_native_profile, prepare_native_profile};
pub(crate) use provisioning::profile_errors;
pub use provisioning::{AccountProfile, ProvisionedAccount};

#[derive(Debug)]
pub enum SignupError {
    Invalid(BTreeMap<String, Vec<String>>),
    InvalidKey,
    RegistrationClosed,
    Unavailable,
}

pub async fn registration_open(
    db: &DatabaseConnection,
    settings: Option<&serde_json::Value>,
) -> Result<bool, SignupError> {
    let transaction = browser::transaction(db).await.map_err(unavailable)?;
    let open = policy::open(&transaction, settings).await?;
    transaction.commit().await.map_err(unavailable)?;
    Ok(open)
}

pub struct SignupInvitationContext {
    account_id: i64,
    person_id: i64,
    user_id: i64,
    email: String,
}

impl SignupInvitationContext {
    pub(crate) fn account_id(&self) -> i64 {
        self.account_id
    }
    pub(crate) fn person_id(&self) -> i64 {
        self.person_id
    }
    pub(crate) fn user_id(&self) -> i64 {
        self.user_id
    }
    pub(crate) fn email(&self) -> &str {
        &self.email
    }
}

pub(crate) fn initial_preferences(
    settings: Option<&serde_json::Value>,
) -> Result<serde_json::Value, SignupError> {
    let zone =
        super::time_zone::preferred(&serde_json::json!({}), settings).map_err(unavailable)?;
    Ok(serde_json::json!({"time_zone": zone.to_string()}))
}

fn operation_error(error: OperationError) -> SignupError {
    match error {
        OperationError::Validation { details } => serde_json::from_value(details["errors"].clone())
            .map(SignupError::Invalid)
            .unwrap_or(SignupError::Unavailable),
        OperationError::Unavailable => SignupError::Unavailable,
        _ => invalid("invitation_token", "is invalid or expired"),
    }
}

fn invalid(field: &str, message: &str) -> SignupError {
    SignupError::Invalid(BTreeMap::from([(field.into(), vec![message.into()])]))
}

fn unavailable<T: 'static>(error: T) -> SignupError {
    let mut sql_state = None;
    let mut constraint = None;
    if let Some(
        sea_orm::DbErr::Exec(sea_orm::RuntimeErr::SqlxError(error))
        | sea_orm::DbErr::Query(sea_orm::RuntimeErr::SqlxError(error)),
    ) = (&error as &dyn std::any::Any).downcast_ref::<sea_orm::DbErr>()
        && let Some(database) = error.as_database_error()
    {
        sql_state = database.code().map(|code| code.into_owned());
        constraint = database.constraint().map(str::to_owned);
    }
    tracing::error!(
        error_type = std::any::type_name::<T>(),
        sql_state,
        constraint,
        "Account setup operation failed"
    );
    SignupError::Unavailable
}

fn sql<I>(statement: &str, values: I) -> Statement
where
    I: IntoIterator<Item = sea_orm::Value>,
{
    Statement::from_sql_and_values(DbBackend::Postgres, statement, values)
}
