use crate::models::errors::OperationError;
use chrono::{DateTime, Utc};
use loco_rs::app::AppContext;
use sea_orm::{ConnectionTrait, DbBackend, Statement};

pub(crate) async fn terminal_cleanup(
    ctx: &AppContext,
    household_id: i64,
    account_id: Option<i64>,
    _now: DateTime<Utc>,
) -> Result<(), OperationError> {
    let transaction = super::super::scheduler::job_transaction(ctx).await?;
    transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT med_tracker.cleanup_reminders($1,$2)",
            [household_id.into(), account_id.into()],
        ))
        .await?;
    transaction.commit().await?;
    Ok(())
}
