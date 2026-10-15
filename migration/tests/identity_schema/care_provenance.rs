use super::Fixture;
use migration::sea_orm::TransactionTrait;
use migration::sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn local_dose_facts_use_original_medicine_windows_and_live_scope() -> TestResult {
    let fixture = seeded_fixture().await?;
    let (runtime, role) = fixture.runtime().await;
    let result = async {
        actor_scope(&runtime).await?;
        insert_takes(&runtime).await?;
        runtime.execute_unprepared("UPDATE schedules SET medication_id=95652 WHERE id=95661; UPDATE person_medications SET active=false,retired_at=now() WHERE id=95662").await?;
        let timing = runtime.query_all_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT window_index,cycle_count,latest_at::text AS latest_at FROM med_tracker.local_dose_timing(
             95621,ARRAY[95602,95602,95602]::bigint[],ARRAY[95651,95651,95652]::bigint[],
             ARRAY['2026-10-15 00:00','2026-10-15 10:00','2026-10-15 00:00']::timestamp[],
             ARRAY['2026-10-16 00:00','2026-10-15 11:00','2026-10-16 00:00']::timestamp[],'2026-10-15 10:30') ORDER BY window_index"
        )).await?;
        let timing = timing.into_iter().map(|row| Ok((row.try_get::<i64>("","window_index")?,row.try_get::<i64>("","cycle_count")?,row.try_get::<Option<String>>("","latest_at")?))).collect::<Result<Vec<_>,DbErr>>()?;
        let completed = runtime.query_all_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT medication_id,medication_name,dose_amount::text AS amount FROM med_tracker.local_dashboard_take_history(95621,ARRAY[95602]::bigint[],'2026-10-15 10:00','2026-10-15 11:00')"
        )).await?;
        let completed = completed.into_iter().map(|row| Ok((row.try_get::<i64>("","medication_id")?,row.try_get::<String>("","medication_name")?,row.try_get::<String>("","amount")?))).collect::<Result<Vec<_>,DbErr>>()?;
        let foreign_household = runtime.query_all_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT * FROM med_tracker.local_dose_timing(95622,ARRAY[95602]::bigint[],ARRAY[95651]::bigint[],ARRAY['2026-10-15']::timestamp[],ARRAY['2026-10-16']::timestamp[],'2026-10-15 11:00')"
        )).await;
        let foreign_medicine = runtime.query_all_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT * FROM med_tracker.local_dose_timing(95621,ARRAY[95602]::bigint[],ARRAY[95653]::bigint[],ARRAY['2026-10-15']::timestamp[],ARRAY['2026-10-16']::timestamp[],'2026-10-15 11:00')"
        )).await;
        runtime.execute_unprepared("SET med_tracker.current_membership_id='95634'").await?;
        let wrong_actor = runtime.query_all_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT * FROM med_tracker.local_dashboard_take_history(95621,ARRAY[95602]::bigint[],'2026-10-15','2026-10-16')"
        )).await;
        actor_scope(&runtime).await?;
        fixture.database.execute_unprepared("UPDATE household_memberships SET status='revoked',revoked_at=now() WHERE id=95631").await?;
        let revoked_timing = runtime.query_all_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT * FROM med_tracker.local_dose_timing(95621,ARRAY[95602]::bigint[],ARRAY[95651]::bigint[],ARRAY['2026-10-15']::timestamp[],ARRAY['2026-10-16']::timestamp[],'2026-10-15 11:00')"
        )).await;
        let revoked_dashboard = runtime.query_all_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT * FROM med_tracker.local_dashboard_take_history(95621,ARRAY[95602]::bigint[],'2026-10-15','2026-10-16')"
        )).await;
        Ok::<_,DbErr>((timing,completed,[foreign_household,foreign_medicine,wrong_actor,revoked_timing,revoked_dashboard]))
    }.await;
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
    let (timing, completed, denied) = result?;
    assert_eq!(
        timing,
        vec![
            (1, 2, Some("2026-10-15 10:00:00".into())),
            (2, 1, Some("2026-10-15 10:00:00".into())),
            (3, 0, None)
        ]
    );
    assert_eq!(
        completed,
        vec![(95651, "Original medicine".into(), "2.00".into())]
    );
    for denied in denied {
        assert_eq!(
            database_code(&denied).as_deref(),
            Some("42501"),
            "{denied:?}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn local_dose_fact_readers_have_narrow_execution_authority() -> TestResult {
    let fixture = Fixture::new().await;
    let rows = fixture.database.query_all_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT p.proname,pg_get_userbyid(p.proowner) AS owner,p.prosecdef,
         p.proconfig=ARRAY['search_path=pg_catalog']::text[] AS fixed_path,
         EXISTS(SELECT 1 FROM aclexplode(COALESCE(p.proacl,acldefault('f',p.proowner))) a WHERE a.grantee=0 AND a.privilege_type='EXECUTE') AS public_execute,
         has_function_privilege('med_tracker_app',p.oid,'EXECUTE') AS app_execute
         FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace
         WHERE n.nspname='med_tracker' AND p.proname IN ('local_dose_timing','local_dashboard_take_history')"
    )).await;
    fixture.close().await;
    let rows = rows?;
    assert_eq!(rows.len(), 2);
    for row in rows {
        assert_eq!(row.try_get::<String>("", "owner")?, "med_tracker_owner");
        assert!(row.try_get::<bool>("", "prosecdef")?);
        assert!(row.try_get::<bool>("", "fixed_path")?);
        assert!(row.try_get::<bool>("", "app_execute")?);
        assert!(!row.try_get::<bool>("", "public_execute")?);
    }
    Ok(())
}

#[tokio::test]
async fn original_history_is_private_append_only_and_retains_its_take() -> TestResult {
    let fixture = seeded_fixture().await?;
    let (runtime, role) = fixture.runtime().await;
    let result=async {
        actor_scope(&runtime).await?;
        insert_takes(&runtime).await?;
        let raw=runtime.execute_unprepared("SELECT snapshot FROM clinical_history_entries").await;
        let forged=runtime.execute_unprepared("INSERT INTO clinical_history_entries(take_id,person_id,origin_household_id,occurred_at,snapshot) VALUES(95671,95603,95621,now(),'{}')").await;
        let update=fixture.database.execute_unprepared("UPDATE clinical_history_entries SET snapshot='{}' WHERE take_id=95671").await;
        let delete=fixture.database.execute_unprepared("DELETE FROM clinical_history_entries WHERE take_id=95671").await;
        let purge=fixture.database.execute_unprepared("DELETE FROM medication_takes WHERE id=95671").await;
        let row=fixture.database.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM medication_takes) AS takes,(SELECT count(*) FROM clinical_history_entries) AS history"
        )).await?.ok_or_else(||DbErr::Custom("missing retention counts".to_owned()))?;
        Ok::<_,DbErr>((raw,forged,update,delete,purge,row.try_get::<i64>("","takes")?,row.try_get::<i64>("","history")?))
    }.await;
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
    let (raw, forged, update, delete, purge, takes, history) = result?;
    assert_eq!(database_code(&raw).as_deref(), Some("42501"));
    assert_eq!(database_code(&forged).as_deref(), Some("42501"));
    assert_eq!(database_code(&update).as_deref(), Some("23514"));
    assert_eq!(database_code(&delete).as_deref(), Some("23514"));
    assert_eq!(database_code(&purge).as_deref(), Some("23001"));
    assert_eq!(
        database_constraint(&purge).as_deref(),
        Some("clinical_history_take_origin")
    );
    assert_eq!((takes, history), (2, 2));
    Ok(())
}

#[tokio::test]
async fn history_capabilities_have_exact_signatures_and_minimal_execution_grants() -> TestResult {
    let fixture = Fixture::new().await;
    let rows=fixture.database.query_all_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT p.proname,pg_get_userbyid(p.proowner) AS owner,p.prosecdef,
         p.proconfig=ARRAY['search_path=pg_catalog']::text[] AS fixed_path,
         EXISTS(SELECT 1 FROM aclexplode(COALESCE(p.proacl,acldefault('f',p.proowner))) a WHERE a.grantee=0 AND a.privilege_type='EXECUTE') AS public_execute,
         has_function_privilege('med_tracker_app',p.oid,'EXECUTE') AS app_execute
         FROM pg_proc p WHERE p.oid IN (
          'med_tracker.append_take_history(bigint,bigint,text,text)'::regprocedure,
          'med_tracker.require_original_take_history()'::regprocedure,
          'med_tracker.local_take_history(bigint[])'::regprocedure,
          'med_tracker.local_take_report(bigint,timestamp,timestamp,bigint,integer)'::regprocedure,
          'med_tracker.local_historical_subject(text)'::regprocedure,
          'med_tracker.guard_take_history()'::regprocedure)"
    )).await;
    fixture.close().await;
    let rows = rows?;
    assert_eq!(rows.len(), 6);
    for row in rows {
        let name: String = row.try_get("", "proname")?;
        assert_eq!(row.try_get::<String>("", "owner")?, "med_tracker_owner");
        assert_eq!(
            row.try_get::<bool>("", "prosecdef")?,
            name != "guard_take_history",
            "{name}"
        );
        assert!(row.try_get::<bool>("", "fixed_path")?, "{name}");
        assert!(!row.try_get::<bool>("", "public_execute")?, "{name}");
        assert_eq!(
            row.try_get::<bool>("", "app_execute")?,
            !matches!(
                name.as_str(),
                "require_original_take_history" | "guard_take_history"
            ),
            "{name}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn new_take_without_original_history_cannot_commit() -> TestResult {
    let fixture = seeded_fixture().await?;
    let (runtime, role) = fixture.runtime().await;
    let result = async {
        actor_scope(&runtime).await?;
        let transaction = runtime.begin().await?;
        transaction.execute_unprepared(
            "INSERT INTO public.medication_takes(id,household_id,schedule_id,taken_from_medication_id,taken_from_location_id,dose_amount,dose_unit,taken_at,created_at,updated_at)
             VALUES(95671,95621,95661,95651,95641,2,'tablet','2026-10-15 10:00:00',now(),now())"
        ).await?;
        Ok::<_, DbErr>(transaction.commit().await)
    }.await;
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
    let commit = result?;
    assert_eq!(
        database_code(&commit).as_deref(),
        Some("23514"),
        "{commit:?}"
    );
    Ok(())
}

async fn seeded_fixture() -> Result<Fixture, DbErr> {
    let fixture = Fixture::new().await;
    let setup = fixture.database.execute_unprepared(
        "INSERT INTO public.people(id,clinical_name,created_at,updated_at) VALUES
          (95601,'Original actor',now(),now()),
          (95602,'Original subject',now(),now()),
          (95603,'Different subject',now(),now());
         INSERT INTO public.accounts(id,person_id,name,email,status,created_at,updated_at) VALUES
          (95611,95601,'Original actor','provenance-actor@example.test',2,now(),now()),
          (95612,95602,'Original subject','provenance-subject@example.test',2,now(),now());
         INSERT INTO public.households(id,name,slug,timezone,created_at,updated_at) VALUES
          (95621,'Origin home','provenance-origin','UTC',now(),now()),
          (95622,'Other home','provenance-other','UTC',now(),now());
         INSERT INTO public.household_memberships(id,person_id,household_id,role,status,joined_at,created_at,updated_at) VALUES
          (95631,95601,95621,'member','active',now(),now(),now()),
          (95632,95602,95621,'member','active',now(),now(),now()),
          (95633,95603,95621,'member','active',now(),now(),now()),
          (95634,95601,95622,'member','active',now(),now(),now());
         INSERT INTO public.locations(id,household_id,name,created_at,updated_at) VALUES
          (95641,95621,'Origin label',now(),now()),
          (95642,95622,'Other label',now(),now()),
          (95643,95621,'Another origin label',now(),now());
         INSERT INTO public.medications(id,household_id,location_id,name,current_supply,created_at,updated_at) VALUES
          (95651,95621,95641,'Original medicine',20,now(),now()),
          (95652,95621,95641,'Replacement medicine',20,now(),now()),
          (95653,95622,95642,'Foreign medicine',20,now(),now()),
          (95654,95621,95641,'Another local medicine',20,now(),now());
         INSERT INTO public.schedules(id,household_id,person_id,medication_id,dose_amount,dose_unit,created_at,updated_at) VALUES
          (95661,95621,95602,95651,2,'tablet',now(),now()),
          (95663,95621,95603,95651,2,'tablet',now(),now()),
          (95665,95621,95602,95652,2,'tablet',now(),now());
         INSERT INTO public.person_medications(id,household_id,person_id,medication_id,dose_amount,dose_unit,position,created_at,updated_at) VALUES
          (95662,95621,95602,95651,2,'tablet',0,now(),now()),
          (95664,95621,95603,95651,2,'tablet',0,now(),now()),
          (95666,95621,95602,95654,2,'tablet',1,now(),now())"
    ).await;
    if let Err(error) = setup {
        fixture.close().await;
        return Err(error);
    }
    Ok(fixture)
}

async fn actor_scope(runtime: &DatabaseConnection) -> Result<(), DbErr> {
    runtime
        .execute_unprepared(
            "SET med_tracker.current_account_id='95611';
         SET med_tracker.current_household_id='95621';
         SET med_tracker.current_membership_id='95631'",
        )
        .await?;
    Ok(())
}

async fn insert_takes(runtime: &DatabaseConnection) -> Result<(), DbErr> {
    runtime.execute_unprepared(
        "INSERT INTO public.medication_takes(id,client_uuid,household_id,schedule_id,taken_from_medication_id,taken_from_location_id,dose_amount,dose_unit,taken_at,created_at,updated_at)
         VALUES(95671,'95600000-0000-4000-8000-000000000071',95621,95661,95651,95641,2,'tablet','2026-10-15 10:00:00',now(),now());
         INSERT INTO public.medication_takes(id,client_uuid,household_id,person_medication_id,taken_from_medication_id,taken_from_location_id,dose_amount,dose_unit,taken_at,created_at,updated_at)
         VALUES(95672,'95600000-0000-4000-8000-000000000072',95621,95662,95651,95641,2,'tablet','2026-10-15 11:00:00',now(),now());
         SELECT med_tracker.append_take_history(95671,95651);
         SELECT med_tracker.append_take_history(95672,95651)"
    ).await?;
    Ok(())
}

fn database_code<T>(result: &Result<T, DbErr>) -> Option<String> {
    let Err(
        DbErr::Exec(migration::sea_orm::RuntimeErr::SqlxError(error))
        | DbErr::Query(migration::sea_orm::RuntimeErr::SqlxError(error)),
    ) = result
    else {
        return None;
    };
    error
        .as_database_error()?
        .code()
        .map(|code| code.into_owned())
}

fn database_constraint<T>(result: &Result<T, DbErr>) -> Option<String> {
    let Err(
        DbErr::Exec(migration::sea_orm::RuntimeErr::SqlxError(error))
        | DbErr::Query(migration::sea_orm::RuntimeErr::SqlxError(error)),
    ) = result
    else {
        return None;
    };
    error.as_database_error()?.constraint().map(str::to_owned)
}

#[tokio::test]
async fn recorded_sources_cannot_reassign_person_identity() -> TestResult {
    let fixture = seeded_fixture().await?;
    let (runtime, role) = fixture.runtime().await;
    let results = async {
        actor_scope(&runtime).await?;
        insert_takes(&runtime).await?;
        change_live_identity_and_sources(&runtime).await?;
        let schedule = runtime
            .execute_unprepared("UPDATE public.schedules SET person_id=95603 WHERE id=95661")
            .await;
        let assignment = runtime
            .execute_unprepared(
                "UPDATE public.person_medications SET person_id=95603 WHERE id=95662",
            )
            .await;
        Ok::<_, DbErr>((schedule, assignment))
    }
    .await;
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
    let (schedule, assignment) = results?;
    assert_eq!(
        (
            database_code(&schedule).as_deref(),
            database_code(&assignment).as_deref()
        ),
        (Some("23514"), Some("23514")),
        "schedule: {schedule:?}; assignment: {assignment:?}"
    );
    Ok(())
}

#[tokio::test]
async fn existing_tenant_protection_blocks_foreign_household_origin_moves() -> TestResult {
    let fixture = seeded_fixture().await?;
    let (runtime, role) = fixture.runtime().await;
    let results = async {
        actor_scope(&runtime).await?;
        insert_takes(&runtime).await?;
        let schedule = runtime.execute_unprepared(
            "UPDATE public.schedules SET household_id=95622,medication_id=95653 WHERE id=95661"
        ).await;
        let assignment = runtime.execute_unprepared(
            "UPDATE public.person_medications SET household_id=95622,medication_id=95653 WHERE id=95662"
        ).await;
        let take = runtime.execute_unprepared(
            "UPDATE public.medication_takes SET household_id=95622 WHERE id=95671"
        ).await;
        Ok::<_, DbErr>([schedule, assignment, take])
    }.await;
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
    for result in results? {
        assert!(
            matches!(database_code(&result).as_deref(), Some("23514" | "42501")),
            "Existing foreign-tenant RLS (42501) or immutable guard (23514) must deny the move: {result:?}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn recorded_takes_cannot_change_source_to_another_person() -> TestResult {
    let fixture = seeded_fixture().await?;
    let (runtime, role) = fixture.runtime().await;
    let results = async {
        actor_scope(&runtime).await?;
        insert_takes(&runtime).await?;
        let schedule = runtime
            .execute_unprepared(
                "UPDATE public.medication_takes SET schedule_id=95663 WHERE id=95671",
            )
            .await;
        let assignment = runtime
            .execute_unprepared(
                "UPDATE public.medication_takes SET person_medication_id=95664 WHERE id=95672",
            )
            .await;
        Ok::<_, DbErr>((schedule, assignment))
    }
    .await;
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
    let (schedule, assignment) = results?;
    assert_eq!(
        (
            database_code(&schedule).as_deref(),
            database_code(&assignment).as_deref()
        ),
        (Some("23514"), Some("23514")),
        "schedule take: {schedule:?}; assignment take: {assignment:?}"
    );
    Ok(())
}

async fn change_live_identity_and_sources(runtime: &DatabaseConnection) -> Result<(), DbErr> {
    runtime
        .execute_unprepared(
            "UPDATE public.accounts SET name='Later actor profile' WHERE id=95611;
         SET med_tracker.current_account_id='95612';
         SET med_tracker.current_membership_id='95632';
         UPDATE public.accounts SET name='Later subject profile' WHERE id=95612;
         UPDATE public.medications SET name='Later inventory label' WHERE id=95651;
         UPDATE public.schedules SET medication_id=95652 WHERE id=95661;
         UPDATE public.person_medications SET medication_id=95652 WHERE id=95662",
        )
        .await?;
    actor_scope(runtime).await
}

#[tokio::test]
async fn recorded_takes_cannot_rebind_sources_even_for_the_same_person() -> TestResult {
    let fixture = seeded_fixture().await?;
    let (runtime, role) = fixture.runtime().await;
    let results = async {
        actor_scope(&runtime).await?;
        insert_takes(&runtime).await?;
        let mut results = Vec::new();
        for sql in [
            "UPDATE public.medication_takes SET schedule_id=95665 WHERE id=95671",
            "UPDATE public.medication_takes SET person_medication_id=95666 WHERE id=95672",
            "UPDATE public.medication_takes SET schedule_id=NULL,person_medication_id=95662 WHERE id=95671",
        ] {
            results.push((sql, runtime.execute_unprepared(sql).await));
        }
        Ok::<_, DbErr>(results)
    }.await;
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
    assert_immutable_updates(results?);
    Ok(())
}

fn assert_immutable_updates(results: Vec<(&str, Result<migration::sea_orm::ExecResult, DbErr>)>) {
    let failures = results
        .into_iter()
        .filter(|(_, result)| database_code(result).as_deref() != Some("23514"))
        .collect::<Vec<_>>();
    assert!(
        failures.is_empty(),
        "Immutable updates were not rejected: {failures:?}"
    );
}

#[tokio::test]
async fn recorded_administration_fields_are_immutable_including_null_transitions() -> TestResult {
    let fixture = seeded_fixture().await?;
    let (runtime, role) = fixture.runtime().await;
    let results = async {
        actor_scope(&runtime).await?;
        insert_takes(&runtime).await?;
        let timestamp_only = runtime.execute_unprepared(
            "UPDATE public.medication_takes SET updated_at=updated_at+interval '1 second' WHERE id=95671"
        ).await?;
        let mut results = Vec::new();
        for sql in immutable_administration_updates() {
            results.push((sql, runtime.execute_unprepared(sql).await));
        }
        Ok::<_, DbErr>((timestamp_only.rows_affected(), results))
    }.await;
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
    let (timestamp_rows, updates) = results?;
    assert_eq!(timestamp_rows, 1, "updated_at alone remains writable");
    assert_immutable_updates(updates);
    Ok(())
}

fn immutable_administration_updates() -> [&'static str; 15] {
    [
        "UPDATE public.medication_takes SET dose_amount=3 WHERE id=95671",
        "UPDATE public.medication_takes SET dose_amount=NULL WHERE id=95671",
        "UPDATE public.medication_takes SET dose_unit='mg' WHERE id=95671",
        "UPDATE public.medication_takes SET dose_unit=NULL WHERE id=95671",
        "UPDATE public.medication_takes SET taken_at='2026-10-15 12:00:00' WHERE id=95671",
        "UPDATE public.medication_takes SET taken_at=NULL WHERE id=95671",
        "UPDATE public.medication_takes SET taken_from_medication_id=95652 WHERE id=95671",
        "UPDATE public.medication_takes SET taken_from_medication_id=NULL WHERE id=95671",
        "UPDATE public.medication_takes SET taken_from_location_id=95643 WHERE id=95671",
        "UPDATE public.medication_takes SET taken_from_location_id=NULL WHERE id=95671",
        "UPDATE public.medication_takes SET client_uuid='95600000-0000-4000-8000-000000000001' WHERE id=95671",
        "UPDATE public.medication_takes SET client_uuid=NULL WHERE id=95671",
        "UPDATE public.medication_takes SET portable_id='95600000-0000-4000-8000-000000000002' WHERE id=95671",
        "UPDATE public.medication_takes SET portable_id=NULL WHERE id=95671",
        "UPDATE public.medication_takes SET id=95679 WHERE id=95671",
    ]
}

#[tokio::test]
async fn recorded_take_creation_timestamp_is_immutable() -> TestResult {
    let fixture = seeded_fixture().await?;
    let (runtime, role) = fixture.runtime().await;
    let result = async {
        actor_scope(&runtime).await?;
        insert_takes(&runtime).await?;
        Ok::<_, DbErr>(runtime.execute_unprepared(
            "UPDATE public.medication_takes SET created_at=created_at+interval '1 second' WHERE id=95671"
        ).await)
    }.await;
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
    let mutation = result?;
    assert_eq!(
        database_code(&mutation).as_deref(),
        Some("23514"),
        "{mutation:?}"
    );
    Ok(())
}

#[tokio::test]
async fn provenance_guards_are_enabled_invokers_without_direct_execution_grants() -> TestResult {
    let fixture = Fixture::new().await;
    let rows = fixture.database.query_all_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT p.proname,pg_get_userbyid(p.proowner) AS owner,p.prosecdef,
          p.proconfig=ARRAY['search_path=pg_catalog']::text[] AS fixed_path,
          EXISTS(SELECT 1 FROM aclexplode(COALESCE(p.proacl,acldefault('f',p.proowner))) a WHERE a.grantee=0 AND a.privilege_type='EXECUTE') AS public_execute,
          has_function_privilege('med_tracker_app',p.oid,'EXECUTE') AS app_execute,
          (SELECT count(*) FROM pg_trigger t WHERE t.tgfoid=p.oid AND NOT t.tgisinternal AND t.tgenabled='O') AS enabled_triggers
         FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace
         WHERE n.nspname='med_tracker' AND p.proname IN ('guard_care_source_identity','guard_medication_take_identity') ORDER BY p.proname"
    )).await;
    fixture.close().await;
    let rows = rows?;
    assert_eq!(rows.len(), 2, "Both dedicated invoker guards must exist");
    for row in rows {
        let name: String = row.try_get("", "proname")?;
        assert_eq!(row.try_get::<String>("", "owner")?, "med_tracker_owner");
        assert!(!row.try_get::<bool>("", "prosecdef")?, "{name}");
        assert!(row.try_get::<bool>("", "fixed_path")?, "{name}");
        assert!(!row.try_get::<bool>("", "public_execute")?, "{name}");
        assert!(!row.try_get::<bool>("", "app_execute")?, "{name}");
        assert_eq!(
            row.try_get::<i64>("", "enabled_triggers")?,
            if name == "guard_care_source_identity" {
                2
            } else {
                1
            }
        );
    }
    Ok(())
}
