use std::collections::BTreeMap;

use axum_session::Session;
use axum_session_sqlx::SessionPgPool;
use chrono::{Months, NaiveDate, Utc};
use loco_rs::{
    bgworker::BackgroundWorker,
    mailer::{DEFAULT_MAILER_PRIORITY, Email, MailerWorker},
};
use sea_orm::{ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, Statement};
use serde::Deserialize;

use super::browser::{self, SignInOutcome};
use crate::models::{care::invitations, errors::OperationError};

mod bootstrap;
mod policy;
mod resend;

pub use resend::{VerificationResend, resend_verification};
mod tokens;

#[derive(Deserialize)]
pub struct SignupInput {
    #[serde(default)]
    pub invitation_token: String,
    #[serde(default)]
    pub email: String,
    pub name: String,
    pub date_of_birth: String,
    pub password: String,
    #[serde(rename = "password-confirm")]
    pub password_confirm: String,
}

#[derive(Debug)]
pub enum SignupError {
    Invalid(BTreeMap<String, Vec<String>>),
    InvalidKey,
    RegistrationClosed,
    Unavailable,
}

pub async fn registration_open(db: &DatabaseConnection) -> Result<bool, SignupError> {
    let transaction = browser::transaction(db).await.map_err(unavailable)?;
    let open = policy::open(&transaction).await?;
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

pub async fn create(
    db: &DatabaseConnection,
    input: &SignupInput,
    origin: &str,
    request_id: Option<&str>,
) -> Result<(), SignupError> {
    let birth = validate(input)?;
    let password = input.password.clone();
    let hash = tokio::task::spawn_blocking(move || bcrypt::hash(password, bcrypt::DEFAULT_COST))
        .await
        .map_err(unavailable)?
        .map_err(unavailable)?;
    let transaction = browser::transaction(db).await.map_err(unavailable)?;
    let invitation = if input.invitation_token.is_empty() {
        if !policy::open(&transaction).await? {
            return Err(SignupError::RegistrationClosed);
        }
        None
    } else {
        Some(
            invitations::signup_invitation(&transaction, &input.invitation_token)
                .await
                .map_err(operation_error)?,
        )
    };
    let email = match &invitation {
        Some(invitation) => invitation.email.clone(),
        None => invitations::validated_email(&input.email).map_err(operation_error)?,
    };
    let account = transaction.query_one_raw(sql(
        "INSERT INTO accounts(email,password_hash,status,created_at,updated_at) VALUES($1,$2,1,now(),now()) ON CONFLICT(email) WHERE status = ANY(ARRAY[1,2]) DO NOTHING RETURNING id",
        [email.clone().into(),hash.clone().into()])).await.map_err(unavailable)?
        .ok_or_else(|| invalid("email", "is already registered"))?;
    let account_id: i64 = account.try_get("", "id").map_err(unavailable)?;
    transaction
        .execute_raw(sql(
            "SELECT set_config('med_tracker.current_account_id',$1,true)",
            [account_id.to_string().into()],
        ))
        .await
        .map_err(unavailable)?;
    let household_id = match &invitation {
        Some(invitation) => invitation.household_id,
        None => bootstrap::household(&transaction, account_id, &input.name).await?,
    };
    let person = transaction.query_one_raw(sql(
        "INSERT INTO people(account_id,household_id,name,email,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES($1,$2,$3,$4,$5,0,true,now(),now()) RETURNING id",
        [account_id.into(),household_id.into(),input.name.trim().to_owned().into(),email.clone().into(),birth.into()])).await.map_err(unavailable)?.ok_or(SignupError::Unavailable)?;
    let person_id: i64 = person.try_get("", "id").map_err(unavailable)?;
    let user = transaction.query_one_raw(sql(
        "INSERT INTO users(person_id,email_address,password_digest,active,created_at,updated_at) VALUES($1,$2,$3,true,now(),now()) RETURNING id",
        [person_id.into(),email.clone().into(),hash.into()])).await.map_err(unavailable)?.ok_or(SignupError::Unavailable)?;
    let user_id = user.try_get("", "id").map_err(unavailable)?;
    let context = SignupInvitationContext {
        account_id,
        person_id,
        user_id,
        email: email.clone(),
    };
    if invitation.is_some() {
        invitations::accept_signup(
            &transaction,
            &context,
            &input.invitation_token,
            request_id.unwrap_or(""),
        )
        .await
        .map_err(operation_error)?;
    } else {
        bootstrap::owner(
            &transaction,
            &context,
            household_id,
            &input.name,
            request_id,
        )
        .await?;
    }
    let key = tokens::random_key()?;
    transaction.execute_raw(sql("INSERT INTO account_verification_keys(account_id,key,created_at,updated_at) VALUES($1,$2,now(),now())", [account_id.into(),key.clone().into()])).await.map_err(unavailable)?;
    browser::record_auth_token(
        &transaction,
        account_id,
        "verification_key",
        "created",
        request_id,
    )
    .await
    .map_err(unavailable)?;
    queue_verification(&transaction, account_id, email, &key, origin).await?;
    transaction.commit().await.map_err(unavailable)?;
    Ok(())
}

async fn queue_verification(
    transaction: &DatabaseTransaction,
    account_id: i64,
    email: String,
    key: &str,
    origin: &str,
) -> Result<(), SignupError> {
    let mut link = url::Url::parse(origin).map_err(unavailable)?;
    link.set_path("/verify-account");
    link.set_query(None);
    link.query_pairs_mut()
        .append_pair("key", &tokens::link_token(account_id, key)?);
    let email = Email {
        to: email,
        subject: "Verify your account".into(),
        text: format!(
            "An email has been sent to you with a link to verify your account. Verify your account: {link}"
        ),
        ..Default::default()
    };
    let payload = serde_json::to_value(email).map_err(unavailable)?;
    transaction.execute_raw(sql("INSERT INTO pg_loco_queue(id,task_data,name,run_at,priority) VALUES($1,$2,$3,clock_timestamp(),$4)", [ulid::Ulid::new().to_string().into(),payload.into(),MailerWorker::class_name().into(),DEFAULT_MAILER_PRIORITY.into()])).await.map_err(unavailable)?;
    Ok(())
}

pub async fn valid_key(db: &DatabaseConnection, token: &str) -> Result<bool, SignupError> {
    let transaction = browser::transaction(db).await.map_err(unavailable)?;
    match matching_account(&transaction, token).await {
        Ok(_) => Ok(true),
        Err(SignupError::InvalidKey) => Ok(false),
        Err(error) => Err(error),
    }
}

pub async fn verify(
    db: &DatabaseConnection,
    session: &Session<SessionPgPool>,
    token: &str,
    request_id: Option<&str>,
) -> Result<SignInOutcome, SignupError> {
    let transaction = browser::transaction(db).await.map_err(unavailable)?;
    let account_id = matching_account(&transaction, token).await?;
    transaction
        .execute_raw(sql(
            "UPDATE accounts SET status=2,updated_at=now() WHERE id=$1 AND status=1",
            [account_id.into()],
        ))
        .await
        .map_err(unavailable)?;
    transaction.execute_raw(sql("UPDATE account_verification_keys SET key=$2,email_last_sent=now()-interval '310 seconds',updated_at=now() WHERE account_id=$1", [account_id.into(),tokens::random_key()?.into()])).await.map_err(unavailable)?;
    browser::record_auth_token(
        &transaction,
        account_id,
        "verification_key",
        "revoked",
        request_id,
    )
    .await
    .map_err(unavailable)?;
    browser::complete_primary_authentication(transaction, session, account_id)
        .await
        .map_err(unavailable)
}

async fn matching_account(
    transaction: &DatabaseTransaction,
    token: &str,
) -> Result<i64, SignupError> {
    let account_id = tokens::account_id(token).ok_or(SignupError::InvalidKey)?;
    let account = transaction
        .query_one_raw(sql(
            "SELECT id FROM accounts WHERE id=$1 AND status=1 FOR UPDATE",
            [account_id.into()],
        ))
        .await
        .map_err(unavailable)?;
    if account.is_none() {
        return Err(SignupError::InvalidKey);
    }
    let rows = transaction
        .query_all_raw(sql(
            "SELECT key FROM account_verification_keys WHERE account_id=$1 FOR UPDATE",
            [account_id.into()],
        ))
        .await
        .map_err(unavailable)?;
    if rows.len() != 1 {
        return Err(SignupError::InvalidKey);
    }
    let key: String = rows[0].try_get("", "key").map_err(unavailable)?;
    if tokens::matches(token, &key)? {
        Ok(account_id)
    } else {
        Err(SignupError::InvalidKey)
    }
}

fn validate(input: &SignupInput) -> Result<NaiveDate, SignupError> {
    let mut errors = BTreeMap::<String, Vec<String>>::new();
    let mut error = |field: &str, message: &str| {
        errors.entry(field.into()).or_default().push(message.into());
    };
    if input.name.trim().is_empty() {
        error("name", "must be present");
    }
    let birth = NaiveDate::parse_from_str(&input.date_of_birth, "%Y-%m-%d").ok();
    match birth {
        None if input.date_of_birth.is_empty() => error("date_of_birth", "must be present"),
        None => error("date_of_birth", "must be a valid date"),
        Some(birth)
            if Utc::now()
                .date_naive()
                .checked_sub_months(Months::new(18 * 12))
                .is_none_or(|latest| birth > latest) =>
        {
            error(
                "date_of_birth",
                "Children must be added by a parent or carer.",
            )
        }
        _ => {}
    }
    if input.password != input.password_confirm {
        error("password-confirm", "does not match password");
    }
    if input.password.chars().count() < 12 {
        error("password", "must be at least 12 characters");
    }
    if input.password.len() > 72 {
        error("password", "must be at most 72 bytes");
    }
    if !input.password.bytes().any(|byte| byte.is_ascii_digit())
        || !input
            .password
            .chars()
            .any(|character| !character.is_ascii_alphanumeric())
    {
        error("password", "requires one number and one special character");
    }
    if errors.is_empty() {
        birth.ok_or(SignupError::Unavailable)
    } else {
        Err(SignupError::Invalid(errors))
    }
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
