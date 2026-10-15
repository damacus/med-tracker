use super::{Fixture, assert_database_code};
use migration::sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

const SIGNATURE: &str = "med_tracker.append_denied_report_request(bigint,bigint,bigint,bigint,text,integer,text,text,text,text,integer)";

async fn seeded() -> Result<Fixture, DbErr> {
    let fixture = Fixture::new().await;
    let result = fixture.database.execute_unprepared(
        "INSERT INTO public.people(id,clinical_name,created_at,updated_at) VALUES(96901,'Synthetic reader',now(),now()),(96902,'Other reader',now(),now());
         INSERT INTO public.accounts(id,person_id,name,email,status,created_at,updated_at) VALUES(96911,96901,'Synthetic reader','report-denial@example.test',2,now(),now());
         INSERT INTO public.households(id,name,slug,timezone,created_at,updated_at) VALUES(96921,'Origin','denied-report-origin','UTC',now(),now()),(96922,'Other','denied-report-other','UTC',now(),now());
         INSERT INTO public.household_memberships(id,person_id,household_id,role,status,joined_at,created_at,updated_at) VALUES(96931,96901,96921,'member','active',now(),now(),now());
         INSERT INTO public.api_sessions(id,account_id,household_membership_id,permissions_version,access_token_digest,refresh_token_digest,access_expires_at,refresh_expires_at,last_used_at,created_at,updated_at) VALUES(96941,96911,96931,1,'synthetic-access-digest','synthetic-refresh-digest',now()+interval '1 hour',now()+interval '1 day',now(),now(),now());
         UPDATE public.accounts SET disabled_at=now() WHERE id=96911;
         UPDATE public.household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE id=96931"
    ).await;
    if let Err(error) = result {
        fixture.close().await;
        return Err(error);
    }
    Ok(fixture)
}

fn call(
    person: i64,
    household: i64,
    kind: &str,
    status: i32,
    method: &str,
    reference: &str,
) -> Statement {
    Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT med_tracker.append_denied_report_request(96911,$1,96931,$2,'member',1,$3,$4,'synthetic-denial-request',$5,$6)",
        [
            person.into(),
            household.into(),
            method.into(),
            reference.into(),
            kind.into(),
            status.into(),
        ],
    )
}

#[tokio::test]
async fn denied_report_appender_retains_attribution_without_restoring_authority()
-> Result<(), DbErr> {
    let fixture = seeded().await?;
    let (runtime, role) = fixture.runtime().await;
    let result = async {
        runtime.execute_raw(call(96901,96921,"reviews",401,"api_session","96941")).await?;
        let row = fixture.database.query_one_raw(Statement::from_string(DbBackend::Postgres,
            r#"SELECT count(*) AS events,count(*) FILTER(WHERE actor_account_id=96911 AND actor_membership_id=96931 AND audit_context->>'actor_person_id'='96901' AND audit_context->>'active_role'='member' AND audit_context->>'permissions_version'='1' AND audit_context->>'authentication_method'='api_session' AND audit_context->>'session_reference'='api_session:96941' AND metadata='{"http_method":"GET","controller":"api/v1/medication_review_reports","action":"show","outcome":"failure","status":401}'::jsonb) AS attributed FROM public.security_audit_events WHERE request_id='synthetic-denial-request'"#
        )).await?.ok_or_else(||DbErr::Custom("missing denial aggregate".into()))?;
        let authority=runtime.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT med_tracker.current_household_id() IS NULL AS absent,(SELECT count(*) FROM public.security_audit_events) AS visible")).await?.ok_or_else(||DbErr::Custom("missing authority aggregate".into()))?;
        let ledger=fixture.database.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS entries FROM public.audit_ledger_entries WHERE source_table='security_audit_events' AND envelope->'request'->>'request_id'='synthetic-denial-request'")).await?.ok_or_else(||DbErr::Custom("missing ledger aggregate".into()))?;
        Ok::<_,DbErr>((row.try_get::<i64>("","events")?,row.try_get::<i64>("","attributed")?,authority.try_get::<bool>("","absent")?,authority.try_get::<i64>("","visible")?,ledger.try_get::<i64>("","entries")?))
    }.await;
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
    let (events, attributed, absent, visible, ledger) = result?;
    assert_eq!((events, attributed, ledger), (1, 1, 1));
    assert!(absent);
    assert_eq!(visible, 0);
    Ok(())
}

#[tokio::test]
async fn denied_report_appender_rejects_mismatches_routes_statuses_and_credentials()
-> Result<(), DbErr> {
    let fixture = seeded().await?;
    let (runtime, role) = fixture.runtime().await;
    let result=async {
        let cases=[(96902,96921,"health",401,"api_session","96941"),(96901,96922,"health",403,"api_session","96941"),(96901,96921,"summary",403,"api_session","96941"),(96901,96921,"health",200,"api_session","96941"),(96901,96921,"health",404,"api_session","96941"),(96901,96921,"health",401,"browser_session","96941"),(96901,96921,"health",401,"api_session","96942"),(96901,96921,"health",401,"api_session","secret-token")];
        for (person,household,kind,status,method,reference) in cases {
            let rejected=runtime.execute_raw(call(person,household,kind,status,method,reference)).await;
            match rejected { Err(error)=>assert_database_code(error,"42501"),Ok(_)=>return Err(DbErr::Custom("invalid denial attribution accepted".into())) }
        }
        let null=runtime.execute_unprepared("SELECT med_tracker.append_denied_report_request(NULL,96901,96931,96921,'member',1,'api_session','96941','synthetic-denial-request','health',401)").await;
        match null { Err(error)=>assert_database_code(error,"42501"),Ok(_)=>return Err(DbErr::Custom("NULL attribution accepted".into())) }
        Ok::<_,DbErr>(())
    }.await;
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
    result
}

#[tokio::test]
async fn denied_report_appender_has_exact_fixed_owner_acl_and_atomic_ledger_failure()
-> Result<(), DbErr> {
    let fixture = seeded().await?;
    let (runtime, role) = fixture.runtime().await;
    let result=async {
        let metadata=fixture.database.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT p.prosecdef AND r.rolname='med_tracker_owner' AND p.proconfig=ARRAY['search_path=pg_catalog']::text[] AND has_function_privilege('med_tracker_app',p.oid,'EXECUTE') AND NOT EXISTS(SELECT 1 FROM aclexplode(p.proacl) a WHERE a.grantee=0 AND a.privilege_type='EXECUTE') AS protected FROM pg_proc p JOIN pg_roles r ON r.oid=p.proowner WHERE p.oid=$1::regprocedure",[SIGNATURE.into()])).await?.ok_or_else(||DbErr::Custom("missing appender metadata".into()))?;
        let protected=metadata.try_get::<bool>("","protected")?;
        fixture.database.execute_unprepared("CREATE FUNCTION public.synthetic_deny_report_ledger() RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog AS $$ BEGIN RAISE EXCEPTION 'synthetic audit failure' USING ERRCODE='23514'; END $$; CREATE TRIGGER synthetic_deny_report_ledger BEFORE INSERT ON public.audit_ledger_entries FOR EACH ROW EXECUTE FUNCTION public.synthetic_deny_report_ledger()").await?;
        let rejected=runtime.execute_raw(call(96901,96921,"health",403,"api_session","96941")).await;
        match rejected { Err(error)=>assert_database_code(error,"23514"),Ok(_)=>return Err(DbErr::Custom("audit failure accepted".into())) }
        let count=fixture.database.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS events FROM public.security_audit_events WHERE request_id='synthetic-denial-request'")).await?.ok_or_else(||DbErr::Custom("missing event aggregate".into()))?.try_get::<i64>("","events")?;
        Ok::<_,DbErr>((protected,count))
    }.await;
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
    let (protected, count) = result?;
    assert!(protected);
    assert_eq!(count, 0);
    Ok(())
}
