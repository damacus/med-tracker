use crate::models::errors::OperationError;
use loco_rs::{
    app::AppContext,
    task::{Task, TaskInfo, Vars},
};
use sea_orm::{ConnectionTrait, DatabaseTransaction, DbBackend, Statement, TransactionTrait};

const PAGE_SIZE: i32 = 500;

#[derive(Default)]
struct ExpiryRun {
    expired: u64,
    failed: u64,
}

async fn expire_one(ctx: &AppContext, id: i64) -> Result<bool, OperationError> {
    let transaction = ctx.db.begin().await?;
    let result = expire_in(&transaction, id).await;
    match result {
        Ok(changed) => {
            transaction.commit().await?;
            Ok(changed)
        }
        Err(error) => {
            transaction.rollback().await?;
            Err(error)
        }
    }
}

pub(super) async fn expire_in(
    transaction: &DatabaseTransaction,
    id: i64,
) -> Result<bool, OperationError> {
    let row = transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT med_tracker.expire_support_access_session($1) AS changed",
            [id.into()],
        ))
        .await?
        .ok_or(OperationError::Unavailable)?;
    Ok(row.try_get::<bool>("", "changed")?)
}

async fn expire_due(ctx: &AppContext) -> Result<ExpiryRun, OperationError> {
    let mut after = 0_i64;
    let mut run = ExpiryRun::default();
    loop {
        let rows = ctx
            .db
            .query_all_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT id FROM med_tracker.due_support_access_sessions($1,$2)",
                [after.into(), PAGE_SIZE.into()],
            ))
            .await?;
        if rows.is_empty() {
            break;
        }
        for row in rows {
            let id = row.try_get::<i64>("", "id")?;
            after = id;
            match expire_one(ctx, id).await {
                Ok(true) => run.expired += 1,
                Ok(false) => {}
                Err(_) => {
                    run.failed += 1;
                    tracing::warn!(
                        reason = "support_expiry_failed",
                        "Support expiry deferred an item until the next run"
                    );
                }
            }
        }
    }
    Ok(run)
}

pub struct SupportExpiryTask;

#[async_trait::async_trait]
impl Task for SupportExpiryTask {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "support-access-expire".into(),
            detail: "Expire due support access with attributable audit records".into(),
        }
    }

    async fn run(&self, ctx: &AppContext, _vars: &Vars) -> loco_rs::Result<()> {
        let run = expire_due(ctx)
            .await
            .map_err(|_| loco_rs::Error::string("Support expiry run failed"))?;
        let outcome = if run.failed == 0 {
            "success"
        } else {
            "partial"
        };
        tracing::info!(
            event_type = "support_access_session.expired",
            outcome,
            processed_count = run.expired,
            deferred_count = run.failed,
            "Support expiry run finished"
        );
        if run.failed > 0 {
            return Err(loco_rs::Error::string("Support expiry run incomplete"));
        }
        Ok(())
    }
}
