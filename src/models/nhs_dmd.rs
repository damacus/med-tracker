mod extract;
mod import;
pub mod storage;

use crate::models::{entities::nhs_dmd_import, errors::OperationError};
use async_trait::async_trait;
use chrono::Utc;
use import::{ImportFailure, ImportFailureCategory, ImportStage};
use loco_rs::{
    app::AppContext,
    bgworker::BackgroundWorker,
    task::{Task, TaskInfo, Vars},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, ExprTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, Statement, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::Write;

#[derive(Clone, Serialize, Deserialize)]
pub struct ImportArgs {
    pub run_id: i64,
    #[serde(default)]
    pub traceparent: Option<String>,
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
        let traceparent = args.traceparent.clone();
        crate::telemetry::units::job_with_parent("import", traceparent.as_deref(), async {
            crate::models::canary::work(&self.ctx).await?;
            run(&self.ctx, args.run_id)
                .await
                .map_err(|_| loco_rs::Error::string("NHS catalogue import could not finish"))
        })
        .await
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
    let store = storage::ArchiveStore::configured(ctx)?;
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
        trace_id: Set(crate::telemetry::ids::current_traceparent()
            .and_then(|value| crate::telemetry::ids::trace_id_from(Some(&value)))),
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
    if ImportWorker::perform_later(
        ctx,
        ImportArgs {
            run_id: row.id,
            traceparent: crate::telemetry::ids::current_traceparent(),
        },
    )
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
            import::fail_owned_run(
                &ctx.db,
                &row,
                &ImportFailure::with_summary(
                    ImportStage::Extract,
                    ImportFailureCategory::ArchiveInvalid,
                    "The release archive is missing. Upload the release again.",
                ),
            )
            .await?;
        }
        return Ok(());
    };
    let store = match storage::ArchiveStore::configured(ctx) {
        Ok(store) => store,
        Err(error) => {
            import::fail_owned_run(
                &ctx.db,
                &row,
                &ImportFailure::new(ImportStage::Extract, ImportFailureCategory::Storage),
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
            import::fail_owned_run(
                &ctx.db,
                &row,
                &ImportFailure::with_summary(
                    ImportStage::Extract,
                    ImportFailureCategory::Storage,
                    "The release archive could not be read. Upload the release again.",
                ),
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
        Err(crate::models::nhs_dmd::import::ImportFailure::with_summary(
            crate::models::nhs_dmd::import::ImportStage::Extract,
            crate::models::nhs_dmd::import::ImportFailureCategory::ArchiveInvalid,
            "Archive integrity verification failed",
        ))
    };
    match &result {
        Err(error) => {
            tracing::error!(
                event.name = "medtracker.import.failed",
                medtracker.import.run_id = row.id,
                medtracker.import.stage = error.stage.name(),
                error.type = error.category.name(),
                medtracker.import.trace_id = row.trace_id.as_deref(),
                "NHS dm+d import failed"
            );
            import::fail_owned_run(&ctx.db, &row, error).await?;
        }
        Ok(()) => {
            let finished = nhs_dmd_import::Entity::find_by_id(row.id)
                .one(&ctx.db)
                .await
                .map_err(|_| OperationError::Unavailable)?
                .unwrap_or_else(|| row.clone());
            tracing::info!(
                event.name = "medtracker.import.completed",
                medtracker.import.run_id = finished.id,
                medtracker.import.trace_id = row.trace_id.as_deref(),
                medtracker.import.created_count = finished.created_count,
                medtracker.import.updated_count = finished.updated_count,
                medtracker.import.imported_count = finished.imported_count,
                medtracker.import.skipped_count = finished.skipped_count,
                "NHS dm+d import completed"
            );
        }
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
    let stale_marked = nhs_dmd_import::Entity::update_many()
        .col_expr(
            nhs_dmd_import::Column::Status,
            sea_orm::sea_query::Expr::value(5),
        )
        .col_expr(
            nhs_dmd_import::Column::ErrorMessage,
            sea_orm::sea_query::Expr::value(
                "The import stopped making progress. Upload the release again.",
            ),
        )
        .col_expr(
            nhs_dmd_import::Column::CompletedAt,
            sea_orm::sea_query::Expr::current_timestamp(),
        )
        .col_expr(
            nhs_dmd_import::Column::UpdatedAt,
            sea_orm::sea_query::Expr::current_timestamp(),
        )
        .filter(nhs_dmd_import::Column::Status.is_in([0, 1, 2, 3]))
        .filter(
            sea_orm::Condition::all().add(
                sea_orm::sea_query::Expr::col(nhs_dmd_import::Column::UpdatedAt).lt(
                    sea_orm::sea_query::Expr::current_timestamp().sub(
                        sea_orm::sea_query::Expr::val("30 minutes")
                            .cast_as(sea_orm::sea_query::Alias::new("interval")),
                    ),
                ),
            ),
        )
        .exec_with_returning(&ctx.db)
        .await?;
    let stale_failed_count = stale_marked.len() as i64;
    for marked in &stale_marked {
        tracing::warn!(
            event.name = "medtracker.import.stale",
            medtracker.import.run_id = marked.id,
            medtracker.import.trace_id = marked.trace_id.as_deref(),
            error.type = ImportFailureCategory::Stale.name(),
            "NHS dm+d import marked stale"
        );
    }
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
    let requeued = stale.len() as i64;
    for row in &stale {
        ImportWorker::perform_later(
            ctx,
            ImportArgs {
                run_id: row.id,
                traceparent: None,
            },
        )
        .await
        .map_err(|_| OperationError::Unavailable)?;
    }
    tracing::info!(
        event.name = "medtracker.import.reconciled",
        medtracker.import.requeued_count = requeued,
        medtracker.import.stale_failed_count = stale_failed_count,
        "NHS dm+d reconcile finished"
    );
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
        crate::telemetry::units::task("nhs_dmd_reconcile", async {
            crate::models::canary::work(ctx).await?;
            reconcile(ctx)
                .await
                .map_err(|_| loco_rs::Error::string("NHS catalogue reconciliation failed"))
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::ImportArgs;

    #[test]
    fn import_args_without_traceparent_deserialises() {
        let args: ImportArgs = serde_json::from_str(r#"{"run_id": 42}"#).expect("legacy payload");
        assert_eq!(args.run_id, 42);
        assert!(args.traceparent.is_none());
        let args: ImportArgs = serde_json::from_str(
            r#"{"run_id": 42, "traceparent": "00-7fa01885bf8086b9a009c00c9ca89e1b-6dd454d737734ffa-01"}"#,
        )
        .expect("new payload");
        assert!(args.traceparent.is_some());
    }
}
