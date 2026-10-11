use super::{Service, reminders};
use crate::models::{
    access::{Actor, HouseholdScope},
    entities::household,
    errors::OperationError,
};
use chrono::{DateTime, Utc};
use loco_rs::{
    app::AppContext,
    bgworker::BackgroundWorker,
    task::{Task, TaskInfo, Vars},
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, Statement, TransactionTrait,
};
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
        match reminders::deliver_account(&self.ctx, &scope, run.at).await {
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
    let households = household::Entity::find()
        .filter(household::Column::Status.eq("active"))
        .filter(household::Column::LifecycleState.eq("active"))
        .all(&ctx.db)
        .await?;
    let mut runs = Vec::new();
    for household in households {
        let transaction = ctx.db.begin().await?;
        transaction
            .execute_unprepared("SET LOCAL ROLE med_tracker_app")
            .await?;
        transaction.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT set_config('med_tracker.current_household_id',$1,true),set_config('med_tracker.current_account_id','',true),set_config('med_tracker.current_membership_id','',true),set_config('med_tracker.current_invitation_token_digest','',true)", [household.id.to_string().into()])).await?;
        let rows = transaction.query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT m.account_id FROM household_memberships m JOIN notification_preferences p ON p.person_id=m.person_id AND p.household_id=m.household_id JOIN accounts a ON a.id=m.account_id WHERE m.household_id=$1 AND m.status='active' AND m.revoked_at IS NULL AND p.enabled AND a.status=2", [household.id.into()])).await?;
        for row in rows {
            runs.push(ReminderRun {
                account_id: row.try_get("", "account_id")?,
                household_id: household.id,
                at,
            });
        }
        transaction.commit().await?;
    }
    Ok(runs)
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
        if ctx.shared_store.get::<Service>().is_none() {
            return Ok(());
        }
        let runs = recipients(ctx, Utc::now())
            .await
            .map_err(|_| loco_rs::Error::string("Browser reminder scheduling failed"))?;
        for run in runs {
            ReminderWorker::perform_later(ctx, run).await?;
        }
        Ok(())
    }
}
