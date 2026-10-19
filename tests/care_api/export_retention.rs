use super::fixture::Fixture;
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, DbErr, QueryResult,
    Statement, TransactionTrait,
};
use std::time::Duration;

fn missing() -> DbErr {
    DbErr::Custom("Synthetic export retention assertion row missing".into())
}

async fn seed(fixture: &Fixture, status: &str) -> Result<(), DbErr> {
    fixture.admin.execute_unprepared("INSERT INTO public.accounts(id,email,status,created_at,updated_at) VALUES(99101,'export-retention-foreign@example.test',2,timezone('UTC',clock_timestamp()),timezone('UTC',clock_timestamp())); INSERT INTO public.households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(99102,99101,'Synthetic foreign export household','synthetic-export-retention-foreign','UTC',timezone('UTC',clock_timestamp()),timezone('UTC',clock_timestamp()))").await?;
    fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO public.household_exports(id,household_id,requested_by_account_id,requested_at,status,expires_at,expired_at,created_at,updated_at) VALUES(99201,72001,71001,timezone('UTC',clock_timestamp())-interval '2 days',$1,timezone('UTC',clock_timestamp())-interval '1 hour',CASE WHEN $1='expired' THEN timezone('UTC',clock_timestamp()) ELSE NULL END,timezone('UTC',clock_timestamp()),timezone('UTC',clock_timestamp()))",
        [status.into()])).await?;
    fixture.admin.execute_unprepared("INSERT INTO public.active_storage_blobs(id,byte_size,filename,key,service_name,content_type,created_at) VALUES(99301,4,'synthetic-export.zip','synthetic-export-retention-object','s3','application/zip',timezone('UTC',clock_timestamp())); INSERT INTO public.active_storage_attachments(id,blob_id,household_id,name,record_id,record_type,created_at) VALUES(99401,99301,72001,'artifact',99201,'HouseholdExport',timezone('UTC',clock_timestamp()))").await?;
    Ok(())
}

async fn hold(fixture: &Fixture, status: &str) -> Result<(), DbErr> {
    fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO public.household_retention_holds(id,household_id,approved_by_account_id,placed_at,reason,review_on,status,released_by_account_id,released_at,created_at,updated_at) VALUES(99501,72001,71001,timezone('UTC',clock_timestamp()),'Synthetic export retention hold',current_date+30,$1,CASE WHEN $1='released' THEN 71001 ELSE NULL END,CASE WHEN $1='released' THEN timezone('UTC',clock_timestamp()) ELSE NULL END,timezone('UTC',clock_timestamp()),timezone('UTC',clock_timestamp()))",
        [status.into()])).await?;
    Ok(())
}

async fn runtime(db: &DatabaseConnection, scoped: bool) -> Result<DatabaseTransaction, DbErr> {
    let transaction = db.begin().await?;
    transaction
        .execute_unprepared("SET LOCAL ROLE med_tracker_app")
        .await?;
    if scoped {
        transaction.execute_unprepared("SELECT set_config('med_tracker.current_account_id','71001',true),set_config('med_tracker.current_household_id','72001',true),set_config('med_tracker.current_membership_id','74001',true),set_config('med_tracker.current_person_id','73001',true)").await?;
    }
    Ok(transaction)
}

async fn expire(transaction: &DatabaseTransaction) -> Result<bool, DbErr> {
    transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT med_tracker.expire_household_export($1,$2) AS changed",
            [99201_i64.into(), 60_i32.into()],
        ))
        .await?
        .ok_or_else(missing)?
        .try_get("", "changed")
}

async fn targets(transaction: &DatabaseTransaction) -> Result<Vec<QueryResult>, DbErr> {
    transaction.query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT blob_id,key,service_name,retained FROM med_tracker.household_export_cleanup_targets($1)",
        [99201_i64.into()])).await
}

async fn state(fixture: &Fixture) -> Result<QueryResult, DbErr> {
    fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT e.status,(SELECT count(*) FROM public.active_storage_attachments WHERE record_type='HouseholdExport' AND record_id=e.id AND name='artifact') AS attachments,(SELECT count(*) FROM public.security_audit_events WHERE event_type='household.export.expired' AND metadata=jsonb_build_object('export_id',e.id,'outcome','expired')) AS audits,(SELECT count(*) FROM public.pg_loco_queue WHERE task_data->>'export_id'=e.id::text AND name='med_tracker::models::care::hosted_exports::ExportCleanupWorker') AS jobs FROM public.household_exports e WHERE e.id=99201")).await?.ok_or_else(missing)
}

#[tokio::test]
async fn logical_expiry_audit_and_cleanup_enqueue_commit_once_after_requester_leaves()
-> Result<(), DbErr> {
    let fixture = Fixture::new().await;
    let result = async {
        seed(&fixture, "ready").await?;
        fixture.admin.execute_unprepared("UPDATE public.household_memberships SET status='revoked',revoked_at=timezone('UTC',clock_timestamp()) WHERE id=74001").await?;
        let transaction = runtime(&fixture.runtime, false).await?;
        let changed = expire(&transaction).await?;
        transaction.commit().await?;
        let transaction = runtime(&fixture.runtime, false).await?;
        let replay = expire(&transaction).await?;
        transaction.commit().await?;
        let row = state(&fixture).await?;
        let audit = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT household_id,audit_context->>'authentication_method' AS method,audit_context->>'active_role' AS role,audit_context->>'requested_by_account_id' AS requester,audit_context->>'actor_account_id' AS actor FROM public.security_audit_events WHERE event_type='household.export.expired' AND metadata->>'export_id'='99201'")).await?.ok_or_else(missing)?;
        assert!(changed);
        assert!(!replay);
        assert_eq!(row.try_get::<String>("", "status")?, "expired");
        assert_eq!(row.try_get::<i64>("", "attachments")?, 1);
        assert_eq!(row.try_get::<i64>("", "audits")?, 1);
        assert_eq!(row.try_get::<i64>("", "jobs")?, 1);
        assert_eq!(audit.try_get::<i64>("", "household_id")?, 72001);
        assert_eq!(audit.try_get::<String>("", "method")?, "system");
        assert_eq!(audit.try_get::<String>("", "role")?, "household_export_expiry_task");
        assert_eq!(audit.try_get::<String>("", "requester")?, "71001");
        assert_eq!(audit.try_get::<Option<String>>("", "actor")?, None);
        Ok(())
    }.await;
    fixture.close().await;
    result
}

#[tokio::test]
async fn active_household_hold_defers_expiry_and_cleanup_without_erasing_artifact_identity()
-> Result<(), DbErr> {
    let fixture = Fixture::new().await;
    let result = async {
        seed(&fixture, "ready").await?;
        hold(&fixture, "active").await?;
        let transaction = runtime(&fixture.runtime, false).await?;
        let changed = expire(&transaction).await?;
        transaction.commit().await?;
        let ready = state(&fixture).await?;
        fixture.admin.execute_unprepared("UPDATE public.household_exports SET status='expired',expired_at=timezone('UTC',clock_timestamp()) WHERE id=99201").await?;
        let transaction = runtime(&fixture.runtime, false).await?;
        let cleanup = targets(&transaction).await?;
        transaction.commit().await?;
        let pending = state(&fixture).await?;
        assert!(!changed);
        assert_eq!(ready.try_get::<String>("", "status")?, "ready");
        assert_eq!(ready.try_get::<i64>("", "audits")?, 0);
        assert_eq!(ready.try_get::<i64>("", "jobs")?, 0);
        assert!(cleanup.is_empty());
        assert_eq!(pending.try_get::<i64>("", "attachments")?, 1);
        Ok(())
    }.await;
    fixture.close().await;
    result
}

async fn retained(fixture: &Fixture) -> Result<bool, DbErr> {
    let transaction = runtime(&fixture.runtime, false).await?;
    let rows = targets(&transaction).await?;
    let row = rows.first().ok_or_else(missing)?;
    let result = row.try_get::<bool>("", "retained")?;
    assert_eq!(rows.len(), 1);
    assert_eq!(row.try_get::<i64>("", "blob_id")?, 99301);
    assert_eq!(
        row.try_get::<String>("", "key")?,
        "synthetic-export-retention-object"
    );
    assert_eq!(row.try_get::<String>("", "service_name")?, "s3");
    transaction.commit().await?;
    Ok(result)
}

#[tokio::test]
async fn cleanup_projection_counts_foreign_attachment_and_account_avatar_references()
-> Result<(), DbErr> {
    let fixture = Fixture::new().await;
    let result = async {
        seed(&fixture, "expired").await?;
        fixture.admin.execute_unprepared("INSERT INTO public.active_storage_attachments(id,blob_id,household_id,name,record_id,record_type,created_at) VALUES(99402,99301,99102,'synthetic-retained',99101,'SyntheticRetainedRecord',timezone('UTC',clock_timestamp()))").await?;
        let attachment = retained(&fixture).await?;
        fixture.admin.execute_unprepared("DELETE FROM public.active_storage_attachments WHERE id=99402; INSERT INTO public.account_avatars(account_id,key,content_type,service_name,byte_size,checksum) VALUES(99101,'synthetic-export-retention-object','image/png','s3',4,'synthetic-checksum')").await?;
        let avatar = retained(&fixture).await?;
        fixture.admin.execute_unprepared("DELETE FROM public.account_avatars WHERE account_id=99101").await?;
        let unshared = retained(&fixture).await?;
        assert!(attachment);
        assert!(avatar);
        assert!(!unshared);
        Ok(())
    }.await;
    fixture.close().await;
    result
}

#[tokio::test]
async fn unknown_artifact_service_fails_closed_without_detaching_pending_export()
-> Result<(), DbErr> {
    let fixture = Fixture::new().await;
    let result = async {
        seed(&fixture,"expired").await?;
        fixture.admin.execute_unprepared("UPDATE public.active_storage_blobs SET service_name='synthetic-unknown-export-service' WHERE id=99301").await?;
        let transaction = runtime(&fixture.runtime,false).await?;
        let rejected = targets(&transaction).await;
        transaction.rollback().await?;
        let row = state(&fixture).await?;
        let error = rejected.err().ok_or_else(|| DbErr::Custom("Unknown artifact service was accepted".into()))?;
        let code = match error { DbErr::Query(sea_orm::RuntimeErr::SqlxError(error)) => error.as_database_error().and_then(|error|error.code()).map(|value|value.into_owned()), _ => None };
        assert_eq!(code.as_deref(),Some("42501"));
        assert_eq!(row.try_get::<i64>("", "attachments")?,1);
        assert_eq!(row.try_get::<i64>("", "audits")?,0);
        Ok(())
    }.await;
    fixture.close().await;
    result
}

async fn hold_cutoff_wait(
    fixture: &Fixture,
    insert: bool,
) -> Result<bool, Box<dyn std::error::Error>> {
    let blocker = runtime(&fixture.runtime, true).await?;
    let blocker_pid: i32 = blocker
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT pg_backend_pid() AS pid FROM public.households WHERE id=72001 FOR UPDATE",
        ))
        .await?
        .ok_or_else(missing)?
        .try_get("", "pid")?;
    let db = fixture.runtime.clone();
    let (started, received) = tokio::sync::oneshot::channel();
    let mut writer = tokio::spawn(async move {
        let transaction = runtime(&db, true).await?;
        let pid = transaction
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT pg_backend_pid() AS pid",
            ))
            .await?
            .ok_or_else(missing)?
            .try_get::<i32>("", "pid")?;
        started.send(pid).map_err(|_| missing())?;
        if insert {
            transaction.execute_unprepared("INSERT INTO public.household_retention_holds(household_id,approved_by_account_id,placed_at,reason,review_on,status,created_at,updated_at) VALUES(72001,71001,timezone('UTC',clock_timestamp()),'Synthetic cutoff hold',current_date+30,'active',timezone('UTC',clock_timestamp()),timezone('UTC',clock_timestamp()))").await?;
        } else {
            let changed = transaction.execute_unprepared("UPDATE public.household_retention_holds SET status='active',released_by_account_id=NULL,released_at=NULL WHERE id=99501").await?;
            if changed.rows_affected() != 1 {
                return Err(missing());
            }
        }
        transaction.commit().await
    });
    let result = async {
        let writer_pid = tokio::time::timeout(Duration::from_secs(3), received).await??;
        let blocked = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                let row = fixture
                    .admin
                    .query_one_raw(Statement::from_sql_and_values(
                        DbBackend::Postgres,
                        "SELECT $1=ANY(pg_blocking_pids($2)) AS blocked",
                        [blocker_pid.into(), writer_pid.into()],
                    ))
                    .await?
                    .ok_or_else(missing)?;
                if row.try_get::<bool>("", "blocked")? {
                    return Ok::<_, DbErr>(true);
                }
                if writer.is_finished() {
                    return Ok(false);
                }
                tokio::task::yield_now().await;
            }
        })
        .await??;
        blocker.rollback().await?;
        tokio::time::timeout(Duration::from_secs(3), &mut writer).await???;
        Ok::<_, Box<dyn std::error::Error>>(blocked)
    }
    .await;
    if !writer.is_finished() {
        writer.abort();
        let _ = writer.await;
    }
    result
}

#[tokio::test]
async fn new_hold_commit_waits_for_household_cleanup_cutoff_via_existing_foreign_key()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new().await;
    let result = async {
        seed(&fixture, "expired").await?;
        hold_cutoff_wait(&fixture, true).await
    }
    .await;
    fixture.close().await;
    assert!(result?, "New hold committed inside the cleanup cutoff");
    Ok(())
}

#[tokio::test]
async fn reactivated_hold_waits_for_household_cleanup_cutoff_with_unchanged_foreign_key()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new().await;
    let result = async {
        seed(&fixture, "expired").await?;
        hold(&fixture, "released").await?;
        hold_cutoff_wait(&fixture, false).await
    }
    .await;
    fixture.close().await;
    assert!(
        result?,
        "Reactivated hold committed inside the cleanup cutoff"
    );
    Ok(())
}
