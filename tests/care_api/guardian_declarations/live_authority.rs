use super::super::platform::{form_page, hidden_field, post_form, sign_in};
use super::{Application, effects, require, withdrawal::declaration};
use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement, TransactionTrait};
use serde_json::{Value, json};
use std::time::Duration;

fn schedule_body(subject: i64) -> Value {
    json!({"schedule":{"person_id":subject.to_string(),"medication_id":"80001","dose_amount":"2","dose_unit":"tablet","start_date":"2026-01-01","end_date":"2027-01-01","schedule_type":"prn","schedule_config":{},"dose_cycle":"daily"}})
}

async fn schedule(
    app: &Application,
    token: &str,
    subject: i64,
) -> Result<reqwest::Response, DbErr> {
    app.client
        .post(format!("{}/api/v1/households/72001/schedules", app.origin))
        .bearer_auth(token)
        .json(&schedule_body(subject))
        .send()
        .await
        .map_err(|_| DbErr::Custom("Live guardian plan request failed before response".into()))
}

async fn manageable(app: &Application, token: &str, subject: i64) -> Result<bool, DbErr> {
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/ui_capabilities",
            app.origin
        ))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|_| {
            DbErr::Custom("Live guardian capability request failed before response".into())
        })?;
    require(
        response.status().as_u16() == 200,
        "Live authority capability read must remain available",
    )?;
    let body: Value = response
        .json()
        .await
        .map_err(|_| DbErr::Custom("Capability response must be JSON".into()))?;
    let ids = body["data"]["people"]["manage_ids"]
        .as_array()
        .ok_or_else(|| DbErr::Custom("Capability response must expose Manage IDs".into()))?;
    Ok(ids.contains(&json!(subject)))
}

async fn authority(
    app: &Application,
    token: &str,
    subject: i64,
    expected: bool,
) -> Result<(), DbErr> {
    require(
        manageable(app, token, subject).await? == expected,
        "Aggregate authority must follow current guardian eligibility",
    )?;
    let before = effects(app).await?;
    let response = schedule(app, token, subject).await?;
    require(
        response.status().as_u16() == if expected { 201 } else { 403 },
        &format!(
            "Direct plan authority must match aggregate eligibility; received{}",
            response.status().as_u16()
        ),
    )?;
    if !expected {
        require(
            effects(app).await? == before,
            "Denied plan writes must preserve clinical, version and queue state",
        )?;
    }
    Ok(())
}

async fn legacy_fixture(app: &Application) -> Result<(), DbErr> {
    app.fixture.admin.execute_unprepared(
        "UPDATE public.accounts SET preferences=jsonb_set(COALESCE(preferences,'{}'::jsonb),'{time_zone}','\"UTC\"') WHERE id=71001;
         INSERT INTO public.person_access_grants(id,household_id,household_membership_id,person_id,access_level,relationship_type,created_at,updated_at) VALUES(78991,72001,74001,73003,'manage','family_member',now(),now())"
    ).await?;
    Ok(())
}

async fn unchanged_alternatives(app: &Application, token: &str) -> Result<(), DbErr> {
    authority(app, token, 73001, true).await?;
    authority(app, token, 73003, true).await
}

async fn declaration_state(app: &Application, id: i64) -> Result<Value, DbErr> {
    app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT jsonb_build_object('withdrawn',d.withdrawn_at IS NOT NULL,'revision',d.revision,'local_active',c.active,'withdrawals',(SELECT count(*) FROM public.versions v WHERE v.item_type='GuardianDeclaration' AND v.item_id=d.id AND v.event='withdraw'),'subject_grants',(SELECT count(*) FROM public.person_access_grants g WHERE g.person_id=d.subject_person_id)) AS state FROM public.guardian_declarations d JOIN public.carer_relationships c ON c.guardian_declaration_id=d.id AND c.household_id=72001 WHERE d.id=$1",
        [id.into()])).await?.ok_or_else(||DbErr::Custom("Expected committed guardian provenance".into()))?.try_get("","state")
}

async fn withdraw(app: &Application, token: &str, id: i64) -> Result<reqwest::Response, DbErr> {
    app.client
        .patch(format!(
            "{}/api/v1/auth/guardian-declarations/{id}",
            app.origin
        ))
        .bearer_auth(token)
        .json(&json!({"guardian_declaration":{"withdrawn":true,"revision":"1"}}))
        .send()
        .await
        .map_err(|_| DbErr::Custom("Live guardian withdrawal failed before response".into()))
}

#[tokio::test]
async fn guardian_authority_stops_at_eighteenth_birthday_without_retyping() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result=async {
        let token=app.token().await;
        legacy_fixture(&app).await?;
        let (subject,id)=declaration(&app,&token).await?;
        app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE public.people SET date_of_birth=(timezone('UTC',clock_timestamp())::date-interval '18 years'+interval '1 day')::date WHERE id=$1",[subject.into()])).await?;
        authority(&app,&token,subject,true).await?;
        unchanged_alternatives(&app,&token).await?;
        let before=declaration_state(&app,id).await?;
        app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE public.people SET date_of_birth=(timezone('UTC',clock_timestamp())::date-interval '18 years')::date WHERE id=$1",[subject.into()])).await?;
        authority(&app,&token,subject,false).await?;
        unchanged_alternatives(&app,&token).await?;
        require(declaration_state(&app,id).await?==before,"Birthday must not withdraw or manufacture declaration authority")?;
        let row=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT person_type,(SELECT count(*) FROM public.accounts WHERE person_id=$1) AS accounts FROM public.people WHERE id=$1",[subject.into()])).await?
            .ok_or_else(||DbErr::Custom("Birthday subject must remain present".into()))?;
        require(row.try_get::<i32>("","person_type")?==1 && row.try_get::<i64>("","accounts")?==0,
            "Eighteenth birthday must not silently retype or create an Account")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_capacity_loss_stops_derived_authority_only() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let token = app.token().await;
        legacy_fixture(&app).await?;
        let (subject, id) = declaration(&app, &token).await?;
        authority(&app, &token, subject, true).await?;
        unchanged_alternatives(&app, &token).await?;
        let before = declaration_state(&app, id).await?;
        app.fixture
            .admin
            .execute_unprepared("UPDATE public.people SET has_capacity=false WHERE id=73001")
            .await?;
        authority(&app, &token, subject, false).await?;
        unchanged_alternatives(&app, &token).await?;
        require(
            declaration_state(&app, id).await? == before,
            "Capacity loss must not rewrite or withdraw personal provenance",
        )?;
        Ok(())
    }
    .await;
    app.close().await;
    result
}

#[tokio::test]
async fn household_manager_removes_local_link_without_withdrawing_personal_declaration()
-> Result<(), DbErr> {
    let mut app = Application::new().await;
    let result=async {
        app.client=reqwest::Client::builder().timeout(Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none()).build()
            .map_err(|_|DbErr::Custom("Owned no-redirect browser client unavailable".into()))?;
        let token=app.token().await;
        legacy_fixture(&app).await?;
        let (subject,id)=declaration(&app,&token).await?;
        authority(&app,&token,subject,true).await?;
        app.fixture.admin.execute_unprepared(
            "INSERT INTO public.people(id,name,person_type,has_capacity,created_at,updated_at) VALUES(73992,'Synthetic local manager',0,true,now(),now());
             INSERT INTO public.accounts(id,person_id,email,password_hash,status,created_at,updated_at) VALUES(71992,73992,'local-guardian-manager@example.test',crypt('password',gen_salt('bf',4)),2,now(),now());
             INSERT INTO public.users(id,person_id,email_address,password_digest,active,created_at,updated_at) VALUES(77992,73992,'local-guardian-manager@example.test',crypt('password',gen_salt('bf',4)),true,now(),now());
             INSERT INTO public.household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74992,72001,73992,'administrator','active',now(),now(),now())"
        ).await?;
        let cookie=sign_in(&app,71992,"local-guardian-manager@example.test").await;
        let (cookies,page)=form_page(&app,&cookie,&format!("/households/persistence-fixture/people/{subject}/carer_relationships")).await;
        let csrf=hidden_field(&page,"authenticity_token");
        let version=hidden_field(&page,"version");
        require(!csrf.is_empty() && !version.is_empty(),"Local manager must reach the actual protected removal form")?;
        let link=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT id FROM public.carer_relationships WHERE guardian_declaration_id=$1 AND household_id=72001",[id.into()])).await?
            .ok_or_else(||DbErr::Custom("Expected active local declaration link".into()))?.try_get::<i64>("","id")?;
        let removed=post_form(&app,&cookies,&format!("/households/persistence-fixture/people/{subject}/carer_relationships/{link}/remove"),
            &[("authenticity_token",csrf.as_str()),("version",version.as_str()),("locale","en")]).await;
        require(removed.status().as_u16()==303,"A different live household manager must be able to remove the local care link")?;
        authority(&app,&token,subject,false).await?;
        unchanged_alternatives(&app,&token).await?;
        let state=declaration_state(&app,id).await?;
        require(state["local_active"]==false && state["withdrawn"]==false && state["revision"]==1
            && state["withdrawals"]==0 && state["subject_grants"]==0,
            "Manager removal must end only the local link, not personal declaration or revive legacy authority")?;
        let own=app.client.get(format!("{}/api/v1/auth/guardian-declarations",app.origin)).bearer_auth(&token).send().await
            .map_err(|_|DbErr::Custom("Own declaration refresh failed before response".into()))?;
        require(own.status().as_u16()==200,"Guardian must retain own declaration read after local removal")?;
        let own:Value=own.json().await.map_err(|_|DbErr::Custom("Own declaration refresh must be JSON".into()))?;
        let expected_id=id.to_string();
        require(own["data"].as_array().is_some_and(|rows|rows.iter().any(|row|row["id"].as_str()==Some(expected_id.as_str()) && row["withdrawn_at"].is_null())),
            "Personal declaration must remain Recorded after household-local removal")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn committed_withdrawal_denies_subsequent_plan_without_effects() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let token = app.token().await;
        legacy_fixture(&app).await?;
        let (subject, id) = declaration(&app, &token).await?;
        authority(&app, &token, subject, true).await?;
        let withdrawn = withdraw(&app, &token, id).await?;
        require(
            withdrawn.status().as_u16() == 200,
            "Guardian withdrawal must commit before the next plan request",
        )?;
        let committed = declaration_state(&app, id).await?;
        require(
            committed["withdrawn"] == true
                && committed["revision"] == 2
                && committed["withdrawals"] == 1,
            "Withdrawal-first scenario must reach its persisted terminal event",
        )?;
        authority(&app, &token, subject, false).await?;
        require(
            declaration_state(&app, id).await? == committed,
            "Denied subsequent plan must preserve the committed withdrawal",
        )?;
        unchanged_alternatives(&app, &token).await?;
        Ok(())
    }
    .await;
    app.close().await;
    result
}

struct PendingRequest(tokio::task::JoinHandle<Result<reqwest::Response, reqwest::Error>>);

impl PendingRequest {
    async fn finish(&mut self) -> Result<reqwest::Response, DbErr> {
        tokio::time::timeout(Duration::from_secs(5), &mut self.0)
            .await
            .map_err(|_| {
                DbErr::Custom(
                    "Owned concurrent request exceeded its bounded completion window".into(),
                )
            })?
            .map_err(|_| DbErr::Custom("Owned concurrent request task did not complete".into()))?
            .map_err(|_| DbErr::Custom("Owned concurrent request failed before response".into()))
    }
}

impl Drop for PendingRequest {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn blocked_waiter(app: &Application, blocker: i32) -> Result<i32, DbErr> {
    for _ in 0..100 {
        let row=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT pid FROM pg_stat_activity WHERE datname=current_database() AND $1::int=ANY(pg_blocking_pids(pid)) ORDER BY pid LIMIT 1",
            [blocker.into()])).await?;
        if let Some(row) = row {
            return row.try_get("", "pid");
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    Err(DbErr::Custom(
        "Expected deterministic PostgreSQL blocking witness was not reached".into(),
    ))
}

async fn second_native_token(app: &Application) -> Result<String, DbErr> {
    app.fixture.admin.execute_unprepared(
        "INSERT INTO public.oauth_grants(id,account_id,oauth_application_id,client_kind,code,code_challenge,code_challenge_method,redirect_uri,scopes,expires_in,authenticated_at,last_used_at,created_at,updated_at) VALUES(76993,71001,75001,'mobile','synthetic-guardian-race-code','E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM','S256','io.damacus.medtracker:/oauth2redirect','medtracker offline_access',now()+interval '5 minutes',now(),now(),now(),now())"
    ).await?;
    let encoded = serde_urlencoded::to_string([
        ("grant_type", "authorization_code"),
        ("client_id", "native"),
        ("code", "synthetic-guardian-race-code"),
        ("redirect_uri", "io.damacus.medtracker:/oauth2redirect"),
        (
            "code_verifier",
            "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk",
        ),
    ])
    .map_err(|_| DbErr::Custom("Second native exchange encoding failed".into()))?;
    let response = app
        .client
        .post(format!("{}/token", app.origin))
        .header("content-type", "application/x-www-form-urlencoded")
        .body(encoded)
        .send()
        .await
        .map_err(|_| DbErr::Custom("Second native exchange failed before response".into()))?;
    require(
        response.status().as_u16() == 200,
        "Race must use two genuinely issued distinct native credentials",
    )?;
    let response: Value = response
        .json()
        .await
        .map_err(|_| DbErr::Custom("Second native exchange must return JSON".into()))?;
    response["access_token"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| DbErr::Custom("Second native access token missing".into()))
}

#[tokio::test]
async fn plan_declaration_share_blocks_withdrawal_until_plan_and_audit_commit() -> Result<(), DbErr>
{
    let app = Application::new().await;
    let result=async {
        let token=app.token().await;
        legacy_fixture(&app).await?;
        let (subject,id)=declaration(&app,&token).await?;
        authority(&app,&token,subject,true).await?;
        unchanged_alternatives(&app,&token).await?;
        let withdrawal_token=second_native_token(&app).await?;
        let before=effects(&app).await?;
        app.fixture.admin.execute_unprepared(
            "CREATE FUNCTION public.synthetic_guardian_plan_audit_gate() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='Schedule' AND NEW.event='create' THEN PERFORM pg_advisory_xact_lock(1746879441,93); END IF; RETURN NEW; END $$;
             CREATE TRIGGER synthetic_guardian_plan_audit_gate BEFORE INSERT ON public.versions FOR EACH ROW EXECUTE FUNCTION public.synthetic_guardian_plan_audit_gate()"
        ).await?;
        let gate=app.fixture.admin.begin().await?;
        let gate_pid=gate.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT pg_backend_pid() AS pid"))
            .await?.ok_or_else(||DbErr::Custom("Owned gate backend PID missing".into()))?.try_get::<i32>("","pid")?;
        gate.execute_unprepared("SELECT pg_advisory_xact_lock(1746879441,93)").await?;
        let client=app.client.clone();
        let endpoint=format!("{}/api/v1/households/72001/schedules",app.origin);
        let plan_token=token.clone();
        let body=schedule_body(subject);
        let mut plan=PendingRequest(tokio::spawn(async move {client.post(endpoint).bearer_auth(plan_token).json(&body).send().await}));
        let plan_pid=blocked_waiter(&app,gate_pid).await?;
        let client=app.client.clone();
        let endpoint=format!("{}/api/v1/auth/guardian-declarations/{id}",app.origin);
        let mut withdrawal=PendingRequest(tokio::spawn(async move {client.patch(endpoint).bearer_auth(withdrawal_token)
            .json(&json!({"guardian_declaration":{"withdrawn":true,"revision":"1"}})).send().await}));
        let withdrawal_pid=blocked_waiter(&app,plan_pid).await?;
        let locks=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT EXISTS(SELECT 1 FROM pg_locks WHERE pid=$1 AND relation='public.guardian_declarations'::regclass AND mode='RowShareLock' AND granted) AS declaration_reached,
              EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='public.security_audit_events'::regclass AND confrelid='public.accounts'::regclass AND contype='f') AS audit_account_fk",
            [withdrawal_pid.into()])).await?.ok_or_else(||DbErr::Custom("Expected declaration/audit lock facts".into()))?;
        require(locks.try_get::<bool>("","declaration_reached")? && locks.try_get::<bool>("","audit_account_fk")?,
            "Independent withdrawal must reach declaration locking while the plan retains real Account audit FK semantics")?;
        require(!plan.0.is_finished() && !withdrawal.0.is_finished(),"Both actual requests must be pending before releasing the plan audit")?;
        require(declaration_state(&app,id).await?["withdrawn"]==false,"Blocked withdrawal must not become visible before plan commit")?;
        gate.commit().await?;
        let saved=plan.finish().await?;
        require(saved.status().as_u16()==201,"Plan holding declaration SHARE must finish with its audit before withdrawal")?;
        let plan_request=saved.headers().get("x-request-id").and_then(|value|value.to_str().ok())
            .filter(|value|!value.is_empty()).ok_or_else(||DbErr::Custom("Committed plan request ID missing".into()))?.to_owned();
        let saved:Value=saved.json().await.map_err(|_|DbErr::Custom("Concurrent plan result must be JSON".into()))?;
        let plan_id=saved["data"]["id"].as_i64().ok_or_else(||DbErr::Custom("Concurrent plan ID missing".into()))?;
        let withdrawn=withdrawal.finish().await?;
        require(withdrawn.status().as_u16()==200,"Waiting withdrawal must commit after the authorised plan and audit")?;
        let audit=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM public.versions WHERE item_type='Schedule' AND item_id=$1 AND event='create') AS plan_versions,(SELECT count(*) FROM public.audit_ledger_entries l JOIN public.versions v ON l.source_table='versions' AND l.source_id=v.id WHERE v.item_type='Schedule' AND v.item_id=$1 AND v.event='create') AS plan_ledger,(SELECT count(*) FROM public.schedules WHERE id=$1 AND person_id=$2) AS plans,(SELECT count(*) FROM public.security_audit_events WHERE request_id=$3 AND actor_account_id=71001 AND event_type='api.request' AND metadata->>'status'='201') AS account_fk_audits",
            [plan_id.into(),subject.into(),plan_request.into()])).await?.ok_or_else(||DbErr::Custom("Concurrent plan audit facts missing".into()))?;
        require(audit.try_get::<i64>("","plans")?==1 && audit.try_get::<i64>("","plan_versions")?==1 && audit.try_get::<i64>("","plan_ledger")?==1
            && audit.try_get::<i64>("","account_fk_audits")?==1,
            "The authorised plan, version/ledger and real Account-FK request audit must remain committed after withdrawal")?;
        let state=declaration_state(&app,id).await?;
        require(state["withdrawn"]==true && state["withdrawals"]==1,"Concurrent withdrawal must commit exactly one terminal event")?;
        require(effects(&app).await?["people"]==before["people"],"Concurrent authority transition must not manufacture identities")?;
        authority(&app,&token,subject,false).await?;
        Ok(())
    }.await;
    app.close().await;
    result
}
