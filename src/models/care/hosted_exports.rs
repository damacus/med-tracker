use std::time::Duration;

use async_trait::async_trait;
use loco_rs::{
    app::AppContext,
    task::{Task, TaskInfo, Vars},
};
use sea_orm::{ConnectionTrait, DatabaseTransaction, DbBackend, Statement};
use serde_json::json;

use crate::models::{errors::OperationError, platform::begin, settings, storage::ArtifactStores};

const DEFAULT_GENERATION_TIMEOUT_MINUTES: i64 = 60;
const OPERATION_TIMEOUT: Duration = Duration::from_secs(60);

const HOUSEHOLD_PAGE: &str =
    "SELECT id FROM public.households WHERE id > $1 ORDER BY id LIMIT 1000";

const DUE_PREDICATE: &str = "((status IN ('ready','downloaded') AND expires_at <= clock_timestamp()) OR (status='generating' AND generation_started_at IS NOT NULL AND generation_started_at <= clock_timestamp() - make_interval(mins => $2::int) AND EXISTS(SELECT 1 FROM public.active_storage_attachments WHERE record_type='HouseholdExport' AND record_id=household_exports.id AND name='artifact')))";

const ACTIVE_HOLD: &str =
    "SELECT 1 FROM public.household_retention_holds WHERE household_id=$1 AND status='active'";

const LOCK_EXPORT: &str = "SELECT id,household_id FROM public.household_exports WHERE id=$1 AND ((status IN ('ready','downloaded') AND expires_at <= clock_timestamp()) OR (status='generating' AND generation_started_at IS NOT NULL AND generation_started_at <= clock_timestamp() - make_interval(mins => $2::int) AND EXISTS(SELECT 1 FROM public.active_storage_attachments WHERE record_type='HouseholdExport' AND record_id=household_exports.id AND name='artifact'))) AND NOT EXISTS(SELECT 1 FROM public.household_retention_holds WHERE household_id=household_exports.household_id AND status='active') FOR UPDATE";

fn generation_timeout_minutes(ctx: &AppContext) -> i64 {
    settings::integer(
        ctx.config.settings.as_ref(),
        &["household_exports", "generation_timeout_minutes"],
    )
    .map(|value| value.clamp(1, 1440))
    .unwrap_or(DEFAULT_GENERATION_TIMEOUT_MINUTES)
}

async fn purge_blob(
    stores: Option<&ArtifactStores>,
    service_name: &str,
    key: &str,
) -> Result<(), OperationError> {
    let Some(backends) = stores.and_then(|stores| stores.0.get(service_name)) else {
        tracing::warn!(
            service_name,
            "unmapped artifact storage service for export blob"
        );
        return Err(OperationError::Unavailable);
    };
    for backend in backends {
        tokio::time::timeout(OPERATION_TIMEOUT, backend.delete(key))
            .await
            .map_err(|_| OperationError::Unavailable)?
            .map_err(|_| OperationError::Unavailable)?;
    }
    Ok(())
}

async fn expire_export_in(
    transaction: &DatabaseTransaction,
    ctx: &AppContext,
    timeout_minutes: i64,
    export_id: i64,
) -> Result<bool, OperationError> {
    transaction
        .execute_unprepared("SAVEPOINT export_expiry")
        .await?;
    let result = expire_export_locked(transaction, ctx, timeout_minutes, export_id).await;
    match result {
        Ok(changed) => {
            transaction
                .execute_unprepared("RELEASE SAVEPOINT export_expiry")
                .await?;
            Ok(changed)
        }
        Err(error) => {
            let _ = transaction
                .execute_unprepared("ROLLBACK TO SAVEPOINT export_expiry")
                .await;
            Err(error)
        }
    }
}

async fn expire_export_locked(
    transaction: &DatabaseTransaction,
    ctx: &AppContext,
    timeout_minutes: i64,
    export_id: i64,
) -> Result<bool, OperationError> {
    let locked = transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            LOCK_EXPORT,
            [export_id.into(), timeout_minutes.into()],
        ))
        .await?;
    let Some(locked) = locked else {
        return Ok(false);
    };
    let household_id: i64 = locked
        .try_get("", "household_id")
        .map_err(|_| OperationError::Unavailable)?;
    let blobs = transaction
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT a.id AS attachment_id,b.id AS blob_id,b.key,b.service_name FROM public.active_storage_attachments a JOIN public.active_storage_blobs b ON b.id=a.blob_id WHERE a.record_type='HouseholdExport' AND a.record_id=$1 AND a.name='artifact'",
            [export_id.into()],
        ))
        .await?;
    let stores = ctx.shared_store.get::<ArtifactStores>();
    for blob in &blobs {
        let attachment_id: i64 = blob
            .try_get("", "attachment_id")
            .map_err(|_| OperationError::Unavailable)?;
        let blob_id: i64 = blob
            .try_get("", "blob_id")
            .map_err(|_| OperationError::Unavailable)?;
        let key: String = blob
            .try_get("", "key")
            .map_err(|_| OperationError::Unavailable)?;
        let service_name: String = blob
            .try_get("", "service_name")
            .map_err(|_| OperationError::Unavailable)?;
        transaction
            .execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT id FROM public.active_storage_blobs WHERE id=$1 FOR UPDATE",
                [blob_id.into()],
            ))
            .await?;
        transaction
            .execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "DELETE FROM public.active_storage_attachments WHERE id=$1",
                [attachment_id.into()],
            ))
            .await?;
        let removed = transaction
            .execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "DELETE FROM public.active_storage_blobs WHERE id=$1 AND NOT EXISTS(SELECT 1 FROM public.active_storage_attachments WHERE blob_id=$1)",
                [blob_id.into()],
            ))
            .await?;
        if removed.rows_affected() > 0 {
            purge_blob(stores.as_ref(), &service_name, &key).await?;
        }
    }
    transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE public.household_exports SET status='expired',expired_at=clock_timestamp(),updated_at=clock_timestamp() WHERE id=$1 AND status <> 'expired'",
            [export_id.into()],
        ))
        .await?;
    transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO public.security_audit_events(household_id,event_type,metadata,audit_context,created_at,updated_at) VALUES($1,'household.export.expired',jsonb_build_object('export_id',$2,'outcome','expired'),'{}'::jsonb,clock_timestamp(),clock_timestamp())",
            [household_id.into(), export_id.into()],
        ))
        .await?;
    Ok(true)
}

async fn expire_household(
    ctx: &AppContext,
    timeout_minutes: i64,
    household_id: i64,
) -> Result<u64, OperationError> {
    let transaction = begin(&ctx.db).await?;
    let result = async {
        transaction
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT set_config('med_tracker.current_household_id', $1, true)",
                [household_id.to_string().into()],
            ))
            .await?;
        let household = transaction
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT id FROM public.households WHERE id=$1 FOR UPDATE",
                [household_id.into()],
            ))
            .await?;
        if household.is_none() {
            return Ok::<_, OperationError>(0_u64);
        }
        let held = transaction
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                ACTIVE_HOLD,
                [household_id.into()],
            ))
            .await?;
        if held.is_some() {
            return Ok::<_, OperationError>(0_u64);
        }
        let due = transaction
            .query_all_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                format!(
                    "SELECT id FROM public.household_exports WHERE household_id=$1 AND {DUE_PREDICATE} ORDER BY id"
                ),
                [household_id.into(), timeout_minutes.into()],
            ))
            .await?;
        let mut processed = 0_u64;
        for row in due {
            let export_id: i64 = row
                .try_get("", "id")
                .map_err(|_| OperationError::Unavailable)?;
            match expire_export_in(&transaction, ctx, timeout_minutes, export_id).await {
                Ok(true) => processed += 1,
                Ok(false) => {}
                Err(error) => {
                    tracing::warn!(household_export_id = export_id, error = %error, "household export expiry skipped an export");
                }
            }
        }
        Ok(processed)
    }
    .await;
    match result {
        Ok(processed) => {
            transaction.commit().await?;
            Ok(processed)
        }
        Err(error) => {
            let _ = transaction.rollback().await;
            Err(error)
        }
    }
}

pub async fn expire_due_exports(ctx: &AppContext) -> Result<u64, OperationError> {
    let timeout_minutes = generation_timeout_minutes(ctx);
    let mut processed = 0_u64;
    let mut last_household_id = 0_i64;
    loop {
        let page = ctx
            .db
            .query_all_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                HOUSEHOLD_PAGE,
                [last_household_id.into()],
            ))
            .await?;
        if page.is_empty() {
            break;
        }
        for row in page {
            let household_id: i64 = row
                .try_get("", "id")
                .map_err(|_| OperationError::Unavailable)?;
            last_household_id = household_id;
            match expire_household(ctx, timeout_minutes, household_id).await {
                Ok(count) => processed += count,
                Err(error) => {
                    tracing::warn!(household_id, error = %error, "household export expiry skipped a household");
                }
            }
        }
    }
    Ok(processed)
}

pub struct HouseholdExportsExpiryTask;

#[async_trait]
impl Task for HouseholdExportsExpiryTask {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "household-exports-expire".into(),
            detail: "Expire due household exports and destroy their artifacts".into(),
        }
    }
    async fn run(&self, ctx: &AppContext, _vars: &Vars) -> loco_rs::Result<()> {
        let processed_count = expire_due_exports(ctx)
            .await
            .map_err(|_| loco_rs::Error::string("Household exports expiry failed"))?;
        tracing::info!(
            "{}",
            json!({"event_type":"household_exports.expiry_run","outcome":"success","processed_count":processed_count})
        );
        Ok(())
    }
}
