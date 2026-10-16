use super::{Application, fixture::Fixture};
use loco_rs::{
    app::Hooks,
    task::{Tasks, Vars},
};
use sea_orm::{
    ConnectionTrait, DatabaseTransaction, DbBackend, DbErr, QueryResult, Statement,
    TransactionTrait,
};

fn missing() -> DbErr {
    DbErr::Custom("Synthetic support expiry assertion row missing".into())
}

async fn seed(fixture: &Fixture, count: i32) -> Result<(), DbErr> {
    fixture.admin.execute_unprepared("INSERT INTO public.platform_admins(account_id,status,created_at,updated_at) VALUES(71001,'active',now(),now()) ON CONFLICT(account_id) DO UPDATE SET status='active'").await?;
    fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO public.support_access_sessions(id,household_id,platform_admin_id,reason,starts_at,expires_at,created_at,updated_at) SELECT 98000+g,72001,(SELECT id FROM public.platform_admins WHERE account_id=71001),'Synthetic worker expiry',clock_timestamp()-interval '2 hours',clock_timestamp()-interval '1 hour',clock_timestamp(),clock_timestamp() FROM generate_series(1,$1::integer) g",
        [count.into()])).await?;
    Ok(())
}

async fn runtime(fixture: &Fixture) -> Result<DatabaseTransaction, DbErr> {
    let transaction = fixture.runtime.begin().await?;
    transaction
        .execute_unprepared("SET LOCAL ROLE med_tracker_app")
        .await?;
    Ok(transaction)
}

async fn page(
    transaction: &DatabaseTransaction,
    after: i64,
    limit: i32,
) -> Result<Vec<i64>, DbErr> {
    transaction
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT id FROM med_tracker.due_support_access_sessions($1,$2)",
            [after.into(), limit.into()],
        ))
        .await?
        .into_iter()
        .map(|row| row.try_get("", "id"))
        .collect()
}

async fn expire(transaction: &DatabaseTransaction, id: i64) -> Result<bool, DbErr> {
    transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT med_tracker.expire_support_access_session($1) AS changed",
            [id.into()],
        ))
        .await?
        .ok_or_else(missing)?
        .try_get("", "changed")
}

async fn state(fixture: &Fixture, id: i64) -> Result<QueryResult, DbErr> {
    fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT expired_at IS NOT NULL AS expired,expired_at::text AS expiry,(SELECT count(*) FROM public.versions WHERE item_type='SupportAccessSession' AND item_id=$1 AND event='platform/support/expired') AS versions,(SELECT count(*) FROM public.security_audit_events WHERE event_type='support_access_session.expired' AND metadata->>'support_access_session_id'=$1::text) AS audits FROM public.support_access_sessions WHERE id=$1",
        [id.into()])).await?.ok_or_else(missing)
}

#[tokio::test]
async fn restricted_expiry_pages_past_500_and_excludes_ineligible_rows() -> Result<(), DbErr> {
    let fixture = Fixture::new().await;
    let result = async {
        seed(&fixture, 504).await?;
        fixture.admin.execute_unprepared("UPDATE public.support_access_sessions SET ended_at=clock_timestamp() WHERE id=98502; UPDATE public.support_access_sessions SET expired_at=clock_timestamp() WHERE id=98503; UPDATE public.support_access_sessions SET expires_at=clock_timestamp()+interval '1 hour' WHERE id=98504").await?;
        let transaction = runtime(&fixture).await?;
        let first = page(&transaction, 0, 500).await?;
        let second = page(&transaction, first.last().copied().ok_or_else(missing)?, 500).await?;
        let invalid = page(&transaction, 0, 501).await;
        transaction.rollback().await?;
        assert_eq!(first.len(), 500);
        assert_eq!(second, vec![98501]);
        assert!(invalid.is_err(), "Paging must not accept an unbounded limit");
        Ok(())
    }.await;
    fixture.close().await;
    result
}

#[tokio::test]
async fn restricted_expiry_records_both_audits_once_after_requester_revocation() -> Result<(), DbErr>
{
    let fixture = Fixture::new().await;
    let result = async {
        seed(&fixture, 1).await?;
        fixture.admin.execute_unprepared("UPDATE public.platform_admins SET status='revoked' WHERE account_id=71001; UPDATE public.household_memberships SET status='revoked',revoked_at=clock_timestamp() WHERE account_id=71001").await?;
        let transaction = runtime(&fixture).await?;
        let changed = expire(&transaction, 98001).await?;
        transaction.commit().await?;
        let original_expiry = state(&fixture, 98001).await?.try_get::<String>("", "expiry")?;
        let transaction = runtime(&fixture).await?;
        let repeated = expire(&transaction, 98001).await?;
        transaction.commit().await?;
        let row = state(&fixture, 98001).await?;
        let attribution = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT audit_context->>'actor_account_id' AS account, audit_context->>'active_role' AS role, whodunnit FROM public.versions WHERE item_type='SupportAccessSession' AND item_id=98001 AND event='platform/support/expired'"))
            .await?.ok_or_else(missing)?;
        assert!(changed);
        assert!(!repeated);
        assert_eq!(row.try_get::<String>("", "expiry")?, original_expiry);
        assert!(row.try_get::<bool>("", "expired")?);
        assert_eq!(row.try_get::<i64>("", "versions")?, 1);
        assert_eq!(row.try_get::<i64>("", "audits")?, 1);
        assert_eq!(attribution.try_get::<String>("", "account")?, "71001");
        assert_eq!(attribution.try_get::<String>("", "role")?, "support_expiry_task");
        assert_eq!(attribution.try_get::<Option<String>>("", "whodunnit")?, None);
        Ok(())
    }.await;
    fixture.close().await;
    result
}

#[tokio::test]
async fn expiry_audit_failure_rolls_back_session_and_version_then_retries() -> Result<(), DbErr> {
    let fixture = Fixture::new().await;
    let result = async {
        seed(&fixture, 1).await?;
        fixture.admin.execute_unprepared("CREATE FUNCTION public.reject_worker_expiry_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type='support_access_session.expired' THEN RAISE EXCEPTION USING ERRCODE='23514',MESSAGE='Synthetic expiry audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_worker_expiry_audit BEFORE INSERT ON public.security_audit_events FOR EACH ROW EXECUTE FUNCTION public.reject_worker_expiry_audit()").await?;
        let transaction = runtime(&fixture).await?;
        let rejected = expire(&transaction, 98001).await;
        transaction.rollback().await?;
        let row = state(&fixture, 98001).await?;
        assert!(rejected.is_err(), "Synthetic audit failure must be reached");
        assert!(!row.try_get::<bool>("", "expired")?);
        assert_eq!(row.try_get::<i64>("", "versions")?, 0);
        assert_eq!(row.try_get::<i64>("", "audits")?, 0);
        fixture.admin.execute_unprepared("DROP TRIGGER reject_worker_expiry_audit ON public.security_audit_events").await?;
        let transaction = runtime(&fixture).await?;
        assert!(expire(&transaction, 98001).await?);
        transaction.commit().await?;
        let row = state(&fixture, 98001).await?;
        assert!(row.try_get::<bool>("", "expired")?);
        assert_eq!(row.try_get::<i64>("", "versions")?, 1);
        assert_eq!(row.try_get::<i64>("", "audits")?, 1);
        Ok(())
    }.await;
    fixture.close().await;
    result
}

#[tokio::test]
async fn expiry_cannot_mutate_future_ended_expired_or_unknown_sessions() -> Result<(), DbErr> {
    let fixture = Fixture::new().await;
    let result = async {
        seed(&fixture, 3).await?;
        fixture.admin.execute_unprepared("UPDATE public.support_access_sessions SET expires_at=clock_timestamp()+interval '1 hour' WHERE id=98001; UPDATE public.support_access_sessions SET ended_at=clock_timestamp() WHERE id=98002; UPDATE public.support_access_sessions SET expired_at=clock_timestamp() WHERE id=98003").await?;
        let transaction = runtime(&fixture).await?;
        transaction.execute_unprepared("SET LOCAL search_path=pg_temp; CREATE TEMP TABLE support_access_sessions(id bigint,expired_at timestamp); INSERT INTO support_access_sessions VALUES(98001,NULL)").await?;
        for id in [98001, 98002, 98003, i64::MAX] {
            assert!(!expire(&transaction, id).await?, "Ineligible session must remain unchanged");
        }
        transaction.commit().await?;
        let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM public.versions WHERE item_type='SupportAccessSession' AND item_id BETWEEN 98001 AND 98003) AS versions,(SELECT count(*) FROM public.security_audit_events WHERE event_type='support_access_session.expired') AS audits"))
            .await?.ok_or_else(missing)?;
        assert_eq!(row.try_get::<i64>("", "versions")?, 0);
        assert_eq!(row.try_get::<i64>("", "audits")?, 0);
        Ok(())
    }.await;
    fixture.close().await;
    result
}

#[tokio::test]
async fn expiry_uses_utc_deadlines_and_timestamps_under_non_utc_caller_timezone()
-> Result<(), DbErr> {
    let fixture = Fixture::new().await;
    let result = async {
        seed(&fixture, 2).await?;
        fixture.admin.execute_unprepared("UPDATE public.support_access_sessions SET expires_at=timezone('UTC',clock_timestamp())+interval '1 hour' WHERE id=98002").await?;
        let transaction = runtime(&fixture).await?;
        transaction.execute_unprepared("SET LOCAL TIME ZONE 'Pacific/Kiritimati'").await?;
        let due = page(&transaction, 0, 500).await?;
        let future = expire(&transaction, 98002).await?;
        let past = expire(&transaction, 98001).await?;
        let row = transaction.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT current_setting('TimeZone') AS timezone,abs(extract(epoch FROM (expired_at-timezone('UTC',clock_timestamp()))))<60 AS utc_stamp FROM public.support_access_sessions WHERE id=98001"))
            .await?.ok_or_else(missing)?;
        let utc_stamp = row.try_get::<bool>("", "utc_stamp")?;
        let timezone = row.try_get::<String>("", "timezone")?;
        transaction.commit().await?;
        let source = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT count(*) AS count,bool_and(abs(extract(epoch FROM (created_at-timezone('UTC',clock_timestamp()))))<60) AS utc FROM (SELECT created_at FROM public.versions WHERE item_type='SupportAccessSession' AND item_id=98001 UNION ALL SELECT created_at FROM public.security_audit_events WHERE event_type='support_access_session.expired' AND metadata->>'support_access_session_id'='98001') s"))
            .await?.ok_or_else(missing)?;
        let ledger = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT count(*) AS count,bool_and(abs(extract(epoch FROM (occurred_at-timezone('UTC',clock_timestamp()))))<60 AND abs(extract(epoch FROM (created_at-timezone('UTC',clock_timestamp()))))<60 AND abs(extract(epoch FROM ((envelope->>'occurred_at')::timestamptz-clock_timestamp())))<60) AS utc FROM public.audit_ledger_entries WHERE (source_table='versions' AND source_payload->>'item_id'='98001') OR (source_table='security_audit_events' AND source_payload->'metadata'->>'support_access_session_id'='98001')"))
            .await?.ok_or_else(missing)?;
        Ok::<_,DbErr>((due,future,past,utc_stamp,timezone,
            source.try_get::<i64>("", "count")?,source.try_get::<bool>("", "utc")?,
            ledger.try_get::<i64>("", "count")?,ledger.try_get::<bool>("", "utc")?))
    }.await;
    fixture.close().await;
    let observed = result?;
    assert_eq!(
        observed,
        (
            vec![98001],
            false,
            true,
            true,
            "Pacific/Kiritimati".to_owned(),
            2,
            true,
            2,
            true
        )
    );
    Ok(())
}

#[tokio::test]
async fn caller_rollback_restores_expiry_and_both_audits() -> Result<(), DbErr> {
    let fixture = Fixture::new().await;
    let result = async {
        seed(&fixture, 1).await?;
        let transaction = runtime(&fixture).await?;
        assert!(expire(&transaction, 98001).await?);
        transaction.rollback().await?;
        let row = state(&fixture, 98001).await?;
        assert!(!row.try_get::<bool>("", "expired")?);
        assert_eq!(row.try_get::<i64>("", "versions")?, 0);
        assert_eq!(row.try_get::<i64>("", "audits")?, 0);
        Ok(())
    }
    .await;
    fixture.close().await;
    result
}

#[tokio::test]
async fn simultaneous_expiry_invocations_create_one_dual_audit() -> Result<(), DbErr> {
    let fixture = Fixture::new().await;
    let result = async {
        seed(&fixture, 1).await?;
        let left = async {
            let transaction = runtime(&fixture).await?;
            let changed = expire(&transaction, 98001).await?;
            transaction.commit().await?;
            Ok::<_, DbErr>(changed)
        };
        let right = async {
            let transaction = runtime(&fixture).await?;
            let changed = expire(&transaction, 98001).await?;
            transaction.commit().await?;
            Ok::<_, DbErr>(changed)
        };
        let (left, right) = tokio::time::timeout(std::time::Duration::from_secs(15), async {
            tokio::join!(left, right)
        })
        .await
        .map_err(|_| DbErr::Custom("Synthetic expiry concurrency timeout".into()))?;
        assert_ne!(
            left?, right?,
            "Exactly one invocation must apply the transition"
        );
        let row = state(&fixture, 98001).await?;
        assert!(row.try_get::<bool>("", "expired")?);
        assert_eq!(row.try_get::<i64>("", "versions")?, 1);
        assert_eq!(row.try_get::<i64>("", "audits")?, 1);
        Ok(())
    }
    .await;
    fixture.close().await;
    result
}

#[tokio::test]
async fn expiry_capabilities_have_fixed_nonlogin_owner_and_no_public_execution() -> Result<(), DbErr>
{
    let fixture = Fixture::new().await;
    let result = async {
        let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT count(*) AS count,bool_and(p.prosecdef AND r.rolname='med_tracker_owner' AND NOT r.rolcanlogin AND NOT r.rolsuper AND NOT r.rolbypassrls AND p.proconfig @> ARRAY['search_path=pg_catalog','TimeZone=UTC'] AND has_function_privilege('med_tracker_app',p.oid,'EXECUTE') AND NOT EXISTS(SELECT 1 FROM aclexplode(COALESCE(p.proacl,acldefault('f',p.proowner))) a WHERE a.grantee=0 AND a.privilege_type='EXECUTE')) AS secured FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace JOIN pg_roles r ON r.oid=p.proowner WHERE n.nspname='med_tracker' AND p.proname IN ('due_support_access_sessions','expire_support_access_session')"))
            .await?.ok_or_else(missing)?;
        assert_eq!(row.try_get::<i64>("", "count")?, 2);
        assert_eq!(row.try_get::<Option<bool>>("", "secured")?, Some(true));
        seed(&fixture, 1).await?;
        fixture.admin.execute_unprepared("INSERT INTO public.accounts(id,email,created_at,updated_at) VALUES(71002,'synthetic-worker-foreign@example.test',now(),now()); INSERT INTO public.households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(72002,71002,'Synthetic foreign worker household','synthetic-worker-foreign','UTC',now(),now()); INSERT INTO public.platform_admins(id,account_id,status,created_at,updated_at) VALUES(98102,71002,'revoked',now(),now()); INSERT INTO public.support_access_sessions(id,household_id,platform_admin_id,reason,starts_at,expires_at,created_at,updated_at) VALUES(98002,72002,98102,'Synthetic foreign worker expiry',now()-interval '2 hours',now()-interval '1 hour',now(),now())").await?;
        let transaction = runtime(&fixture).await?;
        transaction.execute_unprepared("SET LOCAL search_path=pg_temp; CREATE TEMP TABLE versions(id bigint); CREATE TEMP TABLE security_audit_events(id bigint); SELECT set_config('med_tracker.current_household_id','999999',true),set_config('med_tracker.current_account_id','999998',true)").await?;
        assert!(expire(&transaction, 98001).await?);
        assert!(expire(&transaction, 98002).await?);
        let context = transaction.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT current_setting('med_tracker.current_household_id') AS household,current_setting('med_tracker.current_account_id') AS account"))
            .await?.ok_or_else(missing)?;
        assert_eq!(context.try_get::<String>("", "household")?, "999999");
        assert_eq!(context.try_get::<String>("", "account")?, "999998");
        transaction.commit().await?;
        let row = state(&fixture, 98001).await?;
        assert_eq!(row.try_get::<i64>("", "audits")?, 1);
        let original = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT a.actor_account_id,a.household_id,a.metadata->>'outcome' AS outcome,(a.metadata->>'expired_at')::timestamptz=date_trunc('second',s.expires_at)::timestamptz AS original_deadline FROM public.security_audit_events a JOIN public.support_access_sessions s ON s.id=(a.metadata->>'support_access_session_id')::bigint WHERE a.event_type='support_access_session.expired' AND s.id=98002"))
            .await?.ok_or_else(missing)?;
        assert_eq!(original.try_get::<i64>("", "actor_account_id")?, 71002);
        assert_eq!(original.try_get::<i64>("", "household_id")?, 72002);
        assert_eq!(original.try_get::<String>("", "outcome")?, "expired");
        assert!(original.try_get::<bool>("", "original_deadline")?);
        Ok(())
    }.await;
    fixture.close().await;
    result
}

#[test]
fn support_expiry_cli_and_minute_schedules_are_registered() -> Result<(), Box<dyn std::error::Error>>
{
    let mut tasks = Tasks::default();
    med_tracker::app::App::register_tasks(&mut tasks);
    assert!(
        tasks
            .names()
            .iter()
            .any(|name| name == "support-access-expire")
    );
    for environment in [
        loco_rs::environment::Environment::Development,
        loco_rs::environment::Environment::Production,
    ] {
        let config = loco_rs::config::Config::new(&environment)?;
        let scheduler = config.scheduler.ok_or_else(missing)?;
        let job = scheduler
            .jobs
            .get("support_access_expiry")
            .ok_or_else(missing)?;
        assert_eq!(job.run, "support-access-expire");
        assert_eq!(job.cron, "0 * * * * *");
    }
    Ok(())
}

#[tokio::test]
async fn registered_worker_drains_backlog_and_retry_is_a_noop()
-> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new().await;
    let result = async {
        seed(&app.fixture, 1001).await?;
        let mut tasks = Tasks::default();
        med_tracker::app::App::register_tasks(&mut tasks);
        tasks.run(&app.context, "support-access-expire", &Vars::default()).await?;
        tasks.run(&app.context, "support-access-expire", &Vars::default()).await?;
        let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM public.support_access_sessions WHERE id BETWEEN 98001 AND 99001 AND expired_at IS NOT NULL) AS expired,(SELECT count(*) FROM public.versions WHERE item_type='SupportAccessSession' AND event='platform/support/expired') AS versions,(SELECT count(*) FROM public.security_audit_events WHERE event_type='support_access_session.expired') AS audits"))
            .await?.ok_or_else(missing)?;
        assert_eq!(row.try_get::<i64>("", "expired")?, 1001);
        assert_eq!(row.try_get::<i64>("", "versions")?, 1001);
        assert_eq!(row.try_get::<i64>("", "audits")?, 1001);
        Ok::<_, Box<dyn std::error::Error>>(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn registered_worker_pages_past_500_failures_and_retries_them()
-> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new().await;
    let result = async {
        seed(&app.fixture, 501).await?;
        app.fixture.admin.execute_unprepared("CREATE FUNCTION public.reject_first_worker_expiry_audits() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type='support_access_session.expired' AND (NEW.metadata->>'support_access_session_id')::bigint<=98500 THEN RAISE EXCEPTION USING ERRCODE='23514',MESSAGE='Synthetic first expiry page failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_first_worker_expiry_audits BEFORE INSERT ON public.security_audit_events FOR EACH ROW EXECUTE FUNCTION public.reject_first_worker_expiry_audits()").await?;
        let mut tasks = Tasks::default();
        med_tracker::app::App::register_tasks(&mut tasks);
        let partial = tasks.run(&app.context, "support-access-expire", &Vars::default()).await;
        let last = state(&app.fixture, 98501).await?;
        let first = state(&app.fixture, 98001).await?;
        assert!(partial.is_err(), "Partial expiry must be visible to the scheduler");
        assert!(last.try_get::<bool>("", "expired")?, "A failed first page must not starve later rows");
        assert!(!first.try_get::<bool>("", "expired")?);
        assert_eq!(first.try_get::<i64>("", "versions")?, 0);
        assert_eq!(first.try_get::<i64>("", "audits")?, 0);
        app.fixture.admin.execute_unprepared("DROP TRIGGER reject_first_worker_expiry_audits ON public.security_audit_events").await?;
        tasks.run(&app.context, "support-access-expire", &Vars::default()).await?;
        let first = state(&app.fixture, 98001).await?;
        assert!(first.try_get::<bool>("", "expired")?);
        assert_eq!(first.try_get::<i64>("", "versions")?, 1);
        assert_eq!(first.try_get::<i64>("", "audits")?, 1);
        Ok::<_, Box<dyn std::error::Error>>(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn real_listing_and_worker_overlap_without_deadlock_or_lost_audits()
-> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new().await;
    let mut listing = None;
    let mut worker = None;
    let result = async {
        let cookie = super::platform::admin_session(&app).await;
        let older = super::platform::active_support(&app, &cookie).await;
        app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE public.support_access_sessions SET expires_at=timezone('UTC',clock_timestamp())-interval '1 second' WHERE id=$1",[older.into()])).await?;
        let newer = app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO public.support_access_sessions(household_id,platform_admin_id,reason,starts_at,expires_at,created_at,updated_at) SELECT household_id,platform_admin_id,'Synthetic newer overlap session',starts_at,expires_at,created_at,updated_at FROM public.support_access_sessions WHERE id=$1 RETURNING id",[older.into()]))
            .await?.ok_or_else(missing)?.try_get::<i64>("", "id")?;
        let before = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT deadlocks FROM pg_stat_database WHERE datname=current_database()"))
            .await?.ok_or_else(missing)?.try_get::<i64>("", "deadlocks")?;
        app.fixture.admin.execute_unprepared(&format!(
            "CREATE SEQUENCE public.synthetic_support_listing_pid; ALTER SEQUENCE public.synthetic_support_listing_pid OWNER TO med_tracker_owner; CREATE FUNCTION public.synthetic_support_listing_gate() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$ BEGIN IF NEW.item_type='SupportAccessSession' AND NEW.event='platform/support/expired' AND NEW.item_id={newer} THEN PERFORM pg_catalog.setval('public.synthetic_support_listing_pid'::regclass,pg_catalog.pg_backend_pid(),true); PERFORM pg_catalog.pg_advisory_xact_lock(1729712041,1); END IF; RETURN NEW; END $$; ALTER FUNCTION public.synthetic_support_listing_gate() OWNER TO med_tracker_owner; REVOKE ALL ON FUNCTION public.synthetic_support_listing_gate() FROM PUBLIC; CREATE TRIGGER zz_synthetic_support_listing_gate AFTER INSERT ON public.versions FOR EACH ROW EXECUTE FUNCTION public.synthetic_support_listing_gate()"
        )).await?;
        let gate = app.fixture.admin.begin().await?;
        gate.execute_unprepared("SELECT pg_advisory_xact_lock(1729712041,1)").await?;
        let client = app.client.clone();
        let origin = app.origin.clone();
        listing = Some(tokio::spawn(async move {
            client.get(format!("{origin}/platform/support")).header("cookie",cookie)
                .timeout(std::time::Duration::from_secs(30)).send().await.map(|response|response.status())
        }));
        let mut listing_pid = None;
        for _ in 0..100 {
            let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
                "SELECT last_value,is_called FROM public.synthetic_support_listing_pid"))
                .await?.ok_or_else(missing)?;
            if row.try_get::<bool>("", "is_called")? {
                listing_pid = Some(i32::try_from(row.try_get::<i64>("", "last_value")?)?);
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        let listing_pid = listing_pid.ok_or_else(||std::io::Error::other("Real listing did not reach its post-ledger gate"))?;
        let context = app.context.clone();
        worker = Some(tokio::spawn(async move {
            let mut tasks = Tasks::default();
            med_tracker::app::App::register_tasks(&mut tasks);
            tasks.run(&context,"support-access-expire",&Vars::default()).await
        }));
        let mut blocked = false;
        for _ in 0..100 {
            blocked = app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))) AS blocked",[listing_pid.into()]))
                .await?.ok_or_else(missing)?.try_get("", "blocked")?;
            if blocked { break; }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        if !blocked { return Err(std::io::Error::other("Registered worker did not wait on the real listing ledger lock").into()); }
        gate.commit().await?;
        let listing_handle = listing.as_mut().ok_or_else(missing)?;
        let worker_handle = worker.as_mut().ok_or_else(missing)?;
        let (listing_result,worker_result) = tokio::time::timeout(std::time::Duration::from_secs(20), async {
            tokio::join!(listing_handle,worker_handle)
        }).await.map_err(|_|std::io::Error::other("Support listing/worker overlap timed out"))?;
        let listing_status = listing_result??;
        let worker_succeeded = worker_result?.is_ok();
        let mut after = before;
        for _ in 0..20 {
            after = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
                "SELECT deadlocks FROM pg_stat_database WHERE datname=current_database()"))
                .await?.ok_or_else(missing)?.try_get("", "deadlocks")?;
            if after>before { break; }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        let counts = app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM public.versions WHERE item_type='SupportAccessSession' AND event='platform/support/expired' AND item_id IN ($1,$2)) AS versions,(SELECT count(*) FROM public.security_audit_events WHERE event_type='support_access_session.expired' AND metadata->>'support_access_session_id' IN ($1::text,$2::text)) AS security",[older.into(),newer.into()]))
            .await?.ok_or_else(missing)?;
        Ok::<_,Box<dyn std::error::Error>>((listing_status.as_u16(),worker_succeeded,after-before,
            counts.try_get::<i64>("", "versions")?,counts.try_get::<i64>("", "security")?))
    }.await;
    if let Some(handle) = listing
        && !handle.is_finished()
    {
        handle.abort();
        let _ = handle.await;
    }
    if let Some(handle) = worker
        && !handle.is_finished()
    {
        handle.abort();
        let _ = handle.await;
    }
    app.close().await;
    assert_eq!(result?, (200, true, 0, 2, 2));
    Ok(())
}

#[tokio::test]
async fn busy_expiry_is_false_without_audits_and_succeeds_after_unlock() -> Result<(), DbErr> {
    let fixture = Fixture::new().await;
    let result = async {
        seed(&fixture, 1).await?;
        let lock = fixture.admin.begin().await?;
        lock.execute_unprepared(
            "SELECT id FROM public.support_access_sessions WHERE id=98001 FOR UPDATE",
        )
        .await?;
        let transaction = runtime(&fixture).await?;
        let busy = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            expire(&transaction, 98001),
        )
        .await
        .map_err(|_| DbErr::Custom("Busy expiry did not return without waiting".into()))??;
        transaction.commit().await?;
        let before = state(&fixture, 98001).await?;
        let before = (
            before.try_get::<bool>("", "expired")?,
            before.try_get::<i64>("", "versions")?,
            before.try_get::<i64>("", "audits")?,
        );
        lock.commit().await?;
        let transaction = runtime(&fixture).await?;
        let retried = expire(&transaction, 98001).await?;
        transaction.commit().await?;
        let after = state(&fixture, 98001).await?;
        let after = (
            after.try_get::<bool>("", "expired")?,
            after.try_get::<i64>("", "versions")?,
            after.try_get::<i64>("", "audits")?,
        );
        Ok::<_, DbErr>((busy, before, retried, after))
    }
    .await;
    fixture.close().await;
    assert_eq!(result?, (false, (false, 0, 0), true, (true, 1, 1)));
    Ok(())
}
