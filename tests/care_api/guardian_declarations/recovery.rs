use super::{Application, require, storage_boundary::foreign_token, withdrawal::declaration};
use med_tracker::models::{
    access::{self, Actor, HouseholdScope},
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
             INSERT INTO public.users(id,person_id,email_address,password_digest,active,created_at,updated_at) VALUES
               (77995,73995,'candidate-lock@example.test',crypt('password',gen_salt('bf',4)),true,now(),now()),
               (77999,73999,'recovery-admin@example.test',crypt('password',gen_salt('bf',4)),true,now(),now());
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
        app.fixture.admin.execute_unprepared("UPDATE public.accounts SET status=2 WHERE id=71001").await?;
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
        let selected=options.subjects.iter().find(|item|item.id.get()==subject)
            .ok_or_else(||DbErr::Custom("Current minor missing near guardian birthday".into()))?;
        require(selected.has_effective_guardian,
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
             INSERT INTO public.users(id,person_id,email_address,password_digest,active,created_at,updated_at) VALUES(77997,73997,'ordinary-recovery@example.test',crypt('password',gen_salt('bf',4)),true,now(),now());
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
            && selected.restorable.is_some_and(|link|link.declaration_id==declaration_id && link.relationship_id==link_id),
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
        let selected=options.subjects.iter().find(|item|item.id.get()==subject)
            .ok_or_else(||DbErr::Custom("Foreign guarded minor missing".into()))?;
        require(selected.has_effective_guardian,
            "A healthy foreign relationship must count despite local inactivity")?;
        let attempted=recovery::execute(&tenant,RecoveryCommand {
            subject:RecoverySubjectId::parse(subject).map_err(db_error)?,
            guardian:RecoveryGuardianId::parse(73001).map_err(db_error)?,
            expected:selected.snapshot,
            action:RecoveryAction::Appoint,
            reason:"Attempted appointment with foreign authority.".into(),
        }).await;
        require(matches!(attempted,Err(med_tracker::models::errors::OperationError::Conflict{ref code,..}) if code=="guardian_already_effective"),
            "A globally effective foreign guardian must block local appointment")?;
        tenant.rollback().await.map_err(db_error)?;
        app.fixture.admin.execute_unprepared(
            "INSERT INTO public.people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73996,'Synthetic lockout appointee','1980-01-01',0,true,now(),now());
             INSERT INTO public.accounts(id,person_id,email,status,created_at,updated_at) VALUES(71996,73996,'lockout-appointee@example.test',2,now(),now());
             INSERT INTO public.users(id,person_id,email_address,password_digest,active,created_at,updated_at) VALUES(77996,73996,'lockout-appointee@example.test',crypt('password',gen_salt('bf',4)),true,now(),now());
             INSERT INTO public.household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74996,72001,73996,'member','active',now(),now(),now());
             INSERT INTO public.people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73998,'Synthetic lockout manager','1980-01-01',0,true,now(),now());
             INSERT INTO public.accounts(id,person_id,email,status,created_at,updated_at) VALUES(71998,73998,'lockout-manager@example.test',2,now(),now());
             INSERT INTO public.users(id,person_id,email_address,password_digest,active,created_at,updated_at) VALUES(77998,73998,'lockout-manager@example.test',crypt('password',gen_salt('bf',4)),true,now(),now());
             INSERT INTO public.household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74998,72001,73998,'administrator','active',now(),now(),now());
             INSERT INTO public.account_lockouts(account_id,key,deadline) VALUES(71001,'synthetic-foreign-guardian-lock',now()+interval '1 hour')"
        ).await?;
        let manager_scope=HouseholdScope {
            actor:Actor {account_id:71998},
            household_id:72001,
            request_id:"synthetic-foreign-lockout-recovery".into(),
        };
        let tenant=access::begin(&app.fixture.runtime,&manager_scope,None).await.map_err(db_error)?;
        let selected=recovery::options(&tenant).await.map_err(db_error)?.subjects.into_iter()
            .find(|item|item.id.get()==subject)
            .ok_or_else(||DbErr::Custom("Foreign guardian missing after lockout".into()))?;
        require(selected.has_effective_guardian,
            "Temporary foreign guardian login lockout must not create orphanhood")?;
        let attempted=recovery::execute(&tenant,RecoveryCommand {
            subject:RecoverySubjectId::parse(subject).map_err(db_error)?,
            guardian:RecoveryGuardianId::parse(73996).map_err(db_error)?,
            expected:selected.snapshot,
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
             INSERT INTO public.users(id,person_id,email_address,password_digest,active,created_at,updated_at) VALUES(77995,73995,'new-guardian-recovery@example.test',crypt('password',gen_salt('bf',4)),true,now(),now());
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
