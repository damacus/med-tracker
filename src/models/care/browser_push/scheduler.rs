use super::{Service, reminders};
use crate::models::{
    access::{Actor, HouseholdScope},
    errors::OperationError,
    ids::{AccountId, HouseholdId},
};
use chrono::{DateTime, Utc};
use loco_rs::{
    app::AppContext,
    bgworker::BackgroundWorker,
    task::{Task, TaskInfo, Vars},
};
use sea_orm::{ConnectionTrait, DatabaseTransaction, DbBackend, Statement, TransactionTrait};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct ReminderRun {
    pub account_id: i64,
    pub household_id: i64,
    pub at: DateTime<Utc>,
}

pub struct ReminderWorker {
    ctx: AppContext,
}

pub(crate) async fn job_transaction(
    ctx: &AppContext,
) -> Result<DatabaseTransaction, OperationError> {
    let transaction = ctx.db.begin().await?;
    transaction
        .execute_unprepared("SET LOCAL ROLE med_tracker_app")
        .await?;
    Ok(transaction)
}

#[async_trait::async_trait]
impl BackgroundWorker<ReminderRun> for ReminderWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    async fn perform(&self, run: ReminderRun) -> loco_rs::Result<()> {
        let scope = HouseholdScope {
            actor: Actor {
                account_id: run.account_id,
            },
            household_id: run.household_id,
            request_id: format!("browser-reminders:{}", run.at.timestamp()),
        };
        match reminders::deliver_account(&self.ctx, &scope, Utc::now()).await {
            Ok(_)
            | Err(
                OperationError::Forbidden
                | OperationError::NotFound
                | OperationError::Unauthenticated,
            ) => Ok(()),
            Err(_) => Err(loco_rs::Error::string("Browser reminder delivery failed")),
        }
    }
}

pub async fn recipients(
    ctx: &AppContext,
    at: DateTime<Utc>,
) -> Result<Vec<ReminderRun>, OperationError> {
    let transaction = job_transaction(ctx).await?;
    transaction
        .execute_unprepared("SELECT med_tracker.cleanup_reminders(NULL,NULL)")
        .await?;
    transaction.commit().await?;
    recipients_after(ctx, at, None).await
}

#[derive(Clone, Copy)]
pub struct RecipientCursor {
    household_id: HouseholdId,
    account_id: AccountId,
}

impl ReminderRun {
    pub fn cursor(&self) -> RecipientCursor {
        RecipientCursor {
            household_id: HouseholdId::from_database(self.household_id),
            account_id: AccountId::from_database(self.account_id),
        }
    }
}

pub async fn recipients_after(
    ctx: &AppContext,
    at: DateTime<Utc>,
    after: Option<RecipientCursor>,
) -> Result<Vec<ReminderRun>, OperationError> {
    let after = after.unwrap_or(RecipientCursor {
        household_id: HouseholdId::from_database(0),
        account_id: AccountId::from_database(0),
    });
    let transaction = job_transaction(ctx).await?;
    let rows = transaction
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT account_id,household_id FROM med_tracker.reminder_recipients($1,$2,100)",
            [
                after.household_id.get().into(),
                after.account_id.get().into(),
            ],
        ))
        .await?;
    transaction.commit().await?;
    rows.into_iter()
        .map(|row| {
            Ok(ReminderRun {
                account_id: row.try_get("", "account_id")?,
                household_id: row.try_get("", "household_id")?,
                at,
            })
        })
        .collect()
}

pub struct ReminderTask;

#[async_trait::async_trait]
impl Task for ReminderTask {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "browser-reminders".into(),
            detail: "Queue due and missed browser reminders using saved profile preferences".into(),
        }
    }

    async fn run(&self, ctx: &AppContext, _vars: &Vars) -> loco_rs::Result<()> {
        let at = Utc::now();
        let mut page = recipients(ctx, at)
            .await
            .map_err(|_| loco_rs::Error::string("Browser reminder scheduling failed"))?;
        if ctx.shared_store.get::<Service>().is_none() {
            return Ok(());
        }
        while let Some(last) = page.last() {
            let after = last.cursor();
            for run in page {
                ReminderWorker::perform_later(ctx, run).await?;
            }
            page = recipients_after(ctx, at, Some(after))
                .await
                .map_err(|_| loco_rs::Error::string("Browser reminder scheduling failed"))?;
        }
        Ok(())
    }
}
