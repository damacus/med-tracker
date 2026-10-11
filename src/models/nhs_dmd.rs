mod extract;
mod import;
pub mod storage;

use crate::models::{entities::nhs_dmd_import, errors::OperationError};
use async_trait::async_trait;
use chrono::Utc;
use loco_rs::{
    app::AppContext,
    bgworker::BackgroundWorker,
    task::{Task, TaskInfo, Vars},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, Statement, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::Write;

#[derive(Clone, Serialize, Deserialize)]
pub struct ImportArgs {
    pub run_id: i64,
}

pub struct ImportWorker {
    ctx: AppContext,
}

#[async_trait]
impl BackgroundWorker<ImportArgs> for ImportWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }
    async fn perform(&self, args: ImportArgs) -> loco_rs::Result<()> {
        run(&self.ctx, args.run_id)
            .await
            .map_err(|_| loco_rs::Error::string("NHS catalogue import could not finish"))
    }
}

pub async fn upload(
    ctx: &AppContext,
    filename: &str,
    bytes: Vec<u8>,
) -> Result<i64, OperationError> {
    if !filename.to_ascii_lowercase().ends_with(".zip")
        || bytes.is_empty()
        || bytes.len() > storage::MAX_ARCHIVE_BYTES
    {
        return Err(OperationError::Validation {
            details: json!({"error":"Choose a ZIP release archive no larger than 500 MB."}),
        });
    }
    let store = storage::ArchiveStore::configured()?;
    let key = format!("nhs-dmd/{}.zip", uuid::Uuid::new_v4());
    let size = bytes.len() as i64;
    let checksum = storage::checksum(&bytes);
    store.put(&key, bytes).await?;
    let verified = store
        .get(&key)
        .await
        .is_ok_and(|stored| stored.len() as i64 == size && storage::checksum(&stored) == checksum);
    if !verified {
        store.delete(&key).await?;
        return Err(OperationError::Unavailable);
    }
    let now = Utc::now().naive_utc();
    let created = nhs_dmd_import::ActiveModel {
        uploaded_filename: Set(filename.to_owned()),
        status: Set(0),
        archive_key: Set(Some(key.clone())),
        archive_service_name: Set(Some("rustfs".into())),
        archive_checksum: Set(Some(checksum)),
        archive_byte_size: Set(Some(size)),
        created_at: Set(now),
        updated_at: Set(now),
        total_records: Set(0),
        processed_records: Set(0),
        imported_count: Set(0),
        created_count: Set(0),
        updated_count: Set(0),
        unchanged_count: Set(0),
        skipped_count: Set(0),
        skipped_expired_count: Set(0),
        skipped_invalid_count: Set(0),
        skipped_missing_name_count: Set(0),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await;
    let row = match created {
        Ok(row) => row,
        Err(error) => {
            store.delete(&key).await?;
            return Err(
                if matches!(
                    error.sql_err(),
                    Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
                ) {
                    OperationError::Conflict {
                        code: "import_in_progress".into(),
                        details: json!({"error":"An NHS catalogue import is already in progress."}),
                    }
                } else {
                    OperationError::Unavailable
                },
            );
        }
    };
    if ImportWorker::perform_later(ctx, ImportArgs { run_id: row.id })
        .await
        .is_err()
    {
        import::fail_run(
            &ctx.db,
            row.id,
            "The import could not be queued. Upload the release again.",
        )
        .await?;
        cleanup(ctx, &store, &row).await?;
        return Err(OperationError::Unavailable);
    }
    Ok(row.id)
}

pub async fn latest(ctx: &AppContext) -> Result<Vec<nhs_dmd_import::Model>, OperationError> {
    Ok(nhs_dmd_import::Entity::find()
        .order_by_desc(nhs_dmd_import::Column::Id)
        .limit(20)
        .all(&ctx.db)
        .await?)
}

pub async fn run(ctx: &AppContext, id: i64) -> Result<(), OperationError> {
    let lease = ctx.db.begin().await?;
    let locked = lease
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT pg_try_advisory_xact_lock(7369918) AS locked",
        ))
        .await?
        .ok_or(OperationError::Unavailable)?
        .try_get::<bool>("", "locked")?;
    if !locked {
        lease.rollback().await?;
        return Ok(());
    }
    let Some(row) = nhs_dmd_import::Entity::find_by_id(id).one(&ctx.db).await? else {
        return Ok(());
    };
    let Some(key) = row.archive_key.as_deref() else {
        if row.status < 4 {
            import::fail_run(
                &ctx.db,
                id,
                "The release archive is missing. Upload the release again.",
            )
            .await?;
        }
        return Ok(());
    };
    let store = match storage::ArchiveStore::configured() {
        Ok(store) => store,
        Err(error) => {
            import::fail_run(
                &ctx.db,
                id,
                "Archive storage is unavailable. Upload the release again.",
            )
            .await?;
            return Err(error);
        }
    };
    if row.status >= 4 {
        cleanup(ctx, &store, &row).await?;
        lease.commit().await?;
        return Ok(());
    }
    let bytes = match store.get(key).await {
        Ok(bytes) => bytes,
        Err(error) => {
            import::fail_run(
                &ctx.db,
                id,
                "The release archive could not be read. Upload the release again.",
            )
            .await?;
            cleanup(ctx, &store, &row).await?;
            lease.commit().await?;
            return Err(error);
        }
    };
    let valid = row.archive_byte_size == Some(bytes.len() as i64)
        && row.archive_checksum.as_deref() == Some(storage::checksum(&bytes).as_str());
    let result = if valid {
        let mut file = tempfile::NamedTempFile::new().map_err(|_| OperationError::Unavailable)?;
        file.write_all(&bytes)
            .map_err(|_| OperationError::Unavailable)?;
        import::run_import(ctx.db.clone(), id, file.path().to_path_buf()).await
    } else {
        Err("Archive integrity verification failed".into())
    };
    if let Err(error) = result {
        import::fail_run(&ctx.db, id, &error).await?;
    }
    cleanup(ctx, &store, &row).await?;
    lease.commit().await?;
    Ok(())
}

async fn cleanup(
    ctx: &AppContext,
    store: &storage::ArchiveStore,
    row: &nhs_dmd_import::Model,
) -> Result<(), OperationError> {
    if let Some(key) = &row.archive_key {
        store.delete(key).await?;
    }
    nhs_dmd_import::Entity::update_many()
        .col_expr(
            nhs_dmd_import::Column::ArchiveKey,
            sea_orm::sea_query::Expr::value(Option::<String>::None),
        )
        .col_expr(
            nhs_dmd_import::Column::ArchiveChecksum,
            sea_orm::sea_query::Expr::value(Option::<String>::None),
        )
        .col_expr(
            nhs_dmd_import::Column::ArchiveByteSize,
            sea_orm::sea_query::Expr::value(Option::<i64>::None),
        )
        .col_expr(
            nhs_dmd_import::Column::ArchiveServiceName,
            sea_orm::sea_query::Expr::value(Option::<String>::None),
        )
        .filter(nhs_dmd_import::Column::Id.eq(row.id))
        .exec(&ctx.db)
        .await?;
    Ok(())
}

pub async fn reconcile(ctx: &AppContext) -> Result<(), OperationError> {
    ctx.db.execute_unprepared("UPDATE nhs_dmd_imports SET status=5,error_message='The import stopped making progress. Upload the release again.',completed_at=now(),updated_at=now() WHERE status IN (0,1,2,3) AND updated_at < now()-interval '30 minutes'").await?;
    let lease = ctx.db.begin().await?;
    let locked = lease
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT pg_try_advisory_xact_lock(7369918) AS locked",
        ))
        .await?
        .ok_or(OperationError::Unavailable)?
        .try_get::<bool>("", "locked")?;
    if !locked {
        return Ok(());
    }
    lease.commit().await?;
    let stale = nhs_dmd_import::Entity::find()
        .filter(nhs_dmd_import::Column::ArchiveKey.is_not_null())
        .filter(nhs_dmd_import::Column::Status.gte(4))
        .all(&ctx.db)
        .await?;
    for row in stale {
        ImportWorker::perform_later(ctx, ImportArgs { run_id: row.id })
            .await
            .map_err(|_| OperationError::Unavailable)?;
    }
    Ok(())
}

pub struct ReconcileTask;
#[async_trait]
impl Task for ReconcileTask {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "nhs_dmd_reconcile".into(),
            detail: "Fail stale NHS catalogue imports and clean terminal archives".into(),
        }
    }
    async fn run(&self, ctx: &AppContext, _vars: &Vars) -> loco_rs::Result<()> {
        reconcile(ctx)
            .await
            .map_err(|_| loco_rs::Error::string("NHS catalogue reconciliation failed"))
    }
}
