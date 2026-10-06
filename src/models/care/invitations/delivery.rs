use super::*;
use loco_rs::{
    bgworker::BackgroundWorker,
    mailer::{DEFAULT_MAILER_PRIORITY, Email, MailerWorker},
};
use sea_orm::{ConnectionTrait, DbBackend, Statement};

pub(super) async fn enqueue(
    tenant: &TenantTransaction,
    email: Email,
) -> Result<(), OperationError> {
    let payload = serde_json::to_value(email).map_err(|_| OperationError::Unavailable)?;
    tenant.transaction().execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO pg_loco_queue(id,task_data,name,run_at,priority) VALUES($1,$2,$3,clock_timestamp(),$4)",
        [ulid::Ulid::new().to_string().into(),payload.into(),MailerWorker::class_name().into(),DEFAULT_MAILER_PRIORITY.into()],
    )).await?;
    Ok(())
}
