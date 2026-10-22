use better_auth::AuthResult;
use loco_rs::{
    bgworker::BackgroundWorker,
    mailer::{DEFAULT_MAILER_PRIORITY, Email, MailerWorker},
};
use sea_orm::ConnectionTrait;

use super::policy::{database_error, statement};

pub(super) async fn notice_address_in(
    transaction: &impl ConnectionTrait,
    address: &str,
    subject: &str,
    body: &str,
) -> AuthResult<()> {
    let email = Email {
        to: address.into(),
        subject: subject.into(),
        text: body.into(),
        ..Default::default()
    };
    transaction.execute_raw(statement("INSERT INTO pg_loco_queue(id,task_data,name,run_at,priority) VALUES($1,$2,$3,clock_timestamp(),$4)", [ulid::Ulid::generate().to_string().into(), serde_json::to_value(email)?.into(), MailerWorker::class_name().into(), DEFAULT_MAILER_PRIORITY.into()])).await.map_err(database_error)?;
    Ok(())
}
