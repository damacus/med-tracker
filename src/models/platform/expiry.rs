use super::*;
use async_trait::async_trait;
use loco_rs::{
    app::AppContext,
    task::{Task, TaskInfo, Vars},
};

const EXPIRE_SESSION: &str = "WITH updated AS (UPDATE public.support_access_sessions SET expired_at=clock_timestamp(),updated_at=clock_timestamp() WHERE id=$1 AND ended_at IS NULL AND expired_at IS NULL AND expires_at <= clock_timestamp() RETURNING id,household_id,platform_admin_id) INSERT INTO versions (item_type,item_id,event,object,whodunnit,request_id,audit_context,household_id,created_at) SELECT 'SupportAccessSession',updated.id,'platform/support/expired','{\"household_id\":' || updated.household_id::text || '}',NULL,NULL,jsonb_build_object('actor_account_id',pa.account_id,'actor_user_id',NULL,'active_role','support_expiry_task','authentication_method',NULL,'session_reference',NULL,'request_id',NULL,'ip',NULL),updated.household_id,timezone('UTC',clock_timestamp()) FROM updated JOIN public.platform_admins pa ON pa.id=updated.platform_admin_id";

const SET_TENANT: &str = "SELECT set_config('med_tracker.current_household_id', (SELECT household_id::text FROM public.support_access_sessions WHERE id=$1), true)";

const AUDIT_EXPIRED: &str = "INSERT INTO public.security_audit_events(household_id,actor_account_id,event_type,metadata,audit_context,created_at,updated_at) SELECT s.household_id,pa.account_id,'support_access_session.expired',jsonb_build_object('support_access_session_id',s.id,'expired_at',to_char(s.expires_at,'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),'outcome','expired'),'{}'::jsonb,clock_timestamp(),clock_timestamp() FROM public.support_access_sessions s JOIN public.platform_admins pa ON pa.id=s.platform_admin_id WHERE s.id=$1";

const DUE_PAGE: &str = "SELECT id FROM public.support_access_sessions WHERE ended_at IS NULL AND expired_at IS NULL AND expires_at <= clock_timestamp() AND id > $1 ORDER BY id LIMIT 500";

async fn expire_one(db: &DatabaseConnection, id: i64) -> Result<bool, OperationError> {
    let transaction = begin(db).await?;
    let changed = match transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            EXPIRE_SESSION,
            [id.into()],
        ))
        .await
    {
        Ok(result) => result,
        Err(error) => {
            let _ = transaction.rollback().await;
            return Err(error.into());
        }
    };
    if changed.rows_affected() > 0 {
        let audited = async {
            transaction
                .execute_raw(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    SET_TENANT,
                    [id.into()],
                ))
                .await?;
            transaction
                .execute_raw(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    AUDIT_EXPIRED,
                    [id.into()],
                ))
                .await
        }
        .await;
        if let Err(error) = audited {
            let _ = transaction.rollback().await;
            return Err(error.into());
        }
    }
    transaction.commit().await?;
    Ok(changed.rows_affected() > 0)
}

pub async fn expire_due_support_sessions(ctx: &AppContext) -> Result<u64, OperationError> {
    let mut processed = 0_u64;
    let mut last_id = 0_i64;
    loop {
        let transaction = begin(&ctx.db).await?;
        let rows = transaction
            .query_all_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                DUE_PAGE,
                [last_id.into()],
            ))
            .await?;
        transaction.commit().await?;
        if rows.is_empty() {
            break;
        }
        for row in rows {
            let id: i64 = row
                .try_get("", "id")
                .map_err(|_| OperationError::Unavailable)?;
            last_id = id;
            match expire_one(&ctx.db, id).await {
                Ok(true) => processed += 1,
                Ok(false) => {}
                Err(error) => {
                    tracing::warn!(support_access_session_id = id, error = %error, "support access expiry skipped a session");
                }
            }
        }
    }
    Ok(processed)
}

pub struct SupportExpiryTask;

#[async_trait]
impl Task for SupportExpiryTask {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "support-access-expire".into(),
            detail: "Record natural support-access expiry events exactly once".into(),
        }
    }
    async fn run(&self, ctx: &AppContext, _vars: &Vars) -> loco_rs::Result<()> {
        let processed_count = expire_due_support_sessions(ctx)
            .await
            .map_err(|_| loco_rs::Error::string("Support access expiry failed"))?;
        tracing::info!(
            "{}",
            json!({"event_type":"support_access_session.expired","outcome":"success","processed_count":processed_count})
        );
        Ok(())
    }
}
