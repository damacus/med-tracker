use super::{Fixture, assert_database_code};
use migration::sea_orm::{ConnectionTrait, Database, DbBackend, DbErr, Statement};

const SOURCE: &str = "public.account_clinical_access_events";
const SUCCESS: &str = "med_tracker.append_account_clinical_access(text,text,text,uuid)";
const DENIAL: &str =
    "med_tracker.append_denied_account_clinical_access(bigint,bigint,text,text,text,uuid,integer)";
const TRACE: &str = "0e79ca11-5f34-4fb1-bb33-970010000001";

#[tokio::test]
async fn self_history_reader_accepts_a_bounded_empty_account_page() -> Result<(), DbErr> {
    let fixture = seeded().await?;
    let (runtime, role) = fixture.runtime().await;
    let result = async {
        runtime
            .execute_unprepared("SET med_tracker.current_account_id='97011'")
            .await?;
        let rows = runtime
            .query_all_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT * FROM med_tracker.self_medication_history($1,$2,$3,$4,$5)",
                [
                    Option::<sea_orm::prelude::DateTimeUtc>::None.into(),
                    Option::<i64>::None.into(),
                    Option::<sea_orm::prelude::DateTimeUtc>::None.into(),
                    Option::<sea_orm::prelude::DateTimeUtc>::None.into(),
                    51_i32.into(),
                ],
            ))
            .await
            .map_err(|error| {
                let (DbErr::Exec(sea_orm::RuntimeErr::SqlxError(error))
                | DbErr::Query(sea_orm::RuntimeErr::SqlxError(error))) = error
                else {
                    return DbErr::Custom("Reader failed without a database category".into());
                };
                let code = error
                    .as_database_error()
                    .and_then(|error| error.code())
                    .map_or_else(|| "unknown".to_owned(), |code| code.into_owned());
                DbErr::Custom(format!("Reader SQLSTATE: {code}"))
            })?;
        Ok::<_, DbErr>(rows.len())
    }
    .await;
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
    assert_eq!(result?, 0);
    Ok(())
}

async fn seeded() -> Result<Fixture, DbErr> {
    let fixture = Fixture::new().await;
    let seeded = fixture.database.execute_unprepared(
        "INSERT INTO public.people(id,clinical_name,created_at,updated_at) VALUES
           (97001,'PRIVATE_CLINICAL_SENTINEL',now(),now()),
           (97002,'Other synthetic person',now(),now()),
           (97003,'Bound session person',now(),now());
         INSERT INTO public.accounts(id,person_id,name,email,status,created_at,updated_at) VALUES
           (97011,97001,'Synthetic account','PRIVATE_CREDENTIAL_SENTINEL@example.test',2,now(),now()),
           (97012,97002,'Other account','other-audit@example.test',2,now(),now()),
           (97013,97003,'Bound account','bound-audit@example.test',2,now(),now());
         INSERT INTO public.households(id,name,slug,timezone,created_at,updated_at) VALUES
           (97021,'Other home','other-audit-home','UTC',now(),now()),
           (97022,'Bound home','bound-audit-home','UTC',now(),now());
         INSERT INTO public.household_memberships(id,person_id,household_id,role,status,joined_at,created_at,updated_at) VALUES
           (97031,97003,97022,'member','active',now(),now(),now()),
           (97032,97002,97021,'member','active',now(),now(),now());
         INSERT INTO public.oauth_applications(id,client_id,client_kind,name,redirect_uri,scopes,token_endpoint_auth_method,created_at,updated_at) VALUES
           (97061,'synthetic-history-client','mobile','Synthetic client','io.damacus.medtracker:/oauth2redirect','medtracker offline_access','none',now(),now());
         INSERT INTO public.oauth_grants(id,account_id,oauth_application_id,client_kind,redirect_uri,scopes,token_hash,expires_in,authenticated_at,last_used_at,created_at,updated_at) VALUES
           (97051,97011,97061,'mobile','io.damacus.medtracker:/oauth2redirect','medtracker offline_access',translate(encode(digest('synthetic-zero-household-bearer','sha256'),'base64'),'+/','-_'),now()+interval '1 hour',now(),now(),now(),now()),
           (97052,97012,97061,'mobile','io.damacus.medtracker:/oauth2redirect','medtracker offline_access',translate(encode(digest('synthetic-foreign-bearer','sha256'),'base64'),'+/','-_'),now()+interval '1 hour',now(),now(),now(),now());
         INSERT INTO public.api_sessions(id,account_id,household_membership_id,permissions_version,access_token_digest,refresh_token_digest,access_expires_at,refresh_expires_at,last_used_at,created_at,updated_at) VALUES
           (97041,97013,97031,1,'synthetic-access-audit','synthetic-refresh-audit',now()+interval '1 hour',now()+interval '1 day',now(),now(),now()),
           (97042,97012,97032,1,'other-access-audit','other-refresh-audit',now()+interval '1 hour',now()+interval '1 day',now(),now(),now())"
    ).await;
    if let Err(error) = seeded {
        fixture.close().await;
        return Err(error);
    }
    Ok(fixture)
}

fn success(action: &str, method: &str, reference: &str) -> Statement {
    Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT med_tracker.append_account_clinical_access($1,$2,$3,$4::uuid)",
        [action.into(), method.into(), reference.into(), TRACE.into()],
    )
}

fn denial(account: i64, person: i64, action: &str, reference: &str, status: i32) -> Statement {
    Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT med_tracker.append_denied_account_clinical_access($1,$2,$3,'oauth',$4,$5::uuid,$6)",
        [
            account.into(),
            person.into(),
            action.into(),
            reference.into(),
            TRACE.into(),
            status.into(),
        ],
    )
}

#[tokio::test]
async fn account_clinical_access_source_exists_for_zero_household_history() -> Result<(), DbErr> {
    let fixture = seeded().await?;
    let result = fixture
        .database
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM public.account_clinical_access_events",
        ))
        .await;
    fixture.close().await;
    let row = result?.ok_or_else(|| DbErr::Custom("missing source count".into()))?;
    assert_eq!(row.try_get::<i64>("", "count")?, 0);
    Ok(())
}

#[tokio::test]
async fn successful_self_history_read_has_one_minimal_global_source_ledger_and_delivery()
-> Result<(), DbErr> {
    let fixture = seeded().await?;
    let (runtime, role) = fixture.runtime().await;
    let result = async {
        runtime.execute_unprepared("SET med_tracker.current_account_id='97011'; SET med_tracker.current_person_id=''; SET med_tracker.current_household_id=''; SET med_tracker.current_membership_id=''").await?;
        runtime.execute_raw(success("self_medication_history.read", "oauth", "97051")).await?;
        let row = fixture.database.query_one_raw(Statement::from_string(DbBackend::Postgres,
            r#"SELECT count(*) AS n, count(*) FILTER (WHERE e.account_id=97011 AND e.person_id=97001 AND e.event_type='self_medication_history.read' AND e.metadata='{"outcome":"success","status":200}'::jsonb AND e.audit_context='{"actor_account_id":97011,"actor_person_id":97001,"authentication_method":"oauth","session_reference":"oauth_grant:97051","request_id":"0e79ca11-5f34-4fb1-bb33-970010000001"}'::jsonb AND l.household_id IS NULL AND l.chain_key='global' AND l.source_payload=to_jsonb(e) AND l.envelope->'agent'->>'account_id'='97011' AND l.envelope->'request'->>'request_id'='0e79ca11-5f34-4fb1-bb33-970010000001' AND d.status='pending') AS exact, count(*) FILTER (WHERE l.source_payload::text LIKE '%PRIVATE_CLINICAL_SENTINEL%' OR l.envelope::text LIKE '%PRIVATE_CLINICAL_SENTINEL%' OR l.source_payload::text LIKE '%PRIVATE_CREDENTIAL_SENTINEL%' OR l.envelope::text LIKE '%PRIVATE_CREDENTIAL_SENTINEL%') AS leaked FROM public.account_clinical_access_events e LEFT JOIN public.audit_ledger_entries l ON l.source_table='account_clinical_access_events' AND l.source_id=e.id LEFT JOIN public.audit_export_deliveries d ON d.audit_ledger_entry_id=l.id"#
        )).await?.ok_or_else(||DbErr::Custom("missing ledger aggregate".into()))?;
        Ok::<_,DbErr>((row.try_get::<i64>("","n")?,row.try_get::<i64>("","exact")?,row.try_get::<i64>("","leaked")?))
    }.await;
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
    assert_eq!(result?, (1, 1, 0));
    Ok(())
}

#[tokio::test]
async fn household_bound_api_session_cannot_append_self_history_success() -> Result<(), DbErr> {
    let fixture = seeded().await?;
    let (runtime, role) = fixture.runtime().await;
    let result=async {
        runtime.execute_unprepared("SET med_tracker.current_account_id='97013'; SET med_tracker.current_person_id=''; SET med_tracker.current_household_id=''; SET med_tracker.current_membership_id=''").await?;
        match runtime.execute_raw(success("self_medication_history.read","api_session","97041")).await {
            Err(error)=>assert_database_code(error,"42501"),
            Ok(_)=>return Err(DbErr::Custom("household-bound API session gained self-history audit authority".into())),
        }
        let row=fixture.database.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT count(*) AS n FROM public.account_clinical_access_events"
        )).await?.ok_or_else(||DbErr::Custom("missing bound-session aggregate".into()))?;
        row.try_get::<i64>("","n")
    }.await;
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
    assert_eq!(result?, 0);
    Ok(())
}

#[tokio::test]
async fn success_appender_rejects_wrong_credential_action_and_untrusted_trace() -> Result<(), DbErr>
{
    let fixture = seeded().await?;
    let (runtime, role) = fixture.runtime().await;
    let result=async {
        runtime.execute_unprepared("SET med_tracker.current_account_id='97011'; SET med_tracker.current_person_id=''; SET med_tracker.current_household_id=''").await?;
        for invalid in [
            success("self_medication_history.read","oauth","97052"),
            success("self_medication_history.read","oauth","missing"),
            success("self_medication_history.other","oauth","97051"),
            success("self_medication_history.read","api_session","97051"),
        ] {
            match runtime.execute_raw(invalid).await {
                Err(error)=>assert_database_code(error,"42501"),
                Ok(_)=>return Err(DbErr::Custom("invalid account clinical audit accepted".into())),
            }
        }
        let untrusted_trace=runtime.execute_unprepared("SELECT med_tracker.append_account_clinical_access('self_medication_history.read','oauth','97051','PRIVATE_CLINICAL_SENTINEL/unsafe'::uuid)").await;
        match untrusted_trace {
            Err(error)=>assert_database_code(error,"22P02"),
            Ok(_)=>return Err(DbErr::Custom("non-UUID trace accepted".into())),
        }
        let count=fixture.database.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT count(*) AS n FROM public.account_clinical_access_events"
        )).await?.ok_or_else(||DbErr::Custom("missing invalid-case aggregate".into()))?.try_get::<i64>("","n")?;
        Ok::<_,DbErr>(count)
    }.await;
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
    assert_eq!(result?, 0);
    Ok(())
}

#[tokio::test]
async fn revoked_credential_denial_retains_historical_ownership_without_live_authority()
-> Result<(), DbErr> {
    let fixture = seeded().await?;
    let (runtime, role) = fixture.runtime().await;
    let result = async {
        fixture.database.execute_unprepared("UPDATE public.oauth_grants SET revoked_at=now() WHERE id=97051; UPDATE public.accounts SET disabled_at=now() WHERE id=97011").await?;
        runtime.execute_raw(denial(97011,97001,"self_medication_history.download","97051",401)).await?;
        for invalid in [
            denial(97011,97002,"self_medication_history.download","97051",401),
            denial(97011,97001,"self_medication_history.download","97052",401),
            denial(97011,97001,"self_medication_history.download","missing",401),
            denial(97011,97001,"self_medication_history.download","97051",200),
        ] {
            match runtime.execute_raw(invalid).await {
                Err(error) => assert_database_code(error, "42501"),
                Ok(_) => return Err(DbErr::Custom("invalid historical attribution accepted".into())),
            }
        }
        let row=fixture.database.query_one_raw(Statement::from_string(DbBackend::Postgres,
            r#"SELECT count(*) AS n,count(*) FILTER(WHERE e.event_type='self_medication_history.download' AND e.metadata='{"outcome":"denied","status":401}'::jsonb AND e.audit_context->>'actor_account_id'='97011' AND e.audit_context->>'actor_person_id'='97001' AND e.audit_context->>'session_reference'='oauth_grant:97051' AND l.household_id IS NULL AND l.chain_key='global' AND l.source_payload=to_jsonb(e) AND d.status='pending') AS exact FROM public.account_clinical_access_events e LEFT JOIN public.audit_ledger_entries l ON l.source_table='account_clinical_access_events' AND l.source_id=e.id LEFT JOIN public.audit_export_deliveries d ON d.audit_ledger_entry_id=l.id"#
        )).await?.ok_or_else(||DbErr::Custom("missing denial aggregate".into()))?;
        Ok::<_,DbErr>((row.try_get::<i64>("","n")?,row.try_get::<i64>("","exact")?))
    }.await;
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
    assert_eq!(result?, (1, 1));
    Ok(())
}

#[tokio::test]
async fn app_has_void_appenders_but_no_source_or_generic_ledger_capability() -> Result<(), DbErr> {
    let fixture = seeded().await?;
    let (runtime, role) = fixture.runtime().await;
    let result=async {
        let acl=fixture.database.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT has_function_privilege('med_tracker_app',$1::regprocedure,'EXECUTE') AS success_execute,has_function_privilege('med_tracker_app',$2::regprocedure,'EXECUTE') AS denial_execute,has_table_privilege('med_tracker_audit_verifier',$3,'SELECT') AS verifier_select,NOT has_table_privilege('med_tracker_app',$3,'SELECT') AS no_select,NOT has_table_privilege('med_tracker_app',$3,'INSERT') AS no_insert,NOT has_table_privilege('med_tracker_app',$3,'UPDATE') AS no_update,NOT has_table_privilege('med_tracker_app',$3,'DELETE') AS no_delete,NOT has_sequence_privilege('med_tracker_app',pg_get_serial_sequence($3,'id'),'USAGE') AS no_sequence,NOT has_function_privilege('med_tracker_app','public.audit_append_ledger_entry(text,bigint,bigint,jsonb,timestamp with time zone)','EXECUTE') AS no_generic,NOT has_function_privilege('med_tracker_app','public.audit_capture_account_clinical_access()','EXECUTE') AS no_trigger",
            [SUCCESS.into(),DENIAL.into(),SOURCE.into()]
        )).await?.ok_or_else(||DbErr::Custom("missing ACL snapshot".into()))?;
        for key in ["success_execute","denial_execute","verifier_select","no_select","no_insert","no_update","no_delete","no_sequence","no_generic","no_trigger"] {
            if !acl.try_get::<bool>("",key)? { return Err(DbErr::Custom(format!("unexpected audit ACL: {key}"))); }
        }
        for signature in [SUCCESS,DENIAL] {
            let row=fixture.database.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
                "SELECT p.prorettype='void'::regtype AND p.prosecdef AND p.proowner='med_tracker_owner'::regrole AND p.proconfig=ARRAY['search_path=pg_catalog']::text[] AND NOT EXISTS(SELECT 1 FROM aclexplode(p.proacl) acl WHERE acl.grantee=0 AND acl.privilege_type='EXECUTE') AS protected FROM pg_proc p WHERE p.oid=$1::regprocedure",
                [signature.into()]
            )).await?.ok_or_else(||DbErr::Custom("missing appender metadata".into()))?;
            if !row.try_get::<bool>("","protected")? {
                return Err(DbErr::Custom("account clinical appender metadata is not protected".into()));
            }
        }
        for sql in [
            "SELECT count(*) FROM public.account_clinical_access_events",
            "INSERT INTO public.account_clinical_access_events DEFAULT VALUES",
            "UPDATE public.account_clinical_access_events SET event_type='x'",
            "DELETE FROM public.account_clinical_access_events",
            "SELECT public.audit_append_ledger_entry('account_clinical_access_events',1,NULL,'{}'::jsonb,now())",
        ] {
            match runtime.execute_unprepared(sql).await {
                Err(error)=>assert_database_code(error,"42501"),
                Ok(_)=>return Err(DbErr::Custom("restricted runtime gained direct audit capability".into())),
            }
        }
        let policies=fixture.database.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT count(*) FILTER(WHERE polname='account_clinical_access_owner_insert' AND polcmd='a' AND polroles=ARRAY['med_tracker_owner'::regrole::oid]) AS owner_insert,count(*) FILTER(WHERE polname='account_clinical_access_verifier_select' AND polcmd='r' AND polroles=ARRAY['med_tracker_audit_verifier'::regrole::oid]) AS verifier_select,count(*) FILTER(WHERE 'med_tracker_app'::regrole::oid=ANY(polroles)) AS app_policy,bool_and(c.relrowsecurity AND c.relforcerowsecurity) AS forced FROM pg_policy p JOIN pg_class c ON c.oid=p.polrelid WHERE p.polrelid='public.account_clinical_access_events'::regclass"
        )).await?.ok_or_else(||DbErr::Custom("missing policy snapshot".into()))?;
        Ok::<_,DbErr>((policies.try_get::<i64>("","owner_insert")?,policies.try_get::<i64>("","verifier_select")?,policies.try_get::<i64>("","app_policy")?,policies.try_get::<bool>("","forced")?))
    }.await;
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
    assert_eq!(result?, (1, 1, 0, true));
    Ok(())
}

#[tokio::test]
async fn verifier_sees_source_and_failed_ledger_append_rolls_source_back() -> Result<(), DbErr> {
    let fixture = seeded().await?;
    let (runtime, role) = fixture.runtime().await;
    let result=async {
        runtime.execute_unprepared("SET med_tracker.current_account_id='97011'").await?;
        runtime.execute_raw(success("self_medication_history.read","oauth","97051")).await?;
        let verifier_role=format!("audit_verifier_{}",uuid::Uuid::new_v4().simple());
        fixture.admin.execute_unprepared(&format!("CREATE ROLE {verifier_role} LOGIN PASSWORD 'test-password' NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS IN ROLE med_tracker_audit_verifier")).await?;
        let mut verifier_url=fixture.url.clone();
        verifier_url.set_username(&verifier_role).map_err(|()|DbErr::Custom("invalid verifier username".into()))?;
        verifier_url.set_password(Some("test-password")).map_err(|()|DbErr::Custom("invalid verifier password".into()))?;
        let verifier=match Database::connect(verifier_url.as_str()).await {
            Ok(connection)=>connection,
            Err(error)=>{
                fixture.admin.execute_unprepared(&format!("DROP ROLE {verifier_role}")).await?;
                return Err(error);
            }
        };
        let observed=async { verifier.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT count(*) AS n FROM public.account_clinical_access_events"
        )).await?.ok_or_else(||DbErr::Custom("missing verifier count".into()))?.try_get::<i64>("","n") }.await;
        let closed=verifier.close().await;
        fixture.admin.execute_unprepared(&format!("DROP ROLE {verifier_role}")).await?;
        closed?;
        let count=observed?;
        fixture.database.execute_unprepared("CREATE FUNCTION public.synthetic_account_audit_ledger_failure() RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog AS $$ BEGIN RAISE EXCEPTION 'synthetic ledger failure' USING ERRCODE='23514'; END $$; CREATE TRIGGER synthetic_account_audit_ledger_failure BEFORE INSERT ON public.audit_ledger_entries FOR EACH ROW EXECUTE FUNCTION public.synthetic_account_audit_ledger_failure()").await?;
        match runtime.execute_raw(success("self_medication_history.download","oauth","97051")).await {
            Err(error)=>assert_database_code(error,"23514"),
            Ok(_)=>return Err(DbErr::Custom("ledger failure accepted".into())),
        }
        let row=fixture.database.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM public.account_clinical_access_events) AS sources,(SELECT count(*) FROM public.audit_ledger_entries WHERE source_table='account_clinical_access_events') AS ledgers,(SELECT count(*) FROM public.audit_export_deliveries d JOIN public.audit_ledger_entries l ON l.id=d.audit_ledger_entry_id WHERE l.source_table='account_clinical_access_events') AS deliveries"
        )).await?.ok_or_else(||DbErr::Custom("missing atomicity snapshot".into()))?;
        Ok::<_,DbErr>((count,row.try_get::<i64>("","sources")?,row.try_get::<i64>("","ledgers")?,row.try_get::<i64>("","deliveries")?))
    }.await;
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
    assert_eq!(result?, (1, 1, 1, 1));
    Ok(())
}
