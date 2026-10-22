use super::{Application, require, storage_boundary::foreign_token, withdrawal::declaration};
use med_tracker::models::{
    access::{self, Actor, HouseholdScope, PersonAccess},
    care::administration::recovery::{
        self, RecoveryAction, RecoveryCommand, RecoveryGuardianId, RecoveryOutcome,
        RecoverySnapshot, RecoverySubjectId,
    },
};
use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement, TransactionTrait};
use serde_json::{Value, json};

fn db_error(error: impl std::fmt::Display) -> DbErr {
    DbErr::Custom(error.to_string())
}

fn scope() -> HouseholdScope {
    HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-recovery-command".into(),
    }
}

async fn effective_snapshot(
    tenant: &access::TenantTransaction,
    subject: i64,
) -> Result<RecoverySnapshot, DbErr> {
    let query = sea_orm::sea_query::Query::select()
        .columns([
            ("snapshot", "snapshot_token"),
            ("snapshot", "snapshot_revision"),
            ("snapshot", "link_token"),
            ("snapshot", "has_effective_guardian"),
        ])
        .from_function(
            sea_orm::sea_query::Func::cust("med_tracker.guardian_recovery_snapshot")
                .arg(sea_orm::sea_query::Expr::val(subject)),
            "snapshot",
        )
        .to_owned();
    let row = tenant
        .transaction()
        .query_one_raw(tenant.transaction().get_database_backend().build(&query))
        .await?
        .ok_or_else(|| DbErr::Custom("Missing guarded recovery snapshot".into()))?;
    require(
        row.try_get::<bool>("", "has_effective_guardian")?,
        "Canonical snapshot must report effective guardian",
    )?;
    Ok(RecoverySnapshot {
        token: row.try_get("", "snapshot_token")?,
        revision: row.try_get("", "snapshot_revision")?,
        link_token: row.try_get("", "link_token")?,
    })
}

#[tokio::test]
async fn guardian_recovery_account_then_household_closure_order_rejects_without_effects()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let result=async {
        app.fixture.admin.execute_unprepared(
            "INSERT INTO public.people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES
               (73994,'Closure race minor','2020-01-01',1,false,now(),now()),
               (73995,'Closing candidate','1980-01-01',0,true,now(),now());
             INSERT INTO public.household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES
               (74994,72001,73994,'member','active',now(),now(),now()),
               (74995,72001,73995,'member','active',now(),now(),now());
             INSERT INTO public.accounts(id,person_id,email,status,created_at,updated_at) VALUES
               (71995,73995,'closing-candidate@example.test',2,now(),now());
             INSERT INTO public.users(id,person_id,email_address,active,created_at,updated_at) VALUES
               (77995,73995,'closing-candidate@example.test',true,now(),now())"
        ).await?;
        let tenant=access::begin(&app.fixture.runtime,&scope(),None).await.map_err(db_error)?;
        let snapshot=recovery::options(&tenant).await.map_err(db_error)?.subjects.into_iter()
            .find(|item|item.id.get()==73994)
            .ok_or_else(||DbErr::Custom("Missing closure-race minor".into()))?.snapshot;
        tenant.rollback().await.map_err(db_error)?;
        let blocker=app.fixture.admin.begin().await?;
        blocker.execute_unprepared("SELECT id FROM public.accounts WHERE id=71995 FOR UPDATE").await?;
        let runtime=app.fixture.runtime.clone();
        let attempt=tokio::spawn(async move {
            let tenant=access::begin(&runtime,&scope(),None).await.unwrap();
            let outcome=recovery::execute(&tenant,RecoveryCommand {
                subject:RecoverySubjectId::parse(73994).unwrap(),
                guardian:RecoveryGuardianId::parse(73995).unwrap(),
                expected:snapshot,action:RecoveryAction::Appoint,
                reason:"Candidate account closure must invalidate recovery.".into(),
            }).await;
            if outcome.is_ok() {tenant.commit().await.unwrap()} else {tenant.rollback().await.unwrap()}
            outcome.map(|value|value.outcome).map_err(|error|format!("{error:?}"))
        });
        let mut waiting=false;
        for _ in 0..40 {
            let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE wait_event_type='Lock' AND state='active' AND pid<>pg_backend_pid()) AS waiting"))
                .await?.ok_or_else(||DbErr::Custom("Missing closure-race wait state".into()))?;
            waiting=row.try_get("","waiting")?;
            if waiting {break}
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        if !waiting {
            blocker.rollback().await?;
            let _=attempt.await;
            return Err(DbErr::Custom("Recovery did not wait on candidate account".into()));
        }
        let household=tokio::time::timeout(std::time::Duration::from_secs(8),
            blocker.execute_unprepared("SELECT id FROM public.households WHERE id=72001 FOR UPDATE"))
            .await.map_err(db_error)?;
        if let Err(error)=household {
            let _=blocker.rollback().await;
            let _=attempt.await;
            return Err(DbErr::Custom(format!("Closure household lock failed: {error}")));
        }
        blocker.execute_unprepared(
            "UPDATE public.household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE id=74995;
             UPDATE public.users SET active=false WHERE id=77995;
             UPDATE public.accounts SET status=3 WHERE id=71995"
        ).await?;
        blocker.commit().await?;
        let outcome=tokio::time::timeout(std::time::Duration::from_secs(8),attempt)
            .await.map_err(db_error)?.map_err(db_error)?;
        require(outcome.as_ref().is_err_and(|error|error.contains("is no longer eligible")),
            &format!("Recovery must reject closed candidate after account wait: {outcome:?}"))?;
        let state=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM public.guardian_declarations WHERE subject_person_id=73994) AS declarations,
             (SELECT count(*) FROM public.carer_relationships WHERE patient_id=73994) AS links,
             (SELECT count(*) FROM public.security_audit_events WHERE event_type LIKE 'guardian_recovery.%') AS audits,
             (SELECT count(*) FROM public.versions WHERE item_type='GuardianDeclaration' AND event='appointed') AS versions,
             (SELECT permissions_version::bigint FROM public.household_memberships WHERE id=74995) AS permissions_version"))
            .await?.ok_or_else(||DbErr::Custom("Missing closure-race state".into()))?;
        require(state.try_get::<i64>("","declarations")?==0 && state.try_get::<i64>("","links")?==0
            && state.try_get::<i64>("","audits")?==0 && state.try_get::<i64>("","versions")?==0
            && state.try_get::<i64>("","permissions_version")?==2,
            "Rejected recovery must leave no authority, audit, version or extra permission effects")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_options_report_bounded_truncation() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result=async {
        app.fixture.admin.execute_unprepared(
            "INSERT INTO public.people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at)
               SELECT 85000+g,'Synthetic recovery list minor '||g,'2020-01-01',1,false,now(),now()
               FROM generate_series(1,101) AS g;
             INSERT INTO public.household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at)
               SELECT 86000+g,72001,85000+g,'member','active',now(),now(),now()
               FROM generate_series(1,101) AS g"
        ).await?;
        let tenant=access::begin(&app.fixture.runtime,&scope(),None).await.map_err(db_error)?;
        let options=recovery::options(&tenant).await.map_err(db_error)?;
        require(options.subjects.len()==100 && options.subjects_has_more,
            "Bounded subject list must truthfully report unseen eligible local minors")?;
        require(!options.guardians_has_more,"Short guardian list must report completion")?;
        tenant.rollback().await.map_err(db_error)?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_options_reach_orphan_after_101_guarded_minors() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let fixture = access::begin(&app.fixture.admin, &scope(), None).await.map_err(db_error)?;
        fixture.transaction().execute_unprepared(
            "INSERT INTO public.people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at)
               SELECT 85000+g,'A guarded minor '||lpad(g::text,3,'0'),'2020-01-01',1,false,now(),now()
               FROM generate_series(1,101) AS g;
             INSERT INTO public.people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at)
               VALUES(85999,'Zoe late orphan','2020-01-01',1,false,now(),now());
             INSERT INTO public.household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at)
               SELECT 86000+g,72001,85000+g,'member','active',now(),now(),now()
               FROM generate_series(1,101) AS g;
             INSERT INTO public.household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at)
               VALUES(86999,72001,85999,'member','active',now(),now(),now());
             INSERT INTO public.guardian_declarations(subject_person_id,guardian_person_id,origin_household_id,declared_by_account_id,subject_name_at_declaration)
               SELECT 85000+g,73001,72001,71001,'A guarded minor '||g
               FROM generate_series(1,101) AS g;
             INSERT INTO public.carer_relationships(household_id,carer_id,patient_id,guardian_declaration_id,relationship_type,active,created_at,updated_at)
               SELECT 72001,73001,d.subject_person_id,d.id,'family_member',true,now(),now()
               FROM public.guardian_declarations d WHERE d.subject_person_id BETWEEN 85001 AND 85101"
        ).await?;
        fixture.commit().await.map_err(db_error)?;
        let tenant = access::begin(&app.fixture.runtime,&scope(),None).await.map_err(db_error)?;
        let guarded = tenant.transaction().query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT count(*) AS guarded FROM public.people p
               CROSS JOIN LATERAL med_tracker.guardian_recovery_snapshot(p.id) s
               WHERE p.id BETWEEN 85001 AND 85101 AND s.has_effective_guardian"))
            .await?.ok_or_else(||DbErr::Custom("Missing guarded count".into()))?;
        require(guarded.try_get::<i64>("","guarded")? == 101,
            "Fixture must contain 101 canonically effective guardian links")?;
        let options = recovery::options(&tenant).await.map_err(db_error)?;
        require(options.subjects.iter().any(|subject|subject.id.get()==85999)
            && options.subjects.iter().all(|subject|!subject.has_effective_guardian)
            && !options.subjects_has_more,
            "Later orphan must be reachable and warning must count actual orphans")?;
        tenant.rollback().await.map_err(db_error)?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_administrator_appoints_for_minor_without_history() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result=async {
        app.fixture.admin.execute_unprepared(
            "INSERT INTO public.people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES
               (73994,'No-history minor','2020-01-01',1,false,now(),now()),
               (73999,'No-history administrator','1980-01-01',0,true,now(),now());
             INSERT INTO public.household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES
               (74994,72001,73994,'member','active',now(),now(),now()),
               (74999,72001,73999,'administrator','active',now(),now(),now());
             INSERT INTO public.accounts(id,person_id,email,status,created_at,updated_at) VALUES(71999,73999,'no-history-admin@example.test',2,now(),now());
             INSERT INTO public.users(id,person_id,email_address,active,created_at,updated_at) VALUES(77999,73999,'no-history-admin@example.test',true,now(),now())"
        ).await?;
        let manager_scope=HouseholdScope {actor:Actor{account_id:71999},household_id:72001,
            request_id:"synthetic-no-history-administrator".into()};
        let tenant=access::begin(&app.fixture.runtime,&manager_scope,None).await.map_err(db_error)?;
        let selected=recovery::options(&tenant).await.map_err(db_error)?.subjects.into_iter()
            .find(|item|item.id.get()==73994)
            .ok_or_else(||DbErr::Custom("No-history minor missing from options".into()))?;
        require(!selected.has_effective_guardian && selected.restorable.is_none()
            && selected.snapshot.token.is_nil() && selected.snapshot.revision==0,
            "No-history minor must have nil history confirmation and no authority")?;
        let appointed=recovery::execute(&tenant,RecoveryCommand {
            subject:RecoverySubjectId::parse(73994).map_err(db_error)?,
            guardian:RecoveryGuardianId::parse(73001).map_err(db_error)?,
            expected:selected.snapshot,action:RecoveryAction::Appoint,
            reason:"Administrator confirmed adult guardian for this minor.".into(),
        }).await.map_err(db_error)?;
        require(appointed.outcome==RecoveryOutcome::Appointed,
            "Administrator must be able to appoint for a no-history minor")?;
        tenant.commit().await.map_err(db_error)?;
        let state=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM public.guardian_declarations WHERE id=$1 AND subject_person_id=73994 AND guardian_person_id=73001 AND declared_by_account_id=71999 AND source='manager_appointment') AS declarations,(SELECT count(*) FROM public.security_audit_events WHERE event_type='guardian_recovery.appointed' AND actor_account_id=71999 AND metadata->>'subject_person_id'='73994') AS audits",
            [appointed.declaration_id.into()])).await?
            .ok_or_else(||DbErr::Custom("Missing administrator recovery state".into()))?;
        require(state.try_get::<i64>("","declarations")?==1 && state.try_get::<i64>("","audits")?==1,
            "No-history appointment must record the actual administrator and audit")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_latest_history_uses_declaration_time_not_high_old_id()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let result=async {
        app.fixture.admin.execute_unprepared(
            "SELECT setval(pg_get_serial_sequence('public.guardian_declarations','id'),990000,false)"
        ).await?;
        let token=app.token().await;
        let (subject,old_id)=declaration(&app,&token).await?;
        require(old_id==990000,"Fixture must give old initial declaration a high ID")?;
        let withdrawn=app.client.patch(format!("{}/api/v1/auth/guardian-declarations/{old_id}",app.origin))
            .bearer_auth(&token)
            .json(&json!({"guardian_declaration":{"withdrawn":true,"revision":"1"}}))
            .send().await.map_err(db_error)?;
        require(withdrawn.status().as_u16()==200,"High-ID initial declaration withdrawal must succeed")?;
        app.fixture.admin.execute_unprepared(
            "SELECT setval(pg_get_serial_sequence('public.guardian_declarations','id'),500000,false)"
        ).await?;
        let tenant=access::begin(&app.fixture.runtime,&scope(),None).await.map_err(db_error)?;
        let selected=recovery::options(&tenant).await.map_err(db_error)?.subjects.into_iter()
            .find(|item|item.id.get()==subject)
            .ok_or_else(||DbErr::Custom("Missing high-ID history subject".into()))?;
        let appointed=recovery::execute(&tenant,RecoveryCommand {
            subject:RecoverySubjectId::parse(subject).map_err(db_error)?,
            guardian:RecoveryGuardianId::parse(73001).map_err(db_error)?,
            expected:selected.snapshot,action:RecoveryAction::Appoint,
            reason:"Appointed again after retained high-ID withdrawal.".into(),
        }).await.map_err(db_error)?;
        require(appointed.declaration_id<old_id,"New appointment must have a lower generated ID")?;
        tenant.commit().await.map_err(db_error)?;
        let tenant=access::begin(&app.fixture.runtime,&scope(),None).await.map_err(db_error)?;
        let latest=effective_snapshot(&tenant,subject).await?;
        require(!recovery::options(&tenant).await.map_err(db_error)?.subjects.iter()
            .any(|item|item.id.get()==subject),
            "Guarded subject must leave recovery options after appointment")?;
        tenant.rollback().await.map_err(db_error)?;
        let row=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT confirmation_token,revision FROM public.guardian_declarations WHERE id=$1",
            [appointed.declaration_id.into()])).await?
            .ok_or_else(||DbErr::Custom("Missing lower-ID new appointment".into()))?;
        require(latest.token==row.try_get::<uuid::Uuid>("","confirmation_token")?
            && latest.revision==row.try_get::<i64>("","revision")?
            && latest.token!=selected.snapshot.token,
            "Latest confirmation must follow declaration time, including terminal high-ID history")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_after_personal_withdrawal_creates_fresh_appointment() -> Result<(), DbErr>
{
    let app = Application::new().await;
    let result = async {
        let token = app.token().await;
        let (subject, old_id) = declaration(&app, &token).await?;
        let response = app.client.patch(format!("{}/api/v1/auth/guardian-declarations/{old_id}",app.origin))
            .bearer_auth(&token)
            .json(&json!({"guardian_declaration":{"withdrawn":true,"revision":"1"}}))
            .send().await.map_err(db_error)?;
        require(response.status().as_u16()==200,"Withdrawal must succeed before recovery")?;
        let tenant = access::begin(&app.fixture.runtime, &scope(), None).await.map_err(db_error)?;
        let options = recovery::options(&tenant).await.map_err(|error| DbErr::Custom(format!("options: {error:?}")))?;
        let selected = options.subjects.iter().find(|item| item.id.get()==subject)
            .ok_or_else(||DbErr::Custom("Withdrawn minor missing from recovery options".into()))?;
        require(!selected.has_effective_guardian && selected.restorable.is_none(),
            "Withdrawn declaration must not be restored")?;
        require(options.guardians.iter().any(|item| item.id.get()==73001),
            "Eligible same-person appointee must remain available")?;
        let original_snapshot=selected.snapshot;
        let command=RecoveryCommand {
            subject: RecoverySubjectId::parse(subject).map_err(db_error)?,
            guardian: RecoveryGuardianId::parse(73001).map_err(db_error)?,
            expected: original_snapshot,
            action: RecoveryAction::Appoint,
            reason: "Verified the guardian's current authority.".into(),
        };
        let outcome=recovery::execute(&tenant,command).await.map_err(|error| DbErr::Custom(format!("execute: {error:?}")))?;
        require(outcome.outcome==RecoveryOutcome::Appointed && outcome.declaration_id!=old_id,
            "Recovery must create a fresh declaration, never revive withdrawn authority")?;
        tenant.commit().await.map_err(db_error)?;
        let state=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM public.guardian_declarations WHERE subject_person_id=$1 AND withdrawn_at IS NOT NULL AND source='initial_attestation') AS withdrawn,(SELECT count(*) FROM public.guardian_declarations WHERE subject_person_id=$1 AND source='manager_appointment' AND withdrawn_at IS NULL AND superseded_at IS NULL) AS appointed,(SELECT count(*) FROM public.security_audit_events WHERE event_type='guardian_recovery.appointed' AND metadata->>'subject_person_id'=$2) AS audits",
            [subject.into(),subject.to_string().into()])).await?
            .ok_or_else(||DbErr::Custom("Missing recovery state".into()))?;
        require(state.try_get::<i64>("","withdrawn")?==1
            && state.try_get::<i64>("","appointed")?==1
            && state.try_get::<i64>("","audits")?==1,
            "Recovery must retain withdrawn history and audit the fresh appointment")?;
        let own=app.client.get(format!("{}/api/v1/auth/guardian-declarations",app.origin))
            .bearer_auth(&token).send().await.map_err(db_error)?;
        let body:Value=own.json().await.map_err(db_error)?;
        require(body["data"].as_array().is_some_and(|rows|rows.iter().any(|row|
            row["source"]=="manager_appointment" && row["superseded_at"].is_null())),
            "Own API must distinguish the manager appointment")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

async fn concurrent_appointment(
    app: &Application,
    household_id: i64,
    subject: i64,
    snapshot: RecoverySnapshot,
) -> Result<Result<RecoveryOutcome, String>, DbErr> {
    let scope = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id,
        request_id: format!("synthetic-concurrent-recovery-{household_id}"),
    };
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .map_err(db_error)?;
    let result = recovery::execute(
        &tenant,
        RecoveryCommand {
            subject: RecoverySubjectId::parse(subject).map_err(db_error)?,
            guardian: RecoveryGuardianId::parse(73001).map_err(db_error)?,
            expected: snapshot,
            action: RecoveryAction::Appoint,
            reason: "Confirmed the guardian for this household.".into(),
        },
    )
    .await;
    match result {
        Ok(value) => {
            tenant.commit().await.map_err(db_error)?;
            Ok(Ok(value.outcome))
        }
        Err(error) => {
            tenant.rollback().await.map_err(db_error)?;
            Ok(Err(format!("{error:?}")))
        }
    }
}

#[tokio::test]
async fn guardian_recovery_competing_households_serialise_by_subject() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result=async {
        let token=app.token().await;
        let (subject,old_id)=declaration(&app,&token).await?;
        let withdrawn=app.client.patch(format!("{}/api/v1/auth/guardian-declarations/{old_id}",app.origin))
            .bearer_auth(&token).json(&json!({"guardian_declaration":{"withdrawn":true,"revision":"1"}}))
            .send().await.map_err(db_error)?;
        require(withdrawn.status().as_u16()==200,"Concurrent test requires terminal old authority")?;
        app.fixture.admin.execute_unprepared(
            "INSERT INTO public.households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(72994,71001,'Second recovery household','recovery-concurrent','UTC',now(),now());
             INSERT INTO public.household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74994,72994,73001,'owner','active',now(),now(),now())"
        ).await?;
        app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO public.household_memberships(household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(72994,$1,'member','active',now(),now(),now())",
            [subject.into()])).await?;
        let tenant=access::begin(&app.fixture.runtime,&scope(),None).await.map_err(db_error)?;
        let snapshot=recovery::options(&tenant).await.map_err(db_error)?.subjects.into_iter()
            .find(|item|item.id.get()==subject)
            .ok_or_else(||DbErr::Custom("Missing concurrent subject".into()))?.snapshot;
        tenant.rollback().await.map_err(db_error)?;
        let (first,second)=tokio::join!(
            concurrent_appointment(&app,72001,subject,snapshot),
            concurrent_appointment(&app,72994,subject,snapshot),
        );
        let first=first?;
        let second=second?;
        require((first==Ok(RecoveryOutcome::Appointed) && second.as_ref().is_err_and(|text|text.contains("stale_guardian_recovery")))
            || (second==Ok(RecoveryOutcome::Appointed) && first.as_ref().is_err_and(|text|text.contains("stale_guardian_recovery"))),
            &format!("Exactly one competing household must appoint; outcomes {first:?}, {second:?}"))?;
        let state=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM public.guardian_declarations WHERE subject_person_id=$1 AND source='manager_appointment') AS appointments,(SELECT count(*) FROM public.guardian_declarations WHERE subject_person_id=$1 AND withdrawn_at IS NULL AND superseded_at IS NULL) AS open",
            [subject.into()])).await?.ok_or_else(||DbErr::Custom("Missing concurrent state".into()))?;
        require(state.try_get::<i64>("","appointments")?==1 && state.try_get::<i64>("","open")?==1,
            "Subject serialization must leave one new open authority")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_restore_and_personal_withdrawal_serialize_without_deadlock()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let result=async {
        let token=app.token().await;
        let (subject,declaration_id)=declaration(&app,&token).await?;
        app.fixture.admin.execute_unprepared(
            "INSERT INTO public.people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73999,'Restore race administrator','1980-01-01',0,true,now(),now());
             INSERT INTO public.accounts(id,person_id,email,status,created_at,updated_at) VALUES(71999,73999,'restore-race-admin@example.test',2,now(),now());
             INSERT INTO public.users(id,person_id,email_address,active,created_at,updated_at) VALUES(77999,73999,'restore-race-admin@example.test',true,now(),now());
             INSERT INTO public.household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74999,72001,73999,'administrator','active',now(),now(),now())"
        ).await?;
        app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE public.carer_relationships SET active=false WHERE guardian_declaration_id=$1",
            [declaration_id.into()])).await?;
        let manager_scope=HouseholdScope {actor:Actor{account_id:71999},household_id:72001,
            request_id:"synthetic-restore-withdrawal-race".into()};
        let tenant=access::begin(&app.fixture.runtime,&manager_scope,None).await.map_err(db_error)?;
        let selected=recovery::options(&tenant).await.map_err(db_error)?.subjects.into_iter()
            .find(|item|item.id.get()==subject)
            .ok_or_else(||DbErr::Custom("Missing restore-race minor".into()))?;
        let link=selected.restorable.ok_or_else(||DbErr::Custom("Missing restore-race link".into()))?;
        tenant.rollback().await.map_err(db_error)?;
        let blocker=app.fixture.admin.begin().await?;
        blocker.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT id FROM public.guardian_declarations WHERE id=$1 FOR UPDATE",
            [declaration_id.into()])).await?
            .ok_or_else(||DbErr::Custom("Missing declaration lock blocker".into()))?;
        let runtime=app.fixture.runtime.clone();
        let restore=tokio::spawn(async move {
            let tenant=access::begin(&runtime,&manager_scope,None).await.unwrap();
            let result=recovery::execute(&tenant,RecoveryCommand {
                subject:RecoverySubjectId::parse(subject).unwrap(),
                guardian:link.guardian_id,expected:selected.snapshot,
                action:RecoveryAction::Restore {declaration_id:link.declaration_id,
                    relationship_id:link.relationship_id},
                reason:"Confirm existing guardian link before withdrawal.".into(),
            }).await;
            if result.is_ok() {tenant.commit().await.unwrap()} else {tenant.rollback().await.unwrap()}
            result.map(|value|value.outcome).map_err(|error|format!("{error:?}"))
        });
        let mut recovery_waiting=false;
        for _ in 0..40 {
            let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE wait_event_type='Lock' AND query LIKE '%guardian_recovery_command%') AS waiting"))
                .await?.ok_or_else(||DbErr::Custom("Missing recovery wait state".into()))?;
            recovery_waiting=row.try_get("","waiting")?;
            if recovery_waiting {break}
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        if !recovery_waiting {
            blocker.commit().await?;
            let _=restore.await;
            return Err(DbErr::Custom("Restore did not wait on declaration blocker".into()));
        }
        let client=app.client.clone();
        let origin=app.origin.clone();
        let withdrawal=tokio::spawn(async move {
            let response=client.patch(format!("{origin}/api/v1/auth/guardian-declarations/{declaration_id}"))
                .bearer_auth(&token)
                .json(&json!({"guardian_declaration":{"withdrawn":true,"revision":"1"}}))
                .send().await.map_err(|error|error.to_string())?;
            Ok::<u16,String>(response.status().as_u16())
        });
        let mut two_waiters=false;
        for _ in 0..40 {
            let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
                "SELECT count(*) AS waiting FROM pg_stat_activity WHERE wait_event_type='Lock' AND state='active' AND pid<>pg_backend_pid()"))
                .await?.ok_or_else(||DbErr::Custom("Missing two-waiter state".into()))?;
            two_waiters=row.try_get::<i64>("","waiting")?>=2;
            if two_waiters {break}
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        blocker.commit().await?;
        let restore_outcome=tokio::time::timeout(std::time::Duration::from_secs(8),restore)
            .await.map_err(db_error)?.map_err(db_error)?;
        let withdrawal_status=tokio::time::timeout(std::time::Duration::from_secs(8),withdrawal)
            .await.map_err(db_error)?.map_err(db_error)?;
        require(two_waiters,"Both restore and personal withdrawal must enter lock waits")?;
        require(matches!(restore_outcome,Ok(RecoveryOutcome::Restored))
            && matches!(withdrawal_status,Ok(200)),
            &format!("Restore and withdrawal must serialize without lock error: {restore_outcome:?}, {withdrawal_status:?}"))?;
        let state=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM public.guardian_declarations WHERE id=$1 AND withdrawn_at IS NOT NULL AND superseded_at IS NULL) AS terminal,(SELECT count(*) FROM public.security_audit_events WHERE event_type='guardian_recovery.restored' AND metadata->>'subject_person_id'=$2) AS restores,(SELECT count(*) FROM public.versions WHERE item_type='GuardianDeclaration' AND item_id=$1 AND event='withdraw') AS withdrawals",
            [declaration_id.into(),subject.to_string().into()])).await?
            .ok_or_else(||DbErr::Custom("Missing serialized restore/withdrawal state".into()))?;
        require(state.try_get::<i64>("","terminal")?==1 && state.try_get::<i64>("","restores")?==1
            && state.try_get::<i64>("","withdrawals")?==1,
            "Final authority must be terminal with one restore and one personal withdrawal audit")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_candidate_lockout_after_account_wait_rejects_without_effects()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let result=async {
        let token=app.token().await;
        let (subject,_)=declaration(&app,&token).await?;
        app.fixture.admin.execute_unprepared(
            "INSERT INTO public.people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES
               (73995,'Lockout race candidate','1980-01-01',0,true,now(),now()),
               (73999,'Lockout race manager','1980-01-01',0,true,now(),now());
             INSERT INTO public.accounts(id,person_id,email,status,created_at,updated_at) VALUES
               (71995,73995,'lockout-race-candidate@example.test',2,now(),now()),
               (71999,73999,'lockout-race-manager@example.test',2,now(),now());
             INSERT INTO public.users(id,person_id,email_address,active,created_at,updated_at) VALUES
               (77995,73995,'lockout-race-candidate@example.test',true,now(),now()),
               (77999,73999,'lockout-race-manager@example.test',true,now(),now());
             INSERT INTO public.household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES
               (74995,72001,73995,'member','active',now(),now(),now()),
               (74999,72001,73999,'administrator','active',now(),now(),now());
             UPDATE public.accounts SET status=1 WHERE id=71001"
        ).await?;
        let manager_scope=HouseholdScope {actor:Actor{account_id:71999},household_id:72001,
            request_id:"synthetic-candidate-lockout-race".into()};
        let tenant=access::begin(&app.fixture.runtime,&manager_scope,None).await.map_err(db_error)?;
        let selected=recovery::options(&tenant).await.map_err(db_error)?.subjects.into_iter()
            .find(|item|item.id.get()==subject)
            .ok_or_else(||DbErr::Custom("Missing lockout-race minor".into()))?;
        require(!selected.has_effective_guardian,"Disabled old guardian must make the minor orphaned")?;
        tenant.rollback().await.map_err(db_error)?;
        let blocker=app.fixture.admin.begin().await?;
        blocker.execute_unprepared("UPDATE public.accounts SET status=2 WHERE id=71995").await?;
        let runtime=app.fixture.runtime.clone();
        let snapshot=selected.snapshot;
        let attempt=tokio::spawn(async move {
            let tenant=access::begin(&runtime,&manager_scope,None).await.unwrap();
            let result=recovery::execute(&tenant,RecoveryCommand {
                subject:RecoverySubjectId::parse(subject).unwrap(),
                guardian:RecoveryGuardianId::parse(73995).unwrap(),
                expected:snapshot,action:RecoveryAction::Appoint,
                reason:"Lockout race must reject appointment.".into(),
            }).await;
            if result.is_ok() {tenant.commit().await.unwrap()} else {tenant.rollback().await.unwrap()}
            result.map(|value|value.outcome).map_err(|error|format!("{error:?}"))
        });
        let mut waiting=false;
        for _ in 0..40 {
            let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE wait_event_type='Lock' AND query LIKE '%guardian_recovery_command%') AS waiting"))
                .await?.ok_or_else(||DbErr::Custom("Missing lockout-race wait state".into()))?;
            waiting=row.try_get("","waiting")?;
            if waiting {break}
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        if !waiting {
            blocker.commit().await?;
            let _=attempt.await;
            return Err(DbErr::Custom("Recovery did not wait on candidate account".into()));
        }
        blocker.execute_unprepared(
            "INSERT INTO public.account_lockouts(account_id,key,deadline) VALUES(71995,'synthetic-recovery-race',now()+interval '1 hour')"
        ).await?;
        blocker.commit().await?;
        let outcome=attempt.await.map_err(db_error)?;
        require(outcome.as_ref().is_err_and(|error|error.contains("is no longer eligible")),
            &format!("New candidate lockout must reject after wait: {outcome:?}"))?;
        let state=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM public.guardian_declarations WHERE subject_person_id=$1 AND source='manager_appointment') AS appointments,(SELECT count(*) FROM public.carer_relationships WHERE patient_id=$1 AND carer_id=73995) AS links,(SELECT count(*) FROM public.security_audit_events WHERE event_type LIKE 'guardian_recovery.%' AND metadata->>'subject_person_id'=$2) AS audits,(SELECT count(*) FROM public.versions WHERE item_type='GuardianDeclaration' AND event='appointed' AND item_id IN (SELECT id FROM public.guardian_declarations WHERE subject_person_id=$1)) AS versions,(SELECT permissions_version::bigint FROM public.household_memberships WHERE id=74995) AS permissions_version",
            [subject.into(),subject.to_string().into()])).await?
            .ok_or_else(||DbErr::Custom("Missing rejected lockout-race state".into()))?;
        require(state.try_get::<i64>("","appointments")?==0 && state.try_get::<i64>("","links")?==0
            && state.try_get::<i64>("","audits")?==0 && state.try_get::<i64>("","versions")?==0
            && state.try_get::<i64>("","permissions_version")?==1,
            "Rejected lockout-race command must have zero authority, audit and permission side effects")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_waiting_on_candidate_rechecks_restored_old_authority()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let result=async {
        let token=app.token().await;
        let (subject,_)=declaration(&app,&token).await?;
        app.fixture.admin.execute_unprepared(
            "INSERT INTO public.people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES
               (73995,'Candidate lock adult','1980-01-01',0,true,now(),now()),
               (73999,'Recovery administrator','1980-01-01',0,true,now(),now());
             INSERT INTO public.accounts(id,person_id,email,status,created_at,updated_at) VALUES
               (71995,73995,'candidate-lock@example.test',2,now(),now()),
               (71999,73999,'recovery-admin@example.test',2,now(),now());
             INSERT INTO public.users(id,person_id,email_address,active,created_at,updated_at) VALUES
               (77995,73995,'candidate-lock@example.test',true,now(),now()),
               (77999,73999,'recovery-admin@example.test',true,now(),now());
             INSERT INTO public.household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES
               (74995,72001,73995,'member','active',now(),now(),now()),
               (74999,72001,73999,'administrator','active',now(),now(),now());
             UPDATE public.accounts SET status=1 WHERE id=71001"
        ).await?;
        let manager_scope=HouseholdScope {
            actor:Actor{account_id:71999},household_id:72001,
            request_id:"synthetic-candidate-lock-race".into(),
        };
        let tenant=access::begin(&app.fixture.runtime,&manager_scope,None).await.map_err(db_error)?;
        let selected=recovery::options(&tenant).await.map_err(db_error)?.subjects.into_iter()
            .find(|item|item.id.get()==subject)
            .ok_or_else(||DbErr::Custom("Missing lock-race subject".into()))?;
        require(!selected.has_effective_guardian,"Disabled old account must make the minor orphaned")?;
        tenant.rollback().await.map_err(db_error)?;
        let blocker=app.fixture.admin.begin().await?;
        blocker.execute_unprepared("UPDATE public.accounts SET status=2 WHERE id=71995").await?;
        let runtime=app.fixture.runtime.clone();
        let snapshot=selected.snapshot;
        let attempt=tokio::spawn(async move {
            let tenant=access::begin(&runtime,&manager_scope,None).await.unwrap();
            let result=recovery::execute(&tenant,RecoveryCommand {
                subject:RecoverySubjectId::parse(subject).unwrap(),
                guardian:RecoveryGuardianId::parse(73995).unwrap(),
                expected:snapshot,
                action:RecoveryAction::Appoint,
                reason:"Candidate is eligible after account lock.".into(),
            }).await;
            if result.is_ok() {tenant.commit().await.unwrap()} else {tenant.rollback().await.unwrap()}
            result.map(|value|value.outcome).map_err(|error|format!("{error:?}"))
        });
        let mut waiting=false;
        for _ in 0..40 {
            let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE wait_event_type='Lock' AND query LIKE '%guardian_recovery_command%') AS waiting"))
                .await?.ok_or_else(||DbErr::Custom("Missing lock state".into()))?;
            waiting=row.try_get("","waiting")?;
            if waiting {break}
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        if !waiting {
            blocker.commit().await?;
            let _=attempt.await;
            return Err(DbErr::Custom("Recovery was not observed waiting on the candidate lock".into()));
        }
        blocker.execute_unprepared("UPDATE public.accounts SET status=2 WHERE id=71001").await?;
        blocker.commit().await?;
        let outcome=attempt.await.map_err(db_error)?;
        require(outcome.as_ref().is_err_and(|error|error.contains("guardian_already_effective")),
            &format!("Restored old authority must block appointment after lock wait: {outcome:?}"))?;
        let row=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT count(*) AS appointments FROM public.guardian_declarations WHERE subject_person_id=$1 AND source='manager_appointment'",
            [subject.into()])).await?.ok_or_else(||DbErr::Custom("Missing race outcome".into()))?;
        require(row.try_get::<i64>("","appointments")?==0,"Lock-wait race must leave no new authority")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_birthday_bounds_fail_closed() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result=async {
        let token=app.token().await;
        let (subject,_)=declaration(&app,&token).await?;
        app.fixture.admin.execute_unprepared(
            "UPDATE public.people SET date_of_birth=(med_tracker.guardian_recovery_latest_date()-interval '18 years')::date WHERE id=73001;
             INSERT INTO public.people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73996,'Birthday-boundary minor',(med_tracker.guardian_recovery_latest_date()-interval '18 years')::date,1,false,now(),now());
             INSERT INTO public.household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74996,72001,73996,'member','active',now(),now(),now())"
        ).await?;
        let tenant=access::begin(&app.fixture.runtime,&scope(),None).await.map_err(db_error)?;
        let options=recovery::options(&tenant).await.map_err(db_error)?;
        effective_snapshot(&tenant,subject).await?;
        require(!options.subjects.iter().any(|item|item.id.get()==subject),
            "Guardian adult in latest worldwide zone must prevent false orphanhood")?;
        require(!options.guardians.iter().any(|item|item.id.get()==73001),
            "New appointee must be adult in earliest worldwide zone")?;
        require(!options.subjects.iter().any(|item|item.id.get()==73996),
            "Subject approaching 18 in latest zone must be excluded")?;
        tenant.rollback().await.map_err(db_error)?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_definer_rejects_ordinary_member_and_foreign_subject() -> Result<(), DbErr>
{
    let app = Application::new().await;
    let result=async {
        let token=app.token().await;
        let (subject,declaration_id)=declaration(&app,&token).await?;
        app.fixture.admin.execute_unprepared(
            "INSERT INTO public.people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73997,'Ordinary recovery member','1980-01-01',0,true,now(),now());
             INSERT INTO public.accounts(id,person_id,email,status,created_at,updated_at) VALUES(71997,73997,'ordinary-recovery@example.test',2,now(),now());
             INSERT INTO public.users(id,person_id,email_address,active,created_at,updated_at) VALUES(77997,73997,'ordinary-recovery@example.test',true,now(),now());
             INSERT INTO public.household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74997,72001,73997,'member','active',now(),now(),now());
             INSERT INTO public.people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73998,'Foreign recovery minor','2020-01-01',1,false,now(),now())"
        ).await?;
        let member_scope=HouseholdScope {
            actor:Actor{account_id:71997},household_id:72001,
            request_id:"synthetic-ordinary-recovery-denial".into(),
        };
        let tenant=access::begin(&app.fixture.runtime,&member_scope,None).await.map_err(db_error)?;
        require(matches!(recovery::options(&tenant).await,
            Err(med_tracker::models::errors::OperationError::Forbidden)),
            "Ordinary member cannot enumerate recovery options")?;
        let raw=tenant.transaction().query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT outcome FROM med_tracker.guardian_recovery_command($1,73001,'00000000-0000-0000-0000-000000000000'::uuid,0,'00000000-0000-0000-0000-000000000000'::uuid,NULL,NULL,'Unauthorised recovery attempt','synthetic-ordinary')",
            [subject.into()])).await;
        require(raw.is_err(),"Ordinary member cannot invoke the guarded database command")?;
        tenant.rollback().await.map_err(db_error)?;
        let tenant=access::begin(&app.fixture.runtime,&scope(),None).await.map_err(db_error)?;
        let raw=tenant.transaction().query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT outcome FROM med_tracker.guardian_recovery_command(73998,73001,'00000000-0000-0000-0000-000000000000'::uuid,0,'00000000-0000-0000-0000-000000000000'::uuid,NULL,NULL,'Foreign subject attempt','synthetic-foreign')"))
            .await;
        require(raw.is_err(),"Manager cannot recover a nonparticipant minor through raw SQL")?;
        tenant.rollback().await.map_err(db_error)?;
        let tenant=access::begin(&app.fixture.runtime,&scope(),None).await.map_err(db_error)?;
        let forged=tenant.transaction().execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE public.carer_relationships SET confirmation_token=pg_catalog.gen_random_uuid() WHERE guardian_declaration_id=$1",
            [declaration_id.into()])).await;
        require(forged.is_err(),"Caller cannot forge the opaque relationship revision")?;
        tenant.rollback().await.map_err(db_error)?;
        let tenant=access::begin(&app.fixture.runtime,&scope(),None).await.map_err(db_error)?;
        let forged=tenant.transaction().execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO public.guardian_declarations(subject_person_id,guardian_person_id,origin_household_id,declared_by_account_id,subject_name_at_declaration,source,statement_version) VALUES($1,73001,72001,71001,'Forged manager source','manager_appointment',2)",
            [subject.into()])).await;
        require(forged.is_err(),"Caller cannot insert administrative authority outside the guarded command")?;
        tenant.rollback().await.map_err(db_error)?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_restores_only_the_exact_open_local_link() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result=async {
        let token=app.token().await;
        let (subject,declaration_id)=declaration(&app,&token).await?;
        let link=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT id FROM public.carer_relationships WHERE guardian_declaration_id=$1 AND household_id=72001",
            [declaration_id.into()])).await?.ok_or_else(||DbErr::Custom("Missing initial link".into()))?;
        let link_id:i64=link.try_get("","id")?;
        app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE public.carer_relationships SET active=false WHERE id=$1",[link_id.into()])).await?;
        let tenant=access::begin(&app.fixture.runtime,&scope(),None).await.map_err(db_error)?;
        let options=recovery::options(&tenant).await.map_err(db_error)?;
        let selected=options.subjects.iter().find(|item|item.id.get()==subject)
            .ok_or_else(||DbErr::Custom("Inactive link minor missing".into()))?;
        require(!selected.has_effective_guardian
            && selected.restorable.as_ref().is_some_and(|link|link.declaration_id==declaration_id && link.relationship_id==link_id),
            "Only exact open local declaration/link may be restored")?;
        let restored=recovery::execute(&tenant,RecoveryCommand {
            subject:RecoverySubjectId::parse(subject).map_err(db_error)?,
            guardian:RecoveryGuardianId::parse(73001).map_err(db_error)?,
            expected:selected.snapshot,
            action:RecoveryAction::Restore{declaration_id,relationship_id:link_id},
            reason:"Verified the existing link and current guardian.".into(),
        }).await.map_err(db_error)?;
        require(restored.outcome==RecoveryOutcome::Restored && restored.declaration_id==declaration_id
            && restored.relationship_id==link_id,"Restore must preserve exact provenance IDs")?;
        tenant.commit().await.map_err(db_error)?;
        let count=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM public.guardian_declarations WHERE subject_person_id=$1) AS declarations,(SELECT count(*) FROM public.carer_relationships WHERE id=$2 AND active) AS active",
            [subject.into(),link_id.into()])).await?.ok_or_else(||DbErr::Custom("Missing restore state".into()))?;
        require(count.try_get::<i64>("","declarations")?==1 && count.try_get::<i64>("","active")?==1,
            "Restore must not create new declaration history")?;
        app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE public.carer_relationships SET active=false WHERE id=$1",[link_id.into()])).await?;
        let tenant=access::begin(&app.fixture.runtime,&scope(),None).await.map_err(db_error)?;
        let after_cycle=recovery::options(&tenant).await.map_err(db_error)?.subjects.into_iter()
            .find(|item|item.id.get()==subject)
            .ok_or_else(||DbErr::Custom("Missing subject after link cycle".into()))?;
        require(after_cycle.restorable.is_some() && after_cycle.snapshot.token==selected.snapshot.token
            && after_cycle.snapshot.revision==selected.snapshot.revision
            && after_cycle.snapshot.link_token!=selected.snapshot.link_token,
            "Restore/deactivate must rotate only opaque link confirmation")?;
        let stale=recovery::execute(&tenant,RecoveryCommand {
            subject:RecoverySubjectId::parse(subject).map_err(db_error)?,
            guardian:RecoveryGuardianId::parse(73001).map_err(db_error)?,
            expected:selected.snapshot,
            action:RecoveryAction::Restore{declaration_id,relationship_id:link_id},
            reason:"Old restore confirmation must be stale.".into(),
        }).await;
        require(matches!(stale,Err(med_tracker::models::errors::OperationError::Conflict{ref code,..}) if code=="stale_guardian_recovery"),
            "Previously rendered restore must fail after restore/deactivate ABA")?;
        tenant.rollback().await.map_err(db_error)?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_foreign_active_link_blocks_local_takeover() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result=async {
        let token=app.token().await;
        let (subject,declaration_id)=declaration(&app,&token).await?;
        app.fixture.admin.execute_unprepared(
            "INSERT INTO public.households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(72994,71001,'Recovery foreign household','recovery-foreign','UTC',now(),now());
             INSERT INTO public.household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74994,72994,73001,'member','active',now(),now(),now())"
        ).await?;
        app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO public.household_memberships(household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(72994,$1,'member','active',now(),now(),now())",
            [subject.into()])).await?;
        app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE public.carer_relationships SET active=false WHERE guardian_declaration_id=$1 AND household_id=72001",
            [declaration_id.into()])).await?;
        app.fixture.admin.execute_unprepared("ALTER TABLE public.carer_relationships DISABLE TRIGGER guardian_recovery_link_guard").await?;
        let seeded=app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO public.carer_relationships(household_id,carer_id,patient_id,guardian_declaration_id,relationship_type,active,created_at,updated_at) VALUES(72994,73001,$1,$2,'family_member',true,now(),now())",
            [subject.into(),declaration_id.into()])).await;
        app.fixture.admin.execute_unprepared("ALTER TABLE public.carer_relationships ENABLE TRIGGER guardian_recovery_link_guard").await?;
        seeded?;
        let tenant=access::begin(&app.fixture.runtime,&scope(),None).await.map_err(db_error)?;
        let options=recovery::options(&tenant).await.map_err(db_error)?;
        let snapshot=effective_snapshot(&tenant,subject).await?;
        require(!options.subjects.iter().any(|item|item.id.get()==subject),
            "A healthy foreign relationship must count despite local inactivity")?;
        let attempted=recovery::execute(&tenant,RecoveryCommand {
            subject:RecoverySubjectId::parse(subject).map_err(db_error)?,
            guardian:RecoveryGuardianId::parse(73001).map_err(db_error)?,
            expected:snapshot,
            action:RecoveryAction::Appoint,
            reason:"Attempted appointment with foreign authority.".into(),
        }).await;
        require(matches!(attempted,Err(med_tracker::models::errors::OperationError::Conflict{ref code,..}) if code=="guardian_already_effective"),
            "A globally effective foreign guardian must block local appointment")?;
        tenant.rollback().await.map_err(db_error)?;
        app.fixture.admin.execute_unprepared(
            "INSERT INTO public.people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73996,'Synthetic lockout appointee','1980-01-01',0,true,now(),now());
             INSERT INTO public.accounts(id,person_id,email,status,created_at,updated_at) VALUES(71996,73996,'lockout-appointee@example.test',2,now(),now());
             INSERT INTO public.users(id,person_id,email_address,active,created_at,updated_at) VALUES(77996,73996,'lockout-appointee@example.test',true,now(),now());
             INSERT INTO public.household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74996,72001,73996,'member','active',now(),now(),now());
             INSERT INTO public.people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73998,'Synthetic lockout manager','1980-01-01',0,true,now(),now());
             INSERT INTO public.accounts(id,person_id,email,status,created_at,updated_at) VALUES(71998,73998,'lockout-manager@example.test',2,now(),now());
             INSERT INTO public.users(id,person_id,email_address,active,created_at,updated_at) VALUES(77998,73998,'lockout-manager@example.test',true,now(),now());
             INSERT INTO public.household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74998,72001,73998,'administrator','active',now(),now(),now());
             INSERT INTO public.account_lockouts(account_id,key,deadline) VALUES(71001,'synthetic-foreign-guardian-lock',now()+interval '1 hour')"
        ).await?;
        let manager_scope=HouseholdScope {
            actor:Actor {account_id:71998},
            household_id:72001,
            request_id:"synthetic-foreign-lockout-recovery".into(),
        };
        let tenant=access::begin(&app.fixture.runtime,&manager_scope,None).await.map_err(db_error)?;
        let options=recovery::options(&tenant).await.map_err(db_error)?;
        let snapshot=effective_snapshot(&tenant,subject).await?;
        require(!options.subjects.iter().any(|item|item.id.get()==subject),
            "Temporary foreign guardian login lockout must not create orphanhood")?;
        let attempted=recovery::execute(&tenant,RecoveryCommand {
            subject:RecoverySubjectId::parse(subject).map_err(db_error)?,
            guardian:RecoveryGuardianId::parse(73996).map_err(db_error)?,
            expected:snapshot,
            action:RecoveryAction::Appoint,
            reason:"Lockout must not permit a replacement appointment.".into(),
        }).await;
        require(matches!(attempted,Err(med_tracker::models::errors::OperationError::Conflict{ref code,..}) if code=="guardian_already_effective"),
            "Temporary foreign login lockout must block a distinct adult appointment")?;
        tenant.rollback().await.map_err(db_error)?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_new_guardian_records_manager_and_remains_withdrawable()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let result=async {
        let token=app.token().await;
        let (subject,old_declaration)=declaration(&app,&token).await?;
        app.fixture.admin.execute_unprepared(
            "INSERT INTO public.people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73995,'Synthetic new guardian','1980-01-01',0,true,now(),now());
             INSERT INTO public.accounts(id,person_id,email,status,created_at,updated_at) VALUES(71995,73995,'new-guardian-recovery@example.test',2,now(),now());
             INSERT INTO public.users(id,person_id,email_address,active,created_at,updated_at) VALUES(77995,73995,'new-guardian-recovery@example.test',true,now(),now());
             INSERT INTO public.household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74995,72001,73995,'member','active',now(),now(),now())"
        ).await?;
        app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE public.carer_relationships SET active=false WHERE guardian_declaration_id=$1 AND household_id=72001",
            [old_declaration.into()])).await?;
        let tenant=access::begin(&app.fixture.runtime,&scope(),None).await.map_err(db_error)?;
        let options=recovery::options(&tenant).await.map_err(db_error)?;
        let selected=options.subjects.iter().find(|item|item.id.get()==subject)
            .ok_or_else(||DbErr::Custom("Minor missing for new appointee".into()))?;
        require(selected.restorable.is_some() && options.guardians.iter().any(|item|item.id.get()==73995),
            "Old link can restore, while a distinct adult may be deliberately appointed")?;
        let appointed=recovery::execute(&tenant,RecoveryCommand {
            subject:RecoverySubjectId::parse(subject).map_err(db_error)?,
            guardian:RecoveryGuardianId::parse(73995).map_err(db_error)?,
            expected:selected.snapshot,
            action:RecoveryAction::Appoint,
            reason:"Confirmed the new adult accepts this guardianship.".into(),
        }).await.map_err(|error|DbErr::Custom(format!("new appointment: {error:?}")))?;
        require(appointed.outcome==RecoveryOutcome::Appointed,"Distinct adult must be appointed")?;
        tenant.commit().await.map_err(db_error)?;
        let state=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM public.guardian_declarations WHERE id=$1 AND source='manager_appointment' AND declared_by_account_id=71001 AND guardian_person_id=73995 AND statement_version=2) AS appointed,(SELECT count(*) FROM public.guardian_declarations WHERE id=$2 AND superseded_at IS NOT NULL AND superseded_by_account_id=71001) AS superseded,(SELECT count(*) FROM public.security_audit_events WHERE event_type='guardian_recovery.appointed' AND actor_account_id=71001 AND metadata->>'guardian_person_id'='73995') AS audited",
            [appointed.declaration_id.into(),old_declaration.into()])).await?
            .ok_or_else(||DbErr::Custom("Missing manager provenance".into()))?;
        require(state.try_get::<i64>("","appointed")?==1 && state.try_get::<i64>("","superseded")?==1
            && state.try_get::<i64>("","audited")?==1,
            "New authority must credit manager, supersede old source and audit actual appointee")?;
        crate::fixtures::seed_native_password(&app.fixture.admin,71995).await;
        let guardian_token=foreign_token(&app,71995).await?;
        let own=app.client.get(format!("{}/api/v1/auth/guardian-declarations",app.origin))
            .bearer_auth(&guardian_token).send().await.map_err(db_error)?;
        require(own.status().as_u16()==200,"New guardian can read personal history")?;
        let body:Value=own.json().await.map_err(db_error)?;
        let expected_id=appointed.declaration_id.to_string();
        require(body["data"].as_array().is_some_and(|rows|rows.iter().any(|row|
            row["id"]==expected_id && row["source"]=="manager_appointment")),
            "Own history must expose manager appointment source")?;
        let withdraw=app.client.patch(format!("{}/api/v1/auth/guardian-declarations/{}",app.origin,appointed.declaration_id))
            .bearer_auth(&guardian_token)
            .json(&json!({"guardian_declaration":{"withdrawn":true,"revision":"1"}}))
            .send().await.map_err(db_error)?;
        require(withdraw.status().as_u16()==200,"New guardian must retain personal withdrawal")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_superseded_active_link_remains_history_without_authority()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        use med_tracker::models::_entities::{
            accounts, carer_relationships, household_memberships, people, users,
        };
        use sea_orm::{ActiveValue::Set, EntityTrait};
        use sea_orm::sea_query::{Alias, Expr, ExprTrait, JoinType, Query};
        let now=chrono::Utc::now().naive_utc();
        people::Entity::insert_many([
            people::ActiveModel { id:Set(73994), name:Set("Historical guardian subject".into()),
                date_of_birth:Set(chrono::NaiveDate::from_ymd_opt(2020,1,1)),
                person_type:Set(1),has_capacity:Set(false),created_at:Set(now),updated_at:Set(now),
                ..Default::default() },
            people::ActiveModel { id:Set(73995), name:Set("Historical old guardian".into()),
                date_of_birth:Set(chrono::NaiveDate::from_ymd_opt(1980,1,1)),
                person_type:Set(0),has_capacity:Set(true),created_at:Set(now),updated_at:Set(now),
                ..Default::default() },
            people::ActiveModel { id:Set(73996), name:Set("Historical new guardian".into()),
                date_of_birth:Set(chrono::NaiveDate::from_ymd_opt(1980,1,1)),
                person_type:Set(0),has_capacity:Set(true),created_at:Set(now),updated_at:Set(now),
                ..Default::default() },
        ]).exec(&app.fixture.admin).await?;
        household_memberships::Entity::insert_many(
            [ (74994,73994), (74995,73995), (74996,73996) ].map(|(id,person_id)|
                household_memberships::ActiveModel { id:Set(id),household_id:Set(72001),
                    person_id:Set(person_id),role:Set("member".into()),status:Set("active".into()),
                    permissions_version:Set(1),joined_at:Set(now),created_at:Set(now),
                    updated_at:Set(now),..Default::default() })
        ).exec(&app.fixture.admin).await?;
        accounts::Entity::insert_many([
            (71995,73995,"historical-old@example.test"),
            (71996,73996,"historical-new@example.test"),
        ].map(|(id,person_id,email)| accounts::ActiveModel {
            id:Set(id),person_id:Set(person_id),email:Set(email.into()),status:Set(2),
            created_at:Set(now.and_utc()),updated_at:Set(now.and_utc()),..Default::default()
        })).exec(&app.fixture.admin).await?;
        users::Entity::insert_many([
            (77995,73995,"historical-old@example.test"),
            (77996,73996,"historical-new@example.test"),
        ].map(|(id,person_id,email)| users::ActiveModel {
            id:Set(id),person_id:Set(person_id),email_address:Set(email.into()),active:Set(true),
            created_at:Set(now),updated_at:Set(now)
        })).exec(&app.fixture.admin).await?;
        let old_scope = HouseholdScope { actor: Actor { account_id: 71995 },
            household_id: 72001, request_id: "synthetic-old-guardian-authority".into() };
        let old = access::begin(&app.fixture.runtime,&old_scope,None).await.map_err(db_error)?;
        let declaration=Query::insert()
            .into_table((Alias::new("public"),Alias::new("guardian_declarations")))
            .columns(["subject_person_id","guardian_person_id","origin_household_id",
                "declared_by_account_id","subject_name_at_declaration"])
            .values_panic([Expr::val(73994_i64),Expr::val(73995_i64),Expr::val(72001_i64),
                Expr::val(71995_i64),Expr::val("Historical guardian subject")])
            .returning_col("id")
            .to_owned();
        let row=old.transaction().query_one_raw(
            old.transaction().get_database_backend().build(&declaration)).await?
            .ok_or_else(||DbErr::Custom("Missing historical declaration".into()))?;
        let old_declaration:i64=row.try_get("","id")?;
        carer_relationships::Entity::insert(carer_relationships::ActiveModel {
            household_id:Set(72001),carer_id:Set(73995),patient_id:Set(73994),
            guardian_declaration_id:Set(Some(old_declaration)),
            relationship_type:Set(Some("family_member".into())),active:Set(true),
            created_at:Set(now),updated_at:Set(now),..Default::default()
        }).exec(old.transaction()).await?;
        require(access::require_person_access(&old,73994,PersonAccess::Manage).await.is_ok(),
            "Initial active link must grant old guardian effective authority before recovery")?;
        old.commit().await.map_err(db_error)?;
        let disable=Query::update().table(accounts::Entity)
            .value(accounts::Column::Status,1)
            .and_where(Expr::col(accounts::Column::Id).eq(71995_i64)).to_owned();
        app.fixture.admin.execute_raw(
            app.fixture.admin.get_database_backend().build(&disable)).await?;
        let tenant=access::begin(&app.fixture.runtime,&scope(),None).await.map_err(db_error)?;
        let selected=recovery::options(&tenant).await.map_err(db_error)?.subjects.into_iter()
            .find(|item|item.id.get()==73994)
            .ok_or_else(||DbErr::Custom("Historical orphan missing from options".into()))?;
        require(!selected.has_effective_guardian && selected.restorable.is_none(),
            "Unusable former guardian must leave an orphan but no restorable local link")?;
        let appointed=recovery::execute(&tenant,RecoveryCommand {
            subject:RecoverySubjectId::parse(73994).map_err(db_error)?,
            guardian:RecoveryGuardianId::parse(73996).map_err(db_error)?,
            expected:selected.snapshot,action:RecoveryAction::Appoint,
            reason:"Manager appointed another adult while old guardian unusable.".into(),
        }).await.map_err(db_error)?;
        tenant.commit().await.map_err(db_error)?;
        let historical=Query::select()
            .expr_as(Expr::col(("d","superseded_at")).is_not_null(),"terminal")
            .expr_as(Expr::col(("c","active")),"historical_link_active")
            .from_as((Alias::new("public"),Alias::new("guardian_declarations")),"d")
            .join_as(JoinType::InnerJoin,
                (Alias::new("public"),Alias::new("carer_relationships")),"c",
                Expr::col(("c","guardian_declaration_id")).equals(("d","id")))
            .and_where(Expr::col(("d","subject_person_id")).eq(73994_i64))
            .and_where(Expr::col(("d","guardian_person_id")).eq(73995_i64))
            .to_owned();
        let row=app.fixture.admin.query_one_raw(
            app.fixture.admin.get_database_backend().build(&historical)).await?
            .ok_or_else(||DbErr::Custom("Missing historical link".into()))?;
        require(row.try_get::<bool>("","terminal")? && row.try_get::<bool>("","historical_link_active")?
            && appointed.outcome==RecoveryOutcome::Appointed,
            "Superseded declaration and its retained active relationship must remain truthful history")?;
        let enable=Query::update().table(accounts::Entity)
            .value(accounts::Column::Status,2)
            .and_where(Expr::col(accounts::Column::Id).eq(71995_i64)).to_owned();
        app.fixture.admin.execute_raw(
            app.fixture.admin.get_database_backend().build(&enable)).await?;
        let old=access::begin(&app.fixture.runtime,&old_scope,None).await.map_err(db_error)?;
        require(matches!(access::require_person_access(&old,73994,PersonAccess::Manage).await,
            Err(med_tracker::models::errors::OperationError::Forbidden)),
            "Restored old account must not regain authority from retained superseded link")?;
        old.rollback().await.map_err(db_error)?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_stale_confirmation_and_audit_failure_leave_history_unchanged()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let result=async {
        let token=app.token().await;
        let (subject,old_id)=declaration(&app,&token).await?;
        app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE public.carer_relationships SET active=false WHERE guardian_declaration_id=$1",
            [old_id.into()])).await?;
        let tenant=access::begin(&app.fixture.runtime,&scope(),None).await.map_err(db_error)?;
        let selected=recovery::options(&tenant).await.map_err(db_error)?.subjects.into_iter()
            .find(|item|item.id.get()==subject)
            .ok_or_else(||DbErr::Custom("Missing stale-test subject".into()))?;
        tenant.rollback().await.map_err(db_error)?;
        let withdrawn=app.client.patch(format!("{}/api/v1/auth/guardian-declarations/{old_id}",app.origin))
            .bearer_auth(&token)
            .json(&json!({"guardian_declaration":{"withdrawn":true,"revision":"1"}}))
            .send().await.map_err(db_error)?;
        require(withdrawn.status().as_u16()==200,"Personal withdrawal changes confirmation snapshot")?;
        let tenant=access::begin(&app.fixture.runtime,&scope(),None).await.map_err(db_error)?;
        let stale=recovery::execute(&tenant,RecoveryCommand {
            subject:RecoverySubjectId::parse(subject).map_err(db_error)?,
            guardian:RecoveryGuardianId::parse(73001).map_err(db_error)?,
            expected:selected.snapshot,
            action:RecoveryAction::Appoint,
            reason:"A stale confirmation must be rejected.".into(),
        }).await;
        require(matches!(stale,Err(med_tracker::models::errors::OperationError::Conflict{ref code,..}) if code=="stale_guardian_recovery"),
            "Terminal transition must invalidate prior confirmation")?;
        tenant.rollback().await.map_err(db_error)?;
        app.fixture.admin.execute_unprepared(
            "CREATE FUNCTION public.synthetic_reject_recovery_audit() RETURNS trigger LANGUAGE plpgsql AS $body$ BEGIN IF NEW.event_type LIKE 'guardian_recovery.%' THEN RAISE EXCEPTION USING ERRCODE='23514',MESSAGE='synthetic audit outage'; END IF; RETURN NEW; END $body$;
             CREATE TRIGGER synthetic_reject_recovery_audit BEFORE INSERT ON public.security_audit_events FOR EACH ROW EXECUTE FUNCTION public.synthetic_reject_recovery_audit()"
        ).await?;
        let tenant=access::begin(&app.fixture.runtime,&scope(),None).await.map_err(db_error)?;
        let latest=recovery::options(&tenant).await.map_err(db_error)?.subjects.into_iter()
            .find(|item|item.id.get()==subject)
            .ok_or_else(||DbErr::Custom("Missing latest subject".into()))?;
        require(latest.snapshot!=selected.snapshot,"Withdrawn row must rotate opaque confirmation")?;
        let failed=recovery::execute(&tenant,RecoveryCommand {
            subject:RecoverySubjectId::parse(subject).map_err(db_error)?,
            guardian:RecoveryGuardianId::parse(73001).map_err(db_error)?,
            expected:latest.snapshot,
            action:RecoveryAction::Appoint,
            reason:"Auditing must commit with the appointment.".into(),
        }).await;
        require(failed.is_err(),"Audit failure must reject recovery")?;
        tenant.rollback().await.map_err(db_error)?;
        let state=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM public.guardian_declarations WHERE subject_person_id=$1) AS history,(SELECT count(*) FROM public.guardian_declarations WHERE id=$2 AND withdrawn_at IS NOT NULL AND superseded_at IS NULL) AS terminal,(SELECT count(*) FROM public.carer_relationships WHERE patient_id=$1 AND active) AS active,(SELECT count(*) FROM public.versions WHERE item_type='GuardianDeclaration' AND event='appointed' AND item_id IN (SELECT id FROM public.guardian_declarations WHERE subject_person_id=$1)) AS versions",
            [subject.into(),old_id.into()])).await?.ok_or_else(||DbErr::Custom("Missing rollback state".into()))?;
        require(state.try_get::<i64>("","history")?==1 && state.try_get::<i64>("","terminal")?==1
            && state.try_get::<i64>("","active")?==0 && state.try_get::<i64>("","versions")?==0,
            "Audit rejection must rollback declaration, link and versions")?;
        Ok(())
    }.await;
    app.close().await;
    result
}
